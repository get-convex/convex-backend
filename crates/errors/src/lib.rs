#![feature(lazy_cell)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(impl_trait_in_assoc_type)]
use std::{
    borrow::Cow,
    sync::LazyLock,
};

use ::metrics::{
    metric_tag_const,
    metric_tag_const_value,
    MetricTag,
};
use http::StatusCode;
use prometheus::IntCounter;
use tungstenite::protocol::{
    frame::coding::CloseCode,
    CloseFrame,
};

mod metrics;

/// These module prefixes are default-hidden from stacktraces
/// See https://docs.sentry.io/platforms/rust/configuration/options/#in-app-exclude
pub static SENTRY_IN_APP_EXCLUDE: LazyLock<Vec<&'static str>> = LazyLock::new(|| vec!["tokio::"]);

/// ErrorMetadata object can be attached to an anyhow error chain via
/// `.context(e /*ErrorMetadata*/)`. It is a generic object to be used
/// across the codebase to tag errors with information that is used to classify.
///
/// The msg is conveyed as a user facing error message if it makes it to the
/// client.
///
/// The short_msg is used as a tag - available for tests and for metrics
/// logging - to have a message that is resilient to changes in copy. Some
/// protocols may opt to send the short_msg as a separate field (eg ws close
/// code and HTTP endpoint response json).
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
#[error("{msg}")]
pub struct ErrorMetadata {
    /// The error code associated with this ErrorMetadata
    pub code: ErrorCode,
    /// short ScreamingCamelCase. Usable in tests for string matching
    /// w/ a standard test helper.
    /// Eg InvalidModuleName
    pub short_msg: Cow<'static, str>,
    /// human readable - developer facing. Should be longer and descriptive.
    /// Eg "The module name is invalid because it contains an invalid character"
    pub msg: Cow<'static, str>,
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest,
    Unauthenticated,
    Forbidden,
    NotFound,
    ClientDisconnect,

    Overloaded,
    RejectedBeforeExecution,
    OCC,
    PaginationLimit,
    OutOfRetention,

    OperationalInternalServerError,
}

