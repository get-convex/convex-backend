use std::sync::LazyLock;

use common::{
    execution_context::RequestMetadata,
    obj,
    runtime::Runtime,
    types::MemberId,
};
use database::{
    unauthorized_error,
    SystemMetadataModel,
    Transaction,
};
use value::{
    ConvexObject,
    FieldPath,
    ResolvedDocumentId,
    TableName,
};

mod developer_index_config;
pub mod types;

use types::DeploymentAuditLogEvent;

use crate::{
    SystemIndex,
    SystemTable,
};

pub static DEPLOYMENT_AUDIT_LOG_TABLE: TableName = TableName::const_new("_deployment_audit_log");

pub static ACTION_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "action".parse().expect("invalid action field"));

pub struct DeploymentAuditLogsTable;
impl SystemTable for DeploymentAuditLogsTable {
    type Metadata = DeploymentAuditLogEvent;

    fn table_name() -> &'static TableName {
        &DEPLOYMENT_AUDIT_LOG_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }
}

pub struct DeploymentAuditLogModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> DeploymentAuditLogModel<'a, RT> {
    /// This fn should never be called directly. Instead use
    /// `commit_with_audit_log_events` in `application` to ensure that audit
    /// logs are streamed.
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn insert(
        &mut self,
        events: Vec<DeploymentAuditLogEvent>,
        request_metadata: &RequestMetadata,
    ) -> anyhow::Result<Vec<ResolvedDocumentId>> {
        self.insert_with_member_override(events, None, request_metadata)
            .await
    }

    pub async fn insert_with_member_override(
        &mut self,
        events: Vec<DeploymentAuditLogEvent>,
        member_id_override: Option<MemberId>,
        request_metadata: &RequestMetadata,
    ) -> anyhow::Result<Vec<ResolvedDocumentId>> {
        if !(self.tx.identity().is_system() || self.tx.identity().is_admin()) {
            anyhow::bail!(unauthorized_error("insert_deployment_audit_log_event"));
        }
        let member_id = member_id_override.or_else(|| self.tx.identity().member_id());
        let member_id_value = member_id
            .map(|member_id| {
                let member_id_u64: u64 = member_id.into();
                i64::try_from(member_id_u64)
            })
            .transpose()?;
        let token_id = self
            .tx
            .identity()
            .token_id()
            .map(|id| i64::try_from(id.0))
            .transpose()?;
        let app_client_id = self.tx.identity().app_client_id();
        let mut deployment_audit_log_ids = vec![];
        for event in events {
            let mut event_object: ConvexObject = event.try_into()?;
            event_object = match member_id_value {
                Some(member_id) => event_object.shallow_merge(obj!("member_id" => member_id)?)?,
                None => event_object.shallow_merge(obj!("member_id" => null)?)?,
            };
            event_object = match token_id {
                Some(token_id) => event_object.shallow_merge(obj!("token_id" => token_id)?)?,
                None => event_object.shallow_merge(obj!("token_id" => null)?)?,
            };
            event_object = match app_client_id {
                Some(ref app_client_id) => {
                    event_object.shallow_merge(obj!("app_client_id" => app_client_id.as_str())?)?
                },
                None => event_object.shallow_merge(obj!("app_client_id" => null)?)?,
            };
            event_object = match request_metadata.ip.clone() {
                Some(ip) => event_object.shallow_merge(obj!("client_ip" => ip.into_string())?)?,
                None => event_object.shallow_merge(obj!("client_ip" => null)?)?,
            };
            event_object = match request_metadata.user_agent.clone() {
                Some(user_agent) => event_object
                    .shallow_merge(obj!("client_user_agent" => user_agent.into_string())?)?,
                None => event_object.shallow_merge(obj!("client_user_agent" => null)?)?,
            };
            let id = SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&DEPLOYMENT_AUDIT_LOG_TABLE, event_object)
                .await?;
            deployment_audit_log_ids.push(id);
        }
        Ok(deployment_audit_log_ids)
    }
}
