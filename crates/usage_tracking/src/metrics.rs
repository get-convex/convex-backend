pub mod storage {

    use common::types::ModuleEnvironment;
    use metrics::{
        log_counter,
        log_counter_with_labels,
        register_convex_counter,
        MetricLabel,
    };

    register_convex_counter!(STORAGE_INGRESS_BYTES, "Number of storage ingress bytes ");
    register_convex_counter!(STORAGE_EGRESS_BYTES, "Number of storage egress bytes");

    pub fn log_storage_ingress_size(ingress_size: u64) {
        log_counter(&STORAGE_INGRESS_BYTES, ingress_size);
    }

    pub fn log_storage_egress_size(egress_size: u64) {
        log_counter(&STORAGE_EGRESS_BYTES, egress_size);
    }

    register_convex_counter!(STORAGE_CALLS_TOTAL, "Total calls to storage");
    pub fn log_storage_call() {
        log_counter(&STORAGE_CALLS_TOTAL, 1)
    }

    register_convex_counter!(
        USAGE_ACTION_COMPUTE_TOTAL,
        "The total number of times we try to track an action execution",
        &["environment"],
    );
    pub fn log_action_compute(env: &ModuleEnvironment) {
        log_counter_with_labels(
            &USAGE_ACTION_COMPUTE_TOTAL,
            1,
            vec![MetricLabel::new("environment", env.to_string())],
        )
    }
}
