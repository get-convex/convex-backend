use std::{
    error::Error as StdError,
    io::{
        Error as IoError,
        ErrorKind as IoErrorKind,
    },
};

use tungstenite::error::{
    Error as TungsteniteError,
    ProtocolError,
};

pub fn is_connection_closed_error(e: &(dyn StdError + 'static)) -> bool {
    // There's some error handling sloppiness in the axum -> tokio-tungstenite ->
    // tungstenite close path, so we get an error on successful close. Only
    // log if we can't downcast to tungstenite's original error.
    e.sources().any(|e| {
        matches!(
            e.downcast_ref(),
            Some(
                TungsteniteError::ConnectionClosed
                    | TungsteniteError::AlreadyClosed
                    | TungsteniteError::Protocol(
                        ProtocolError::SendAfterClosing
                        | tungstenite::error::ProtocolError::ResetWithoutClosingHandshake
                        // Mobile safari sometimes sends opcode 11 which we consider invalid.
                        // Just count it as a client disconnect.
                        | tungstenite::error::ProtocolError::InvalidOpcode(11)
                    )
            )
        ) || matches!(
            e.downcast_ref::<IoError>(),
            Some(e) if matches!(e.kind(), IoErrorKind::BrokenPipe | IoErrorKind::ConnectionReset)
        )
    })
}
