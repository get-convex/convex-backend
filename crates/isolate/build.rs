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
};

use anyhow::Context;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sha2::{
    Digest as _,
    Sha256,
};
use walkdir::WalkDir;

const PACKAGES_DIR: &str = "../../npm-packages";
const NPM_DIR: &str = "../../npm-packages/convex";
const SYSTEM_UDFS_DIR: &str = "../system-udfs/convex/_system";
const UDF_RUNTIME_DIR: &str = "../udf-runtime/src";
const UDF_TESTS_DIR: &str = "../../npm-packages/tests/udf-tests";
const NODE_EXECUTOR_DIST_DIR: &str = "../../npm-packages/node-executor/dist";

const COMPONENT_TESTS_DIR: &str = "../../npm-packages/tests/component-tests";
/// Exceptions to the rule that all directories in `component-tests` are
/// components.
const COMPONENT_TESTS_CHILD_DIR_EXCEPTIONS: &[&str] = &[
    // stale in pre-migration checkouts
    ".rush",
    ".turbo",
    "node_modules",
    "projects",
];
/// Directory where test projects that use components live.
const COMPONENT_TESTS_PROJECTS_DIR: &str = "../../npm-packages/tests/component-tests/projects";
const COMPONENT_TESTS_PROJECTS: &[&str] = &[
    "basic",
    "with-schema",
    "schema_with_index",
    "mounted",
    "empty",
    "http_actions",
    "http_mount_routing",
    "http_prefix_and_mount_routing",
    "http_legacy_routes",
    "http_no_prefix_mounting",
    "env_vars",
];
/// Components in `component-tests` directory that are used in projects.
const COMPONENTS: &[&str] = &[
    "component",
    "componentWithEnv",
    "envVars",
    "errors",
    "httpComponent",
    "httpGrandchild",
];

const ADMIN_KEY: &str = include_str!("../keybroker/dev/admin_key.txt");

/// The pinned JS tools in scripts/node_modules, as paths relative to
/// PACKAGES_DIR.
#[derive(Clone, Copy, PartialEq, Eq)]
enum JsTool {
    Pnpm,
    Turbo,
}

impl JsTool {
    #[cfg(not(target_os = "windows"))]
    fn path(self) -> &'static str {
        match self {
            JsTool::Pnpm => "../scripts/node_modules/.bin/pnpm",
            JsTool::Turbo => "../scripts/node_modules/.bin/turbo",
        }
    }

    #[cfg(target_os = "windows")]
    fn path(self) -> &'static str {
        match self {
            JsTool::Pnpm => "../../scripts/node_modules/.bin/pnpm.cmd",
            JsTool::Turbo => "../../scripts/node_modules/.bin/turbo.cmd",
        }
    }
}
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
        eprintln!("Loading _system/{path}");

        // Ugh, is there a better way to dump large string literals from a build script?
        // Unparse each string as a raw string literal for the source and source map.
        sha.update(source.as_bytes());
        let source = format!("r####\"{source}\"####");
        if let Some(ref source_map) = source_map {
            sha.update(source_map.as_bytes());
        }
        let source_map = source_map
            .map(|s| format!("Some(r####\"{s}\"####)"))
            .unwrap_or_else(|| "None".to_owned());
        writeln!(out, r#"    "{path}" => ({source}, {source_map}),"#)?;
    }
    writeln!(out, "}};")?;

    let digest: [u8; 32] = sha.finalize().into();
    writeln!(out, "pub const FILES_SHA256: [u8; 32] = {digest:?};")?;

    Ok(())
}

