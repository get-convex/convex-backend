use metrics::{
    log_distribution,
    register_convex_histogram,
    StatusTimer,
    STATUS_LABEL,
};

register_convex_histogram!(
    SIGN_URL_SECONDS,
    "Time to fetch a presigned S3 link",
    &STATUS_LABEL
);
pub fn sign_url_timer() -> StatusTimer {
    StatusTimer::new(&SIGN_URL_SECONDS)
}

register_convex_histogram!(
    AWS_S3_PART_UPLOAD_SIZE_BYTES,
    "The size in bytes of each part of an s3 multi part upload.",
);
pub fn log_aws_s3_part_upload_size_bytes(size: usize) {
    log_distribution(&AWS_S3_PART_UPLOAD_SIZE_BYTES, size as f64)
}
