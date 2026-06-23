use std::sync::LazyLock;

use common::{
    document::CREATION_TIME_FIELD_PATH,
    execution_context::RequestMetadata,
    query::{
        Cursor,
        CursorPosition,
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        IndexName,
        MemberId,
    },
};
use database::{
    query::{
        PaginationOptions,
        TableFilter,
    },
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    val,
    ConvexObject,
    FieldName,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

mod developer_index_config;
pub mod types;

use types::{
    DeploymentAuditLogEntry,
    DeploymentAuditLogEvent,
};

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
            event_object = event_object.insert(
                const { FieldName::const_new("member_id") },
                match member_id_value {
                    Some(member_id) => val!(member_id),
                    None => val!(null),
                },
            )?;
            event_object = event_object.insert(
                const { FieldName::const_new("token_id") },
                match token_id {
                    Some(token_id) => val!(token_id),
                    None => val!(null),
                },
            )?;
            event_object = event_object.insert(
                const { FieldName::const_new("app_client_id") },
                match &app_client_id {
                    Some(app_client_id) => val!(app_client_id.as_str()),
                    None => val!(null),
                },
            )?;
            event_object = event_object.insert(
                const { FieldName::const_new("client_ip") },
                match &request_metadata.ip {
                    Some(ip) => val!(ip.as_str()),
                    None => val!(null),
                },
            )?;
            event_object = event_object.insert(
                const { FieldName::const_new("client_user_agent") },
                match &request_metadata.user_agent {
                    Some(user_agent) => val!(user_agent.as_str()),
                    None => val!(null),
                },
            )?;
            let id = SystemMetadataModel::new_global(self.tx)
                .insert_metadata(&DEPLOYMENT_AUDIT_LOG_TABLE, event_object)
                .await?;
            deployment_audit_log_ids.push(id);
        }
        Ok(deployment_audit_log_ids)
    }

    pub async fn list_events_from_time(
        &mut self,
        from_ts_ms: u64,
        cursor: Option<Cursor>,
        limit: usize,
    ) -> anyhow::Result<(Vec<DeploymentAuditLogEntry>, Option<Cursor>)> {
        let query = Query::index_range(IndexRange {
            index_name: IndexName::by_creation_time(DEPLOYMENT_AUDIT_LOG_TABLE.clone()),
            range: vec![IndexRangeExpression::Gte(
                CREATION_TIME_FIELD_PATH.clone(),
                (from_ts_ms as f64).into(),
            )],
            order: Order::Asc,
        });
        let mut query_stream = ResolvedQuery::new_bounded(
            self.tx,
            TableNamespace::Global,
            query,
            PaginationOptions::ManualPagination {
                start_cursor: cursor,
                maximum_rows_read: Some(limit),
                maximum_bytes_read: None,
            },
            None,
            TableFilter::IncludePrivateSystemTables,
        )?;

        let mut events = Vec::with_capacity(limit);
        while events.len() < limit
            && let Some(document) = query_stream.next(self.tx, None).await?
        {
            events.push(DeploymentAuditLogEntry::try_from(document)?);
        }

        let next_cursor = match query_stream.cursor() {
            Some(cursor) if !matches!(cursor.position, CursorPosition::End) => Some(cursor),
            _ => None,
        };
        Ok((events, next_cursor))
    }
}
