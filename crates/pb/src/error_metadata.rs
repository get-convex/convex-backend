use std::fmt::Display;

use anyhow::Context;
use errors::{
    ErrorCode,
    ErrorMetadata,
};
use prost::Message;

use crate::errors::{
    ErrorCode as ErrorCodeProto,
    ErrorMetadata as ErrorMetadataProto,
    OccInfo as OccInfoProto,
    StatusDetails as StatusDetailsProto,
};

impl From<ErrorCode> for ErrorCodeProto {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::BadRequest => ErrorCodeProto::BadRequest,
            ErrorCode::Conflict => ErrorCodeProto::Conflict,
            ErrorCode::Unauthenticated => ErrorCodeProto::Unauthenticated,
            ErrorCode::AuthUpdateFailed => ErrorCodeProto::AuthUpdateFailed,
            ErrorCode::Forbidden => ErrorCodeProto::Forbidden,
            ErrorCode::NotFound => ErrorCodeProto::TransientNotFound,
            ErrorCode::ClientDisconnect => ErrorCodeProto::ClientDisconnect,
            ErrorCode::RateLimited => ErrorCodeProto::RateLimited,
            ErrorCode::Overloaded => ErrorCodeProto::Overloaded,
            ErrorCode::FeatureTemporarilyUnavailable => {
                ErrorCodeProto::FeatureTemporarilyUnavailable
            },
            ErrorCode::RejectedBeforeExecution => ErrorCodeProto::RejectedBeforeExecution,
            ErrorCode::OCC { .. } => ErrorCodeProto::Occ,
            ErrorCode::PaginationLimit => ErrorCodeProto::PaginationLimit,
            ErrorCode::OutOfRetention => ErrorCodeProto::OutOfRetention,
            ErrorCode::OperationalInternalServerError => {
                ErrorCodeProto::OperationalInternalServerError
            },
            ErrorCode::MisdirectedRequest => ErrorCodeProto::MisdirectedRequest,
        }
    }
}

impl ErrorCodeProto {
    fn into_rust_type(self, occ_info: OccInfoProto) -> ErrorCode {
        match self {
            ErrorCodeProto::BadRequest => ErrorCode::BadRequest,
            ErrorCodeProto::Conflict => ErrorCode::Conflict,
            ErrorCodeProto::Unauthenticated => ErrorCode::Unauthenticated,
            ErrorCodeProto::AuthUpdateFailed => ErrorCode::AuthUpdateFailed,
            ErrorCodeProto::Forbidden => ErrorCode::Forbidden,
            ErrorCodeProto::TransientNotFound => ErrorCode::NotFound,
            ErrorCodeProto::ClientDisconnect => ErrorCode::ClientDisconnect,
            ErrorCodeProto::RateLimited => ErrorCode::RateLimited,
            ErrorCodeProto::Overloaded => ErrorCode::Overloaded,
            ErrorCodeProto::FeatureTemporarilyUnavailable => {
                ErrorCode::FeatureTemporarilyUnavailable
            },
            ErrorCodeProto::RejectedBeforeExecution => ErrorCode::RejectedBeforeExecution,
            ErrorCodeProto::Occ => ErrorCode::OCC {
                table_name: occ_info.table_name,
                document_id: occ_info.document_id,
                write_source: occ_info.write_source,
                is_system: occ_info.is_system,
            },
            ErrorCodeProto::PaginationLimit => ErrorCode::PaginationLimit,
            ErrorCodeProto::OutOfRetention => ErrorCode::OutOfRetention,
            ErrorCodeProto::OperationalInternalServerError => {
                ErrorCode::OperationalInternalServerError
            },
            ErrorCodeProto::MisdirectedRequest => ErrorCode::MisdirectedRequest,
        }
    }
}

impl From<ErrorMetadata> for ErrorMetadataProto {
    fn from(metadata: ErrorMetadata) -> Self {
        ErrorMetadataProto {
            code: ErrorCodeProto::from(metadata.code.clone()).into(),
            short_msg: Some(metadata.short_msg.to_string()),
            msg: Some(metadata.msg.to_string()),
            occ_info: match metadata.code {
                ErrorCode::OCC {
                    table_name,
                    document_id,
                    write_source,
                    is_system,
                } => Some(OccInfoProto {
                    table_name,
                    document_id,
                    write_source,
                    is_system,
                }),
                _ => None,
            },
            source: metadata.source,
        }
    }
}

impl TryFrom<ErrorMetadataProto> for ErrorMetadata {
    type Error = anyhow::Error;

