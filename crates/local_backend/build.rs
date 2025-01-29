use vergen::EmitBuilder;

fn main() -> anyhow::Result<()> {
    // recompile when there's a new git hash for beacon.
    EmitBuilder::builder()
        .git_sha(false)
        .git_commit_timestamp()
        .emit()?;
    Ok(())
}
