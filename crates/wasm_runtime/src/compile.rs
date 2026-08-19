//! Turning a deployment's modules into a runnable wasm artifact.
//!
//! ```text
//! module sources -> esbuild -> Wizer (against the shared guest) -> snapshot
//! ```
//!
//! The guest binary carries no deployment code: it reads its bundle from a
//! preopened directory while `wizer_initialize` runs, so every deployment
//! shares one guest build and only the snapshot is per-deployment. Bundling and
//! preinitialization are still seconds of work the first time a module set is
//! seen, so the result is cached on disk under a key covering both the sources
//! and the guest itself. That is fine for a test harness; anything serving
//! traffic needs these artifacts built somewhere other than the request path.

use std::{
    collections::BTreeMap,
    env,
    fs,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

use anyhow::Context as _;
use sha2::{
    Digest,
    Sha256,
};
use wizer::Wizer;

use crate::host::GUEST_BUNDLE_DIR;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn target_subdir(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    fn cargo_args(self) -> &'static [&'static str] {
        match self {
            Self::Debug => &[],
            Self::Release => &["--release"],
        }
    }
}

/// The guest sources this host was built against. A change to any of them has
/// to invalidate cached artifacts, and hashing them here is what ties the two
/// together without a version number anyone has to remember to bump.
const GUEST_SOURCES: &[&str] = &[
    include_str!("../guest_js/src/lib.rs"),
    include_str!("../guest_js/Cargo.toml"),
    // The guest's JS lives in its own files rather than inline in `lib.rs`, so
    // each one has to be listed here: a change to any of them changes what a
    // snapshot evaluates without changing a byte of Rust.
    include_str!("../guest_js/src/js/shared_prelude.js"),
    include_str!("../guest_js/src/js/udf_globals.js"),
    include_str!("../guest_js/src/js/fixture_globals.js"),
    include_str!("../guest_js/src/js/invoke_runtime.js"),
    include_str!("../guest_js/src/js/udf_invoke_runtime.js"),
    include_str!("../guest_js/src/js/describe_exception.js"),
    include_str!("../../../npm-packages/wasm-runtime-fixtures/scripts/bundle-modules.mjs"),
];

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `wasm-runtime-fixtures` package in `npm-packages`, which owns the
/// bundling scripts and the esbuild install they run against.
pub fn bundler_package_dir() -> PathBuf {
    PathBuf::from(env!("WASM_RUNTIME_FIXTURES_DIR"))
}

/// Where compiled artifacts are kept between runs. Override with
/// `CONVEX_WASM_RUNTIME_CACHE` to share one cache across checkouts.
pub fn cache_root() -> PathBuf {
    match env::var_os("CONVEX_WASM_RUNTIME_CACHE") {
        Some(dir) => PathBuf::from(dir),
        None => crate_root().join("target").join("udf-artifacts"),
    }
}

/// A preinitialized guest with one deployment's modules baked in.
pub struct CompiledModules {
    pub wasm: Vec<u8>,
    /// Directory holding the intermediate bundle and its source map, for
    /// mapping stack frames back to the original modules.
    pub artifact_dir: PathBuf,
    pub cache_hit: bool,
}

/// Compile `sources` into a wasm artifact whose exports are `entry`'s exports.
///
/// `sources` is the whole module tree keyed by deployment-relative path
/// (`messages.js`, `_deps/abc.js`); `entry` names the module whose functions
/// are being run.
pub fn compile_modules(
    sources: &BTreeMap<String, String>,
    entry: &str,
) -> anyhow::Result<CompiledModules> {
    anyhow::ensure!(
        sources.contains_key(entry),
        "entry module {entry} is not among the {} sources provided",
        sources.len()
    );

    let artifact_dir = cache_root().join(cache_key(sources, entry));
    let wasm_path = artifact_dir.join("module.wasm");
    if let Ok(wasm) = fs::read(&wasm_path) {
        return Ok(CompiledModules {
            wasm,
            artifact_dir,
            cache_hit: true,
        });
    }

    let source_dir = artifact_dir.join("src");
    let bundle_dir = artifact_dir.join("bundle");
    fs::create_dir_all(&bundle_dir)
        .with_context(|| format!("failed to create {}", bundle_dir.display()))?;

    for (path, source) in sources {
        let target = source_dir.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&target, source)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }

    bundle_modules(&source_dir, entry, &bundle_dir)?;
    let guest_wasm = build_guest(Profile::Release, &guest_target_dir())?;
    let wasm = preinitialize_guest(&guest_wasm, &bundle_dir)?;

    fs::write(&wasm_path, &wasm)
        .with_context(|| format!("failed to write {}", wasm_path.display()))?;

    Ok(CompiledModules {
        wasm,
        artifact_dir,
        cache_hit: false,
    })
}

fn guest_target_dir() -> PathBuf {
    cache_root().join("guest-target")
}

fn cache_key(sources: &BTreeMap<String, String>, entry: &str) -> String {
    let mut hasher = Sha256::new();
    for guest_source in GUEST_SOURCES {
        hasher.update(guest_source.as_bytes());
    }
    hasher.update(entry.as_bytes());
    for (path, source) in sources {
        // Length-prefixed so no combination of paths and sources can collide
        // with a different split of the same bytes.
        hasher.update((path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    const_hex::encode(hasher.finalize())
}

/// Bundle a module tree into the single script the guest evaluates, alongside a
/// manifest that marks it as a UDF bundle.
pub fn bundle_modules(source_dir: &Path, entry: &str, out_dir: &Path) -> anyhow::Result<()> {
    run(Command::new("node")
        .arg("scripts/bundle-modules.mjs")
        .arg(source_dir)
        .arg(entry)
        .arg(out_dir)
        .current_dir(bundler_package_dir()))
    .with_context(|| format!("failed to bundle {entry}"))
}

/// Build the guest, which carries no deployment code: every artifact shares one
/// build, and cargo's own target-directory lock makes concurrent callers wait
/// for it rather than race.
pub fn build_guest(profile: Profile, target_dir: &Path) -> anyhow::Result<PathBuf> {
    run(Command::new("cargo")
        .arg("build")
        .args(profile.cargo_args())
        .arg("--manifest-path")
        .arg("guest_js/Cargo.toml")
        .arg("--target")
        .arg("wasm32-wasip1")
        .arg("--target-dir")
        .arg(target_dir)
        .current_dir(crate_root()))
    .context("failed to build the guest")?;

    Ok(target_dir
        .join("wasm32-wasip1")
        .join(profile.target_subdir())
        .join("guest_js.wasm"))
}

/// Preinitialize the guest against the bundle in `bundle_dir`, which it reads
/// through the mapped directory while `wizer_initialize` runs. The bundle is
/// evaluated into the snapshot, so the result needs no filesystem to serve
/// requests.
pub fn preinitialize_guest(wasm_path: &Path, bundle_dir: &Path) -> anyhow::Result<Vec<u8>> {
    let input_wasm =
        fs::read(wasm_path).with_context(|| format!("failed to read {}", wasm_path.display()))?;
    Wizer::new()
        .allow_wasi(true)
        .context("failed to enable WASI for Wizer")?
        .map_dir(GUEST_BUNDLE_DIR, bundle_dir)
        .init_func("wizer_initialize")
        .run(&input_wasm)
        .context("Wizer failed")
}

fn run(command: &mut Command) -> anyhow::Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to execute {:?}", command.get_program()))?;
    anyhow::ensure!(
        status.success(),
        "{:?} exited with {status}",
        command.get_program()
    );
    Ok(())
}
