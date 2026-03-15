//! Implementation of the `#[derive(State)]` derive macro.
//!
//! Generates channel configuration metadata from struct field annotations.
//! Fields annotated with `#[reducer(topic)]` produce [`Topic`] channels;
//! all other fields default to [`LastValue`] channels.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

/// The channel type determined from field annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReducerKind {
    /// Keeps only the most recent value.
    LastValue,
    /// Accumulates all values in order.
    Topic,
}

/// Parsed information about a single struct field.
struct FieldInfo {
    name: String,
    reducer: ReducerKind,
}

/// Parses the `#[reducer(...)]` attribute from a field's attributes.
fn parse_reducer(attrs: &[syn::Attribute]) -> ReducerKind {
    for attr in attrs {
        if !attr.path().is_ident("reducer") {
            continue;
        }
        // Parse the form: #[reducer(topic)] or #[reducer(last_value)]
        let Ok(inner) = attr.parse_args::<syn::Ident>() else {
            continue;
        };
        return if inner == "topic" {
            ReducerKind::Topic
        } else {
            ReducerKind::LastValue
        };
    }
    ReducerKind::LastValue
}

/// Core implementation of the `#[derive(State)]` macro.
pub fn derive_state_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse2(input) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = &input.ident;

    let Data::Struct(data_struct) = &input.data else {
        return syn::Error::new_spanned(&input, "State can only be derived for structs")
            .to_compile_error();
    };

    let Fields::Named(named_fields) = &data_struct.fields else {
        return syn::Error::new_spanned(
            &input,
            "State can only be derived for structs with named fields",
        )
        .to_compile_error();
    };

    let fields: Vec<FieldInfo> = named_fields
        .named
        .iter()
        .filter_map(|f| {
            let name = f.ident.as_ref()?.to_string();
            let reducer = parse_reducer(&f.attrs);
            Some(FieldInfo { name, reducer })
        })
        .collect();

    let channel_entries: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let channel_ctor = match f.reducer {
                ReducerKind::LastValue => {
                    quote! {
                        ::std::boxed::Box::new(
                            ::synwire_orchestrator::channels::LastValue::new(#name)
                        ) as ::std::boxed::Box<dyn ::synwire_orchestrator::channels::BaseChannel>
                    }
                }
                ReducerKind::Topic => {
                    quote! {
                        ::std::boxed::Box::new(
                            ::synwire_orchestrator::channels::Topic::new(#name)
                        ) as ::std::boxed::Box<dyn ::synwire_orchestrator::channels::BaseChannel>
                    }
                }
            };
            quote! {
                __channels.push((::std::string::String::from(#name), #channel_ctor));
            }
        })
        .collect();

    // Generate from_channels field extraction for each field.
    let field_extractions: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let field_ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! {
                let #field_ident = channels.get(#name)
                    .and_then(|c| c.get())
                    .map(|v| ::serde_json::from_value(v.clone()))
                    .transpose()
                    .map_err(|e| ::synwire_orchestrator::error::GraphError::DeserializationError {
                        field: ::std::string::String::from(#name),
                        message: e.to_string(),
                    })?
                    .unwrap_or_default();
            }
        })
        .collect();

    let field_names: Vec<syn::Ident> = fields
        .iter()
        .map(|f| syn::Ident::new(&f.name, proc_macro2::Span::call_site()))
        .collect();

    let channel_count = fields.len();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let output = quote! {
        impl #impl_generics ::synwire_orchestrator::graph::state::State
            for #struct_name #ty_generics #where_clause
        {
            /// Returns the channel configuration derived from the struct fields.
            ///
            /// Each field maps to a channel:
            /// - `#[reducer(topic)]` fields use a [`Topic`] channel.
            /// - All other fields use a [`LastValue`] channel.
            fn channels() -> ::std::vec::Vec<(
                ::std::string::String,
                ::std::boxed::Box<dyn ::synwire_orchestrator::channels::BaseChannel>,
            )> {
                let mut __channels = ::std::vec::Vec::with_capacity(#channel_count);
                #(#channel_entries)*
                __channels
            }

            fn from_channels(
                channels: &::std::collections::HashMap<
                    ::std::string::String,
                    ::std::boxed::Box<dyn ::synwire_orchestrator::channels::BaseChannel>,
                >,
            ) -> ::std::result::Result<Self, ::synwire_orchestrator::error::GraphError> {
                #(#field_extractions)*
                Ok(Self { #(#field_names),* })
            }
        }
    };

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reducer_default() {
        let attrs: Vec<syn::Attribute> = vec![];
        assert_eq!(parse_reducer(&attrs), ReducerKind::LastValue);
    }

    #[test]
    fn parse_reducer_topic() {
        let input: syn::Field = syn::parse_quote! {
            #[reducer(topic)]
            messages: Vec<String>
        };
        assert_eq!(parse_reducer(&input.attrs), ReducerKind::Topic);
    }

    #[test]
    fn parse_reducer_last_value_explicit() {
        let input: syn::Field = syn::parse_quote! {
            #[reducer(last_value)]
            step: String
        };
        assert_eq!(parse_reducer(&input.attrs), ReducerKind::LastValue);
    }

    #[test]
    fn derive_produces_impl_block() {
        let input = quote! {
            struct MyState {
                messages: Vec<String>,
                #[reducer(topic)]
                log: Vec<String>,
                current: String,
            }
        };
        let output = derive_state_impl(input);
        let output_str = output.to_string();
        assert!(output_str.contains("channels"));
        assert!(output_str.contains("Topic"));
        assert!(output_str.contains("LastValue"));
    }

    #[test]
    fn derive_rejects_enum() {
        let input = quote! {
            enum NotAStruct {
                A,
                B,
            }
        };
        let output = derive_state_impl(input);
        let output_str = output.to_string();
        assert!(output_str.contains("compile_error"));
    }

    #[test]
    fn derive_rejects_tuple_struct() {
        let input = quote! {
            struct TupleStruct(String, i32);
        };
        let output = derive_state_impl(input);
        let output_str = output.to_string();
        assert!(output_str.contains("compile_error"));
    }

    /// T016: derive generates `impl State` with correct `channels()`.
    #[test]
    fn derive_generates_impl_state_with_channels() {
        let input = quote! {
            struct AgentState {
                query: String,
                response: String,
            }
        };
        let output = derive_state_impl(input);
        let output_str = output.to_string();
        // Should produce `impl State for AgentState`, not `impl AgentState`.
        assert!(
            output_str.contains("State"),
            "output must contain State trait impl"
        );
        assert!(
            output_str.contains("synwire_orchestrator :: graph :: state :: State"),
            "output must use fully qualified State path, got: {output_str}"
        );
        assert!(
            output_str.contains("fn channels"),
            "output must contain channels method"
        );
        assert!(
            output_str.contains("LastValue"),
            "default fields should use LastValue channels"
        );
    }

    /// T017: derive generates `from_channels()` that deserialises correctly.
    #[test]
    fn derive_generates_from_channels() {
        let input = quote! {
            struct SimpleState {
                name: String,
                count: i32,
            }
        };
        let output = derive_state_impl(input);
        let output_str = output.to_string();
        assert!(
            output_str.contains("from_channels"),
            "output must contain from_channels method"
        );
        assert!(
            output_str.contains("serde_json :: from_value"),
            "from_channels must deserialise via serde_json::from_value, got: {output_str}"
        );
        assert!(
            output_str.contains("unwrap_or_default"),
            "missing channels should fall back to Default"
        );
        assert!(
            output_str.contains("DeserializationError"),
            "errors must use GraphError::DeserializationError"
        );
    }

    /// T018: derive with `#[reducer(topic)]` maps to Topic channel.
    #[test]
    fn derive_with_reducer_topic_maps_to_topic_channel() {
        let input = quote! {
            struct ChatState {
                #[reducer(topic)]
                messages: Vec<String>,
                current_speaker: String,
            }
        };
        let output = derive_state_impl(input);
        let output_str = output.to_string();
        // The messages field should produce a Topic channel.
        assert!(
            output_str.contains("Topic :: new"),
            "reducer(topic) field must use Topic channel, got: {output_str}"
        );
        // The current_speaker field should produce a LastValue channel.
        assert!(
            output_str.contains("LastValue :: new"),
            "unannotated field must use LastValue channel"
        );
        // Both fields should appear in from_channels.
        assert!(
            output_str.contains("\"messages\""),
            "from_channels must extract messages field"
        );
        assert!(
            output_str.contains("\"current_speaker\""),
            "from_channels must extract current_speaker field"
        );
    }
}
