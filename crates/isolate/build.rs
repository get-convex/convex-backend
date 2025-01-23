use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    env,
    fs::{
        self,
        File,
    },
    io::{
        self,
        Write,
    },
    path::Path,
    process::Command,
    thread,
    time::Duration,
};

use anyhow::Context;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use value::sha256::Sha256;
use walkdir::WalkDir;

const PACKAGES_DIR: &str = "../../npm-packages";
const NPM_DIR: &str = "../../npm-packages/convex";
const SYSTEM_UDFS_DIR: &str = "../system-udfs/convex/_system";
const UDF_RUNTIME_DIR: &str = "../udf-runtime/src";
const UDF_TESTS_DIR: &str = "../../npm-packages/udf-tests";
const NODE_EXECUTOR_DIST_DIR: &str = "../../npm-packages/node-executor/dist";

const COMPONENT_TESTS_DIR: &str = "../../npm-packages/component-tests";
/// Exceptions to the rule that all directories in `component-tests` are
/// components.
const COMPONENT_TESTS_CHILD_DIR_EXCEPTIONS: [&str; 3] = [".rush", "node_modules", "projects"];
/// Directory where test projects that use components live.
const COMPONENT_TESTS_PROJECTS_DIR: &str = "../../npm-packages/component-tests/projects";
const COMPONENT_TESTS_PROJECTS: [&str; 4] = ["basic", "with-schema", "mounted", "empty"];
/// Components in `component-tests` directory that are used in projects.
const COMPONENTS: [&str; 3] = ["component", "envVars", "errors"];

const ADMIN_KEY: &str = include_str!("../keybroker/dev/admin_key.txt");

#[cfg(not(target_os = "windows"))]
const RUSH: &str = "../scripts/node_modules/.bin/rush";
#[cfg(target_os = "windows")]
const RUSH: &str = "../../scripts/node_modules/.bin/rush.cmd";
#[cfg(not(target_os = "windows"))]
const NPM: &str = "npm";
#[cfg(target_os = "windows")]
const NPM: &str = "npm.cmd";
const CONVEX: &str = "node_modules/convex/bin/main.js";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bundle {
    path: String,
    source: String,
    source_map: Option<String>,
}

// Cargo silently drops paths that don't exist and then reruns the build script
// on every invocation. This fallback isn't great, since it'll silently degrade
// build times, so check that the path actually exists with this helper.
fn rerun_if_changed(path: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        Path::new(path).exists(),
        "Non-existent dependency path: {path}"
    );
    println!("cargo:rerun-if-changed={path}");
    Ok(())
}

