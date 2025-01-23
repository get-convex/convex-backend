use std::{
    fmt::{
        self,
        Debug,
    },
    str::FromStr,
};

use anyhow::Context;
use metrics::StaticMetricLabel;
use pb::common::UdfType as UdfTypeProto;
use serde::{
    Deserialize,
    Serialize,
};
use value::{
    heap_size::HeapSize,
    id_v6::DeveloperDocumentId,
};

use super::HttpActionRoute;
use crate::{
    components::CanonicalizedComponentFunctionPath,
    version::ClientVersion,
};

#[derive(Serialize, Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum UdfType {
    Query,
    Mutation,
    Action,
    HttpAction,
}

impl UdfType {
    pub fn metric_label(self) -> StaticMetricLabel {
        StaticMetricLabel::new("udf_type", self.to_lowercase_string())
    }

    pub fn to_lowercase_string(self) -> &'static str {
        match self {
            UdfType::Query => "query",
            UdfType::Mutation => "mutation",
            UdfType::Action => "action",
            UdfType::HttpAction => "http_action",
        }
    }
}

impl FromStr for UdfType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Query" | "query" => Ok(Self::Query),
            "Mutation" | "mutation" => Ok(Self::Mutation),
            "Action" | "action" => Ok(Self::Action),
            "HttpEndpoint" | "httpEndpoint" | "HttpAction" | "httpAction" => Ok(Self::HttpAction),
            _ => anyhow::bail!("Expected UdfType, got {:?}", s),
        }
    }
}

impl fmt::Display for UdfType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UdfType::Query => "Query",
            UdfType::Mutation => "Mutation",
            UdfType::Action => "Action",
            UdfType::HttpAction => "HttpAction",
        };
        write!(f, "{s}")
    }
}

impl HeapSize for UdfType {
    fn heap_size(&self) -> usize {
        0
    }
}

impl From<UdfType> for UdfTypeProto {
    fn from(u: UdfType) -> UdfTypeProto {
        match u {
            UdfType::Query => UdfTypeProto::Query,
            UdfType::Mutation => UdfTypeProto::Mutation,
            UdfType::Action => UdfTypeProto::Action,
            UdfType::HttpAction => UdfTypeProto::HttpAction,
        }
    }
}

impl From<UdfTypeProto> for UdfType {
    fn from(u: UdfTypeProto) -> UdfType {
        match u {
            UdfTypeProto::Query => UdfType::Query,
            UdfTypeProto::Mutation => UdfType::Mutation,
            UdfTypeProto::Action => UdfType::Action,
            UdfTypeProto::HttpAction => UdfType::HttpAction,
        }
    }
}

/// A unique identifier for a UDF
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum UdfIdentifier {
    Function(CanonicalizedComponentFunctionPath),
    Http(HttpActionRoute),
    SystemJob(String),
}

impl UdfIdentifier {
    pub fn into_component_and_udf_path(self) -> (Option<String>, String) {
        match self {
            UdfIdentifier::Function(path) => {
                let (component_path, udf_path) = path.clone().into_component_and_udf_path();
                (component_path.serialize(), udf_path.to_string())
            },
            UdfIdentifier::Http(_) | UdfIdentifier::SystemJob(_) => (None, self.to_string()),
        }
    }
}

