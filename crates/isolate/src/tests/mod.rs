use std::fmt::Display;

mod action;
mod adversarial;
mod analyze;
mod args_validation;
mod r#async;
mod auth;
mod backend_state;
mod basic;
mod creation_time;
mod custom_errors;
mod environment_variables;
mod fetch;
mod globals;
mod http_action;
mod id_encoding;
mod id_strings;
mod import;
mod internal;
mod js_builtins;
mod logging;
mod module_loader;
mod query;
mod scheduler;
mod schema;
mod search;
mod shapes;
mod size_errors;
mod source_maps;
mod system_udfs;
mod unicode;
mod user_error;
mod values;
mod vector_search;

pub fn assert_contains(error: &impl Display, expected: &str) {
    assert!(
        format!("{}", error).contains(expected),
        "\nExpected: {expected}\nActual: {error}"
    );
}
