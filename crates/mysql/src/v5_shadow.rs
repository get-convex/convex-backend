//! The complete production dispatch boundary for optional V5 shadows.
use std::sync::Arc;

use common::{
    persistence::Persistence,
    runtime::Runtime,
    shutdown::ShutdownSignal,
};
use futures::future::BoxFuture;

use crate::{
    v5,
    ConvexMySqlPool,
    DeploymentDeleter,
    IndexesLogMaintenance,
    MySqlOptions,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "Consumed by the opt-in shadow implementation in the follow-up PR"
)]
pub(crate) enum ShadowMode {
    Off,
    Observe,
    Drain,
}

/// Off is decided before invoking any hook, including constructing maintenance
/// futures. Implementations own only the explicitly selected clusters.
#[async_trait::async_trait]
pub(crate) trait V5Shadow<RT: Runtime>: Send + Sync {
    fn mode(&self, cluster: &str) -> ShadowMode;

    async fn connect(
        &self,
        mode: ShadowMode,
        pool: Arc<ConvexMySqlPool<RT>>,
        db_name: String,
        options: MySqlOptions,
        shutdown: ShutdownSignal,
    ) -> anyhow::Result<Arc<dyn Persistence>>;

    fn maintenance(
        &self,
        mode: ShadowMode,
        runtime: RT,
        pool: Arc<ConvexMySqlPool<RT>>,
        db_name: String,
        owner: String,
    ) -> Option<BoxFuture<'static, IndexesLogMaintenance<RT>>>;

    async fn cleanup(&self, deleter: &DeploymentDeleter<RT>) -> anyhow::Result<()>;
}

pub(crate) struct NoOp;

#[async_trait::async_trait]
impl<RT: Runtime> V5Shadow<RT> for NoOp {
    fn mode(&self, _: &str) -> ShadowMode {
        ShadowMode::Off
    }

    async fn connect(
        &self,
        _: ShadowMode,
        pool: Arc<ConvexMySqlPool<RT>>,
        db_name: String,
        options: MySqlOptions,
        shutdown: ShutdownSignal,
    ) -> anyhow::Result<Arc<dyn Persistence>> {
        v5::connect(pool, db_name, options, shutdown).await
    }

    fn maintenance(
        &self,
        _: ShadowMode,
        _: RT,
        _: Arc<ConvexMySqlPool<RT>>,
        _: String,
        _: String,
    ) -> Option<BoxFuture<'static, IndexesLogMaintenance<RT>>> {
        None
    }

    async fn cleanup(&self, _: &DeploymentDeleter<RT>) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(crate) fn active_shadow<RT: Runtime>() -> impl V5Shadow<RT> {
    NoOp
}

pub(crate) fn connection_mode<RT: Runtime, S: V5Shadow<RT>>(
    shadow: &S,
    cluster: &str,
    multitenant: bool,
) -> ShadowMode {
    if multitenant {
        shadow.mode(cluster)
    } else {
        ShadowMode::Off
    }
}

/// None means the caller has no shadow task to spawn.
pub fn maintenance<RT: Runtime>(
    runtime: RT,
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    owner: String,
) -> Option<BoxFuture<'static, IndexesLogMaintenance<RT>>> {
    maintenance_with(active_shadow::<RT>(), runtime, pool, db_name, owner)
}

pub(crate) fn maintenance_with<RT: Runtime, S: V5Shadow<RT>>(
    shadow: S,
    runtime: RT,
    pool: Arc<ConvexMySqlPool<RT>>,
    db_name: String,
    owner: String,
) -> Option<BoxFuture<'static, IndexesLogMaintenance<RT>>> {
    let mode = shadow.mode(pool.cluster_name());
    if mode == ShadowMode::Off {
        return None;
    }
    shadow.maintenance(mode, runtime, pool, db_name, owner)
}

pub(crate) async fn cleanup<RT: Runtime>(deleter: &DeploymentDeleter<RT>) -> anyhow::Result<()> {
    cleanup_with(&active_shadow::<RT>(), deleter).await
}

pub(crate) async fn cleanup_with<RT: Runtime, S: V5Shadow<RT>>(
    shadow: &S,
    deleter: &DeploymentDeleter<RT>,
) -> anyhow::Result<()> {
    if deleter.v5_instance_name().is_none()
        || shadow.mode(deleter.pool().cluster_name()) == ShadowMode::Off
    {
        return Ok(());
    }
    shadow.cleanup(deleter).await
}
