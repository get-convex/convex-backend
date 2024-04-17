use std::fmt;

use common::{
    errors::JsError,
    log_lines::LogLines,
    RequestId,
};
use http::StatusCode;
use isolate::HttpActionResponse;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::json;
use sync_types::{
    types::ErrorPayload,
    LogLinesMessage,
};
use value::{
    sha256::Sha256,
    ConvexValue,
};

/// List of log lines from a Convex function execution, redacted to only
/// contain information that clients are allowed to see.
///
/// The level of redaction will depend on the configuration of the backend and
/// could be anything from full information to completely stripped out.
#[derive(Debug, Serialize, Deserialize)]
pub struct RedactedLogLines(Vec<String>);

impl RedactedLogLines {
    pub fn from_log_lines(log_lines: LogLines, block_logging: bool) -> Self {
        Self(if block_logging {
            vec![]
        } else {
            log_lines
                .into_iter()
                .map(|l| l.to_pretty_string())
                .collect()
        })
    }

    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<RedactedLogLines> for LogLinesMessage {
    fn from(l: RedactedLogLines) -> Self {
        Self(l.0)
    }
}

/// An Error emitted from a Convex Function execution. redacted to only
/// contain information that clients are allowed to see.
///
/// The level of redaction will depend on the configuration of the backend and
/// could be anything from full information to completely stripped out.
#[derive(thiserror::Error, Debug)]
pub struct RedactedJsError {
    error: JsError,
    block_logging: bool,
    request_id: RequestId,
}

impl RedactedJsError {
    pub fn from_js_error(error: JsError, block_logging: bool, request_id: RequestId) -> Self {
        Self {
            error,
            block_logging,
            request_id,
        }
    }

    // TODO: eliminate this method. CX-4094
    // This is because of a layering issue
    // where we are converting back and forth between redacted and unredacted form.
    // A refactor should make it possible to eliminate this. Unredaction is not
    // actually possible.
    // Note - this also happens to clean up duplicat request ids in nested
    // UDFs (e.g. action -> action that throws). If we remove this method, we can
    // switch to the nested_to_string method below when formatting to get a similar
    // result.
    pub fn pretend_to_unredact(self) -> JsError {
        if self.block_logging {
            return JsError::from_message("Server Error".to_string());
        }
        self.error
    }

    pub fn custom_data_if_any(self) -> Option<ConvexValue> {
        self.error.custom_data
    }

    pub fn into_error_payload(self) -> ErrorPayload<ConvexValue> {
        let message = format!("{self}");
        if let Some(data) = self.custom_data_if_any() {
            ErrorPayload::ErrorData { message, data }
        } else {
            ErrorPayload::Message(message)
        }
    }

    /// Update the given digest with the contents of this error in a way that's
    /// suitable for comparing the content, not than origin, of the error.
    ///
    /// request_id is excluded because it's based on the calling context and
    /// does not influence the content of the underlying error.
    pub fn deduplication_hash(&self, digest: &mut Sha256) {
        digest.update(self.error.to_string().as_bytes());
        digest.update(if self.block_logging { &[1u8] } else { &[0u8] });
    }

    /// Format the exception when it is or will be nested inside of another
    /// RedactedJsError
    ///
    /// In particular we don't want to print 'Server Error' or the request id
    /// multiple times in the same stack trace.
    pub fn nested_to_string(&self) -> String {
        if self.block_logging {
            "Server Error".to_string()
        } else {
            format!("{}", self.error)
        }
    }
}

impl fmt::Display for RedactedJsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Request ID: {}] Server Error", self.request_id)?;
        if !self.block_logging {
            write!(f, "\n{}", self.error)?;
        }
        Ok(())
    }
}

impl From<RedactedJsError> for HttpActionResponse {
    fn from(value: RedactedJsError) -> Self {
        let code = if value.block_logging {
            "Server Error".to_string()
        } else {
            format!("Server Error: {}", value.error.message.to_owned())
        };
        let code = format!("[Request ID: {}] {}", value.request_id, code);
        let mut body = json!({
            "code": code,
        });
        if !value.block_logging {
            body["trace"] = value.error.to_string().into();
        }
        if let Some(custom_data) = value.custom_data_if_any() {
            body["data"] = custom_data.into();
        }
        HttpActionResponse::from_json(StatusCode::INTERNAL_SERVER_ERROR, body)
    }
}

#[cfg(test)]
pub mod tests {
    use common::{
        errors::JsError,
        RequestId,
    };
    use isolate::HttpActionResponse;
    use must_let::must_let;
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;

    use crate::redaction::RedactedJsError;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn format_when_redacted_includes_request_id_but_no_error(
            js_error in any::<JsError>(), request_id in any::<RequestId>()
        ) {
            let redacted = RedactedJsError::from_js_error(js_error, true, request_id.clone());
            let formatted = format!("{redacted}");
            assert_eq!(formatted, format!("[Request ID: {}] Server Error", request_id));
        }

        #[test]
        fn format_when_not_redacted_includes_request_id_and_error(
            js_error in any::<JsError>(), request_id in any::<RequestId>()
        ) {
            let redacted =
                RedactedJsError::from_js_error(js_error.clone(), false, request_id.clone());
            let formatted = format!("{redacted}");
            assert_eq!(
                formatted,
                format!("[Request ID: {}] Server Error\n{}", request_id, js_error)
            );
        }

        #[test]
        fn http_response_when_edacted_includes_request_id_not_error(
            js_error in any::<JsError>(), request_id in any::<RequestId>()
        ) {
            let redacted =
                RedactedJsError::from_js_error(js_error.clone(), true, request_id.clone());
            let http_response = HttpActionResponse::from(redacted);
            let code = get_code(http_response);

            assert_eq!(code, format!("[Request ID: {}] Server Error", request_id));
        }

        #[test]
        fn http_response_when_not_redacted_includes_request_id_and_error(
            js_error in any::<JsError>(), request_id in any::<RequestId>()
        ) {
            let redacted =
                RedactedJsError::from_js_error(js_error.clone(), false, request_id.clone());
            let http_response = HttpActionResponse::from(redacted);
            let code = get_code(http_response);

            assert_eq!(
                code,
                format!(
                    "[Request ID: {}] Server Error: {}",
                    request_id,
                    js_error.message
                )
            );
        }
    }

    fn get_code(http_response: HttpActionResponse) -> String {
        let json = serde_json::from_slice(http_response.body().as_ref().unwrap()).unwrap();
        must_let!(let JsonValue::Object(map) = json);
        must_let!(let JsonValue::String(ref code) = map.get("code").unwrap());
        code.clone()
    }
}
