use std::{
    ops::Deref,
    path::PathBuf,
    str::FromStr,
};

use itertools::Itertools;
use sync_types::path::check_valid_path_component;
use value::identifier::Identifier;

// All components under a component have a unique `ComponentName`. For example,
// the root app component may have a waitlist component identified by
// "chatWaitlist".
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ComponentName(Identifier);

impl FromStr for ComponentName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl From<Identifier> for ComponentName {
    fn from(id: Identifier) -> Self {
        Self(id)
    }
}

impl From<ComponentName> for Identifier {
    fn from(name: ComponentName) -> Identifier {
        name.0
    }
}

impl From<ComponentName> for String {
    fn from(name: ComponentName) -> String {
        name.0.to_string()
    }
}

impl Deref for ComponentName {
    type Target = Identifier;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Path within the component tree for a particular component. Note that this
// path can potentially change when the component tree changes during a push, so
// we should resolve this path to a `ComponentId` within a transaction
// as soon as possible.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ComponentPath {
    pub path: Vec<ComponentName>,
}

impl From<ComponentPath> for String {
    fn from(path: ComponentPath) -> String {
        path.path.iter().map(|name| &***name).join("/")
    }
}

impl FromStr for ComponentPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            path: s.split('/').map(str::parse).try_collect()?,
        })
    }
}

// Path relative to the `convex` directory for each bundle.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentDefinitionPath(String);

impl FromStr for ComponentDefinitionPath {
    type Err = anyhow::Error;

    fn from_str(p: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(p);
        for component in path.components() {
            match component {
                std::path::Component::Normal(c) => {
                    let s = c.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Path {p} contains an invalid Unicode character.")
                    })?;
                    check_valid_path_component(s)?;
                },
                // Component paths are allowed to have `..` (since they're relative from the root
                // component's source directory).
                std::path::Component::ParentDir => (),
                std::path::Component::RootDir => {
                    anyhow::bail!("Component paths must be relative ({p} is absolute).")
                },
                c => anyhow::bail!("Invalid path component {c:?} in {p}."),
            }
        }
        Ok(Self(path.into_os_string().into_string().map_err(|_| {
            anyhow::anyhow!("Path {p} contains an invalid Unicode character.")
        })?))
    }
}

impl From<ComponentDefinitionPath> for String {
    fn from(value: ComponentDefinitionPath) -> Self {
        value.0
    }
}
