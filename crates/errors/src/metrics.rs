use metrics::register_convex_counter;

register_convex_counter!(pub BAD_REQUEST_ERROR_TOTAL, "Count of bad request errors");
register_convex_counter!(pub CLIENT_DISCONNECT_ERROR_TOTAL, "Count of client disconnect errors");
register_convex_counter!(pub SYNC_AUTH_ERROR_TOTAL, "Count of sync auth errors");
register_convex_counter!(pub FORBIDDEN_ERROR_TOTAL, "Count of forbidden errors");
register_convex_counter!(pub COMMIT_RACE_TOTAL, "Total count of commit race errors");
