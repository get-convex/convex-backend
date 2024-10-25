use std::fmt;

use anyhow::Context;
use common::{
    errors::JsError,
    log_lines::LogLines,
    RequestId,
};
use http::StatusCode;
use isolate::HttpActionResponsePart;
use pb::common::{
    RedactedJsError as RedactedJsErrorProto,
    RedactedLogLines as RedactedLogLinesProto,
};
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
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Clone, PartialEq)
)]
pub struct RedactedLogLines(Vec<String>);

impl RedactedLogLines {
    pub fn from_log_lines(log_lines: LogLines, block_logging: bool) -> Self {
        Self(if block_logging {
            vec![]
        } else {
            log_lines
                .into_iter()
                .flat_map(|l| l.to_pretty_strings())
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

impl From<RedactedLogLines> for RedactedLogLinesProto {
    fn from(value: RedactedLogLines) -> Self {
        Self { log_lines: value.0 }
    }
}

impl TryFrom<RedactedLogLinesProto> for RedactedLogLines {
    type Error = anyhow::Error;

    fn try_from(msg: RedactedLogLinesProto) -> anyhow::Result<Self> {
        Ok(Self(msg.log_lines))
    }
}

/// An Error emitted from a Convex Function execution. redacted to only
/// contain information that clients are allowed to see.
///
/// The level of redaction will depend on the configuration of the backend and
/// could be anything from full information to completely stripped out.
#[derive(thiserror::Error, Debug)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(proptest_derive::Arbitrary, Clone, PartialEq)
)]
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

    pub fn to_http_response_parts(self) -> Vec<HttpActionResponsePart> {
        let code = if self.block_logging {
            "Server Error".to_string()
        } else {
            format!("Server Error: {}", self.error.message.to_owned())
        };
        let code = format!("[Request ID: {}] {}", self.request_id, code);
        let mut body = json!({
            "code": code,
        });
        if !self.block_logging {
            body["trace"] = self.error.to_string().into();
        }
        if let Some(custom_data) = self.custom_data_if_any() {
            body["data"] = custom_data.into();
        }
        HttpActionResponsePart::from_json(StatusCode::INTERNAL_SERVER_ERROR, body)
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

impl TryFrom<RedactedJsError> for RedactedJsErrorProto {
    type Error = anyhow::Error;

    fn try_from(value: RedactedJsError) -> anyhow::Result<Self> {
        Ok(Self {
            error: Some(value.error.try_into()?),
            block_logging: Some(value.block_logging),
            request_id: Some(value.request_id.into()),
        })
    }
}

impl TryFrom<RedactedJsErrorProto> for RedactedJsError {
    type Error = anyhow::Error;

    fn try_from(msg: RedactedJsErrorProto) -> anyhow::Result<Self> {
        let error = msg.error.context("Missing `error` field")?.try_into()?;
        let block_logging = msg.block_logging.context("Missing `block_logging` field")?;
        let request_id = msg
            .request_id
            .context("Missing `request_id` field")?
            .try_into()?;
        Ok(RedactedJsError {
            error,
            block_logging,
            request_id,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use cmd_util::env::env_config;
    use common::{
        errors::JsError,
        RequestId,
    };
    use isolate::HttpActionResponsePart;
    use must_let::must_let;
    use pb::common::{
        RedactedJsError as RedactedJsErrorProto,
        RedactedLogLines as RedactedLogLinesProto,
    };
    use proptest::prelude::*;
    use serde_json::Value as JsonValue;
    use value::testing::assert_roundtrips;

    use crate::redaction::{
        RedactedJsError,
        RedactedLogLines,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
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
            let http_response_parts = redacted.to_http_response_parts();
            let code = get_code(http_response_parts);

            assert_eq!(code, format!("[Request ID: {}] Server Error", request_id));
        }

        #[test]
        fn http_response_when_not_redacted_includes_request_id_and_error(
            js_error in any::<JsError>(), request_id in any::<RequestId>()
        ) {
            let redacted =
                RedactedJsError::from_js_error(js_error.clone(), false, request_id.clone());
            let http_response_parts = redacted.to_http_response_parts();
            let code = get_code(http_response_parts);

            assert_eq!(
                code,
                format!(
                    "[Request ID: {}] Server Error: {}",
                    request_id,
                    js_error.message
                )
            );
        }

        #[test]
        fn test_redacted_js_error_roundtrips(left in any::<RedactedJsError>()) {
            assert_roundtrips::<RedactedJsError, RedactedJsErrorProto>(left);
        }

        #[test]
        fn test_redacted_log_lines_roundtrips(left in any::<RedactedLogLines>()) {
            assert_roundtrips::<RedactedLogLines, RedactedLogLinesProto>(left);
        }
    }

    fn get_code(http_response_parts: Vec<HttpActionResponsePart>) -> String {
        let mut body_bytes = vec![];
        for part in http_response_parts {
            match part {
                HttpActionResponsePart::BodyChunk(b) => body_bytes.extend(b),
                HttpActionResponsePart::Head(_) => (),
            }
        }
        let json = serde_json::from_slice(&body_bytes).unwrap();
        must_let!(let JsonValue::Object(map) = json);
        must_let!(let JsonValue::String(ref code) = map.get("code").unwrap());
        code.clone()
    }
}
