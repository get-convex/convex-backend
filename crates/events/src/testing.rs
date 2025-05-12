use std::{
    collections::BTreeMap,
    sync::Arc,
};

use async_trait::async_trait;
use common::types::ModuleEnvironment;
use parking_lot::Mutex;

use crate::usage::{
    UsageEvent,
    UsageEventLogger,
};

#[derive(Debug, Clone)]
pub struct TestUsageEventLogger {
    state: Arc<Mutex<UsageCounterState>>,
}

impl TestUsageEventLogger {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(UsageCounterState::default()));
        Self { state }
    }

    pub fn collect(&self) -> UsageCounterState {
        let mut state = self.state.lock();
        UsageCounterState {
            recent_calls: std::mem::take(&mut state.recent_calls),
            recent_calls_by_tag: std::mem::take(&mut state.recent_calls_by_tag),
            recent_storage_ingress_size: std::mem::take(&mut state.recent_storage_ingress_size),
            recent_storage_egress_size: std::mem::take(&mut state.recent_storage_egress_size),
            recent_storage_calls: std::mem::take(&mut state.recent_storage_calls),
            recent_v8_action_compute_time: std::mem::take(&mut state.recent_v8_action_compute_time),
            recent_node_action_compute_time: std::mem::take(
                &mut state.recent_node_action_compute_time,
            ),
            recent_database_ingress_size: std::mem::take(&mut state.recent_database_ingress_size),
            recent_database_egress_size: std::mem::take(&mut state.recent_database_egress_size),
            recent_vector_ingress_size: std::mem::take(&mut state.recent_vector_ingress_size),
            recent_vector_egress_size: std::mem::take(&mut state.recent_vector_egress_size),
        }
    }
}

#[async_trait]
impl UsageEventLogger for TestUsageEventLogger {
    async fn record_async(&self, events: Vec<UsageEvent>) {
        let mut state = self.state.lock();
        for event in events {
            state.record_event(event);
        }
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

type TableName = String;
type FunctionName = String;
type StorageAPI = String;
type FunctionTag = String;

/// The state maintained by backend usage counters
#[derive(Default, Debug)]
pub struct UsageCounterState {
    pub recent_calls: BTreeMap<FunctionName, u64>,
    pub recent_calls_by_tag: BTreeMap<FunctionTag, u64>,
    pub recent_node_action_compute_time: BTreeMap<FunctionName, u64>,
    pub recent_v8_action_compute_time: BTreeMap<FunctionName, u64>,

    // Storage - note that we don't break storage by function since it can also
    // be called outside of function.
    pub recent_storage_calls: BTreeMap<StorageAPI, u64>,
    pub recent_storage_ingress_size: u64,
    pub recent_storage_egress_size: u64,

    // Bandwidth by table
    pub recent_database_ingress_size: BTreeMap<TableName, u64>,
    pub recent_database_egress_size: BTreeMap<TableName, u64>,
    pub recent_vector_ingress_size: BTreeMap<TableName, u64>,
    pub recent_vector_egress_size: BTreeMap<TableName, u64>,
}

impl UsageCounterState {
    fn record_event(&mut self, event: UsageEvent) {
        match event {
            UsageEvent::FunctionCall { fields } => {
                if fields.is_tracked {
                    let fn_name = if let Some(mut component) = fields.component_path {
                        component.push('/');
                        component.push_str(&fields.udf_id);
                        component
                    } else {
                        fields.udf_id.clone()
                    };
                    *self.recent_calls.entry(fn_name).or_default() += 1;
                    *self.recent_calls_by_tag.entry(fields.tag).or_default() += 1;

                    // Convert into MB-milliseconds of compute time
                    let value = fields.duration_millis * fields.memory_megabytes;
                    if fields.environment == ModuleEnvironment::Isolate.to_string() {
                        *self
                            .recent_v8_action_compute_time
                            .entry(fields.udf_id)
                            .or_default() += value;
                    } else if fields.environment == ModuleEnvironment::Node.to_string() {
                        *self
                            .recent_node_action_compute_time
                            .entry(fields.udf_id)
                            .or_default() += value;
                    }
                }
            },
            UsageEvent::FunctionStorageCalls { call, .. } => {
                *self.recent_storage_calls.entry(call.clone()).or_default() += 1;
            },
            UsageEvent::FunctionStorageBandwidth {
                ingress, egress, ..
            } => {
                self.recent_storage_ingress_size += ingress;
                self.recent_storage_egress_size += egress;
            },
            UsageEvent::StorageCall { call, .. } => {
                *self.recent_storage_calls.entry(call).or_default() += 1;
            },
            UsageEvent::StorageBandwidth {
                ingress, egress, ..
            } => {
                self.recent_storage_ingress_size += ingress;
                self.recent_storage_egress_size += egress;
            },
            UsageEvent::DatabaseBandwidth {
                table_name,
                ingress,
                egress,
                ..
            } => {
                *self
                    .recent_database_ingress_size
                    .entry(table_name.clone())
                    .or_default() += ingress;
                *self
                    .recent_database_egress_size
                    .entry(table_name)
                    .or_default() += egress;
            },
            UsageEvent::VectorBandwidth {
                table_name,
                ingress,
                egress,
                ..
            } => {
                *self
                    .recent_vector_ingress_size
                    .entry(table_name.clone())
                    .or_default() += ingress;
                *self
                    .recent_vector_egress_size
                    .entry(table_name)
                    .or_default() += egress;
            },
            UsageEvent::CurrentVectorStorage { tables: _ } => todo!(),
            UsageEvent::CurrentDatabaseStorage {
                tables: _,
                system_tables: _,
            } => todo!(),
            UsageEvent::CurrentFileStorage {
                tag: _,
                total_size: _,
                total_user_file_size: _,
                total_cloud_backup_size: _,
                total_snapshot_export_size: _,
            } => todo!(),
            UsageEvent::CurrentDocumentCounts {
                tables: _,
                system_tables: _,
            } => todo!(),
            UsageEvent::InsightReadLimit {
                id: _,
                request_id: _,
                udf_id: _,
                component_path: _,
                calls: _,
                success: _,
            } => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicTestUsageEventLogger {
    state: Arc<Mutex<Vec<UsageEvent>>>,
}

impl BasicTestUsageEventLogger {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(vec![]));
        Self { state }
    }

    pub fn record(&mut self, events: Vec<UsageEvent>) {
        let mut state = self.state.lock();
        state.extend(events);
    }

    pub fn collect(&self) -> Vec<UsageEvent> {
        let mut state = self.state.lock();
        std::mem::take(&mut *state)
    }
}

#[async_trait]
impl UsageEventLogger for BasicTestUsageEventLogger {
    async fn record_async(&self, events: Vec<UsageEvent>) {
        let mut state = self.state.lock();
        state.extend(events);
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
