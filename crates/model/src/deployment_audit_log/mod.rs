use std::sync::LazyLock;

use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    obj,
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
    types::MemberId,
};
use database::{
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    ConvexObject,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

pub mod types;

use types::DeploymentAuditLogEvent;

use crate::{
    SystemIndex,
    SystemTable,
};

pub static DEPLOYMENT_AUDIT_LOG_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_deployment_audit_log"
        .parse()
        .expect("Invalid deployment audit log table")
});

pub static ACTION_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "action".parse().expect("invalid action field"));

pub struct DeploymentAuditLogsTable;
impl SystemTable for DeploymentAuditLogsTable {
    fn table_name(&self) -> &'static TableName {
        &DEPLOYMENT_AUDIT_LOG_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<DeploymentAuditLogEvent>::try_from(document).map(|_| ())
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
    ) -> anyhow::Result<Vec<ResolvedDocumentId>> {
        self.insert_with_member_override(events, None).await
    }

    pub async fn insert_with_member_override(
        &mut self,
        events: Vec<DeploymentAuditLogEvent>,
        member_id_override: Option<MemberId>,
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
        let mut deployment_audit_log_ids = vec![];
        for event in events {
            let event_object: ConvexObject = event.try_into()?;
            let event_object_with_member_id = match member_id_value {
                Some(member_id) => event_object.shallow_merge(obj!("member_id" => member_id)?)?,
                None => event_object.shallow_merge(obj!("member_id" => null)?)?,
            };
            let id = SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&DEPLOYMENT_AUDIT_LOG_TABLE, event_object_with_member_id)
                .await?;
            deployment_audit_log_ids.push(id);
        }
        Ok(deployment_audit_log_ids)
    }

    #[cfg(any(test, feature = "testing"))]
    pub async fn insert_single(
        &mut self,
        event: DeploymentAuditLogEvent,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let ids = self.insert(vec![event]).await?;
        Ok(ids[0])
    }

    pub async fn list(&mut self) -> anyhow::Result<Vec<ParsedDocument<DeploymentAuditLogEvent>>> {
        let value_query = Query::full_table_scan(DEPLOYMENT_AUDIT_LOG_TABLE.clone(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, value_query)?;
        let mut result = vec![];
        while let Some(doc) = query_stream.next(self.tx, None).await? {
            let row: ParsedDocument<DeploymentAuditLogEvent> = doc.try_into()?;
            result.push(row);
        }
        Ok(result)
    }
}
