//! Proptest strategies for agent core types.

use proptest::prelude::*;
use serde_json::{Value, json};
use synwire_core::agents::directive::Directive;
use synwire_core::agents::streaming::{AgentEvent, TerminationReason};
use synwire_core::agents::usage::Usage;
use synwire_core::vfs::grep_options::{GrepOptions, GrepOutputMode};

// ---------------------------------------------------------------------------
// Directive
// ---------------------------------------------------------------------------

/// Strategy for generating arbitrary [`Directive`] variants.
pub fn arb_directive() -> impl Strategy<Value = Directive> {
    prop_oneof![
        arb_emit_directive(),
        arb_stop_directive(),
        arb_spawn_task_directive(),
        arb_stop_task_directive(),
    ]
}

fn arb_emit_directive() -> impl Strategy<Value = Directive> {
    arb_agent_event().prop_map(|event| Directive::Emit { event })
}

fn arb_stop_directive() -> impl Strategy<Value = Directive> {
    proptest::option::of(".{0,100}").prop_map(|reason| Directive::Stop { reason })
}

fn arb_spawn_task_directive() -> impl Strategy<Value = Directive> {
    ("[a-z_]{1,20}", arb_simple_json())
        .prop_map(|(description, input)| Directive::SpawnTask { description, input })
}

fn arb_stop_task_directive() -> impl Strategy<Value = Directive> {
    "[a-z0-9-]{1,36}".prop_map(|task_id| Directive::StopTask { task_id })
}

// ---------------------------------------------------------------------------
// AgentEvent
// ---------------------------------------------------------------------------

/// Strategy for generating arbitrary [`AgentEvent`] variants.
pub fn arb_agent_event() -> impl Strategy<Value = AgentEvent> {
    prop_oneof![
        arb_text_delta(),
        arb_usage_update(),
        arb_status_update(),
        arb_turn_complete(),
        arb_error_event(),
    ]
}

fn arb_text_delta() -> impl Strategy<Value = AgentEvent> {
    ".{0,200}".prop_map(|content| AgentEvent::TextDelta { content })
}

fn arb_usage_update() -> impl Strategy<Value = AgentEvent> {
    (any::<u64>(), any::<u64>()).prop_map(|(input, output)| AgentEvent::UsageUpdate {
        usage: Usage {
            input_tokens: input,
            output_tokens: output,
            ..Usage::default()
        },
    })
}

fn arb_status_update() -> impl Strategy<Value = AgentEvent> {
    (".{1,80}", proptest::option::of(0.0_f32..=1.0_f32)).prop_map(|(status, pct)| {
        AgentEvent::StatusUpdate {
            status,
            progress_pct: pct,
        }
    })
}

fn arb_turn_complete() -> impl Strategy<Value = AgentEvent> {
    prop_oneof![
        Just(TerminationReason::Complete),
        Just(TerminationReason::MaxTurnsExceeded),
        Just(TerminationReason::BudgetExceeded),
        Just(TerminationReason::Stopped),
        Just(TerminationReason::Aborted),
        Just(TerminationReason::Error),
    ]
    .prop_map(|reason| AgentEvent::TurnComplete { reason })
}

fn arb_error_event() -> impl Strategy<Value = AgentEvent> {
    ".{1,200}".prop_map(|message| AgentEvent::Error { message })
}

// ---------------------------------------------------------------------------
// GrepOptions
// ---------------------------------------------------------------------------

/// Strategy for generating arbitrary [`GrepOptions`].
pub fn arb_grep_options() -> impl Strategy<Value = GrepOptions> {
    (any::<bool>(), any::<bool>(), 0_u32..=10_u32, 0_u32..=10_u32).prop_map(
        |(case_insensitive, invert, before_context, after_context)| GrepOptions {
            case_insensitive,
            invert,
            before_context,
            after_context,
            line_numbers: true,
            output_mode: GrepOutputMode::Content,
            ..GrepOptions::default()
        },
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn arb_simple_json() -> impl Strategy<Value = Value> {
    prop_oneof![
        ".*".prop_map(Value::String),
        any::<i64>().prop_map(|n| json!(n)),
        any::<bool>().prop_map(Value::Bool),
    ]
}
