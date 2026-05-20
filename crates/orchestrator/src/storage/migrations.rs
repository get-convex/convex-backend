use super::{
    pool::PgPool,
    schema::SCHEMA_SQL,
};

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    let conn = pool.acquire().await?;
    conn.client().batch_execute(SCHEMA_SQL).await?;
    // One-shot cleanup: earlier builds soft-deleted projects (`deleted = TRUE`),
    // which kept the `UNIQUE(team_id, slug)` slot busy and blocked re-creating
    // a project with the same slug. We now hard-delete on Delete Project; sweep
    // any leftover tombstones here so existing deployments stop "holding" slugs
    // they no longer use.
    conn.client()
        .batch_execute("DELETE FROM projects WHERE deleted = TRUE")
        .await?;
    // Project-backend-knobs migration. Idempotent — `IF NOT EXISTS` on each
    // column means re-running this against an already-migrated DB is a no-op.
    conn.client()
        .batch_execute(
            r#"
            ALTER TABLE projects
              ADD COLUMN IF NOT EXISTS tier TEXT NOT NULL DEFAULT 'S16',
              ADD COLUMN IF NOT EXISTS knob_overrides JSONB NOT NULL DEFAULT '{}'::jsonb;
            ALTER TABLE deployments
              ADD COLUMN IF NOT EXISTS tier TEXT NOT NULL DEFAULT 'S16',
              ADD COLUMN IF NOT EXISTS knob_overrides JSONB NOT NULL DEFAULT '{}'::jsonb;
            "#,
        )
        .await?;
    Ok(())
}
