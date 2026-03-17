//! Implementation of the `#[tool]` attribute macro.
//!
//! Transforms an async function into a [`StructuredTool`] factory by generating
//! a companion `{name}_tool()` function that returns a fully configured
//! `StructuredTool`.
//!
//! # Attribute syntax
//!
//! ```ignore
//! #[tool]
//! #[tool(kind = "edit")]
//! #[tool(category = "mcp")]
//! #[tool(kind = "read", category = "builtin")]
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Lit, Meta, Pat, Type};

// ---------------------------------------------------------------------------
// Attribute parsing (T306)
// ---------------------------------------------------------------------------

/// Parsed options from the `#[tool(...)]` attribute.
#[derive(Default)]
struct ToolAttrs {
    /// `kind = "read"` → `ToolKind::Read`, etc.
    kind: Option<String>,
    /// `category = "mcp"` → `ToolCategory::Mcp`, etc.
    category: Option<String>,
}

impl ToolAttrs {
    /// Parse a `#[tool(...)]` attribute token stream.
    ///
    /// The attr token stream contains the raw comma-separated key = value pairs
    /// that appear inside `#[tool(...)]`. Emits a compile error token stream if
    /// an unknown key or invalid value is encountered.
    fn parse(attr: TokenStream) -> Result<Self, TokenStream> {
        let mut out = Self::default();
        if attr.is_empty() {
            return Ok(out);
        }
        // Parse directly as Punctuated<Meta, ,> — attr does NOT wrap in a List.
        let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> =
            match syn::parse::Parser::parse2(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                attr,
            ) {
                Ok(n) => n,
                Err(e) => return Err(e.to_compile_error()),
            };
        for item in nested {
            let Meta::NameValue(nv) = item else {
                return Err(
                    syn::Error::new_spanned(item, "#[tool] expects key = \"value\" pairs")
                        .to_compile_error(),
                );
            };
            let key = nv
                .path
                .get_ident()
                .map(std::string::ToString::to_string)
                .unwrap_or_default();
            let syn::Expr::Lit(expr_lit) = &nv.value else {
                return Err(syn::Error::new_spanned(
                    &nv.value,
                    "attribute value must be a string literal",
                )
                .to_compile_error());
            };
            let Lit::Str(s) = &expr_lit.lit else {
                return Err(syn::Error::new_spanned(
                    &expr_lit.lit,
                    "attribute value must be a string literal",
                )
                .to_compile_error());
            };
            let value = s.value();
            match key.as_str() {
                "kind" => {
                    if !matches!(
                        value.as_str(),
                        "read" | "edit" | "search" | "execute" | "other"
                    ) {
                        return Err(syn::Error::new_spanned(
                            s,
                            format!(
                                "unknown kind \"{value}\"; expected one of: read, edit, search, execute, other"
                            ),
                        )
                        .to_compile_error());
                    }
                    out.kind = Some(value);
                }
                "category" => {
                    if !matches!(
                        value.as_str(),
                        "builtin" | "custom" | "mcp" | "remote" | "workflow_as_tool"
                    ) {
                        return Err(syn::Error::new_spanned(
                            s,
                            format!(
                                "unknown category \"{value}\"; expected one of: builtin, custom, mcp, remote, workflow_as_tool"
                            ),
                        )
                        .to_compile_error());
                    }
                    out.category = Some(value);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        nv.path,
                        format!("unknown #[tool] attribute key \"{key}\""),
                    )
                    .to_compile_error());
                }
            }
        }
        Ok(out)
    }

    /// Returns the `ToolKind` token corresponding to the parsed kind string.
    fn kind_token(&self) -> TokenStream {
        let variant = match self.kind.as_deref() {
            Some("read") => quote! { Read },
            Some("edit") => quote! { Edit },
            Some("search") => quote! { Search },
            Some("execute") => quote! { Execute },
            _ => quote! { Other },
        };
        quote! { ::synwire_core::tools::ToolKind::#variant }
    }

    /// Returns the `ToolCategory` token corresponding to the parsed category string.
    fn category_token(&self) -> TokenStream {
        let variant = match self.category.as_deref() {
            Some("builtin") => quote! { Builtin },
            Some("mcp") => quote! { Mcp },
            Some("remote") => quote! { Remote },
            Some("workflow_as_tool") => quote! { WorkflowAsTool },
            _ => quote! { Custom },
        };
        quote! { ::synwire_core::tools::ToolCategory::#variant }
    }
}

/// Extracts documentation comments from the function's attributes.
fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            let syn::Meta::NameValue(nv) = &attr.meta else {
                return None;
            };
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            else {
                return None;
            };
            Some(s.value())
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

/// Represents a parsed function parameter for schema generation.
struct ToolParam {
    name: String,
    json_type: String,
}

/// Maps a Rust type to a JSON Schema type string.
///
/// Falls back to `"string"` for unknown types.
fn rust_type_to_json_type(ty: &Type) -> String {
    let type_str = quote!(#ty).to_string().replace(' ', "");
    match type_str.as_str() {
        "String" | "&str" | "Cow<'_,str>" | "Cow<str>" => "string".to_owned(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => "integer".to_owned(),
        "f32" | "f64" => "number".to_owned(),
        "bool" => "boolean".to_owned(),
        _ if type_str.starts_with("Vec<") || type_str.starts_with("&[") => "array".to_owned(),
        _ => "string".to_owned(),
    }
}

/// Extracts parameter name and type from function arguments, skipping `self`.
fn extract_params(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Vec<ToolParam> {
    inputs
        .iter()
        .filter_map(|arg| {
            let FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let Pat::Ident(ident) = pat_type.pat.as_ref() else {
                return None;
            };
            let name = ident.ident.to_string();
            let json_type = rust_type_to_json_type(&pat_type.ty);
            Some(ToolParam { name, json_type })
        })
        .collect()
}

/// Generates the JSON Schema property and required entries.
fn generate_schema_tokens(params: &[ToolParam]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let properties = params
        .iter()
        .map(|p| {
            let name = &p.name;
            let json_type = &p.json_type;
            quote! { #name: { "type": #json_type } }
        })
        .collect();

    let required = params
        .iter()
        .map(|p| {
            let name = &p.name;
            quote! { #name }
        })
        .collect();

    (properties, required)
}

/// Generates extraction code that parses each parameter from a JSON input value.
fn generate_param_extractions(params: &[ToolParam]) -> Vec<TokenStream> {
    params
        .iter()
        .map(|p| {
            let name_str = &p.name;
            let ident = format_ident!("{}", p.name);
            match p.json_type.as_str() {
                "integer" => quote! {
                    let #ident = __input
                        .get(#name_str)
                        .and_then(|v| v.as_i64())
                        .unwrap_or_default();
                },
                "number" => quote! {
                    let #ident = __input
                        .get(#name_str)
                        .and_then(|v| v.as_f64())
                        .unwrap_or_default();
                },
                "boolean" => quote! {
                    let #ident = __input
                        .get(#name_str)
                        .and_then(|v| v.as_bool())
                        .unwrap_or_default();
                },
                // "string", "array", and all other types default to string extraction
                _ => quote! {
                    let #ident = __input
                        .get(#name_str)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_owned();
                },
            }
        })
        .collect()
}

/// Core implementation of the `#[tool]` attribute macro.
pub fn tool_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse optional kind/category attributes (T306).
    let tool_attrs = match ToolAttrs::parse(attr) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let input: ItemFn = match syn::parse2(item) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    // T307: Compile-time validation — at least one non-self parameter.
    let params = extract_params(&input.sig.inputs);
    if params.is_empty() {
        return syn::Error::new_spanned(
            &input.sig.ident,
            "#[tool] requires at least one non-self parameter",
        )
        .to_compile_error();
    }

    let fn_name = &input.sig.ident;
    let tool_fn_name = format_ident!("{fn_name}_tool");
    let fn_name_str = fn_name.to_string();
    let kind_ident = format_ident!("{}_TOOL_KIND", fn_name_str.to_uppercase());
    let category_ident = format_ident!("{}_TOOL_CATEGORY", fn_name_str.to_uppercase());
    let kind_token = tool_attrs.kind_token();
    let category_token = tool_attrs.category_token();

    let description = extract_doc_comment(&input.attrs);
    let description = if description.is_empty() {
        fn_name_str.clone()
    } else {
        description
    };

    let (property_entries, required_entries) = generate_schema_tokens(&params);
    let param_extractions = generate_param_extractions(&params);

    let param_idents: Vec<proc_macro2::Ident> =
        params.iter().map(|p| format_ident!("{}", p.name)).collect();

    quote! {
        #input

        /// [`ToolKind`](::synwire_core::tools::ToolKind) for this tool.
        pub const #kind_ident: ::synwire_core::tools::ToolKind = #kind_token;
        /// [`ToolCategory`](::synwire_core::tools::ToolCategory) for this tool.
        pub const #category_ident: ::synwire_core::tools::ToolCategory = #category_token;

        /// Creates a [`synwire_core::tools::StructuredTool`] wrapping
        #[doc = concat!("[`", stringify!(#fn_name), "`].")]
        pub fn #tool_fn_name() -> ::std::result::Result<
            ::synwire_core::tools::StructuredTool,
            ::synwire_core::error::SynwireError,
        > {
            ::synwire_core::tools::StructuredTool::builder()
                .name(#fn_name_str)
                .description(#description)
                .schema(::synwire_core::tools::ToolSchema {
                    name: ::std::string::String::from(#fn_name_str),
                    description: ::std::string::String::from(#description),
                    parameters: ::serde_json::json!({
                        "type": "object",
                        "properties": { #(#property_entries),* },
                        "required": [ #(#required_entries),* ]
                    }),
                })
                .func(|__input: ::serde_json::Value| {
                    ::std::boxed::Box::pin(async move {
                        #(#param_extractions)*
                        let __result = #fn_name(#(#param_idents),*).await;
                        match __result {
                            ::std::result::Result::Ok(content) => {
                                ::std::result::Result::Ok(::synwire_core::tools::ToolOutput {
                                    content,
                                    artifact: ::std::option::Option::None,
                                    ..::std::default::Default::default()
                                })
                            }
                            ::std::result::Result::Err(e) => ::std::result::Result::Err(e),
                        }
                    })
                })
                .build()
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn extract_doc_from_attrs() {
        let input: ItemFn = syn::parse_quote! {
            /// Searches the web.
            /// Returns results.
            async fn search(query: String) -> Result<String, Error> {
                Ok(query)
            }
        };
        let doc = extract_doc_comment(&input.attrs);
        assert!(doc.contains("Searches the web."));
        assert!(doc.contains("Returns results."));
    }

    #[test]
    fn extract_params_from_fn() {
        let input: ItemFn = syn::parse_quote! {
            async fn search(query: String, count: i32) -> Result<String, Error> {
                Ok(query)
            }
        };
        let params = extract_params(&input.sig.inputs);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "query");
        assert_eq!(params[0].json_type, "string");
        assert_eq!(params[1].name, "count");
        assert_eq!(params[1].json_type, "integer");
    }

    #[test]
    fn type_mapping_coverage() {
        let cases = [
            ("String", "string"),
            ("i32", "integer"),
            ("u64", "integer"),
            ("f64", "number"),
            ("bool", "boolean"),
            ("Vec<String>", "array"),
        ];
        for (rust_ty, expected) in cases {
            let ty: Type = syn::parse_str(rust_ty)
                .unwrap_or_else(|_| panic!("failed to parse type: {rust_ty}"));
            assert_eq!(
                rust_type_to_json_type(&ty),
                expected,
                "type {rust_ty} should map to {expected}"
            );
        }
    }

    #[test]
    fn tool_impl_produces_output() {
        let input = quote! {
            /// A test tool.
            async fn my_tool(query: String) -> Result<String, synwire_core::error::SynwireError> {
                Ok(query)
            }
        };
        let output = tool_impl(TokenStream::new(), input);
        let output_str = output.to_string();
        assert!(output_str.contains("my_tool_tool"));
        assert!(output_str.contains("StructuredTool"));
        assert!(output_str.contains("A test tool."));
    }

    #[test]
    fn empty_doc_uses_fn_name() {
        let input = quote! {
            async fn bare_fn(x: String) -> Result<String, synwire_core::error::SynwireError> {
                Ok(x)
            }
        };
        let output = tool_impl(TokenStream::new(), input);
        let output_str = output.to_string();
        assert!(output_str.contains("\"bare_fn\""));
    }

    // T306: kind/category attribute tests
    #[test]
    fn kind_attribute_generates_constant() {
        let attr = quote! { kind = "edit" };
        let input = quote! {
            async fn write_file(path: String) -> Result<String, synwire_core::error::SynwireError> {
                Ok(path)
            }
        };
        let output = tool_impl(attr, input);
        let s = output.to_string();
        assert!(s.contains("ToolKind"), "expected ToolKind in output: {s}");
        assert!(s.contains("Edit"), "expected Edit variant: {s}");
        assert!(
            s.contains("WRITE_FILE_TOOL_KIND"),
            "expected WRITE_FILE_TOOL_KIND const: {s}"
        );
    }

    #[test]
    fn category_attribute_generates_constant() {
        let attr = quote! { category = "mcp" };
        let input = quote! {
            async fn fetch_data(url: String) -> Result<String, synwire_core::error::SynwireError> {
                Ok(url)
            }
        };
        let output = tool_impl(attr, input);
        let s = output.to_string();
        assert!(
            s.contains("ToolCategory"),
            "expected ToolCategory in output: {s}"
        );
        assert!(s.contains("Mcp"), "expected Mcp variant: {s}");
        assert!(
            s.contains("FETCH_DATA_TOOL_CATEGORY"),
            "expected FETCH_DATA_TOOL_CATEGORY const: {s}"
        );
    }

    #[test]
    fn both_attributes_together() {
        let attr = quote! { kind = "read", category = "builtin" };
        let input = quote! {
            async fn list_files(dir: String) -> Result<String, synwire_core::error::SynwireError> {
                Ok(dir)
            }
        };
        let output = tool_impl(attr, input);
        let s = output.to_string();
        assert!(s.contains("Read"), "expected Read kind: {s}");
        assert!(s.contains("Builtin"), "expected Builtin category: {s}");
    }

    #[test]
    fn unknown_kind_is_error() {
        let attrs = ToolAttrs::parse(quote! { kind = "unknown_kind" });
        assert!(attrs.is_err(), "expected error for unknown kind");
    }

    #[test]
    fn unknown_category_is_error() {
        let attrs = ToolAttrs::parse(quote! { category = "invalid" });
        assert!(attrs.is_err(), "expected error for unknown category");
    }

    // T307: compile-time validation test
    #[test]
    fn no_params_produces_error() {
        let input = quote! {
            async fn no_args() -> Result<String, synwire_core::error::SynwireError> {
                Ok("".into())
            }
        };
        let output = tool_impl(TokenStream::new(), input);
        let s = output.to_string();
        assert!(
            s.contains("compile_error") || s.contains("at least one"),
            "expected compile error for no params: {s}"
        );
    }

    #[test]
    fn default_kind_is_other() {
        let attrs = ToolAttrs::parse(TokenStream::new()).unwrap();
        let token_str = attrs.kind_token().to_string();
        assert!(
            token_str.contains("Other"),
            "expected Other as default kind: {token_str}"
        );
    }

    #[test]
    fn default_category_is_custom() {
        let attrs = ToolAttrs::parse(TokenStream::new()).unwrap();
        let token_str = attrs.category_token().to_string();
        assert!(
            token_str.contains("Custom"),
            "expected Custom as default category: {token_str}"
        );
    }
}
