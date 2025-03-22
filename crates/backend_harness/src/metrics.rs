use metrics::{
    log_counter_with_labels,
    register_convex_counter,
    StaticMetricLabel,
};

use crate::ProvisionRequest;

register_convex_counter!(
    PROVISIONS_TOTAL,
    "Number of provisions",
    &["status", "load_description", "provision_type"]
);
pub fn log_provision(
    metric_label: StaticMetricLabel,
    is_ok: bool,
    provision_type: &ProvisionRequest,
) {
    let provision_type_label = match provision_type {
        ProvisionRequest::ExistingProject { .. } => "existing_project",
        ProvisionRequest::NewProject => "new_project",
        ProvisionRequest::Preview { .. } => "preview",
    };
    log_counter_with_labels(
        &PROVISIONS_TOTAL,
        1,
        vec![
            metric_label,
            StaticMetricLabel::status(is_ok),
            StaticMetricLabel::new("provision_type", provision_type_label),
        ],
    );
}

register_convex_counter!(
    DEACTIVATES_TOTAL,
    "Number of deactivates",
    &["status", "load_description"]
);
pub fn log_deactivate(metric_label: StaticMetricLabel, is_ok: bool) {
    log_counter_with_labels(
        &DEACTIVATES_TOTAL,
        1,
        vec![metric_label, StaticMetricLabel::status(is_ok)],
    );
}

register_convex_counter!(
    PUSH_TOTAL,
    "Number of pushes",
    &["status", "load_description"]
);
pub fn log_push(metric_label: StaticMetricLabel, is_ok: bool) {
    log_counter_with_labels(
        &PUSH_TOTAL,
        1,
        vec![metric_label, StaticMetricLabel::status(is_ok)],
    );
}
