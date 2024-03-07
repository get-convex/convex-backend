use std::env;

use vergen::EmitBuilder;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=FORCE_EMIT");

    if cfg!(not(debug_assertions)) || env::var("FORCE_EMIT").is_ok() {
        // Emit git sha
        EmitBuilder::builder().git_sha(false).emit()?;
    } else {
        println!("cargo:rustc-env=VERGEN_GIT_SHA=dev");
    }

    Ok(())
}
