mod airbyte_import;
mod analyze;
mod auth;
mod auth_config;
pub mod components;
mod cron_jobs;
mod environment_variables;
mod fivetran_import;
mod http_action;
mod indexes;
mod mutation;
mod occ_retries;
mod push;
mod query_cache;
mod returns_validation;
mod scheduled_jobs;
mod schema;
mod source_package;
mod storage;

const NODE_SOURCE: &str = r#"
var nodeFunction = () => {};
nodeFunction.isRegistered = true;
nodeFunction.isAction = true;
nodeFunction.invokeAction = nodeFunction;

export { nodeFunction };
"#;