impl fmt::Display for UdfIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UdfIdentifier::Function(path) => write!(f, "{}", path.debug_str()),
            UdfIdentifier::Http(route) => write!(f, "{}", route.path),
            UdfIdentifier::SystemJob(command) => write!(f, "_system_job/{command}"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
pub enum AllowedVisibility {
    PublicOnly,
    All,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum FunctionCaller {
    SyncWorker(ClientVersion),
    HttpApi(ClientVersion),
    /// Used by function tester in the dashboard
    Tester(ClientVersion),
    // This is a user defined http actions called externally. If the http action
    // calls other functions, their caller would be `Action`.
    HttpEndpoint,
    Cron,
    Scheduler {
        job_id: DeveloperDocumentId,
    },
    Action {
        parent_scheduled_job: Option<DeveloperDocumentId>,
    },
    #[cfg(any(test, feature = "testing"))]
    #[proptest(weight = 0)]
    Test,
}

impl FunctionCaller {
    pub fn client_version(&self) -> Option<ClientVersion> {
        match self {
            FunctionCaller::SyncWorker(c) => Some(c),
            FunctionCaller::HttpApi(c) => Some(c),
            FunctionCaller::Tester(c) => Some(c),
            FunctionCaller::HttpEndpoint
            | FunctionCaller::Cron
            | FunctionCaller::Scheduler { .. }
            | FunctionCaller::Action { .. } => None,
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => None,
        }
        .cloned()
    }

    pub fn parent_scheduled_job(&self) -> Option<DeveloperDocumentId> {
        match self {
            FunctionCaller::SyncWorker(_)
            | FunctionCaller::HttpApi(_)
            | FunctionCaller::Tester(_)
            | FunctionCaller::HttpEndpoint
            | FunctionCaller::Cron => None,
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => None,
            FunctionCaller::Scheduler { job_id } => Some(*job_id),
            FunctionCaller::Action {
                parent_scheduled_job,
            } => *parent_scheduled_job,
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            FunctionCaller::SyncWorker(_)
            | FunctionCaller::HttpApi(_)
            | FunctionCaller::Tester(_)
            | FunctionCaller::HttpEndpoint
            | FunctionCaller::Cron
            | FunctionCaller::Scheduler { .. } => true,
            FunctionCaller::Action { .. } => false,
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => true,
        }
    }

    pub fn run_until_completion_if_cancelled(&self) -> bool {
        // If the action is called from a web socket or http we want to continue
        // to run it even if the client goes away. However, we preserve the right
        // to interrupt actions if the backend restarts.
        match self {
            FunctionCaller::SyncWorker(_)
            | FunctionCaller::HttpApi(_)
            | FunctionCaller::HttpEndpoint
            | FunctionCaller::Tester(_) => true,
            FunctionCaller::Cron
            | FunctionCaller::Scheduler { .. }
            | FunctionCaller::Action { .. } => false,
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => true,
        }
    }

    pub fn allowed_visibility(&self) -> AllowedVisibility {
        match self {
            FunctionCaller::SyncWorker(_) | FunctionCaller::HttpApi(_) => {
                AllowedVisibility::PublicOnly
            },
            // NOTE: Allowed visibility doesn't make sense in the context of an
            // user defined http action since all http actions are public, and
            // we shouldn't be checking visibility. We define this for completeness.
            FunctionCaller::HttpEndpoint => AllowedVisibility::PublicOnly,
            FunctionCaller::Tester(_)
            | FunctionCaller::Cron
            | FunctionCaller::Scheduler { .. }
            | FunctionCaller::Action { .. } => AllowedVisibility::All,
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => AllowedVisibility::PublicOnly,
        }
    }
}

impl fmt::Display for FunctionCaller {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FunctionCaller::SyncWorker(_) => "SyncWorker",
            FunctionCaller::HttpApi(_) => "HttpApi",
            FunctionCaller::Tester(_) => "Tester",
            FunctionCaller::HttpEndpoint => "HttpEndpoint",
            FunctionCaller::Cron => "Cron",
            FunctionCaller::Scheduler { .. } => "Scheduler",
            FunctionCaller::Action { .. } => "Action",
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => "Test",
        };
        write!(f, "{s}")
    }
}

