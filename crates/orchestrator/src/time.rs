use std::time::{
    SystemTime,
    UNIX_EPOCH,
};

pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn ms_to_f64(ms: i64) -> f64 {
    ms as f64
}
