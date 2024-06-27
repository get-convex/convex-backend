use application::EnvVarChange;
use axum::{
    extract::State,
    response::IntoResponse,
};
use common::http::{
    extract::Json,
    HttpResponseError,
};
use http::StatusCode;
use model::environment_variables::types::{
    EnvVarName,
    EnvVarValue,
    EnvironmentVariable,
};
use serde::Deserialize;

use crate::{
    admin::must_be_admin_with_write_access,
    authentication::ExtractIdentity,
    LocalAppState,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEnvVarRequest {
    name: String,
    value: Option<String>, // None → delete existing
}

impl UpdateEnvVarRequest {
    pub async fn into_env_var_changes(self) -> anyhow::Result<Vec<EnvVarChange>> {
        match self {
            UpdateEnvVarRequest {
                name,
                value: Some(value),
            } => {
                let env_var = validate_env_var(&name, &value)?;
                Ok(vec![EnvVarChange::Set(env_var)])
            },
            UpdateEnvVarRequest { name, value: None } => {
                let name = name.parse()?;
                Ok(vec![EnvVarChange::Unset(name)])
            },
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateEnvVarsRequest {
    changes: Vec<UpdateEnvVarRequest>,
}

pub async fn update_environment_variables(
    State(st): State<LocalAppState>,
    ExtractIdentity(identity): ExtractIdentity,
    Json(UpdateEnvVarsRequest { changes }): Json<UpdateEnvVarsRequest>,
) -> Result<impl IntoResponse, HttpResponseError> {
    must_be_admin_with_write_access(&identity)?;

    let mut env_var_changes = vec![];
    for change in changes {
        env_var_changes.extend(change.into_env_var_changes().await?);
    }
    env_var_changes.sort();

    let mut tx = st.application.begin(identity).await?;
    let audit_events = st
        .application
        .update_environment_variables(&mut tx, env_var_changes)
        .await?;

    st.application
        .commit_with_audit_log_events(tx, audit_events, "update_env_vars")
        .await?;

    Ok(StatusCode::OK)
}

fn validate_env_var(name: &String, value: &String) -> anyhow::Result<EnvironmentVariable> {
    let name: EnvVarName = name.parse()?;
    let value: EnvVarValue = value.parse()?;
    Ok(EnvironmentVariable::new(name, value))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::headers::authorization::Credentials;
    use common::types::{
        EnvVarName,
        EnvVarValue,
    };
    use http::Request;
    use hyper::Body;
    use keybroker::Identity;
    use maplit::btreemap;
    use model::environment_variables::EnvironmentVariablesModel;
    use runtime::prod::ProdRuntime;
    use serde_json::json;

    use crate::test_helpers::{
        setup_backend_for_test,
        TestLocalBackend,
    };

    async fn update_environment_variables(
        backend: &TestLocalBackend,
        changes: serde_json::Value,
    ) -> anyhow::Result<()> {
        let json_body = json!({"changes": changes});
        let body = Body::from(serde_json::to_vec(&json_body)?);
        let req = Request::builder()
            .uri("/api/update_environment_variables")
            .method("POST")
            .header("Content-Type", "application/json")
            .header("Authorization", backend.admin_auth_header.0.encode())
            .body(body)?;
        backend.expect_success(req).await?;
        Ok(())
    }

    async fn list_environment_variables(
        backend: &TestLocalBackend,
    ) -> anyhow::Result<BTreeMap<EnvVarName, EnvVarValue>> {
        let mut tx = backend.st.application.begin(Identity::system()).await?;
        let envs = EnvironmentVariablesModel::new(&mut tx).get_all().await?;
        Ok(envs)
    }

    #[convex_macro::prod_rt_test]
    async fn test_create_env_vars(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        update_environment_variables(
            &backend,
            json!([
                {"name": "name1", "value": "value1"},
                {"name": "name2", "value": "value2"},
            ]),
        )
        .await?;
        assert_eq!(
            list_environment_variables(&backend).await?,
            btreemap! {
                "name1".parse()? => "value1".parse()?,
                "name2".parse()? => "value2".parse()?,
            }
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_update_env_vars(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        update_environment_variables(
            &backend,
            json!([
                {"name": "name1", "value": "value1"},
                {"name": "name2", "value": "value2"},
            ]),
        )
        .await?;
        update_environment_variables(
            &backend,
            json!([
                {"name": "name2", "value": "value2b"},
                {"name": "name3", "value": "value3"},
            ]),
        )
        .await?;
        assert_eq!(
            list_environment_variables(&backend).await?,
            btreemap! {
                "name1".parse()? => "value1".parse()?,
                "name2".parse()? => "value2b".parse()?,
                "name3".parse()? => "value3".parse()?,
            }
        );
        Ok(())
    }

    #[convex_macro::prod_rt_test]
    async fn test_delete_env_vars(rt: ProdRuntime) -> anyhow::Result<()> {
        let backend = setup_backend_for_test(rt).await?;
        update_environment_variables(
            &backend,
            json!([
                {"name": "name1", "value": "value1"},
                {"name": "name2", "value": "value2"},
            ]),
        )
        .await?;
        update_environment_variables(
            &backend,
            json!([
                {"name": "name2"},
                {"name": "name3"},
            ]),
        )
        .await?;
        assert_eq!(
            list_environment_variables(&backend).await?,
            btreemap! {
                "name1".parse()? => "value1".parse()?,
            }
        );
        Ok(())
    }
}