impl From<FunctionCaller> for pb::common::FunctionCaller {
    fn from(caller: FunctionCaller) -> Self {
        let caller = match caller {
            FunctionCaller::SyncWorker(client_version) => {
                pb::common::function_caller::Caller::SyncWorker(client_version.into())
            },
            FunctionCaller::HttpApi(client_version) => {
                pb::common::function_caller::Caller::HttpApi(client_version.into())
            },
            FunctionCaller::Tester(client_version) => {
                pb::common::function_caller::Caller::Tester(client_version.into())
            },
            FunctionCaller::HttpEndpoint => pb::common::function_caller::Caller::HttpEndpoint(()),
            FunctionCaller::Cron => pb::common::function_caller::Caller::Cron(()),
            FunctionCaller::Scheduler { job_id } => {
                let caller = pb::common::SchedulerFunctionCaller {
                    job_id: Some(job_id.into()),
                };
                pb::common::function_caller::Caller::Scheduler(caller)
            },
            FunctionCaller::Action {
                parent_scheduled_job,
            } => {
                let caller = pb::common::ActionFunctionCaller {
                    parent_scheduled_job: parent_scheduled_job.map(|job_id| job_id.into()),
                };
                pb::common::function_caller::Caller::Action(caller)
            },
            #[cfg(any(test, feature = "testing"))]
            FunctionCaller::Test => panic!("Can't use test function caller"),
        };
        Self {
            caller: Some(caller),
        }
    }
}

impl TryFrom<pb::common::FunctionCaller> for FunctionCaller {
    type Error = anyhow::Error;

    fn try_from(msg: pb::common::FunctionCaller) -> anyhow::Result<Self> {
        let caller = match msg.caller {
            Some(pb::common::function_caller::Caller::SyncWorker(client_version)) => {
                FunctionCaller::SyncWorker(client_version.try_into()?)
            },
            Some(pb::common::function_caller::Caller::HttpApi(client_version)) => {
                FunctionCaller::HttpApi(client_version.try_into()?)
            },
            Some(pb::common::function_caller::Caller::Tester(client_version)) => {
                FunctionCaller::Tester(client_version.try_into()?)
            },
            Some(pb::common::function_caller::Caller::HttpEndpoint(())) => {
                FunctionCaller::HttpEndpoint
            },
            Some(pb::common::function_caller::Caller::Cron(())) => FunctionCaller::Cron,
            Some(pb::common::function_caller::Caller::Scheduler(caller)) => {
                let pb::common::SchedulerFunctionCaller { job_id } = caller;
                let job_id = job_id.context("Missing `job_id` field")?.try_into()?;
                FunctionCaller::Scheduler { job_id }
            },
            Some(pb::common::function_caller::Caller::Action(caller)) => {
                let pb::common::ActionFunctionCaller {
                    parent_scheduled_job,
                } = caller;
                let parent_scheduled_job = parent_scheduled_job
                    .map(|job_id| job_id.try_into())
                    .transpose()?;
                FunctionCaller::Action {
                    parent_scheduled_job,
                }
            },
            None => anyhow::bail!("Missing `caller` field"),
        };
        Ok(caller)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum ModuleEnvironment {
    Isolate,
    Node,
    /// The function doesn't exist (the argument/path are invalid/no accessible
    /// to the caller or analyze fails)
    Invalid,
}

impl FromStr for ModuleEnvironment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let environment = match s {
            "node" => ModuleEnvironment::Node,
            "isolate" => ModuleEnvironment::Isolate,
            "invalid" => ModuleEnvironment::Invalid,
            _ => anyhow::bail!("Invalid environment {s}"),
        };
        Ok(environment)
    }
}

impl fmt::Display for ModuleEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ModuleEnvironment::Isolate => "isolate",
            ModuleEnvironment::Node => "node",
            ModuleEnvironment::Invalid => "invalid",
        };
        write!(f, "{s}")
    }
}

impl ModuleEnvironment {
    pub fn as_sentry_tag(&self) -> &'static str {
        match self {
            // "isolate" is an internal term. Simply the default environment externally.
            ModuleEnvironment::Isolate => "default",
            ModuleEnvironment::Node => "node",
            ModuleEnvironment::Invalid => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;

    use super::{
        UdfType,
        UdfTypeProto,
    };
    use crate::types::FunctionCaller;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_udf_type_roundtrips(u in any::<UdfType>()) {
            assert_roundtrips::<UdfType, UdfTypeProto>(u);
        }

        #[test]
        fn test_function_caller_roundtrips(u in any::<FunctionCaller>()) {
            assert_roundtrips::<FunctionCaller, pb::common::FunctionCaller>(u);
        }
    }
}
