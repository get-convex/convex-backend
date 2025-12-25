pub mod storage {

    use metrics::{
        log_counter,
        register_convex_counter,
    };

    register_convex_counter!(STORAGE_INGRESS_BYTES, "Number of storage ingress bytes ");
    register_convex_counter!(STORAGE_EGRESS_BYTES, "Number of storage egress bytes");

    pub fn log_storage_ingress(ingress: u64) {
        log_counter(&STORAGE_INGRESS_BYTES, ingress);
    }

    pub fn log_storage_egress(egress: u64) {
        log_counter(&STORAGE_EGRESS_BYTES, egress);
    }

    register_convex_counter!(STORAGE_CALLS_TOTAL, "Total calls to storage");
    pub fn log_storage_call() {
        log_counter(&STORAGE_CALLS_TOTAL, 1)
    }
}