impl ErrorMetadata {
    /// Bad Request. Maps to 400 in HTTP.
    ///
    /// The short_msg should be a CapitalCamelCased describing the error.
    /// The msg should be a descriptive message targeted toward the developer.
    pub fn bad_request(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::BadRequest,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    /// Resource not found. Maps to 404 in HTTP.
    ///
    /// The short_msg should be a CapitalCamelCased describing the error (eg
    /// FileNotFound). The msg should be a descriptive message targeted
    /// toward the developer.
    pub fn not_found(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::NotFound,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    /// Not authenticated. Maps to 401 in HTTP.
    ///
    /// The short_msg should be a CapitalCamelCased describing the error (eg
    /// InvalidHeader). The msg should be a descriptive message targeted
    /// toward the developer.
    pub fn unauthenticated(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::Unauthenticated,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    /// Forbidden. Maps to 403 in HTTP.
    ///
    /// The short_msg should be a CapitalCamelCased describing the error (eg
    /// TooManyTeams). The msg should be a descriptive message targeted
    /// toward the developer.
    pub fn forbidden(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::Forbidden,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    /// Client disconnected the connection.
    pub fn client_disconnect() -> Self {
        Self {
            code: ErrorCode::ClientDisconnect,
            short_msg: CLIENT_DISCONNECTED.into(),
            msg: CLIENT_DISCONNECTED_MSG.into(),
        }
    }

    /// Operational Internal Server Error (maps to 500 in HTTP)
    ///
    /// Produces a very general error message for the user. Should be
    /// used in situations where the error is caused by a known operational
    /// source of downtime (eg during a restart or backend code push)
    pub fn operational_internal_server_error() -> Self {
        Self {
            code: ErrorCode::OperationalInternalServerError,
            short_msg: INTERNAL_SERVER_ERROR.into(),
            msg: INTERNAL_SERVER_ERROR_MSG.into(),
        }
    }

    /// Internal error with a user visible message indicating that the user has
    /// hit some defensive limit in Convex. Maps to 503 in HTTP.
    ///
    /// Ideally no user would ever these errors, but we have some systems that
    /// do not currently scale. Throwing an overloaded in the short term in
    /// these cases is preferable to the instance falling over.
    ///
    /// If the limit being hit is used for pagination limiting, use that error
    /// instead of this method.
    ///
    /// If you do not need a custom error message, do not use this method.
    /// Instead use anyhow without any ErrorMetadata, which will automatically
    /// be shown to the user as a generic internal server error.
    ///
    /// The short_msg should be a CapitalCamelCased describing the error (eg
    /// InvalidHeader). The msg should be a descriptive message targeted
    /// toward the developer.
    pub fn overloaded(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::Overloaded,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    // This is similar to `overloaded` but also guarantees the request was
    // rejected before it has been started. You should generally prefer to use
    // `overloaded`` instead of this error code and decide if an operation is safe
    // to retry based on the fact if its idempotent. This error code can be used
    // in very specific situations, e.g. actions that have been rejected before
    // they have been started, and thus can be safely retries.
    pub fn rejected_before_execution(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::RejectedBeforeExecution,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    /// Internal Optimistic Concurrency Control / Commit Race Error.
    ///
    /// These come from sqlx, or are caused by OCCs on system tables.
    pub fn system_occ() -> Self {
        Self {
            code: ErrorCode::OCC,
            short_msg: OCC_ERROR.into(),
            msg: OCC_ERROR_MSG.into(),
        }
    }

    /// User-caused Optimistic Concurrency Control / Commit Race Error
    pub fn user_occ(table_name: Option<String>, occ_write_source: Option<String>) -> Self {
        let table_description = table_name
            .map(|name| format!("the \"{name}\" table"))
            .unwrap_or("some table".to_owned());
        let write_source_description = occ_write_source
            .map(|source| format!("{}. ", source))
            .unwrap_or_default();
        Self {
            code: ErrorCode::OCC,
            short_msg: OCC_ERROR.into(),
            msg: format!(
                "Documents read from or written to {table_description} \
                changed while this mutation was being run and on every \
                subsequent retry. {write_source_description}See https://docs.convex.dev/error#1",
            )
            .into(),
        }
    }

    /// Out of Retention
    ///
    /// An error we produce if executing a read at a point that has been removed
    /// due to retention.
    pub fn out_of_retention() -> Self {
        Self {
            code: ErrorCode::OutOfRetention,
            short_msg: INTERNAL_SERVER_ERROR.into(),
            msg: INTERNAL_SERVER_ERROR_MSG.into(),
        }
    }

    /// Hit some kind of external facing pagination limit (eg too many
    /// documents, too much memory used).
    ///
    /// The short_msg should be a CapitalCamelCased describing the error (eg
    /// QueryScannedTooManyDocuments).
    /// The msg should be a descriptive message targeted toward the developer.
    pub fn pagination_limit(
        short_msg: impl Into<Cow<'static, str>>,
        msg: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code: ErrorCode::PaginationLimit,
            short_msg: short_msg.into(),
            msg: msg.into(),
        }
    }

    pub fn is_occ(&self) -> bool {
        self.code == ErrorCode::OCC
    }

    pub fn is_pagination_limit(&self) -> bool {
        self.code == ErrorCode::PaginationLimit
    }

    pub fn is_unauthenticated(&self) -> bool {
        self.code == ErrorCode::Unauthenticated
    }

    pub fn is_out_of_retention(&self) -> bool {
        self.code == ErrorCode::OutOfRetention
    }

    pub fn is_bad_request(&self) -> bool {
        self.code == ErrorCode::BadRequest
    }

    pub fn is_overloaded(&self) -> bool {
        self.code == ErrorCode::Overloaded
    }

    pub fn is_rejected_before_execution(&self) -> bool {
        self.code == ErrorCode::RejectedBeforeExecution
    }

    pub fn is_forbidden(&self) -> bool {
        self.code == ErrorCode::Forbidden
    }

    /// Return true if this error is deterministically caused by user. If so,
    /// we can propagate it into JS out of a syscall, and cache it if it is the
    /// full UDF result.
    pub fn is_deterministic_user_error(&self) -> bool {
        match self.code {
            ErrorCode::BadRequest
            | ErrorCode::NotFound
            | ErrorCode::PaginationLimit
            | ErrorCode::Unauthenticated
            | ErrorCode::Forbidden => true,
            ErrorCode::OperationalInternalServerError
            | ErrorCode::ClientDisconnect
            | ErrorCode::OCC
            | ErrorCode::OutOfRetention
            | ErrorCode::Overloaded
            | ErrorCode::RejectedBeforeExecution => false,
        }
    }

    pub fn should_report_to_sentry(&self) -> Option<(sentry::Level, Option<f64>)> {
        match self.code {
            ErrorCode::ClientDisconnect => None,
            ErrorCode::BadRequest
            | ErrorCode::NotFound
            | ErrorCode::PaginationLimit
            | ErrorCode::Unauthenticated
            | ErrorCode::Forbidden => Some((sentry::Level::Info, None)),
            ErrorCode::OutOfRetention
            | ErrorCode::Overloaded
            | ErrorCode::RejectedBeforeExecution
            | ErrorCode::OperationalInternalServerError => Some((sentry::Level::Warning, None)),

            // 1% sampling for OCC, since we only really care about the details if they
            // happen at high volume.
            ErrorCode::OCC => Some((sentry::Level::Warning, Some(0.01))),
        }
    }

    fn metric_server_error_tag_value(&self) -> Option<&'static str> {
        match self.code {
            ErrorCode::BadRequest
            | ErrorCode::NotFound
            | ErrorCode::PaginationLimit
            | ErrorCode::Unauthenticated
            | ErrorCode::Forbidden
            | ErrorCode::ClientDisconnect => None,
            ErrorCode::OCC => Some("occ"),
            ErrorCode::OutOfRetention => Some("out_of_retention"),
            ErrorCode::Overloaded => Some("overloaded"),
            ErrorCode::RejectedBeforeExecution => Some("rejected_before_execution"),
            ErrorCode::OperationalInternalServerError => Some("operational"),
        }
    }

    pub fn metric_server_error_tag(&self) -> Option<MetricTag> {
        self.metric_server_error_tag_value()
            .map(|v| metric_tag_const_value("type", v))
    }

    pub fn custom_metric(&self) -> Option<&'static IntCounter> {
        match self.code {
            ErrorCode::BadRequest => Some(&crate::metrics::BAD_REQUEST_ERROR_TOTAL),
            ErrorCode::ClientDisconnect => Some(&crate::metrics::CLIENT_DISCONNECT_ERROR_TOTAL),
            ErrorCode::Unauthenticated => Some(&crate::metrics::SYNC_AUTH_ERROR_TOTAL),
            ErrorCode::Forbidden => Some(&crate::metrics::FORBIDDEN_ERROR_TOTAL),
            ErrorCode::OCC => Some(&crate::metrics::COMMIT_RACE_TOTAL),
            ErrorCode::NotFound => None,
            ErrorCode::PaginationLimit => None,
            ErrorCode::OutOfRetention => None,
            ErrorCode::Overloaded => None,
            ErrorCode::RejectedBeforeExecution => None,
            ErrorCode::OperationalInternalServerError => None,
        }
    }

    pub fn close_frame(&self) -> Option<CloseFrame<'static>> {
        let code = match self.code {
            ErrorCode::NotFound
            | ErrorCode::PaginationLimit
            | ErrorCode::Forbidden
            | ErrorCode::ClientDisconnect => Some(CloseCode::Normal),
            ErrorCode::OCC
            | ErrorCode::OutOfRetention
            | ErrorCode::Overloaded
            | ErrorCode::RejectedBeforeExecution => Some(CloseCode::Again),
            ErrorCode::OperationalInternalServerError => Some(CloseCode::Error),
            // These ones are client errors - so no close code - the client
            // will handle and close the connection instead.
            ErrorCode::BadRequest | ErrorCode::Unauthenticated => None,
        }?;
        // According to the WebSocket protocol specification (RFC 6455), the reason
        // string (if present) is limited to 123 bytes. This is because the
        // Close frame may contain a body, with the first two bytes representing
        // the close code followed by the optional reason string. The whole
        // Close frame's payload is limited to 125 bytes, so after accounting for
        // the 2-byte close code, 123 bytes remain for the reason string.
        let mut reason = self.short_msg.to_string();
        reason.truncate(123);
        let reason = reason.into();
        Some(CloseFrame { code, reason })
    }
}

impl ErrorCode {
    fn http_status_code(&self) -> StatusCode {
        match self {
            ErrorCode::BadRequest | ErrorCode::PaginationLimit => StatusCode::BAD_REQUEST,
            // HTTP has the unfortunate naming of 401 as unauthorized when it's
            // really about authentication.
            // https://stackoverflow.com/questions/3297048/403-forbidden-vs-401-unauthorized-http-responses
            ErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::OperationalInternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::OCC
            | ErrorCode::OutOfRetention
            | ErrorCode::Overloaded
            | ErrorCode::RejectedBeforeExecution => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ClientDisconnect => StatusCode::REQUEST_TIMEOUT,
        }
    }

    pub fn grpc_status_code(&self) -> tonic::Code {
        match self {
            ErrorCode::BadRequest => tonic::Code::InvalidArgument,
            ErrorCode::Unauthenticated => tonic::Code::Unauthenticated,
            ErrorCode::Forbidden => tonic::Code::FailedPrecondition,
            ErrorCode::NotFound => tonic::Code::NotFound,
            ErrorCode::ClientDisconnect => tonic::Code::Aborted,
            ErrorCode::Overloaded | ErrorCode::RejectedBeforeExecution => {
                tonic::Code::ResourceExhausted
            },
            ErrorCode::OCC => tonic::Code::ResourceExhausted,
            ErrorCode::PaginationLimit => tonic::Code::InvalidArgument,
            ErrorCode::OutOfRetention => tonic::Code::OutOfRange,
            ErrorCode::OperationalInternalServerError => tonic::Code::Internal,
        }
    }

    pub fn from_http_status_code(code: StatusCode) -> Option<Self> {
        match code {
            StatusCode::UNAUTHORIZED => Some(ErrorCode::Unauthenticated),
            StatusCode::FORBIDDEN => Some(ErrorCode::Forbidden),
            StatusCode::NOT_FOUND => Some(ErrorCode::NotFound),
            // Tries to categorize in one of the above more specific 4xx codes first,
            // otherwise categorizes as a general 4xx via BadRequest
            v if v.is_client_error() => Some(ErrorCode::BadRequest),
            v if v.is_server_error() => Some(ErrorCode::Overloaded),
            _ => None,
        }
    }
}

pub trait ErrorMetadataAnyhowExt {
    fn is_occ(&self) -> bool;
    fn is_pagination_limit(&self) -> bool;
    fn is_unauthenticated(&self) -> bool;
    fn is_out_of_retention(&self) -> bool;
    fn is_bad_request(&self) -> bool;
    fn is_overloaded(&self) -> bool;
    fn is_rejected_before_execution(&self) -> bool;
    fn is_forbidden(&self) -> bool;
    fn should_report_to_sentry(&self) -> Option<(sentry::Level, Option<f64>)>;
    fn is_deterministic_user_error(&self) -> bool;
    fn user_facing_message(&self) -> String;
    fn short_msg(&self) -> &str;
    fn msg(&self) -> &str;
    fn metric_server_error_tag(&self) -> Option<MetricTag>;
    fn metric_status_tag_value(&self) -> &'static str;
    fn close_frame(&self) -> Option<CloseFrame<'static>>;
    fn http_status(&self) -> StatusCode;
    fn map_error_metadata<F: FnOnce(ErrorMetadata) -> ErrorMetadata>(self, f: F) -> Self;
    fn wrap_error_message<F>(self, f: F) -> Self
    where
        F: FnOnce(String) -> String;
    fn last_second_classification(self) -> Self;
}

impl ErrorMetadataAnyhowExt for anyhow::Error {
    /// Returns true if error is tagged as OCC
    fn is_occ(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_occ();
        }
        false
    }

    /// Returns true if error is tagged as PaginationLimit
    fn is_pagination_limit(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_pagination_limit();
        }
        false
    }

