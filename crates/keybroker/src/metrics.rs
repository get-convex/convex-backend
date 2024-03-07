use metrics::{
    log_counter,
    register_convex_counter,
};

register_convex_counter!(
    KEYBROKER_STORE_FILE_EXPIRED_TOTAL,
    "Number of times a store file authorization was rejected because it was expired"
);
pub fn log_store_file_auth_expired() {
    log_counter(&KEYBROKER_STORE_FILE_EXPIRED_TOTAL, 1);
}

register_convex_counter!(
    KEYBROKER_ACTIONS_TOKEN_EXPIRED_TOTAL,
    "Number of times an action token was rejected because it was expired"
);
pub fn log_actions_token_expired() {
    log_counter(&KEYBROKER_ACTIONS_TOKEN_EXPIRED_TOTAL, 1);
}