    fn try_from(metadata: ErrorMetadataProto) -> anyhow::Result<Self> {
        let code = ErrorCodeProto::try_from(metadata.code)?
            .into_rust_type(metadata.occ_info.unwrap_or_default());
        let short_msg = metadata.short_msg.context("Missing `short_msg` field")?;
        let msg = metadata.msg.context("Missing `msg` field")?;
        Ok(Self {
            code,
            short_msg: short_msg.into(),
            msg: msg.into(),
            source: metadata.source,
        })
    }
}

pub trait ErrorMetadataStatusExt {
    fn from_anyhow(error: anyhow::Error) -> Self;
    fn into_anyhow(self) -> anyhow::Error;
    fn context<C>(self, context: C) -> anyhow::Error
    where
        C: Display + Send + Sync + 'static;
}

impl ErrorMetadataStatusExt for tonic::Status {
    fn from_anyhow(error: anyhow::Error) -> Self {
        let message = format!("{error:#}");
        if let Some(metadata) = error.downcast_ref::<ErrorMetadata>().cloned() {
            let code: tonic::Code = metadata.code.grpc_status_code();
            let details = StatusDetailsProto {
                error_metadata: Some(metadata.into()),
            };
            tonic::Status::with_details(code, message, details.encode_to_vec().into())
        } else {
            tonic::Status::internal(message)
        }
    }

    fn into_anyhow(self) -> anyhow::Error {
        let details = match StatusDetailsProto::decode(self.details()) {
            Ok(details) => details,
            Err(err) => {
                return anyhow::anyhow!("Failed to decode StatusDetails proto: {}", err);
            },
        };
        let mut error: anyhow::Error = self.into();
        if let Some(error_metadata) = details.error_metadata {
            let error_metadata = match ErrorMetadata::try_from(error_metadata) {
                Ok(error_metadata) => error_metadata,
                Err(err) => return err.context("Failed to parse ErrorMetadata proto"),
            };
            error = error.context(error_metadata)
        } else if error.downcast_ref::<tonic::transport::Error>().is_some() {
            error = error.context(ErrorMetadata::operational_internal_server_error());
        }
        error
    }

    fn context<C>(self, context: C) -> anyhow::Error
    where
        C: Display + Send + Sync + 'static,
    {
        self.into_anyhow().context(context)
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use errors::{
        ErrorMetadataAnyhowExt,
        INTERNAL_SERVER_ERROR_MSG,
    };
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::ErrorMetadata;
    use crate::{
        error_metadata::ErrorMetadataStatusExt,
        errors::ErrorMetadata as ErrorMetadataProto,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_error_metadata_roundtrips(left in any::<ErrorMetadata>()) {
            assert_roundtrips::<ErrorMetadata, ErrorMetadataProto>(left);
        }

        #[test]
        fn test_status_propagates_metadata(original_metadata in any::<ErrorMetadata>()) {
            let status = tonic::Status::from_anyhow(anyhow::anyhow!("Error").context(original_metadata.clone()));
            let error = status.into_anyhow();
            if let Some(received_metadata) = error.downcast_ref::<ErrorMetadata>() {
                assert_eq!(*received_metadata, original_metadata);
            } else {
                panic!("Didn't propagate error_metadata via Status");
            }
        }
    }

    #[test]
    fn test_status_no_error_metadata() {
        let status = tonic::Status::from_anyhow(anyhow::anyhow!("Error"));
        // Empty status details should parse as zero bytes.
        assert!(status.details().is_empty());

        // We should have no ErrorMetadata in the context.
        let error = status.into_anyhow();
        assert!(error.downcast_ref::<ErrorMetadata>().is_none());
    }

    #[test]
    fn test_context_no_error_metadata() {
        let status = tonic::Status::from_anyhow(anyhow::anyhow!("My special error"));

        let error = status.context("Test context");
        // Check the error we log to sentry includes the original error and the context
        let error_string = format!("{error:#}");
        assert!(error_string.contains("My special error"));
        assert!(error_string.contains("Test context"));

        // Check that the user facing portions haven't changed
        assert_eq!(error.user_facing_message(), INTERNAL_SERVER_ERROR_MSG);
    }

    #[test]
    fn test_context_with_error_metadata() {
        let status = tonic::Status::from_anyhow(
            ErrorMetadata::overloaded("ShortMsg", "Test long message").into(),
        );

        let error = status.context("Test context");
        // Check the error we log to sentry includes the original error and the context
        let error_string = format!("{error:#}");
        assert!(error_string.contains("Test long message"));
        assert!(error_string.contains("Test context"));

        // Check that the user facing portions haven't changed
        assert_eq!(error.user_facing_message(), "Test long message");
        assert_eq!(error.short_msg(), "ShortMsg")
    }
}
