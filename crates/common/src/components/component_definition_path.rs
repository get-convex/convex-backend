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
pub struct ComponentDefinitionPath {
    path: PathBuf,
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
                        .context("Path {s} has an invalid Unicode character")?;
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
            .context("Path {s} has an invalid Unicode character")?;
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
