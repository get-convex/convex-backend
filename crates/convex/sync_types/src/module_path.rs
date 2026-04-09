use std::{
    fmt,
    path::{
        Component,
        Path,
        PathBuf,
    },
    str::FromStr,
};

use anyhow::Context as _;

use crate::path::{
    check_valid_path_component,
    PathComponent,
};

pub const SYSTEM_UDF_DIR: &str = "_system";
pub const DEPS_DIR: &str = "_deps";
pub const ACTIONS_DIR: &str = "actions";
pub const HTTP_PATH: &str = "http.js";
pub const CRON_PATH: &str = "crons.js";

/// User-specified path to a loaded module.
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ModulePath {
    path: PathBuf,
    is_system: bool,
    is_deps: bool,
    is_http: bool,
    is_cron: bool,
}

impl ModulePath {
    /// NOTE: This constructor should only be used when converting from protos.
    /// Otherwise, prefer parsing the path from a `str` so that it gets
    /// validated.
    pub fn new(
        path: PathBuf,
        is_system: bool,
        is_deps: bool,
        is_http: bool,
        is_cron: bool,
    ) -> Self {
        Self {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        }
    }

    /// View the module path as a `str`.
    pub fn as_str(&self) -> &str {
        self.path
            .to_str()
            .expect("Non-unicode data in module path?")
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }

    // TODO: it should not be possible for this to return Err,
    // but `"_.js".strip().components()` will do this
    pub fn components(&self) -> impl Iterator<Item = anyhow::Result<PathComponent>> + '_ {
        self.path.components().map(|component| match component {
            Component::Normal(c) => c
                .to_str()
                .with_context(|| format!("Non-unicode data in module path {}", self.as_str()))?
                .parse()
                .with_context(|| {
                    format!("Invalid component {c:?} in module path {}", self.as_str())
                }),
            c => anyhow::bail!(
                "Unexpected component {c:?} in module path {}",
                self.as_str()
            ),
        })
    }

    /// Does a module live within the `_system/` directory?
    pub fn is_system(&self) -> bool {
        self.is_system
    }

    /// Does a module live within the `_deps/` directory?
    pub fn is_deps(&self) -> bool {
        self.is_deps
    }

    /// Is this module the (single) HTTP router for the deployment?
    pub fn is_http(&self) -> bool {
        self.is_http
    }

    /// Is this module the (single) crons module for the deployment?
    pub fn is_cron(&self) -> bool {
        self.is_cron
    }

    pub fn canonicalize(self) -> CanonicalizedModulePath {
        let Self {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        } = self;
        let path = canonicalize_path_buf(path);
        CanonicalizedModulePath {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        }
    }

    pub fn assume_canonicalized(self) -> anyhow::Result<CanonicalizedModulePath> {
        let Self {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        } = self;
        let ext = path
            .extension()
            .ok_or_else(|| anyhow::anyhow!("Path {path:?} doesn't have an extension."))?;
        anyhow::ensure!(ext == "js", "Path {path:?} doesn't have a '.js' extension.");
        Ok(CanonicalizedModulePath {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        })
    }
}

fn canonicalize_path_buf(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("js");
    }
    path
}

/// Parse a module path from a `str`.
impl FromStr for ModulePath {
    type Err = anyhow::Error;

    fn from_str(p: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(p);
        if path.file_name().is_none() {
            anyhow::bail!("Module path {p} doesn't have a filename.");
        }
        if let Some(ext) = path.extension() {
            if ext != "js" {
                anyhow::bail!("Module path ({}) has an extension that isn't 'js'.", p);
            }
        }

        let components = path
            .components()
            .map(|component| match component {
                Component::Normal(c) => c.to_str().ok_or_else(|| {
                    anyhow::anyhow!("Path {p} contains an invalid Unicode character.")
                }),
                Component::RootDir => {
                    anyhow::bail!("Module paths must be relative ({p} is absolute).")
                },
                c => anyhow::bail!("Invalid path component {c:?} in {p}."),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        anyhow::ensure!(!components.is_empty(), "Module paths must be nonempty.");

        // Determine the module type based on the first components.
        let is_system = matches!(&components[..], &[SYSTEM_UDF_DIR, ..]);
        let is_deps = matches!(
            &components[..],
            &[DEPS_DIR, ..] | &[ACTIONS_DIR, DEPS_DIR, ..],
        );

        // Check all components (canonicalized). Important to re-check first
        // component because canonicalization can change components.
        let canonicalized = canonicalize_path_buf(path.clone());
        for component in canonicalized.components() {
            let Component::Normal(component) = component else {
                anyhow::bail!("Invalid path component in {p}");
            };
            let component = component.to_str().ok_or_else(|| {
                anyhow::anyhow!("Path {p} contains an invalid Unicode character.")
            })?;
            check_valid_path_component(component)?;
        }

        let canonicalized_string = canonicalized
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Path {p} contains an invalid Unicode character."))?;
        let is_http = canonicalized_string == HTTP_PATH;
        let is_cron = canonicalized_string == CRON_PATH;

        Ok(Self {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        })
    }
}

impl From<ModulePath> for String {
    fn from(p: ModulePath) -> Self {
        p.path
            .into_os_string()
            .into_string()
            .expect("ModulePath had invalid Unicode data?")
    }
}

impl From<CanonicalizedModulePath> for ModulePath {
    fn from(p: CanonicalizedModulePath) -> Self {
        let CanonicalizedModulePath {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        } = p;
        Self {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        }
    }
}

impl fmt::Debug for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Module paths are allowed to omit the `.js` extension, but the canonical
/// module path stored in the database must have the `.js` extension. This
/// separate type guarantees that the path contains its extension.
#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CanonicalizedModulePath {
    path: PathBuf,
    is_system: bool,
    is_deps: bool,
    is_http: bool,
    is_cron: bool,
}

impl CanonicalizedModulePath {
    /// NOTE: This constructor should only be used when converting from protos.
    /// Otherwise, prefer the [`FromStr`] implementation since it includes
    /// validation.
    pub fn new(
        path: PathBuf,
        is_system: bool,
        is_deps: bool,
        is_http: bool,
        is_cron: bool,
    ) -> Self {
        Self {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        }
    }

    pub fn as_str(&self) -> &str {
        self.path
            .to_str()
            .expect("Non-unicode data in module path?")
    }

    pub fn is_system(&self) -> bool {
        self.is_system
    }

    pub fn is_deps(&self) -> bool {
        self.is_deps
    }

    pub fn is_http(&self) -> bool {
        self.is_http
    }

    pub fn is_cron(&self) -> bool {
        self.is_cron
    }

    pub fn strip(self) -> ModulePath {
        let Self {
            mut path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        } = self;
        if let Some(ext) = path.extension() {
            if ext == "js" {
                path.set_extension("");
            }
        }
        ModulePath {
            path,
            is_system,
            is_deps,
            is_http,
            is_cron,
        }
    }

}

impl FromStr for CanonicalizedModulePath {
    type Err = anyhow::Error;

    fn from_str(p: &str) -> Result<Self, Self::Err> {
        let path = ModulePath::from_str(p)?;
        Ok(path.canonicalize())
    }
}

impl From<CanonicalizedModulePath> for String {
    fn from(p: CanonicalizedModulePath) -> Self {
        p.path.into_os_string().into_string().unwrap()
    }
}

impl fmt::Debug for CanonicalizedModulePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
