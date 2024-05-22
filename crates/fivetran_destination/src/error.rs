use thiserror::Error;

#[derive(Debug, Error)]
pub enum DestinationError {
    #[error("The key for a file is invalid")]
    InvalidKey,
}
