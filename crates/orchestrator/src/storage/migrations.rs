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
    Ok(())
}