// Concurrent pnpm installs serialize on the store and modules-dir locks
// (though pnpm has had bugs with concurrent installs in one worktree, e.g.
// pnpm/pnpm#7335). turbo has no cross-process task lock, so concurrent runs
// (e.g. `just turbo` racing this build script during a parallel CI job) could
// execute a task twice and race writes to shared outputs/cache entries. flock
// on a per-checkout lock file (shared with the `just turbo` recipe) serializes
// them; hosts without flock(1) (macOS, Windows) run unlocked, where such races
// are transient and a rerun fixes them.
fn flock_available() -> bool {
    Command::new("flock")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn run_js_tool(tool: JsTool, args: &[&str]) -> anyhow::Result<()> {
    // turbo shells out to `pnpm` by name, so the pinned copy in
    // scripts/node_modules must be on PATH.
    let bin_dir = fs::canonicalize(Path::new(PACKAGES_DIR).join("../scripts/node_modules/.bin"))?;
    let mut paths = vec![bin_dir];
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    let mut command = if tool == JsTool::Turbo && flock_available() {
        fs::create_dir_all(Path::new(PACKAGES_DIR).join(".turbo"))?;
        let mut c = Command::new("flock");
        c.arg(".turbo/turbo.lock").arg(tool.path());
        c
    } else {
        Command::new(tool.path())
    };
    let output = command
        .current_dir(Path::new(PACKAGES_DIR))
        .env("PATH", env::join_paths(paths)?)
        // Keep turbo hermetic inside cargo builds: no first-run telemetry
        // banner/phone-home in the output, and no user-exported TURBO_* (UI
        // mode, remote-cache tokens) changing behavior.
        .env("TURBO_TELEMETRY_DISABLED", "1")
        .env_remove("TURBO_UI")
        .env_remove("TURBO_TOKEN")
        .env_remove("TURBO_TEAM")
        .args(args)
        .output()
        .with_context(|| format!("Failed to run {} {}", tool.path(), args.join(" ")))?;
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    anyhow::ensure!(
        output.status.success(),
        "Failed on {} {}",
        tool.path(),
        args.join(" ")
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // TODO: Have higher accuracy change tracking here.
    rerun_if_changed("../../npm-packages/convex/src/bundler")?;
    rerun_if_changed("../../npm-packages/convex/src/server")?;
    rerun_if_changed("../../npm-packages/convex/scripts/bundle-server.mjs")?;
    rerun_if_changed("../../npm-packages/convex/package.json")?;
    rerun_if_changed("../../npm-packages/convex/scripts/build.py")?;

    // Dependency resolution inputs: a dep bump or override change alters the
    // compiled-in bundles without touching any source watched above.
    rerun_if_changed("../../npm-packages/pnpm-lock.yaml")?;
    rerun_if_changed("../../npm-packages/pnpm-workspace.yaml")?;
    rerun_if_changed("../../npm-packages/turbo.json")?;

    rerun_if_changed("../../npm-packages/node-executor/src")?;
    rerun_if_changed("../../npm-packages/node-executor/package.json")?;

    rerun_if_changed("../../npm-packages/system-udfs/convex/")?;

    // Note that we only include the component directory,`convex` directory, and
    // package.json so we ignore changes to other workspace files.
    let has_tests = Path::new("../../npm-packages/tests/udf-tests/convex/").exists();
    if has_tests {
        rerun_if_changed("../../npm-packages/tests/udf-tests/convex/")?;
        rerun_if_changed("../../npm-packages/tests/udf-tests/src/")?;
        rerun_if_changed("../../npm-packages/tests/udf-tests/package.json")?;
        rerun_if_changed("../../npm-packages/tests/component-tests/package.json")?;
        for component in COMPONENTS {
            rerun_if_changed(&format!(
                "../../npm-packages/tests/component-tests/{component}/"
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
                        "Found directory in component-tests that is not in `COMPONENTS`. Please \
                         add it: {}",
                        dir_name
                    );
                }
            }
        }
        rerun_if_changed("../../npm-packages/tests/component-tests/component/")?;
        rerun_if_changed("../../npm-packages/tests/component-tests/envVars/")?;
        rerun_if_changed("../../npm-packages/tests/component-tests/errors/")?;
        for project in COMPONENT_TESTS_PROJECTS {
            rerun_if_changed(&format!(
                "../../npm-packages/tests/component-tests/projects/{project}/convex"
            ))?;
            rerun_if_changed(&format!(
                "../../npm-packages/tests/component-tests/projects/{project}/package.json"
            ))?;
        }
    }

    // This is a little janky because we aren't including the node_modules directory
    // which has real sources in it! I'm not including it because it appears to
    // change every build and hopefully package.json catches the real semantic
    // changes.
    rerun_if_changed("../../npm-packages/udf-runtime/src/")?;
    rerun_if_changed("../../npm-packages/udf-runtime/package.json")?;
    rerun_if_changed("../../npm-packages/system-udfs/convex/_system")?;
    rerun_if_changed("../../npm-packages/system-udfs/package.json")?;
    rerun_if_changed("../../npm-packages/system-udfs/tsconfig.json")?;

    // Step 1: Ensure the `server`, `dashboard`, and `cli` deps are installed.
    // CI jobs whose workflow already installed and built these packages before
    // cargo runs set CONVEX_PREBUILT_JS to skip this re-verification pass.
    // Only sound where the JS build strictly precedes cargo: with concurrent
    // JS builds this run must stay (under the turbo flock) so it blocks until
    // the dist outputs it bundles below are complete. Keep the package list in
    // sync with the `Build JS required by Isolate` step in rust.yml.
    println!("cargo:rerun-if-env-changed=CONVEX_PREBUILT_JS");
    if env::var_os("CONVEX_PREBUILT_JS").is_none() {
        run_js_tool(JsTool::Pnpm, &["install", "--frozen-lockfile"])?;
        let mut pkgs = vec!["convex", "node-executor", "udf-runtime"];
        if has_tests {
            pkgs.extend(["simulation", "udf-tests"]);
        }
        let mut args = vec!["run".to_owned(), "build".to_owned()];
        for pkg in pkgs {
            // `--filter=pkg...` builds the package and its workspace dependencies,
            // matching `rush build -t pkg`.
            args.push(format!("--filter={pkg}..."));
        }
        run_js_tool(
            JsTool::Turbo,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )?;
    }
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

    if has_tests {
        // Step 5: Build and bundle the udf test project.
        eprintln!("Building udf test bundle");
        write_udf_test_bundle(out_dir)?;

        // Step 6: Build and bundle component-test projects.
        for entry in fs::read_dir(COMPONENT_TESTS_PROJECTS_DIR)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                eprintln!("Building component test bundle {path:?}");
                let out_path = &out_dir.join(&path);
                if Path::exists(out_path) {
                    fs::remove_dir_all(out_path)?;
                }
                let suffix = path.strip_prefix(COMPONENT_TESTS_PROJECTS_DIR)?;
                anyhow::ensure!(
                    COMPONENT_TESTS_PROJECTS.contains(
                        &suffix
                            .to_str()
                            .context("Failed to convert suffix to string")?,
                    ),
                    "Unexpected component test project {suffix:?} (missing in \
                     COMPONENT_TESTS_PROJECTS?)"
                );
                let out_with_project = out_dir.join(suffix);
                fs::create_dir_all(&out_with_project)?;
                write_start_push_request(&path, &out_with_project.join("start_push_request"))?;
            }
        }

        // Step 7: Record dependencies for the simulation test build. It's a bit of a
        // hack that it's in this build script, but it keeps all the JS builds in
        // one place.
        let metafile = Path::new(PACKAGES_DIR).join("tests/simulation/dist/metafile.json");
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
            let path = fs::canonicalize(
                Path::new(PACKAGES_DIR)
                    .join("tests/simulation")
                    .join(rel_path),
            )?;
            rerun_if_changed(path.as_os_str().to_str().unwrap())?;
        }
        for entry in WalkDir::new(Path::new(PACKAGES_DIR).join("tests/simulation/convex")) {
            rerun_if_changed(entry?.path().to_str().expect("Invalid path"))?;
        }
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
            "--push-all-modules",
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
