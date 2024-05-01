use std::{
    fmt::{
        self,
        Debug,
    },
    str::FromStr,
};

use metrics::StaticMetricLabel;
use pb::funrun::UdfType as UdfTypeProto;
use serde::{
    Deserialize,
    Serialize,
};
use sync_types::CanonicalizedUdfPath;
use value::{
    heap_size::HeapSize,
    id_v6::DocumentIdV6,
};

use super::HttpActionRoute;
use crate::version::ClientVersion;

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
    Function(CanonicalizedUdfPath),
    Http(HttpActionRoute),
    Cli(String),
}

impl fmt::Display for UdfIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UdfIdentifier::Function(path) => write!(f, "{}", path),
            UdfIdentifier::Http(route) => write!(f, "{}", route.path),
            UdfIdentifier::Cli(command) => write!(f, "_cli/{command}"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AllowedVisibility {
    PublicOnly,
    All,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum FunctionCaller {
    SyncWorker(ClientVersion),
    HttpApi(ClientVersion),
    Tester(ClientVersion),
    HttpEndpoint,
    Cron,
    Scheduler {
        job_id: DocumentIdV6,
    },
    Action {
        parent_scheduled_job: Option<DocumentIdV6>,
    },
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
        }
        .cloned()
    }

    pub fn parent_scheduled_job(&self) -> Option<DocumentIdV6> {
        match self {
            FunctionCaller::SyncWorker(_)
            | FunctionCaller::HttpApi(_)
            | FunctionCaller::Tester(_)
            | FunctionCaller::HttpEndpoint
            | FunctionCaller::Cron => None,
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
        };
        write!(f, "{s}")
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
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;

    use super::{
        UdfType,
        UdfTypeProto,
    };

    proptest! {
        #[test]
        fn test_udf_type_roundtrips(u in any::<UdfType>()) {
            assert_roundtrips::<UdfType, UdfTypeProto>(u);
        }
    }
}
