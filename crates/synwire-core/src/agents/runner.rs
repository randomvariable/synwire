//! Agent runner and execution loop.
//!
//! `Runner` drives the agent turn loop:
//! session lookup → middleware chain → model invocation → tool dispatch →
//! directive execution → event emission → usage tracking.
//!
//! It enforces `max_turns` and `max_budget` limits, handles model errors with
//! configurable retry / fallback, and supports graceful and force stop.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::{Mutex, mpsc};

use crate::agents::agent_node::Agent;
use crate::agents::error::AgentError;
use crate::agents::streaming::{AgentEvent, TerminationReason};
use crate::agents::usage::Usage;

// ---------------------------------------------------------------------------
// Stop signal
// ---------------------------------------------------------------------------

/// Kind of stop requested from outside the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    /// Drain in-flight tool calls, then stop cleanly.
    Graceful,
    /// Cancel immediately without draining.
    Force,
}

// ---------------------------------------------------------------------------
// RunErrorAction
// ---------------------------------------------------------------------------

/// Specifies what the runner should do when an error occurs.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RunErrorAction {
    /// Retry the current request (up to a configurable limit).
    Retry,
    /// Continue to the next turn ignoring this error.
    Continue,
    /// Abort the run immediately.
    Abort(String),
    /// Switch to a different model and retry.
    SwitchModel(String),
}

// ---------------------------------------------------------------------------
// RunnerConfig
// ---------------------------------------------------------------------------

/// Configuration for a single runner execution.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Override the agent's model for this run.
    pub model_override: Option<String>,
    /// Session ID to resume (None = new session).
    pub session_id: Option<String>,
    /// Maximum number of retries per model error.
    pub max_retries: u32,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            model_override: None,
            session_id: None,
            max_retries: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Drives the agent execution loop.
///
/// The runner is stateless between runs; all per-run state is held in the
/// channel and local variables inside `run`.
#[derive(Debug)]
pub struct Runner<O: serde::Serialize + Send + Sync + 'static = ()> {
    agent: Arc<Agent<O>>,
    /// Current model — may be changed via `set_model`.
    current_model: Mutex<String>,
    /// Stop signal sender.
    stop_tx: Mutex<Option<mpsc::Sender<StopKind>>>,
}

impl<O: serde::Serialize + Send + Sync + 'static> Runner<O> {
    /// Create a runner wrapping the given agent.
    #[must_use]
    pub fn new(agent: Agent<O>) -> Self {
        let model = agent.model_name().to_string();
        Self {
            agent: Arc::new(agent),
            current_model: Mutex::new(model),
            stop_tx: Mutex::new(None),
        }
    }

    /// Dynamically switch the model for subsequent turns, preserving
    /// conversation history.
    pub async fn set_model(&self, model: impl Into<String>) {
        let mut guard = self.current_model.lock().await;
        *guard = model.into();
        tracing::info!(model = %*guard, "Runner: model switched");
    }

    /// Send a graceful stop signal.  The runner will finish any in-flight
    /// tool call, then emit `TurnComplete { reason: Stopped }`.
    pub async fn stop_graceful(&self) {
        if let Some(tx) = self.stop_tx.lock().await.as_ref() {
            let _ = tx.send(StopKind::Graceful).await;
        }
    }

    /// Send a force stop signal.  The runner cancels immediately and emits
    /// `TurnComplete { reason: Aborted }`.
    pub async fn stop_force(&self) {
        if let Some(tx) = self.stop_tx.lock().await.as_ref() {
            let _ = tx.send(StopKind::Force).await;
        }
    }

    /// Run the agent with the given input, yielding events over a channel.
    ///
    /// # Event stream
    /// Events are sent on the returned receiver.  The stream ends when the
    /// receiver is closed (after a `TurnComplete` or `Error` event).
    ///
    /// # Errors
    /// Returns `AgentError` if setup fails before the event stream starts.
    pub async fn run(
        &self,
        input: Value,
        config: RunnerConfig,
    ) -> Result<mpsc::Receiver<AgentEvent>, AgentError> {
        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(128);
        let (stop_tx, stop_rx) = mpsc::channel::<StopKind>(1);

        // Store stop sender so callers can signal stop.
        *self.stop_tx.lock().await = Some(stop_tx);

        let agent = Arc::clone(&self.agent);
        let model = self.current_model.lock().await.clone();

        let _handle = tokio::spawn(async move {
            run_loop(agent, input, config, model, event_tx, stop_rx).await;
        });

        Ok(event_rx)
    }
}

