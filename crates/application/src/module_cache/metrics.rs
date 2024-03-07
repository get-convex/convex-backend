use metrics::{
    register_convex_histogram,
    StatusTimer,
    STATUS_LABEL,
};

register_convex_histogram!(
    MODULE_CACHE_GET_MODULE_SECONDS,
    "Time taken to retrieve a module from the cache",
    &STATUS_LABEL
);
pub fn module_cache_get_module_timer() -> StatusTimer {
    StatusTimer::new(&MODULE_CACHE_GET_MODULE_SECONDS)
}
