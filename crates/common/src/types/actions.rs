use std::{
    fmt::{
        self,
        Debug,
        Display,
    },
    str::FromStr,
};

use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use value::codegen_convex_serialization;

use crate::heap_size::HeapSize;

/// Token that give Node executor permissions to use the actions internal API.
pub type ActionCallbackToken = String;

/// Represents an external dependency that should be installed and uploaded
/// separately in Lambda. TODO: parse version instead of relying on strings
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NodeDependency {
    pub package: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
struct SerializedNodeDependency {
    package: String,
    version: String,
}

impl TryFrom<NodeDependency> for SerializedNodeDependency {
    type Error = anyhow::Error;

    fn try_from(dep: NodeDependency) -> Result<Self, Self::Error> {
        Ok(Self {
            package: dep.package,
            version: dep.version,
        })
    }
}

impl TryFrom<SerializedNodeDependency> for NodeDependency {
    type Error = anyhow::Error;

    fn try_from(dep: SerializedNodeDependency) -> Result<Self, Self::Error> {
        Ok(Self {
            package: dep.package,
            version: dep.version,
        })
    }
}

codegen_convex_serialization!(NodeDependency, SerializedNodeDependency);

impl From<NodeDependency> for JsonValue {
    fn from(dep: NodeDependency) -> Self {
        json!({
            "package": dep.package,
            "version": dep.version
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum RoutableMethod {
    Delete,
    Get,
    Options,
    Patch,
    Post,
    Put,
}

impl FromStr for RoutableMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DELETE" => Ok(Self::Delete),
            "GET" => Ok(Self::Get),
            "OPTIONS" => Ok(Self::Options),
            "PATCH" => Ok(Self::Patch),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "HEAD" => Ok(Self::Get),
            _ => anyhow::bail!("Expected routable HTTP method, got {:?}", s),
        }
    }
}

impl fmt::Display for RoutableMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RoutableMethod::Delete => "DELETE",
            RoutableMethod::Get => "GET",
            RoutableMethod::Options => "OPTIONS",
            RoutableMethod::Patch => "PATCH",
            RoutableMethod::Post => "POST",
            RoutableMethod::Put => "PUT",
        };
        write!(f, "{s}")
    }
}

impl TryFrom<http::Method> for RoutableMethod {
    type Error = anyhow::Error;

    fn try_from(method: http::Method) -> anyhow::Result<Self> {
        match method {
            http::Method::DELETE => Ok(Self::Delete),
            http::Method::GET => Ok(Self::Get),
            http::Method::OPTIONS => Ok(Self::Options),
            http::Method::PATCH => Ok(Self::Patch),
            http::Method::POST => Ok(Self::Post),
            http::Method::PUT => Ok(Self::Put),
            http::Method::HEAD => Ok(Self::Get),
            _ => anyhow::bail!("Expected routable HTTP method, got {:?}", method),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct HttpActionRoute {
    pub path: String,
    pub method: RoutableMethod,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for HttpActionRoute {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (r"/([a-z0-9_]/)+(\*)?", any::<RoutableMethod>())
            .prop_map(|(path, method)| Self { path, method })
    }
}

impl HeapSize for HttpActionRoute {
    fn heap_size(&self) -> usize {
        self.path.heap_size()
    }
}

impl Display for HttpActionRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.method, self.path)
    }
}

impl FromStr for HttpActionRoute {
    type Err = anyhow::Error;

    fn from_str(p: &str) -> Result<Self, Self::Err> {
        let (method, path) = match p.rsplit_once(' ') {
            Some((method, path)) => (method.parse()?, path.to_owned()),
            None => anyhow::bail!("Invalid HTTP action route"),
        };
        Ok(Self { method, path })
    }
}

#[cfg(test)]
mod tests {
    use value::assert_obj;

    use super::NodeDependency;

    #[test]
    fn test_backwards_compatibility() {
        let serialized = assert_obj!(
            "package" => "foo",
            "version" => "1.0.0",
        );
        let deserialized: NodeDependency = serialized.try_into().unwrap();
        assert_eq!(
            deserialized,
            NodeDependency {
                package: "foo".to_string(),
                version: "1.0.0".to_string(),
            }
        );
    }
}