fn write_bundles(out_dir: &Path, out_name: &str, bundles: Vec<Bundle>) -> anyhow::Result<()> {
    let mut sha = Sha256::new();
    let mut out = File::create(out_dir.join(out_name))?;
    writeln!(out, "use phf::phf_map;")?;
    writeln!(
        out,
        "pub static FILES: phf::Map<&'static str, (&'static str, Option<&'static str>)> = \
         phf_map! {{"
    )?;
    for Bundle {
        path,
        source,
        source_map,
    } in bundles
    {
        eprintln!("Loading _system/{}", path);

        // Ugh, is there a better way to dump large string literals from a build script?
        // Unparse each string as a raw string literal for the source and source map.
        sha.update(source.as_bytes());
        let source = format!("r####\"{}\"####", source);
        if let Some(ref source_map) = source_map {
            sha.update(source_map.as_bytes());
        }
        let source_map = source_map
            .map(|s| format!("Some(r####\"{}\"####)", s))
            .unwrap_or_else(|| "None".to_owned());
        writeln!(out, r#"    "{path}" => ({source}, {source_map}),"#)?;
    }
    writeln!(out, "}};")?;

    let digest: [u8; 32] = *sha.finalize();
    writeln!(out, "pub const FILES_SHA256: [u8; 32] = {digest:?};")?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // TODO: Have higher accuracy change tracking here.
    rerun_if_changed("../../npm-packages/convex/src/bundler")?;
    rerun_if_changed("../../npm-packages/convex/src/server")?;
    rerun_if_changed("../../npm-packages/convex/scripts/bundle-server.mjs")?;
    rerun_if_changed("../../npm-packages/convex/package.json")?;
    rerun_if_changed("../../npm-packages/convex/scripts/build.py")?;

    rerun_if_changed("../../npm-packages/node-executor/src")?;
    rerun_if_changed("../../npm-packages/node-executor/package.json")?;

    rerun_if_changed("../../npm-packages/system-udfs/convex/")?;

    // Note that we only include the component directory,`convex` directory, and
    // package.json so we ignore changes to rush files.
    rerun_if_changed("../../npm-packages/udf-tests/convex/")?;
    rerun_if_changed("../../npm-packages/udf-tests/package.json")?;
    rerun_if_changed("../../npm-packages/component-tests/package.json")?;
    for component in COMPONENTS {
        rerun_if_changed(&format!(
            "../../npm-packages/component-tests/{}/",
            component
        ))?;
    }
    // Make sure we are not missing any directories that could be components.
    for dir in fs::read_dir(COMPONENT_TESTS_DIR)? {
        let dir = dir?;
        if dir.path().is_dir() {
            let dir_name = dir.file_name();
            let dir_name = dir_name
                .to_str()
                .context("Failed to convert dir_name to string")?;
            if !COMPONENTS.contains(&dir_name)
                && !COMPONENT_TESTS_CHILD_DIR_EXCEPTIONS.contains(&dir_name)
            {
                anyhow::bail!(
                    "Found directory in component-tests that is not in `COMPONENTS`. Please add \
                     it: {}",
                    dir_name
                );
            }
        }
    }
    rerun_if_changed("../../npm-packages/component-tests/component/")?;
    rerun_if_changed("../../npm-packages/component-tests/envVars/")?;
    rerun_if_changed("../../npm-packages/component-tests/errors/")?;
    for project in COMPONENT_TESTS_PROJECTS {
        rerun_if_changed(&format!(
            "../../npm-packages/component-tests/projects/{}/convex",
            project
        ))?;
        rerun_if_changed(&format!(
            "../../npm-packages/component-tests/projects/{}/package.json",
            project
        ))?;
    }

    // This is a little janky because we aren't inlcuding the node_modules directory
    // which has real sources in it! I'm not including it because it appears to
    // change every build and hopefully package.json catches the real semantic
    // changes.
    rerun_if_changed("../../npm-packages/udf-runtime/src/")?;
    rerun_if_changed("../../npm-packages/udf-runtime/package.json")?;
    rerun_if_changed("../../npm-packages/system-udfs/convex/_system")?;
    rerun_if_changed("../../npm-packages/system-udfs/package.json")?;
    rerun_if_changed("../../npm-packages/system-udfs/tsconfig.json")?;

    // Step 1: Ensure the `server`, `dashboard`, and `cli` deps are installed.
    for _ in 0..3 {
        let output = Command::new(RUSH)
            .current_dir(Path::new(PACKAGES_DIR))
            .args(["install"])
            .output()
            .context("Failed on rush install")?;
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();
        if String::from_utf8_lossy(&output.stdout)
            .contains("Another Rush command is already running in this repository.")
        {
            // Sometimes editors/etc might run another rush install. Just wait a moment and
            // try again.
            thread::sleep(Duration::from_secs(1));
            continue;
        }
        anyhow::ensure!(output.status.success(), "Failed to 'rush install'");
        break;
    }
    let status = Command::new(RUSH)
        .current_dir(PACKAGES_DIR)
        .args([
            "build",
            "-t",
            "convex",
            "-t",
            "node-executor",
            "-t",
            "udf-runtime",
            "-t",
            "udf-tests",
            "-t",
            "simulation",
        ])
        .status()
        .context("Failed on rush build")?;
    anyhow::ensure!(status.success(), "Failed to 'rush build'");
    // Step 2: Use `build-server` to package up our builtin `_system` UDFs.
    let output = Command::new(NPM)
        .current_dir(NPM_DIR)
        .arg("run")
        .arg("--silent")
        .arg("bundle-server")
        .arg(Path::new(UDF_RUNTIME_DIR))
        .arg(Path::new(SYSTEM_UDFS_DIR))
        .output()
        .context("Failed on npm run bundler")?;
    anyhow::ensure!(
        output.status.success(),
        "Failed to run bundler:\n{}",
        String::from_utf8(output.stderr)?,
    );

    let bundles: Vec<Bundle> = serde_json::from_slice(&output.stdout)?;

    // Check that all the paths are unique.
    let bundle_paths = bundles.iter().map(|b| &b.path).collect::<BTreeSet<_>>();
    anyhow::ensure!(bundle_paths.len() == bundles.len());

    // Step 3: Use the output to generate a compile-time hashtable with all of the
    // system bundles.
    let out_dir_s = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_s);
    write_bundles(out_dir, "system_udf_js_data.rs", bundles)?;

    // Step 4: Copy node executor files. They are already bundled.
    let mut bundles = Vec::new();
    for file_name in ["local.cjs", "aws_lambda.cjs"] {
        let path = Path::new(NODE_EXECUTOR_DIST_DIR).join(file_name);
        let source =
            fs::read_to_string(path.to_str().unwrap()).context("Failed on read_to_string")?;

        let source_map_path =
            Path::new(NODE_EXECUTOR_DIST_DIR).join(file_name.to_string() + ".map");
        let source_map = fs::read_to_string(source_map_path.to_str().unwrap())
            .context("Failed on read_to_string")?;
        bundles.push(Bundle {
            path: file_name.to_owned(),
            source,
            source_map: Some(source_map),
        });
    }
    write_bundles(out_dir, "node_executor_js_data.rs", bundles)?;

    // Step 5: Build and bundle the udf test project.
    write_udf_test_bundle(out_dir)?;

    // Step 6: Build and bundle component-test projects.
    for entry in fs::read_dir(COMPONENT_TESTS_PROJECTS_DIR)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let out_path = &out_dir.join(&path);
            if Path::exists(out_path) {
                fs::remove_dir_all(out_path)?;
            }
            let suffix = path.strip_prefix(COMPONENT_TESTS_PROJECTS_DIR)?;
            anyhow::ensure!(&COMPONENT_TESTS_PROJECTS.contains(
                &suffix
                    .to_str()
                    .context("Failed to convert suffix to string")?
            ));
            let out_with_project = out_dir.join(suffix);
            fs::create_dir_all(&out_with_project)?;
            write_start_push_request(&path, &out_with_project.join(format!("start_push_request")))?;
        }
    }

    // Step 7: Record dependencies for the simulation test build. It's a bit of a
    // hack that it's in this build script, but we can't safely invoke Rush
    // across two build scripts since it'll fail if called concurrently.
    let metafile = Path::new(PACKAGES_DIR).join("simulation/dist/metafile.json");
    let metafile_contents = fs::read_to_string(metafile).context("Failed to read metafile")?;
    let metafile: Metafile =
        serde_json::from_str(&metafile_contents).context("Failed to parse metafile")?;

    for (rel_path, _) in metafile.inputs {
        // TODO: Building `convex` seems to bump the files' mtime even on cache hit.
        // [simulation 0.1.0] ==[ convex ]==============================[ 1 of 2 ]==
        // [simulation 0.1.0] "convex" was restored from the build cache.
        if rel_path.contains("convex/dist/esm") {
            continue;
        }
        let path = fs::canonicalize(Path::new(PACKAGES_DIR).join("simulation").join(rel_path))?;
        rerun_if_changed(path.as_os_str().to_str().unwrap())?;
    }
    for entry in WalkDir::new(Path::new(PACKAGES_DIR).join("simulation/convex")) {
        rerun_if_changed(entry?.path().to_str().expect("Invalid path"))?;
    }

    Ok(())
}

