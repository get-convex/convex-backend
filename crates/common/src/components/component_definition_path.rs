use std::{
    ops::Deref,
    path::{
        Component as PathComponent,
        PathBuf,
    },
    str::FromStr,
};

use anyhow::Context;
use sync_types::path::check_valid_path_component;

// Path relative to a project's `convex/` directory for each component
// definition's folder. This path is project-level and originates from
// a developer's source code.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentDefinitionPath {
    path: PathBuf,
}

impl ComponentDefinitionPath {
    pub fn min() -> Self {
        ComponentDefinitionPath {
            path: PathBuf::new(),
        }
    }

    pub fn root() -> Self {
        ComponentDefinitionPath {
            path: PathBuf::new(),
        }
    }

    pub fn is_root(&self) -> bool {
        self.path.as_os_str().is_empty()
    }
}

impl FromStr for ComponentDefinitionPath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        for component in path.components() {
            match component {
                PathComponent::Normal(c) => {
                    let s = c
                        .to_str()
                        .with_context(|| format!("Path {s} has an invalid Unicode character"))?;
                    check_valid_path_component(s)?;
                },
                // Component paths are allowed to have `..` (since they're relative from the root
                // component's source directory).
                PathComponent::ParentDir => (),
                PathComponent::RootDir => {
                    anyhow::bail!("Component paths must be relative ({s} is absolute).")
                },
                c => anyhow::bail!("Invalid path component {c:?} in {s}."),
            }
        }
        path.as_os_str()
            .to_str()
            .with_context(|| format!("Path {s} has an invalid Unicode character"))?;
        Ok(ComponentDefinitionPath { path })
    }
}

impl Deref for ComponentDefinitionPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.path
            .as_os_str()
            .to_str()
            .expect("Invalid Unicode in ComponentDefinitionPath")
    }
}

impl From<ComponentDefinitionPath> for String {
    fn from(value: ComponentDefinitionPath) -> Self {
        value
            .path
            .into_os_string()
            .into_string()
            .expect("Invalid Unicode in ComponentDefinitionPath?")
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for ComponentDefinitionPath {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = ComponentDefinitionPath>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        (
            0..=4,
            prop::collection::vec(any::<super::ComponentName>(), 0..=4),
        )
            .prop_map(|(depth, components)| {
                let mut path = String::new();
                for _ in 0..depth {
                    path.push_str("../");
                }
                for component in components {
                    path.push_str(&component);
                    path.push('/');
                }
                path.parse().unwrap()
            })
            .boxed()
    }
}
