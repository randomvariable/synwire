//! Lifecycle hooks for agent execution.
//!
//! Hooks are short-lived async callbacks invoked at well-defined points in the
//! agent lifecycle.  Each hook type carries its own payload.  Hooks are matched
//! by an optional tool-name glob pattern and executed with an enforced timeout;
//! hooks that exceed their timeout are skipped with a warning rather than
//! failing the agent.

use std::sync::Arc;

use serde_json::Value;
use tokio::time::{Duration, timeout};

use crate::BoxFuture;
use crate::agents::error::AgentError;

// ---------------------------------------------------------------------------
// Payload types
// ---------------------------------------------------------------------------

/// Context passed to pre-tool-use hooks.
#[derive(Debug, Clone)]
pub struct PreToolUseContext {
    /// Tool name.
    pub tool_name: String,
    /// Tool arguments.
    pub arguments: Value,
}

/// Context passed to post-tool-use (success) hooks.
#[derive(Debug, Clone)]
pub struct PostToolUseContext {
    /// Tool name.
    pub tool_name: String,
    /// Tool arguments.
    pub arguments: Value,
    /// Tool output.
    pub output: Value,
}

/// Context passed to post-tool-use (failure) hooks.
#[derive(Debug, Clone)]
pub struct PostToolUseFailureContext {
    /// Tool name.
    pub tool_name: String,
    /// Tool arguments.
    pub arguments: Value,
    /// Error message.
    pub error: String,
}

/// Context passed to notification hooks.
#[derive(Debug, Clone)]
pub struct NotificationContext {
    /// Notification message.
    pub message: String,
    /// Severity level.
    pub level: String,
}

/// Context passed to subagent start hooks.
#[derive(Debug, Clone)]
pub struct SubagentStartContext {
    /// Subagent name.
    pub agent_name: String,
    /// Initial message.
    pub initial_message: Option<String>,
}

/// Context passed to subagent stop hooks.
#[derive(Debug, Clone)]
pub struct SubagentStopContext {
    /// Subagent name.
    pub agent_name: String,
    /// Termination reason.
    pub reason: String,
}

/// Context passed to pre-compact hooks.
#[derive(Debug, Clone)]
pub struct PreCompactContext {
    /// Message count before compaction.
    pub message_count: usize,
    /// Token count estimate.
    pub token_count: u64,
}

/// Context passed to post-compact hooks.
#[derive(Debug, Clone)]
pub struct PostCompactContext {
    /// Message count after compaction.
    pub message_count: usize,
    /// Token count estimate after compaction.
    pub token_count: u64,
}

/// Context passed to session start hooks.
#[derive(Debug, Clone)]
pub struct SessionStartContext {
    /// Session ID.
    pub session_id: String,
    /// Whether this is a resumed session.
    pub resumed: bool,
}

/// Context passed to session end hooks.
#[derive(Debug, Clone)]
pub struct SessionEndContext {
    /// Session ID.
    pub session_id: String,
    /// Termination reason.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Hook result
// ---------------------------------------------------------------------------

/// Result returned by a hook invocation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HookResult {
    /// Continue normal execution.
    Continue,
    /// Abort the current operation with a message.
    Abort(String),
}

// ---------------------------------------------------------------------------
// Hook matcher
// ---------------------------------------------------------------------------

/// Matcher that selects which events a hook applies to.
#[derive(Debug, Clone)]
pub struct HookMatcher {
    /// Optional tool name glob (e.g. `"read_file"`, `"*_file"`).
    /// `None` matches all.
    pub tool_name_pattern: Option<String>,
    /// Timeout for this hook invocation.
    pub timeout: Duration,
}

impl Default for HookMatcher {
    fn default() -> Self {
        Self {
            tool_name_pattern: None,
            timeout: Duration::from_secs(30),
        }
    }
}

impl HookMatcher {
    /// Returns `true` if the matcher applies to the given tool name.
    #[must_use]
    pub fn matches_tool(&self, tool_name: &str) -> bool {
        self.tool_name_pattern
            .as_ref()
            .is_none_or(|pattern| glob_match(pattern, tool_name))
    }
}

