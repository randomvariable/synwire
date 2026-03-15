//! Implementation of the `#[tool]` attribute macro.
//!
//! Transforms an async function into a [`StructuredTool`] factory by generating
//! a companion `{name}_tool()` function that returns a fully configured
//! `StructuredTool`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat, Type};

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
pub fn tool_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = match syn::parse2(item) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error(),
    };

    let fn_name = &input.sig.ident;
    let tool_fn_name = format_ident!("{fn_name}_tool");
    let fn_name_str = fn_name.to_string();

    let description = extract_doc_comment(&input.attrs);
    let description = if description.is_empty() {
        fn_name_str.clone()
    } else {
        description
    };

    let params = extract_params(&input.sig.inputs);
    let (property_entries, required_entries) = generate_schema_tokens(&params);
    let param_extractions = generate_param_extractions(&params);

    let param_idents: Vec<proc_macro2::Ident> =
        params.iter().map(|p| format_ident!("{}", p.name)).collect();

    quote! {
        #input

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
#[allow(clippy::panic)]
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
}
