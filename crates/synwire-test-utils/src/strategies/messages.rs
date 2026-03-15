//! Proptest strategies for [`Message`] and related types.

use std::collections::HashMap;

use proptest::prelude::*;
use serde_json::Value;
use synwire_core::messages::{
    ContentBlock, InvalidToolCall, Message, MessageContent, ToolCall, ToolStatus, UsageMetadata,
};

/// Strategy for generating arbitrary [`MessageContent`].
pub fn arb_message_content() -> impl Strategy<Value = MessageContent> {
    prop_oneof![
        ".*".prop_map(MessageContent::Text),
        prop::collection::vec(arb_content_block(), 1..=4).prop_map(MessageContent::Blocks),
    ]
}

/// Strategy for generating arbitrary [`ContentBlock`].
pub fn arb_content_block() -> impl Strategy<Value = ContentBlock> {
    prop_oneof![
        ".*".prop_map(|text| ContentBlock::Text { text }),
        "https?://.*".prop_map(|url| ContentBlock::Image { url, detail: None }),
        ".*".prop_map(|text| ContentBlock::Reasoning { text }),
        ".*".prop_map(|text| ContentBlock::Thinking { text }),
    ]
}

/// Strategy for generating arbitrary [`ToolCall`].
pub fn arb_tool_call() -> impl Strategy<Value = ToolCall> {
    ("[a-z_]{1,16}", "[a-z_]{1,16}", arb_json_map()).prop_map(|(id, name, arguments)| ToolCall {
        id,
        name,
        arguments,
    })
}

/// Strategy for generating arbitrary [`InvalidToolCall`].
pub fn arb_invalid_tool_call() -> impl Strategy<Value = InvalidToolCall> {
    (
        proptest::option::of("[a-z_]{1,16}"),
        proptest::option::of(".*"),
        proptest::option::of("[a-z_]{1,16}"),
        ".*",
    )
        .prop_map(|(name, arguments, id, error)| InvalidToolCall {
            name,
            arguments,
            id,
            error,
        })
}

/// Strategy for generating arbitrary [`UsageMetadata`].
pub fn arb_usage_metadata() -> impl Strategy<Value = UsageMetadata> {
    (0u64..10_000, 0u64..10_000).prop_map(|(input, output)| UsageMetadata {
        input_tokens: input,
        output_tokens: output,
        total_tokens: input + output,
        input_token_details: None,
        output_token_details: None,
    })
}

/// Strategy for generating arbitrary [`ToolStatus`].
pub fn arb_tool_status() -> impl Strategy<Value = ToolStatus> {
    prop_oneof![Just(ToolStatus::Success), Just(ToolStatus::Error),]
}

/// Strategy for generating a simple JSON key-value map.
pub fn arb_json_map() -> impl Strategy<Value = HashMap<String, Value>> {
    prop::collection::hash_map("[a-z]{1,8}", arb_json_value(), 0..=3)
}

/// Strategy for generating simple JSON values.
pub fn arb_json_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        ".*".prop_map(Value::String),
        any::<i64>().prop_map(|n| Value::Number(n.into())),
        any::<bool>().prop_map(Value::Bool),
        Just(Value::Null),
    ]
}

/// Strategy for generating arbitrary [`Message`] across all variants.
pub fn arb_message() -> impl Strategy<Value = Message> {
    prop_oneof![
        arb_human_message(),
        arb_ai_message(),
        arb_system_message(),
        arb_tool_message(),
        arb_chat_message(),
    ]
}

/// Strategy for generating a human [`Message`].
pub fn arb_human_message() -> impl Strategy<Value = Message> {
    (
        proptest::option::of("[a-z0-9_]{1,16}"),
        proptest::option::of("[a-z]{1,16}"),
        arb_message_content(),
        arb_json_map(),
    )
        .prop_map(|(id, name, content, additional_kwargs)| Message::Human {
            id,
            name,
            content,
            additional_kwargs,
        })
}

/// Strategy for generating an AI [`Message`].
pub fn arb_ai_message() -> impl Strategy<Value = Message> {
    (
        proptest::option::of("[a-z0-9_]{1,16}"),
        proptest::option::of("[a-z]{1,16}"),
        arb_message_content(),
        prop::collection::vec(arb_tool_call(), 0..=2),
        proptest::option::of(arb_usage_metadata()),
        arb_json_map(),
    )
        .prop_map(
            |(id, name, content, tool_calls, usage, additional_kwargs)| Message::AI {
                id,
                name,
                content,
                tool_calls,
                invalid_tool_calls: Vec::new(),
                usage,
                response_metadata: None,
                additional_kwargs,
            },
        )
}

/// Strategy for generating a system [`Message`].
pub fn arb_system_message() -> impl Strategy<Value = Message> {
    (
        proptest::option::of("[a-z0-9_]{1,16}"),
        proptest::option::of("[a-z]{1,16}"),
        arb_message_content(),
        arb_json_map(),
    )
        .prop_map(|(id, name, content, additional_kwargs)| Message::System {
            id,
            name,
            content,
            additional_kwargs,
        })
}

/// Strategy for generating a tool [`Message`].
pub fn arb_tool_message() -> impl Strategy<Value = Message> {
    (
        proptest::option::of("[a-z0-9_]{1,16}"),
        proptest::option::of("[a-z]{1,16}"),
        arb_message_content(),
        "[a-z0-9_]{1,16}",
        arb_tool_status(),
        arb_json_map(),
    )
        .prop_map(
            |(id, name, content, tool_call_id, status, additional_kwargs)| Message::Tool {
                id,
                name,
                content,
                tool_call_id,
                status,
                artifact: None,
                additional_kwargs,
            },
        )
}

/// Strategy for generating a chat [`Message`] with custom role.
pub fn arb_chat_message() -> impl Strategy<Value = Message> {
    (
        proptest::option::of("[a-z0-9_]{1,16}"),
        proptest::option::of("[a-z]{1,16}"),
        "[a-z]{1,16}",
        arb_message_content(),
        arb_json_map(),
    )
        .prop_map(
            |(id, name, role, content, additional_kwargs)| Message::Chat {
                id,
                name,
                role,
                content,
                additional_kwargs,
            },
        )
}
