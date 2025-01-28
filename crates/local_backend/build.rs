use std::env;

use vergen::EmitBuilder;

fn main() -> anyhow::Result<()> {
    // recompile when there's a new git hash for /rev endpoint.
    if cfg!(not(debug_assertions)) || env::var("FORCE_EMIT").is_ok() {
        // Emit git sha
        EmitBuilder::builder()
            .git_sha(false)
            .git_commit_timestamp()
            .emit()?;
    } else {
        println!("cargo:rustc-env=VERGEN_GIT_SHA=dev");
        println!("cargo:rustc-env=VERGEN_GIT_COMMIT_TIMESTAMP=0000-00-00T00:00:00.000000000Z");
    }
    Ok(())
}
