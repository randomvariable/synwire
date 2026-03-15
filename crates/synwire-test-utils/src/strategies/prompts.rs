//! Proptest strategies for `PromptTemplate` variables.

use std::collections::HashMap;

use proptest::prelude::*;

/// Strategy for generating a set of template variable names.
pub fn arb_variable_names() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z_]{1,12}", 1..=5)
}

/// Strategy for generating a variables map that matches a given set of names.
pub fn arb_variables_for(names: Vec<String>) -> impl Strategy<Value = HashMap<String, String>> {
    names
        .into_iter()
        .map(|name| (".{0,50}").prop_map(move |value| (name.clone(), value)))
        .fold(Just(HashMap::new()).boxed(), |acc, item| {
            (acc, item)
                .prop_map(|(mut map, (k, v))| {
                    let _ = map.insert(k, v);
                    map
                })
                .boxed()
        })
}

/// Strategy for generating a template string with `{var}` placeholders and
/// its corresponding variable names.
pub fn arb_template_with_vars() -> impl Strategy<Value = (String, Vec<String>)> {
    arb_variable_names().prop_flat_map(|names| {
        let template = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                if i == 0 {
                    format!("{{{name}}}")
                } else {
                    format!(" {{{name}}}")
                }
            })
            .collect::<String>();
        let prefix = "Hello ";
        let full_template = format!("{prefix}{template}");
        Just((full_template, names))
    })
}