/// Simple glob: `*` matches any sequence of characters.
fn glob_match(pattern: &str, input: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == input;
    }
    let mut remaining = input;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if let Some(pos) = remaining.find(part) {
            remaining = &remaining[pos + part.len()..];
        } else {
            return false;
        }
    }
    if !pattern.ends_with('*') && !remaining.is_empty() {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Type-erased hook function wrappers
// ---------------------------------------------------------------------------

type PreToolUseFn = Arc<dyn Fn(PreToolUseContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type PostToolUseFn =
    Arc<dyn Fn(PostToolUseContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type PostToolUseFailureFn =
    Arc<dyn Fn(PostToolUseFailureContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type NotificationFn =
    Arc<dyn Fn(NotificationContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type SubagentStartFn =
    Arc<dyn Fn(SubagentStartContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type SubagentStopFn =
    Arc<dyn Fn(SubagentStopContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type PreCompactFn = Arc<dyn Fn(PreCompactContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type PostCompactFn =
    Arc<dyn Fn(PostCompactContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type SessionStartFn =
    Arc<dyn Fn(SessionStartContext) -> BoxFuture<'static, HookResult> + Send + Sync>;
type SessionEndFn = Arc<dyn Fn(SessionEndContext) -> BoxFuture<'static, HookResult> + Send + Sync>;

enum HookEntry {
    PreToolUse(HookMatcher, PreToolUseFn),
    PostToolUse(HookMatcher, PostToolUseFn),
    PostToolUseFailure(HookMatcher, PostToolUseFailureFn),
    Notification(HookMatcher, NotificationFn),
    SubagentStart(HookMatcher, SubagentStartFn),
    SubagentStop(HookMatcher, SubagentStopFn),
    PreCompact(HookMatcher, PreCompactFn),
    PostCompact(HookMatcher, PostCompactFn),
    SessionStart(HookMatcher, SessionStartFn),
    SessionEnd(HookMatcher, SessionEndFn),
}

impl std::fmt::Debug for HookEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreToolUse(m, _) => write!(f, "PreToolUse({m:?})"),
            Self::PostToolUse(m, _) => write!(f, "PostToolUse({m:?})"),
            Self::PostToolUseFailure(m, _) => write!(f, "PostToolUseFailure({m:?})"),
            Self::Notification(m, _) => write!(f, "Notification({m:?})"),
            Self::SubagentStart(m, _) => write!(f, "SubagentStart({m:?})"),
            Self::SubagentStop(m, _) => write!(f, "SubagentStop({m:?})"),
            Self::PreCompact(m, _) => write!(f, "PreCompact({m:?})"),
            Self::PostCompact(m, _) => write!(f, "PostCompact({m:?})"),
            Self::SessionStart(m, _) => write!(f, "SessionStart({m:?})"),
            Self::SessionEnd(m, _) => write!(f, "SessionEnd({m:?})"),
        }
    }
}

// ---------------------------------------------------------------------------
// HookRegistry
// ---------------------------------------------------------------------------

/// Registry of lifecycle hooks with typed registration and timeout enforcement.
#[derive(Debug, Default)]
pub struct HookRegistry {
    hooks: Vec<HookEntry>,
}

impl HookRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // --- Registration ---

    /// Register a pre-tool-use hook.
    pub fn on_pre_tool_use<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(PreToolUseContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks.push(HookEntry::PreToolUse(matcher, Arc::new(f)));
    }

    /// Register a post-tool-use hook.
    pub fn on_post_tool_use<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(PostToolUseContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::PostToolUse(matcher, Arc::new(f)));
    }

    /// Register a post-tool-use failure hook.
    pub fn on_post_tool_use_failure<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(PostToolUseFailureContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::PostToolUseFailure(matcher, Arc::new(f)));
    }

    /// Register a notification hook.
    pub fn on_notification<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(NotificationContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::Notification(matcher, Arc::new(f)));
    }

    /// Register a subagent start hook.
    pub fn on_subagent_start<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(SubagentStartContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::SubagentStart(matcher, Arc::new(f)));
    }

    /// Register a subagent stop hook.
    pub fn on_subagent_stop<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(SubagentStopContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::SubagentStop(matcher, Arc::new(f)));
    }

    /// Register a pre-compact hook.
    pub fn on_pre_compact<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(PreCompactContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks.push(HookEntry::PreCompact(matcher, Arc::new(f)));
    }

    /// Register a post-compact hook.
    pub fn on_post_compact<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(PostCompactContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::PostCompact(matcher, Arc::new(f)));
    }

    /// Register a session start hook.
    pub fn on_session_start<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(SessionStartContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks
            .push(HookEntry::SessionStart(matcher, Arc::new(f)));
    }

    /// Register a session end hook.
    pub fn on_session_end<F>(&mut self, matcher: HookMatcher, f: F)
    where
        F: Fn(SessionEndContext) -> BoxFuture<'static, HookResult> + Send + Sync + 'static,
    {
        self.hooks.push(HookEntry::SessionEnd(matcher, Arc::new(f)));
    }

    // --- Invocation ---

    /// Run all matching pre-tool-use hooks in registration order.
    ///
    /// Returns the first `Abort` result encountered, or `Continue` if all pass.
    /// Hooks that exceed their timeout are skipped with a `warn!` log.
    pub async fn run_pre_tool_use(&self, ctx: PreToolUseContext) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::PreToolUse(matcher, f) = entry {
                if !matcher.matches_tool(&ctx.tool_name) {
                    continue;
                }
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all matching post-tool-use hooks.
    pub async fn run_post_tool_use(
        &self,
        ctx: PostToolUseContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::PostToolUse(matcher, f) = entry {
                if !matcher.matches_tool(&ctx.tool_name) {
                    continue;
                }
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all matching post-tool-use failure hooks.
    pub async fn run_post_tool_use_failure(
        &self,
        ctx: PostToolUseFailureContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::PostToolUseFailure(matcher, f) = entry {
                if !matcher.matches_tool(&ctx.tool_name) {
                    continue;
                }
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all notification hooks.
    pub async fn run_notification(
        &self,
        ctx: NotificationContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::Notification(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all session start hooks.
    pub async fn run_session_start(
        &self,
        ctx: SessionStartContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::SessionStart(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all session end hooks.
    pub async fn run_session_end(&self, ctx: SessionEndContext) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::SessionEnd(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all subagent start hooks.
    pub async fn run_subagent_start(
        &self,
        ctx: SubagentStartContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::SubagentStart(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all subagent stop hooks.
    pub async fn run_subagent_stop(
        &self,
        ctx: SubagentStopContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::SubagentStop(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all pre-compact hooks.
    pub async fn run_pre_compact(&self, ctx: PreCompactContext) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::PreCompact(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }

    /// Run all post-compact hooks.
    pub async fn run_post_compact(
        &self,
        ctx: PostCompactContext,
    ) -> Result<HookResult, AgentError> {
        for entry in &self.hooks {
            if let HookEntry::PostCompact(matcher, f) = entry {
                if let HookResult::Abort(msg) =
                    run_with_timeout(f(ctx.clone()), matcher.timeout).await
                {
                    return Ok(HookResult::Abort(msg));
                }
            }
        }
        Ok(HookResult::Continue)
    }
}

/// Run a hook future with a timeout; return `Continue` on timeout with a warning.
async fn run_with_timeout(fut: BoxFuture<'static, HookResult>, duration: Duration) -> HookResult {
    match timeout(duration, fut).await {
        Ok(result) => result,
        Err(_elapsed) => {
            tracing::warn!(?duration, "Hook timed out — skipping");
            HookResult::Continue
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pre_tool_use_abort() {
        let mut registry = HookRegistry::new();
        registry.on_pre_tool_use(HookMatcher::default(), |_ctx| {
            Box::pin(async { HookResult::Abort("blocked".to_string()) })
        });
        let ctx = PreToolUseContext {
            tool_name: "read_file".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = registry.run_pre_tool_use(ctx).await.unwrap();
        assert!(matches!(result, HookResult::Abort(_)));
    }

    #[tokio::test]
    async fn test_tool_name_pattern_no_match() {
        let mut registry = HookRegistry::new();
        registry.on_pre_tool_use(
            HookMatcher {
                tool_name_pattern: Some("write_*".to_string()),
                timeout: Duration::from_secs(5),
            },
            |_ctx| Box::pin(async { HookResult::Abort("blocked".to_string()) }),
        );
        let ctx = PreToolUseContext {
            tool_name: "read_file".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = registry.run_pre_tool_use(ctx).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_timeout_skips_hook() {
        let mut registry = HookRegistry::new();
        registry.on_pre_tool_use(
            HookMatcher {
                tool_name_pattern: None,
                timeout: Duration::from_millis(10),
            },
            |_ctx| {
                Box::pin(async {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    HookResult::Abort("late abort".to_string())
                })
            },
        );
        let ctx = PreToolUseContext {
            tool_name: "read_file".to_string(),
            arguments: serde_json::json!({}),
        };
        // Hook times out — must not abort.
        let result = registry.run_pre_tool_use(ctx).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("write_*", "write_file"));
        assert!(!glob_match("write_*", "read_file"));
        assert!(glob_match("*_file", "read_file"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "not_exact"));
    }
}
