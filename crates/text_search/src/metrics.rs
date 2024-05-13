use metrics::{
    log_distribution,
    prometheus::VMHistogram,
    register_convex_histogram,
    Timer,
};

register_convex_histogram!(
    LOAD_ALIVE_BITSET_SECONDS,
    "Time to load AliveBitSet in seconds"
);
pub fn load_alive_bitset_timer() -> Timer<VMHistogram> {
    Timer::new(&LOAD_ALIVE_BITSET_SECONDS)
}

register_convex_histogram!(ALIVE_BITSET_SIZE_BYTES, "Size of AliveBitSet in bytes");
pub fn log_alive_bitset_size(size: usize) {
    log_distribution(&ALIVE_BITSET_SIZE_BYTES, size as f64)
}

register_convex_histogram!(
    LOAD_DELETED_TERMS_TABLE_SECONDS,
    "Time to load deleted terms table in seconds"
);
pub fn load_deleted_terms_table_timer() -> Timer<VMHistogram> {
    Timer::new(&LOAD_DELETED_TERMS_TABLE_SECONDS)
}

register_convex_histogram!(
    DELETED_TERMS_TABLE_SIZE_BYTES,
    "Size of deleted terms table in bytes"
);
pub fn log_deleted_terms_table_size(size: usize) {
    log_distribution(&DELETED_TERMS_TABLE_SIZE_BYTES, size as f64)
}