fn write_udf_test_bundle(out_dir: &Path) -> anyhow::Result<()> {
    let bundle_dir = out_dir.join("udf_test_bundle");
    // clear the existing content
    if Path::exists(&bundle_dir) {
        fs::remove_dir_all(bundle_dir.clone())?;
    }
    let output = Command::new("node")
        .current_dir(UDF_TESTS_DIR)
        .args([
            CONVEX,
            "deploy",
            "--debug-bundle-path",
            bundle_dir.to_str().unwrap(),
            "--codegen=disable",
            "--typecheck=disable",
            "--url",
            "http://127.0.0.1:8000",
            "--admin-key",
            ADMIN_KEY,
        ])
        .output()
        .context("Unable to run npx convex deploy")?;
    anyhow::ensure!(
        output.status.success(),
        "Failed to run convex deploy:\n{}\n{}",
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?
    );
    Ok(())
}

fn write_start_push_request(project_directory: &Path, out_file: &Path) -> anyhow::Result<()> {
    if Path::exists(out_file) {
        fs::remove_file(out_file)?;
    }
    let output = Command::new("node")
        .current_dir(project_directory)
        .args([
            CONVEX,
            "deploy",
            "--write-push-request",
            out_file.to_str().unwrap(),
            "--url",
            "http://127.0.0.1:8000",
            "--admin-key",
            ADMIN_KEY,
        ])
        .output()
        .context("Unable to run `npx convex deploy --write-push-request`")?;
    anyhow::ensure!(
        output.status.success(),
        "Failed to run convex deploy --write-push-request:\n{}\n{}",
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?
    );
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Metafile {
    inputs: BTreeMap<String, JsonValue>,
}