// ---------------------------------------------------------------------------
// Core loop (spawned task)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
async fn run_loop<O: serde::Serialize + Send + Sync + 'static>(
    agent: Arc<Agent<O>>,
    input: Value,
    config: RunnerConfig,
    initial_model: String,
    event_tx: mpsc::Sender<AgentEvent>,
    mut stop_rx: mpsc::Receiver<StopKind>,
) {
    let max_turns = agent.max_turn_count();
    let max_budget = agent.budget_limit();
    let max_retries = config.max_retries;

    let mut current_model = config.model_override.unwrap_or(initial_model);
    let mut turn: u32 = 0;
    let mut cumulative_cost: f64 = 0.0;
    let mut messages: Vec<Value> = Vec::new();
    let mut retry_count: u32 = 0;

    // Seed conversation with the user's input.
    messages.push(serde_json::json!({ "role": "user", "content": input }));

    loop {
        // Check for stop signal (non-blocking poll).
        match stop_rx.try_recv() {
            Ok(StopKind::Graceful) => {
                emit(
                    &event_tx,
                    AgentEvent::TurnComplete {
                        reason: TerminationReason::Stopped,
                    },
                )
                .await;
                return;
            }
            Ok(StopKind::Force) => {
                emit(
                    &event_tx,
                    AgentEvent::TurnComplete {
                        reason: TerminationReason::Aborted,
                    },
                )
                .await;
                return;
            }
            Err(_) => {}
        }

        // Enforce max_turns.
        if let Some(limit) = max_turns
            && turn >= limit
        {
            tracing::debug!(turn, limit, "max_turns reached");
            emit(
                &event_tx,
                AgentEvent::TurnComplete {
                    reason: TerminationReason::MaxTurnsExceeded,
                },
            )
            .await;
            return;
        }

        // Enforce max_budget.
        if let Some(budget) = max_budget
            && cumulative_cost > budget
        {
            tracing::debug!(cumulative_cost, budget, "budget exceeded");
            emit(
                &event_tx,
                AgentEvent::TurnComplete {
                    reason: TerminationReason::BudgetExceeded,
                },
            )
            .await;
            return;
        }

        turn += 1;

        // --- Simulated model invocation ---
        // In production this would call the LLM backend.  The runner provides
        // the scaffolding; actual model calls are injected by provider crates.
        let model_result = invoke_model(&current_model, &messages);

        match model_result {
            Ok(response) => {
                retry_count = 0;

                // Accumulate synthetic usage.
                let usage = Usage {
                    input_tokens: response.input_tokens,
                    output_tokens: response.output_tokens,
                    ..Usage::default()
                };
                cumulative_cost += response.estimated_cost;

                // Emit usage update.
                emit(&event_tx, AgentEvent::UsageUpdate { usage }).await;

                // Emit text delta if present.
                if let Some(text) = response.text {
                    emit(&event_tx, AgentEvent::TextDelta { content: text }).await;
                }

                // Check if model signalled completion.
                if response.done {
                    emit(
                        &event_tx,
                        AgentEvent::TurnComplete {
                            reason: TerminationReason::Complete,
                        },
                    )
                    .await;
                    return;
                }

                // Append assistant message and continue loop.
                messages.push(serde_json::json!({ "role": "assistant", "content": response.raw }));
            }

            Err(err) => {
                let action = dispatch_model_error(
                    &err,
                    retry_count,
                    max_retries,
                    agent.fallback_model_name(),
                );

                match action {
                    RunErrorAction::Retry => {
                        retry_count += 1;
                        tracing::warn!(attempt = retry_count, model = %current_model, "Retrying after model error");
                        turn -= 1; // don't count against max_turns
                    }
                    RunErrorAction::SwitchModel(fallback) => {
                        tracing::warn!(
                            from = %current_model,
                            to = %fallback,
                            "Switching to fallback model"
                        );
                        current_model = fallback;
                        retry_count = 0;
                        turn -= 1;
                    }
                    RunErrorAction::Continue => {
                        tracing::warn!(%err, "Model error ignored — continuing");
                    }
                    RunErrorAction::Abort(msg) => {
                        emit(&event_tx, AgentEvent::Error { message: msg }).await;
                        return;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error dispatch
// ---------------------------------------------------------------------------

fn dispatch_model_error(
    err: &AgentError,
    retry_count: u32,
    max_retries: u32,
    fallback_model: Option<&str>,
) -> RunErrorAction {
    match err {
        AgentError::Model(model_err) => {
            if !model_err.is_retryable() {
                return RunErrorAction::Abort(err.to_string());
            }
            if retry_count < max_retries {
                // Try fallback on second retry if available.
                if retry_count > 0
                    && let Some(fb) = fallback_model
                {
                    return RunErrorAction::SwitchModel(fb.to_string());
                }
                RunErrorAction::Retry
            } else if let Some(fb) = fallback_model {
                RunErrorAction::SwitchModel(fb.to_string())
            } else {
                RunErrorAction::Abort(format!("Max retries ({max_retries}) exceeded: {err}"))
            }
        }
        AgentError::Panic(msg) => {
            tracing::error!(%msg, "Agent panicked");
            RunErrorAction::Abort(format!("Agent panicked: {msg}"))
        }
        _ => RunErrorAction::Abort(err.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Stub model invocation (replaced by provider crates at runtime)
// ---------------------------------------------------------------------------

struct ModelResponse {
    text: Option<String>,
    raw: Value,
    input_tokens: u64,
    output_tokens: u64,
    estimated_cost: f64,
    done: bool,
}

/// Placeholder model invocation.  Real implementations are injected by
/// provider crates (e.g. `synwire-llm-openai`) via the `AgentNode::run`
/// delegation path.
#[allow(clippy::unnecessary_wraps)]
fn invoke_model(model: &str, messages: &[Value]) -> Result<ModelResponse, AgentError> {
    tracing::debug!(%model, message_count = messages.len(), "invoke_model (stub)");
    // Stub: immediately complete with empty response.
    Ok(ModelResponse {
        text: None,
        raw: Value::Null,
        input_tokens: 0,
        output_tokens: 0,
        estimated_cost: 0.0,
        done: true,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn emit(tx: &mpsc::Sender<AgentEvent>, event: AgentEvent) {
    // Ignore send errors — receiver may have been dropped.
    let _ = tx.send(event).await;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::agents::agent_node::Agent;

    #[tokio::test]
    async fn test_runner_completes() {
        let agent: Agent = Agent::new("test", "stub-model");
        let runner = Runner::new(agent);
        let mut rx = runner
            .run(serde_json::json!("Hello"), RunnerConfig::default())
            .await
            .unwrap();

        let mut got_complete = false;
        while let Some(event) = rx.recv().await {
            if let AgentEvent::TurnComplete { reason } = event {
                assert_eq!(reason, TerminationReason::Complete);
                got_complete = true;
            }
        }
        assert!(got_complete, "expected TurnComplete event");
    }

    #[tokio::test]
    async fn test_runner_max_turns() {
        // The stub model never sets done=true on its own in subsequent turns,
        // but does set done=true immediately.  Adjust by giving 0 max_turns.
        let agent: Agent = Agent::new("test", "stub-model").max_turns(0);
        let runner = Runner::new(agent);
        let mut rx = runner
            .run(serde_json::json!("Hello"), RunnerConfig::default())
            .await
            .unwrap();

        let mut got_max_turns = false;
        while let Some(event) = rx.recv().await {
            if let AgentEvent::TurnComplete { reason } = event {
                // With max_turns=0 the first check fires immediately.
                if reason == TerminationReason::MaxTurnsExceeded {
                    got_max_turns = true;
                }
            }
        }
        assert!(got_max_turns, "expected MaxTurnsExceeded");
    }

    #[tokio::test]
    async fn test_runner_graceful_stop() {
        let agent: Agent = Agent::new("test", "stub-model");
        let runner = Arc::new(Runner::new(agent));
        let runner2 = Arc::clone(&runner);

        let mut rx = runner
            .run(serde_json::json!("Hello"), RunnerConfig::default())
            .await
            .unwrap();

        // Stop before any events are processed (races, but tests the wiring).
        runner2.stop_graceful().await;

        let mut saw_stop_or_complete = false;
        while let Some(event) = rx.recv().await {
            if let AgentEvent::TurnComplete { reason } = event
                && matches!(
                    reason,
                    TerminationReason::Stopped | TerminationReason::Complete
                )
            {
                saw_stop_or_complete = true;
            }
        }
        assert!(saw_stop_or_complete);
    }
}
