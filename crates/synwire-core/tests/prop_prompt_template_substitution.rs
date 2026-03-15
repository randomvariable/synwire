//! Property test: `PromptTemplate` substitution correctness.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proptest::prelude::*;
use synwire_test_utils::prompts::{arb_template_with_vars, arb_variables_for};

proptest! {
    /// When all required variables are provided, format() should succeed
    /// and the result should not contain any `{varname}` placeholders.
    #[test]
    fn template_substitution_replaces_all_vars(
        (template, vars) in arb_template_with_vars()
            .prop_flat_map(|(t, v)| {
                arb_variables_for(v).prop_map(move |vals| (t.clone(), vals))
            })
    ) {
        let input_variables: Vec<String> = template
            .split('{')
            .filter_map(|s| s.split('}').next())
            .filter(|s| !s.is_empty() && !s.contains(' '))
            .map(String::from)
            .collect();

        let tpl = synwire_core::prompts::PromptTemplate::new(
            template,
            input_variables.clone(),
        );
        let result = tpl.format(&vars).unwrap();

        // No remaining `{var}` placeholders for known variables.
        for var in &input_variables {
            let placeholder = format!("{{{var}}}");
            assert!(
                !result.contains(&placeholder),
                "result still contains placeholder {placeholder}: {result}"
            );
        }
    }

    /// When a required variable is missing, format() should return an error.
    #[test]
    fn template_missing_var_errors(
        (template, var_names) in arb_template_with_vars()
    ) {
        let tpl = synwire_core::prompts::PromptTemplate::new(
            template,
            var_names,
        );
        // Empty map => missing variables.
        let result = tpl.format(&std::collections::HashMap::new());
        assert!(result.is_err());
    }
}