    /// Returns true if error is tagged as Unauthenticated
    fn is_unauthenticated(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_unauthenticated();
        }
        false
    }

    /// Returns true if error is tagged as OutOfRetention
    fn is_out_of_retention(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_out_of_retention();
        }
        false
    }

    /// Returns true if error is tagged as BadRequest
    fn is_bad_request(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_bad_request();
        }
        false
    }

    /// Returns true if error is tagged as Overloaded
    fn is_overloaded(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_overloaded();
        }
        false
    }

    /// Returns true if error is tagged as RejectedBeforeExecution
    fn is_rejected_before_execution(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_rejected_before_execution();
        }
        false
    }

    /// Returns true if error is tagged as Forbidden
    fn is_forbidden(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_forbidden();
        }
        false
    }

    /// Returns the level at which the given error should report to sentry
    /// INFO -> it's a client-at-fault error
    /// WARNING -> it's a server-at-fault error that is expected
    /// ERROR -> it's a server-at-fault error that is unexpected (probably a
    /// bug)
    /// FATAL -> it crashes the backend
    fn should_report_to_sentry(&self) -> Option<(sentry::Level, Option<f64>)> {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.should_report_to_sentry();
        }
        Some((sentry::Level::Error, None))
    }

    /// Return true if this error is deterministically caused by user. If so,
    /// we can propagate it into JS out of a syscall, and cache it if it is the
    /// full UDF result.
    /// We can also use it to determine if clients should be requested to retry.
    fn is_deterministic_user_error(&self) -> bool {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.is_deterministic_user_error();
        }
        false
    }

    fn user_facing_message(&self) -> String {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.to_string();
        }
        INTERNAL_SERVER_ERROR_MSG.to_string()
    }

    /// Return the short_msg associated with this Error
    fn short_msg(&self) -> &str {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return &e.short_msg;
        }
        INTERNAL_SERVER_ERROR
    }

    /// Return the descriptive msg associated with this Error
    fn msg(&self) -> &str {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return &e.msg;
        }
        INTERNAL_SERVER_ERROR_MSG
    }

    /// Return the tag to use on a server error metric
    fn metric_server_error_tag(&self) -> Option<MetricTag> {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.metric_server_error_tag();
        }
        Some(metric_tag_const("type:internal"))
    }

    /// Return the tag to use on a server status metric
    fn metric_status_tag_value(&self) -> &'static str {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return match e.metric_server_error_tag_value() {
                Some(v) => v,
                None => MetricTag::STATUS_DEVELOPER_ERROR.split_key_value().1,
            };
        }
        MetricTag::STATUS_ERROR.split_key_value().1
    }

    /// Return the CloseCode to use on websocket
    fn close_frame(&self) -> Option<CloseFrame<'static>> {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.close_frame();
        }
        Some(CloseFrame {
            code: CloseCode::Error,
            reason: INTERNAL_SERVER_ERROR.to_owned().into(),
        })
    }

    /// Return the HttpStatus code to use on response
    fn http_status(&self) -> StatusCode {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>() {
            return e.code.http_status_code();
        }
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn map_error_metadata<F>(self, f: F) -> Self
    where
        F: FnOnce(ErrorMetadata) -> ErrorMetadata,
    {
        if let Some(e) = self.downcast_ref::<ErrorMetadata>().cloned() {
            return self.context(f(e));
        }
        self
    }

    /// Wrap the underlying error message, maintaining the underlying error
    /// metadata short code if it exists.
    fn wrap_error_message<F>(self, f: F) -> Self
    where
        F: FnOnce(String) -> String,
    {
        if let Some(mut em) = self.downcast_ref::<ErrorMetadata>().cloned() {
            // Underlying ErrorMetadata. Reuse and reattach it.
            em.msg = f(em.msg.to_string()).into();
            return self.context(em);
        }

        // No underlying code. Just use .context()
        let new_msg = f(self.to_string());
        self.context(new_msg)
    }

    /// Escape hatch classification function.
    /// Call this near the edge of the system to do a last-second classification
    /// on the way out. This is not an ideal place to do error
    /// classification. It is much better to do it at the point it is being
    /// thrown.
    ///
    /// Reality is that in some cases it's not ergonomic or possible to classify
    /// during throw, so leaving ourselves an escape hatch
    fn last_second_classification(self) -> Self {
        // Each classification here should have a comment explaining why we're doing
        // it last second. We'd much rather prefer doing it at the time of throw.

        let as_string = self.to_string();
        // Just doing this as a quick hack because sqlx::query has 100 throw sites.
        // Ideally, we would wrap sqlx and do handling there, but punting to save time.
        let occs = [
            "could not serialize access due to read/write dependencies among transactions",
            "could not serialize access due to concurrent update",
        ];
        if occs.into_iter().any(|occ| as_string.contains(occ)) {
            return self.context(ErrorMetadata::system_occ()).context(as_string);
        }

        self
    }
}

