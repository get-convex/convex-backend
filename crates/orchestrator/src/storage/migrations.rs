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
    // Phase B migration: per-deployment desired settings columns.
    conn.client()
        .batch_execute(
            r#"
            ALTER TABLE deployments
              ADD COLUMN IF NOT EXISTS desired_tier TEXT,
              ADD COLUMN IF NOT EXISTS desired_overrides JSONB NOT NULL DEFAULT '{}'::jsonb;
            "#,
        )
        .await?;
    // v3 migration: per-deployment storage mode + sidecar credentials.
    // Existing rows default to 'volume-sqlite' (v2 behavior preserved).
    // Credentials are plaintext at rest — encrypt the orchestrator DB at
    // the disk layer. The CHECK constraint from schema.rs only applies to
    // fresh DB inits; orchestrator-side validation enforces the same
    // invariant for ALTERed DBs.
    conn.client()
        .batch_execute(
            r#"
            ALTER TABLE deployments
              ADD COLUMN IF NOT EXISTS storage_mode TEXT NOT NULL DEFAULT 'volume-sqlite',
              ADD COLUMN IF NOT EXISTS pg_password TEXT,
              ADD COLUMN IF NOT EXISTS minio_root_user TEXT,
              ADD COLUMN IF NOT EXISTS minio_root_password TEXT;
            "#,
        )
        .await?;
    // Hotfix migration: persist the 64-hex INSTANCE_SECRET env var
    // separately from the `instance_secret` column (which actually holds
    // the backend-produced admin key). Without this, restart_deployment
    // would feed the admin key back as INSTANCE_SECRET and the backend
    // would fail to hex-decode it.
    conn.client()
        .batch_execute(
            r#"
            ALTER TABLE deployments
              ADD COLUMN IF NOT EXISTS backend_instance_secret TEXT;
            "#,
        )
        .await?;
    // Custom-domain certificate management. Domains created before this
    // migration used a Traefik-side cert resolver and carry no challenge
    // preference, so default them to `http-01`: it needs no credentials and
    // matches the behaviour they already had.
    conn.client()
        .batch_execute(
            r#"
            ALTER TABLE custom_domains
              ADD COLUMN IF NOT EXISTS challenge_type TEXT NOT NULL DEFAULT 'http-01',
              ADD COLUMN IF NOT EXISTS dns_credential_id BIGINT
                REFERENCES dns_provider_credentials(id) ON DELETE SET NULL,
              ADD COLUMN IF NOT EXISTS last_error TEXT;
            "#,
        )
        .await?;
    Ok(())
}
