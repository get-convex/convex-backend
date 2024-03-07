use std::{
    collections::BTreeMap,
    fmt::{
        self,
        Debug,
        Display,
    },
    str::FromStr,
};

use serde_json::{
    json,
    Value as JsonValue,
};
use value::{
    obj,
    ConvexObject,
    ConvexValue,
};

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

impl TryFrom<ConvexObject> for NodeDependency {
    type Error = anyhow::Error;

    fn try_from(obj: ConvexObject) -> Result<Self, Self::Error> {
        let mut fields = BTreeMap::from(obj);

        let package: String = match fields.remove("package") {
            Some(ConvexValue::String(s)) => s.into(),
            _ => anyhow::bail!("Invalid or missing 'package' in NodeDependency: {fields:?}"),
        };
        let version: String = match fields.remove("version") {
            Some(ConvexValue::String(s)) => s.into(),
            _ => anyhow::bail!("Invalid or missing 'version' in NodeDependency: {fields:?}"),
        };
        Ok(Self { package, version })
    }
}

impl TryFrom<NodeDependency> for ConvexObject {
    type Error = anyhow::Error;

    fn try_from(value: NodeDependency) -> Result<Self, Self::Error> {
        obj!(
            "package" => value.package,
            "version" => value.version
        )
    }
}

impl TryFrom<ConvexValue> for NodeDependency {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        if let ConvexValue::Object(o) = value {
            o.try_into()
        } else {
            anyhow::bail!("NodeDependency expected an Object, got {value:?}")
        }
    }
}

impl TryFrom<NodeDependency> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(value: NodeDependency) -> Result<Self, Self::Error> {
        let obj: ConvexObject = value.try_into()?;
        Ok(ConvexValue::Object(obj))
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct HttpActionRoute {
    pub path: String,
    pub method: RoutableMethod,
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
