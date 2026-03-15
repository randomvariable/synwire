//! # synwire-derive
//!
//! Procedural macros for the Synwire framework.
//!
//! ## `#[tool]` attribute macro
//!
//! Transforms an annotated async function into a `StructuredTool` factory.
//! The original function is preserved, and a companion `{name}_tool()` function
//! is generated that returns a fully-configured `StructuredTool`.
//!
//! ```rust,ignore
//! use synwire_derive::tool;
//!
//! #[tool]
//! /// Searches the web for information.
//! async fn search(query: String) -> Result<String, SynwireError> {
//!     Ok(format!("Results for: {query}"))
//! }
//!
//! // Generates `search_tool()` -> Result<StructuredTool, SynwireError>
//! ```
//!
//! ## `#[derive(State)]` derive macro
//!
//! Generates channel configuration from struct field annotations.
//! Fields annotated with `#[reducer(topic)]` use a `Topic` channel;
//! all others default to `LastValue`.
//!
//! ```rust,ignore
//! use synwire_derive::State;
//!
//! #[derive(State)]
//! struct MyState {
//!     #[reducer(topic)]
//!     messages: Vec<String>,
//!     current_step: String,
//! }
//!
//! // Generates `MyState::channels()` returning channel definitions
//! ```

#![deny(unsafe_code)]

mod state;
mod tool;

/// Attribute macro that generates a `StructuredTool` factory from an async
/// function.
///
/// The original function is preserved unchanged. A companion function named
/// `{original_name}_tool()` is generated that returns
/// `Result<StructuredTool, SynwireError>`.
///
/// # Parameters
///
/// Function parameters are mapped to JSON Schema types:
/// - `String` / `&str` -> `"string"`
/// - `i32`, `u64`, etc. -> `"integer"`
/// - `f32`, `f64` -> `"number"`
/// - `bool` -> `"boolean"`
/// - `Vec<T>` -> `"array"`
///
/// # Documentation
///
/// Doc comments on the function become the tool's description. If no doc
/// comment is present, the function name is used as the description.
#[proc_macro_attribute]
pub fn tool(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    tool::tool_impl(attr.into(), item.into()).into()
}

/// Derive macro that generates channel configuration from struct fields.
///
/// Produces a `channels()` associated function returning a
/// `Vec<(String, Box<dyn BaseChannel>)>`.
///
/// # Field attributes
///
/// - `#[reducer(topic)]` -- uses a `Topic` channel (accumulates values)
/// - `#[reducer(last_value)]` -- uses a `LastValue` channel (explicit default)
/// - No attribute -- defaults to `LastValue`
#[proc_macro_derive(State, attributes(reducer))]
pub fn derive_state(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    state::derive_state_impl(input.into()).into()
}
