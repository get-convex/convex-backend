use super::Storage;

#[derive(Debug, Clone)]
pub struct DefaultEnvVar {
    pub project_id: i64,
    pub name: String,
    pub value: String,
    pub deployment_types: Vec<String>,
}

impl Storage {
    pub async fn list_default_env_vars(
        &self,
        project_id: i64,
    ) -> anyhow::Result<Vec<DefaultEnvVar>> {
        let conn = self.pool().acquire().await?;
        let rows = conn
            .client()
            .query(
                "SELECT project_id, name, value, deployment_types
                 FROM default_env_vars WHERE project_id = $1",
                &[&project_id],
            )
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let dts_json: serde_json::Value = r.get(3);
            let dts = serde_json::from_value::<Vec<String>>(dts_json).unwrap_or_default();
            out.push(DefaultEnvVar {
                project_id: r.get(0),
                name: r.get(1),
                value: r.get(2),
                deployment_types: dts,
            });
        }
        Ok(out)
    }

    pub async fn upsert_default_env_var(
        &self,
        project_id: i64,
        name: &str,
        value: &str,
        deployment_types: &[String],
    ) -> anyhow::Result<()> {
        let dts = serde_json::to_value(deployment_types)?;
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "INSERT INTO default_env_vars (project_id, name, value, deployment_types)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (project_id, name) DO UPDATE SET
                     value = EXCLUDED.value,
                     deployment_types = EXCLUDED.deployment_types",
                &[&project_id, &name, &value, &dts],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_default_env_var(
        &self,
        project_id: i64,
        name: &str,
    ) -> anyhow::Result<()> {
        let conn = self.pool().acquire().await?;
        conn.client()
            .execute(
                "DELETE FROM default_env_vars WHERE project_id = $1 AND name = $2",
                &[&project_id, &name],
            )
            .await?;
        Ok(())
    }
}
