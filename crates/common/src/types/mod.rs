//! Common types representing database identifiers.
pub use sync_types::{
    SessionId,
    SessionRequestSeqNumber,
    Timestamp,
};
use tuple_struct::{
    tuple_struct_string,
    tuple_struct_u64,
};
pub use value::{
    FieldName,
    FieldType,
    TableName,
};

mod actions;
mod admin_key;
mod backend_state;
mod environment_variables;
mod functions;
mod index;
mod maybe_value;
mod object_key;
mod table;
mod timestamp;

pub use actions::{
    ActionCallbackToken,
    HttpActionRoute,
    NodeDependency,
    RoutableMethod,
    SerializedHttpActionRoute,
};
pub use admin_key::{
    format_admin_key,
    remove_type_prefix_from_admin_key,
    remove_type_prefix_from_instance_name,
    split_admin_key,
    AdminKey,
    AdminKeyParts,
    PreviewDeploymentAdminKeyParts,
    SystemKey,
};
pub use backend_state::BackendState;
pub use environment_variables::{
    env_var_limit_met,
    env_var_name_forbidden,
    env_var_name_not_unique,
    EnvVarName,
    EnvVarValue,
    EnvironmentVariable,
    ENV_VAR_LIMIT,
};
pub use functions::{
    AllowedVisibility,
    FunctionCaller,
    ModuleEnvironment,
    UdfIdentifier,
    UdfType,
};
pub use index::{
    DatabaseIndexUpdate,
    DatabaseIndexValue,
    GenericIndexName,
    IndexDescriptor,
    IndexDiff,
    IndexId,
    IndexName,
    IndexTableIdentifier,
    StableIndexName,
    TabletIndexName,
    INDEX_BY_CREATION_TIME_DESCRIPTOR,
    INDEX_BY_ID_DESCRIPTOR,
};
pub use maybe_value::MaybeValue;
pub use object_key::ObjectKey;
pub use table::TableStats;
#[cfg(any(test, feature = "testing"))]
pub use timestamp::unchecked_repeatable_ts;
pub use timestamp::{
    RepeatableReason,
    RepeatableTimestamp,
    WriteTimestamp,
};

// A developer using convex
tuple_struct_u64!(MemberId);
tuple_struct_u64!(TeamId);
tuple_struct_string!(ConvexOrigin);
tuple_struct_string!(ConvexSite);

/// A unique id for a subscription.
pub type SubscriberId = usize;

/// We cursor through logs with a monotonic f64 of milliseconds since epoch.
pub type CursorMs = f64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistenceVersion {
    V5,
}

#[cfg(any(test, feature = "testing"))]
impl Default for PersistenceVersion {
    fn default() -> Self {
        Self::V5
    }
}

impl PersistenceVersion {
    /// When migrating to this PersistenceVersion causes index key encoding
    /// to change, return base_version + 1.
    /// After the migration is complete, bump base_version at all call-sites
    /// and return base_version here.
    pub fn index_key_version(&self, base_version: u8) -> u8 {
        match self {
            PersistenceVersion::V5 => base_version,
        }
    }

    pub fn version(&self) -> usize {
        match self {
            PersistenceVersion::V5 => 5,
        }
    }
}
