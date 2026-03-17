//! Tool types, traits, and schemas.

pub mod search_index;
mod structured;
mod traits;
mod types;

pub use search_index::{
    DisclosureDepth, IntentExtractor, QueryPreprocessor, ToolSearchArgs, ToolSearchIndex,
    ToolSearchResult, ToolTransitionGraph, allocate_budget, run_tool_list, run_tool_search,
    verify_parameter_types,
};
pub use structured::{
    CompositeToolProvider, NameCollisionPolicy, StaticToolProvider, StructuredTool,
    StructuredToolBuilder,
};
pub use traits::{Tool, ToolProvider, validate_tool_name};
pub use types::{
    BinaryResult, TimeoutBehavior, ToolAnnotations, ToolCategory, ToolConfig, ToolContentType,
    ToolKind, ToolOutput, ToolResult, ToolResultStatus, ToolSchema,
};
