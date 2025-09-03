use convex_fivetran_common::{
    config::{
        AllowAllHosts,
        Config,
    },
    fivetran_sdk::{
        schema_response,
        source_connector_server::SourceConnector,
        test_response,
        ConfigurationFormRequest,
        ConfigurationFormResponse,
        ConfigurationTest,
        SchemaRequest,
        SchemaResponse,
        TestRequest,
        TestResponse,
        UpdateRequest,
        UpdateResponse,
        UpdateResponse as FivetranUpdateResponse,
    },
};
use futures::{
    stream::BoxStream,
    StreamExt,
    TryStreamExt,
};
use tonic::{
    Request,
    Response,
    Status,
};

use crate::{
    api_types::selection::DEFAULT_FIVETRAN_SCHEMA_NAME,
    convex_api::{
        ComponentPath,
        ConvexApi,
        Source,
    },
    log::log,
    schema::generate_fivetran_schema,
    sync::{
        sync,
        State,
    },
};

/// Implements the gRPC server endpoints used by Fivetran.
#[derive(Debug)]
pub struct ConvexConnector {
    pub allow_all_hosts: AllowAllHosts,
}

type ConnectorResult<T> = Result<Response<T>, Status>;

impl ConvexConnector {
    async fn _schema(&self, request: Request<SchemaRequest>) -> anyhow::Result<SchemaResponse> {
        let config =
            Config::from_parameters(request.into_inner().configuration, self.allow_all_hosts)?;
        log(&format!("schema request for {}", config.deploy_url));

        let source = ConvexApi { config };

        let tables_by_component = source.get_table_column_names().await?;

        if tables_by_component
            .contains_key(&ComponentPath(DEFAULT_FIVETRAN_SCHEMA_NAME.to_string()))
        {
            anyhow::bail!(
                "Your Convex deployment contains a component named \
                 `{DEFAULT_FIVETRAN_SCHEMA_NAME}`. This conflicts with the name of the default \
                 Fivetran schema. Please rename the component to avoid issues.",
            );
        }

        Ok(SchemaResponse {
            response: Some(schema_response::Response::WithSchema(
                generate_fivetran_schema(tables_by_component),
            )),
            selection_not_supported: Some(true),
        })
    }
}

#[tonic::async_trait]
impl SourceConnector for ConvexConnector {
    type UpdateStream = BoxStream<'static, Result<UpdateResponse, Status>>;

    async fn configuration_form(
        &self,
        _: Request<ConfigurationFormRequest>,
    ) -> ConnectorResult<ConfigurationFormResponse> {
        log("configuration form request");
        Ok(Response::new(ConfigurationFormResponse {
            schema_selection_supported: false,
            table_selection_supported: false,
            fields: Config::fivetran_fields(),
            tests: vec![ConfigurationTest {
                name: "connection".to_string(),
                label: "Test connection".to_string(),
            }],
        }))
    }

    async fn test(&self, request: Request<TestRequest>) -> ConnectorResult<TestResponse> {
        log(&format!("test request"));
        let config =
            match Config::from_parameters(request.into_inner().configuration, self.allow_all_hosts)
            {
                Ok(config) => config,
                Err(error) => {
                    return Ok(Response::new(TestResponse {
                        response: Some(test_response::Response::Failure(error.to_string())),
                    }));
                },
            };
        log(&format!("test request for {}", config.deploy_url));
        let source = ConvexApi { config };

        // Perform an API request to verify if the credentials work
        match source.test_streaming_export_connection().await {
            Ok(_) => Ok(Response::new(TestResponse {
                response: Some(test_response::Response::Success(true)),
            })),
            Err(e) => Ok(Response::new(TestResponse {
                response: Some(test_response::Response::Failure(e.to_string())),
            })),
        }
    }

    async fn schema(&self, request: Request<SchemaRequest>) -> ConnectorResult<SchemaResponse> {
        log(&format!("schema request"));
        self._schema(request)
            .await
            .map(Response::new)
            .map_err(|error| Status::internal(error.to_string()))
    }

    async fn update(&self, request: Request<UpdateRequest>) -> ConnectorResult<Self::UpdateStream> {
        log(&format!("update request"));
        let inner = request.into_inner();
        let config = match Config::from_parameters(inner.configuration, self.allow_all_hosts) {
            Ok(config) => config,
            Err(error) => {
                return Err(Status::internal(error.to_string()));
            },
        };
        log(&format!("update request for {}", config.deploy_url));

        let state = deserialize_state_json(inner.state_json.as_deref().unwrap_or("{}"))
            .map_err(|error| Status::internal(error.to_string()))?;

        log(&format!(
            "update request for {} at checkpoint {:?}",
            config.deploy_url,
            state.as_ref().map(|s| &s.checkpoint)
        ));

        let source = ConvexApi { config };

        let sync = sync(source, state);
        Ok(Response::new(
            sync.map_ok(FivetranUpdateResponse::from)
                .map_err(|error| Status::internal(error.to_string()))
                .boxed(),
        ))
    }
}

fn deserialize_state_json(state_json: &str) -> anyhow::Result<Option<State>> {
    // Deserialize to a serde_json::Value first
    let state: serde_json::Value = serde_json::from_str(state_json)?;
    // Special case {} - which means we're initializing from fresh state
    let state = if state == serde_json::json!({}) {
        None
    } else {
        Some(serde_json::from_value(state)?)
    };
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::deserialize_state_json;
    use crate::sync::{
        Checkpoint,
        State,
    };

    #[test]
    fn test_deserialize_state_json() -> anyhow::Result<()> {
        assert_eq!(deserialize_state_json("{}")?, None);
        assert!(deserialize_state_json("{'invalid':'things'}").is_err());
        assert_eq!(
            deserialize_state_json(
                "{ \"version\": 1, \"checkpoint\": { \"DeltaUpdates\": { \"cursor\": 42 } } }"
            )?,
            Some(State::create(
                Checkpoint::DeltaUpdates { cursor: 42.into() },
                None,
            ))
        );
        Ok(())
    }
}
