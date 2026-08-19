//! Builds the fixture bundles the tests and benchmarks run, by way of the
//! `wasm-runtime-fixtures` package in `npm-packages`.

use std::fs;

use anyhow::Context;
use js_build::{
    js_prebuilt,
    packages_dir,
    pnpm_install,
    rerun_if_changed,
    turbo_build,
};

const FIXTURES_PKG: &str = "wasm-runtime-fixtures";

fn main() -> anyhow::Result<()> {
    let packages_dir = packages_dir()?;
    let fixtures_dir = packages_dir.join(FIXTURES_PKG);

    rerun_if_changed(fixtures_dir.join("fixtures"))?;
    rerun_if_changed(fixtures_dir.join("scripts"))?;
    rerun_if_changed(fixtures_dir.join("package.json"))?;

    // The fixtures bundle the `convex` package, so its sources and build inputs
    // feed the bundles too.
    rerun_if_changed(packages_dir.join("convex/src"))?;
    rerun_if_changed(packages_dir.join("convex/scripts"))?;
    rerun_if_changed(packages_dir.join("convex/package.json"))?;

    // Dependency resolution inputs: a dep bump or override change alters the
    // bundles without touching any source watched above.
    rerun_if_changed(packages_dir.join("pnpm-lock.yaml"))?;
    rerun_if_changed(packages_dir.join("pnpm-workspace.yaml"))?;
    rerun_if_changed(packages_dir.join("turbo.json"))?;

    // Keep the package in sync with the `Build JS required by Rust` step in
    // rust.yml, which is what lets the cargo steps there set CONVEX_PREBUILT_JS.
    if !js_prebuilt() {
        pnpm_install()?;
        turbo_build(&[FIXTURES_PKG])?;
    }

    let bundle_dir = fixtures_dir.join("dist");
    anyhow::ensure!(
        bundle_dir.exists(),
        "{} is missing: did `{FIXTURES_PKG}` build?",
        bundle_dir.display()
    );
    // Canonical so that the tests and the benchmark can use these from any
    // working directory.
    println!(
        "cargo:rustc-env=WASM_RUNTIME_FIXTURES_DIR={}",
        fs::canonicalize(&fixtures_dir)
            .with_context(|| format!("Failed to canonicalize {fixtures_dir:?}"))?
            .display()
    );
    println!(
        "cargo:rustc-env=WASM_RUNTIME_FIXTURE_BUNDLES_DIR={}",
        fs::canonicalize(&bundle_dir)
            .with_context(|| format!("Failed to canonicalize {bundle_dir:?}"))?
            .display()
    );

    Ok(())
}
