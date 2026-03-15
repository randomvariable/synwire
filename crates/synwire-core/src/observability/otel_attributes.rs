//! OpenTelemetry `GenAI` semantic convention attribute constants.
//!
//! These follow the [`OTel` `GenAI` Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/).

/// `OTel` `GenAI` attribute key constants.
///
/// # Example
///
/// ```
/// use synwire_core::observability::gen_ai;
///
/// assert_eq!(gen_ai::OPERATION_NAME, "gen_ai.operation.name");
/// ```
pub mod gen_ai {
    /// The name of the operation being performed (e.g. `chat`, `text_completion`).
    pub const OPERATION_NAME: &str = "gen_ai.operation.name";

    /// The name of the `GenAI` provider (e.g. `openai`, `anthropic`).
    pub const PROVIDER_NAME: &str = "gen_ai.provider.name";

    /// The model name as requested.
    pub const REQUEST_MODEL: &str = "gen_ai.request.model";

    /// The temperature parameter of the request.
    pub const REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";

    /// The maximum number of tokens requested.
    pub const REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";

    /// The model name as returned in the response.
    pub const RESPONSE_MODEL: &str = "gen_ai.response.model";

    /// The finish reasons for the generation.
    pub const RESPONSE_FINISH_REASONS: &str = "gen_ai.response.finish_reasons";

    /// The unique identifier for the response.
    pub const RESPONSE_ID: &str = "gen_ai.response.id";

    /// The number of input tokens used.
    pub const USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";

    /// The number of output tokens generated.
    pub const USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
}