pub const INTERNAL_SERVER_ERROR_MSG: &str = "Your request couldn't be completed. Try again later.";
pub const INTERNAL_SERVER_ERROR: &str = "InternalServerError";
pub const OCC_ERROR_MSG: &str = "Data read or written in \
                                 this mutation changed while it was being run. Consider reducing \
                                 the amount of data read by using indexed queries with selective \
                                 index range expressions (https://docs.convex.dev/database/indexes/).";
pub const OCC_ERROR: &str = "OptimisticConcurrencyControlFailure";
const CLIENT_DISCONNECTED_MSG: &str = "Your request couldn't be completed. Try again later.";
const CLIENT_DISCONNECTED: &str = "ClientDisconnected";

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::prelude::*;

    use super::{
        ErrorCode,
        ErrorMetadata,
    };

    impl Arbitrary for ErrorMetadata {
        type Parameters = ();

        type Strategy = impl Strategy<Value = Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<ErrorCode>().prop_map(|ec| match ec {
                ErrorCode::BadRequest => ErrorMetadata::bad_request("bad", "request"),
                ErrorCode::NotFound => ErrorMetadata::not_found("not", "found"),
                ErrorCode::PaginationLimit => {
                    ErrorMetadata::pagination_limit("pagination", "limit")
                },
                ErrorCode::OCC => ErrorMetadata::system_occ(),
                ErrorCode::OutOfRetention => ErrorMetadata::out_of_retention(),
                ErrorCode::Unauthenticated => ErrorMetadata::unauthenticated("un", "auth"),
                ErrorCode::Forbidden => ErrorMetadata::forbidden("for", "bidden"),
                ErrorCode::Overloaded => ErrorMetadata::overloaded("overloaded", "error"),
                ErrorCode::RejectedBeforeExecution => {
                    ErrorMetadata::rejected_before_execution("rejected_before_execution", "error")
                },
                ErrorCode::OperationalInternalServerError => {
                    ErrorMetadata::operational_internal_server_error()
                },
                ErrorCode::ClientDisconnect => ErrorMetadata::client_disconnect(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::{
        ErrorCode,
        ErrorMetadata,
        INTERNAL_SERVER_ERROR,
        OCC_ERROR,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_server_error_visibility(err in any::<ErrorMetadata>()) {
            // Error has visibility through sentry or custom metric.
            assert!(err.should_report_to_sentry().is_some() || err.custom_metric().is_some());
            if err.metric_server_error_tag().is_some() {
                assert!(err.should_report_to_sentry().unwrap().0 >= sentry::Level::Warning);
                if err.code == ErrorCode::Overloaded ||
                    err.code == ErrorCode::RejectedBeforeExecution {
                    // Overloaded messages come with custom messaging
                } else if err.code == ErrorCode::OCC {
                    assert_eq!(err.short_msg, OCC_ERROR);
                } else {
                    // User is informed that they are not responsible.
                    assert_eq!(err.short_msg, INTERNAL_SERVER_ERROR);
                }
            } else {
                if let Some((level, _)) = err.should_report_to_sentry() {
                    assert_eq!(level, sentry::Level::Info);
                }
                // User is responsible for error.
                assert_ne!(err.short_msg, INTERNAL_SERVER_ERROR);
            }
        }
    }
}
