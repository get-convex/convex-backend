use async_trait::async_trait;
use common::{
    runtime::Runtime,
    types::AllowedVisibility,
};
use database::Transaction;
use keybroker::Identity;

/// Trait for handling logging visibility.
#[async_trait]
pub trait LogVisibility<RT: Runtime>: Send + Sync {
    /// If true, then block logging from reaching clients unless they have
    /// admin authorization.
    async fn should_redact_logs_and_error(
        &self,
        tx: &mut Transaction<RT>,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
    ) -> anyhow::Result<bool>;
}

pub struct RedactLogsToClient {
    redact: bool,
}

impl RedactLogsToClient {
    pub fn new(redact: bool) -> Self {
        Self { redact }
    }
}

#[async_trait]
impl<RT: Runtime> LogVisibility<RT> for RedactLogsToClient {
    async fn should_redact_logs_and_error(
        &self,
        _tx: &mut Transaction<RT>,
        _identity: Identity,
        _allowed_visibility: AllowedVisibility,
    ) -> anyhow::Result<bool> {
        Ok(self.redact)
    }
}
