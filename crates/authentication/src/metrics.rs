use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    StaticMetricLabel,
};

register_convex_counter!(pub DEPLOY_KEY_USE_TOTAL, "Count of deploy key uses", &["key_type"]);

pub enum DeployKeyType {
    Legacy,
    System,
    Unknown,
    AccessToken,
}

pub fn log_deploy_key_use(key_type: DeployKeyType) {
    let key_type_label = match key_type {
        DeployKeyType::Legacy => "legacy",
        DeployKeyType::System => "system",
        DeployKeyType::AccessToken => "access_token",
        DeployKeyType::Unknown => "unknown",
    };
    log_counter_with_labels(
        &DEPLOY_KEY_USE_TOTAL,
        1,
        vec![StaticMetricLabel::new("key_type", key_type_label)],
    );
}
