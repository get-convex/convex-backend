mod analyze;
mod auth_config;
mod cron_jobs;
mod environment_variables;
mod mutation;
mod occ_retries;
mod scheduled_jobs;
mod schema;
mod source_package;

const NODE_SOURCE: &str = r#"
var nodeFunction = () => {};
nodeFunction.isRegistered = true;
nodeFunction.isAction = true;
nodeFunction.invokeAction = nodeFunction;

export { nodeFunction };
"#;
