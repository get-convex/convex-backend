use vergen::EmitBuilder;

fn main() -> anyhow::Result<()> {
    // Recompile when there's a new git hash for beacon.
    // This is a workaround for https://github.com/rustyhorde/vergen/issues/174
    // In docker builds, we need a way to pass overrides to Vergen when there's no
    // actual git repo. We'll try emitting as usual, then fall back to env vars
    // that might have been set in the docker build before falling back to empty
    // strings.
    if EmitBuilder::builder()
        .git_sha(false)
        .git_commit_timestamp()
        .fail_on_error()
        .emit()
        .is_err()
    {
        println!("cargo:rerun-if-changed=build.rs");
        println!(
            "cargo:rustc-env=VERGEN_GIT_SHA={}",
            option_env!("VERGEN_GIT_SHA").unwrap_or_else(|| "unknown")
        );
        println!(
            "cargo:rustc-env=VERGEN_GIT_COMMIT_TIMESTAMP={}",
            option_env!("VERGEN_GIT_COMMIT_TIMESTAMP").unwrap_or_else(|| "unknown")
        );
    }
    Ok(())
}
