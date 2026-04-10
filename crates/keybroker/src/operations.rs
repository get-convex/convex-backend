use errors::ErrorMetadata;
use pb::convex_identity::DeploymentOperation as ProtoDeploymentOperation;
use serde::{
    Deserialize,
    Serialize,
};

/// Operations that a deployment identity is allowed to perform.
///
/// Serializes to PascalCase strings (e.g. `"ViewLogs"`) for the HTTP API.
/// The `Unknown` variant is a catch-all for forward compatibility: if the
/// producer sends a new operation that this consumer doesn't recognize,
/// it deserializes as `Unknown` rather than failing the entire response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentOp {
    Deploy,
    ViewEnvironmentVariables,
    WriteEnvironmentVariables,
    PauseDeployment,
    UnpauseDeployment,
    ViewLogs,
    ViewMetrics,
    ViewIntegrations,
    WriteIntegrations,
    ViewData,
    WriteData,
    ViewBackups,
    CreateBackups,
    DownloadBackups,
    DeleteBackups,
    ImportBackups,
    ActAsUser,
    RunInternalQueries,
    RunInternalMutations,
    RunInternalActions,
    RunTestQuery,
    ViewAuditLog,
    #[serde(other)]
    Unknown,
}

impl From<DeploymentOp> for ProtoDeploymentOperation {
    fn from(op: DeploymentOp) -> ProtoDeploymentOperation {
        match op {
            DeploymentOp::Deploy => ProtoDeploymentOperation::Deploy,
            DeploymentOp::ViewEnvironmentVariables => {
                ProtoDeploymentOperation::ViewEnvironmentVariables
            },
            DeploymentOp::WriteEnvironmentVariables => {
                ProtoDeploymentOperation::WriteEnvironmentVariables
            },
            DeploymentOp::PauseDeployment => ProtoDeploymentOperation::PauseDeployment,
            DeploymentOp::UnpauseDeployment => ProtoDeploymentOperation::UnpauseDeployment,
            DeploymentOp::ViewLogs => ProtoDeploymentOperation::ViewLogs,
            DeploymentOp::ViewMetrics => ProtoDeploymentOperation::ViewMetrics,
            DeploymentOp::ViewIntegrations => ProtoDeploymentOperation::ViewIntegrations,
            DeploymentOp::WriteIntegrations => ProtoDeploymentOperation::WriteIntegrations,
            DeploymentOp::ViewData => ProtoDeploymentOperation::ViewData,
            DeploymentOp::WriteData => ProtoDeploymentOperation::WriteData,
            DeploymentOp::ViewBackups => ProtoDeploymentOperation::ViewBackups,
            DeploymentOp::CreateBackups => ProtoDeploymentOperation::CreateBackups,
            DeploymentOp::DownloadBackups => ProtoDeploymentOperation::DownloadBackups,
            DeploymentOp::DeleteBackups => ProtoDeploymentOperation::DeleteBackups,
            DeploymentOp::ImportBackups => ProtoDeploymentOperation::ImportBackups,
            DeploymentOp::ActAsUser => ProtoDeploymentOperation::ActAsUser,
            DeploymentOp::RunInternalQueries => ProtoDeploymentOperation::RunInternalQueries,
            DeploymentOp::RunInternalMutations => ProtoDeploymentOperation::RunInternalMutations,
            DeploymentOp::RunInternalActions => ProtoDeploymentOperation::RunInternalActions,
            DeploymentOp::RunTestQuery => ProtoDeploymentOperation::RunTestQuery,
            DeploymentOp::ViewAuditLog => ProtoDeploymentOperation::ViewAuditLog,
            DeploymentOp::Unknown => ProtoDeploymentOperation::Unspecified,
        }
    }
}

impl TryFrom<ProtoDeploymentOperation> for DeploymentOp {
    type Error = anyhow::Error;

    fn try_from(proto_op: ProtoDeploymentOperation) -> anyhow::Result<Self> {
        match proto_op {
            ProtoDeploymentOperation::Unspecified => {
                Err(anyhow::anyhow!("unspecified deployment operation"))
            },
            ProtoDeploymentOperation::Deploy => Ok(Self::Deploy),
            ProtoDeploymentOperation::ViewEnvironmentVariables => {
                Ok(Self::ViewEnvironmentVariables)
            },
            ProtoDeploymentOperation::WriteEnvironmentVariables => {
                Ok(Self::WriteEnvironmentVariables)
            },
            ProtoDeploymentOperation::PauseDeployment => Ok(Self::PauseDeployment),
            ProtoDeploymentOperation::UnpauseDeployment => Ok(Self::UnpauseDeployment),
            ProtoDeploymentOperation::ViewLogs => Ok(Self::ViewLogs),
            ProtoDeploymentOperation::ViewMetrics => Ok(Self::ViewMetrics),
            ProtoDeploymentOperation::ViewIntegrations => Ok(Self::ViewIntegrations),
            ProtoDeploymentOperation::WriteIntegrations => Ok(Self::WriteIntegrations),
            ProtoDeploymentOperation::ViewData => Ok(Self::ViewData),
            ProtoDeploymentOperation::WriteData => Ok(Self::WriteData),
            ProtoDeploymentOperation::ViewBackups => Ok(Self::ViewBackups),
            ProtoDeploymentOperation::CreateBackups => Ok(Self::CreateBackups),
            ProtoDeploymentOperation::DownloadBackups => Ok(Self::DownloadBackups),
            ProtoDeploymentOperation::DeleteBackups => Ok(Self::DeleteBackups),
            ProtoDeploymentOperation::ImportBackups => Ok(Self::ImportBackups),
            ProtoDeploymentOperation::ActAsUser => Ok(Self::ActAsUser),
            ProtoDeploymentOperation::RunInternalQueries => Ok(Self::RunInternalQueries),
            ProtoDeploymentOperation::RunInternalMutations => Ok(Self::RunInternalMutations),
            ProtoDeploymentOperation::RunInternalActions => Ok(Self::RunInternalActions),
            ProtoDeploymentOperation::RunTestQuery => Ok(Self::RunTestQuery),
            ProtoDeploymentOperation::ViewAuditLog => Ok(Self::ViewAuditLog),
        }
    }
}

pub fn bad_admin_key_error(instance_name: Option<String>) -> ErrorMetadata {
    let msg = match instance_name {
        Some(name) => format!(
            "The provided deploy key was invalid for deployment '{name}'. Double check that the \
             environment this key was generated for matches the desired deployment."
        ),
        None => "The provided deploy key was invalid for this deployment. Double check that the \
                 environment this key was generated for matches the desired deployment."
            .to_string(),
    };
    ErrorMetadata::forbidden("BadDeployKey", msg)
}

pub fn read_only_operations() -> Vec<DeploymentOp> {
    vec![
        DeploymentOp::ViewEnvironmentVariables,
        DeploymentOp::ViewLogs,
        DeploymentOp::ViewMetrics,
        DeploymentOp::ViewIntegrations,
        DeploymentOp::ViewData,
        DeploymentOp::ViewBackups,
        DeploymentOp::DownloadBackups,
        DeploymentOp::RunInternalQueries,
        DeploymentOp::RunTestQuery,
    ]
}

pub fn operations_for_deploy_key(is_read_only: bool) -> Vec<DeploymentOp> {
    if is_read_only {
        read_only_operations()
    } else {
        // Empty means all operations are allowed.
        vec![]
    }
}
