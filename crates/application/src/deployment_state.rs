use common::runtime::Runtime;
use keybroker::Identity;
use model::{
    backend_state::{
        types::BackendState,
        BackendStateModel,
    },
    deployment_audit_log::types::DeploymentAuditLogEvent,
};

use crate::Application;

impl<RT: Runtime> Application<RT> {
    pub async fn change_deployment_state(
        &self,
        identity: Identity,
        new_state: BackendState,
    ) -> anyhow::Result<()> {
        let mut tx = self.begin(identity).await?;
        let mut model = BackendStateModel::new(&mut tx);
        let old_state = model.get_backend_state().await?.into_value();
        model.toggle_backend_state(new_state).await?;
        let deployment_audit_log_event = DeploymentAuditLogEvent::ChangeDeploymentState {
            old_state,
            new_state,
        };
        self.commit_with_audit_log_events(
            tx,
            vec![deployment_audit_log_event],
            "change_deployment_state",
        )
        .await?;
        Ok(())
    }
}
