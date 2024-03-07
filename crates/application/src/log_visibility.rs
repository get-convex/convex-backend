use async_trait::async_trait;
use common::{
    runtime::Runtime,
    types::AllowedVisibility,
};
use database::Transaction;
use keybroker::Identity;

/// Public trait for handling logging visibility.
#[async_trait]
pub trait LogVisibility<RT: Runtime>: Send + Sync {
    async fn should_redact_logs_and_error(
        &self,
        tx: &mut Transaction<RT>,
        identity: Identity,
        allowed_visibility: AllowedVisibility,
    ) -> anyhow::Result<bool>;
}

pub struct AllowLogging;

#[async_trait]
impl<RT: Runtime> LogVisibility<RT> for AllowLogging {
    async fn should_redact_logs_and_error(
        &self,
        _tx: &mut Transaction<RT>,
        _identity: Identity,
        _allowed_visibility: AllowedVisibility,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }
}
