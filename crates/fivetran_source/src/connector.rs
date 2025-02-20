use convex_fivetran_common::{
    config::{
        AllowAllHosts,
        Config,
    },
    fivetran_sdk::{
        schema_response,
        source_connector_server::SourceConnector,
        test_response,
        Column,
        ConfigurationFormRequest,
        ConfigurationFormResponse,
        ConfigurationTest,
        DataType,
        SchemaRequest,
        SchemaResponse,
        Table,
        TableList,
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
    convex_api::{
        ConvexApi,
        Source,
    },
    log,
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

        let columns = source.get_tables_and_columns().await?;

        let tables = TableList {
            tables: columns
                .into_iter()
                .map(|(table_name, column_names)| Table {
                    name: table_name.to_string(),
                    columns: column_names
                        .into_iter()
                        .map(|column_name| {
                            let column_name: String = column_name.to_string();
                            Column {
                                name: column_name.clone(),
                                r#type: match column_name.as_str() {
                                    "_id" => DataType::String,
                                    "_creationTime" => DataType::UtcDatetime,
                                    // We map every non-system column to the “unspecified” data type
                                    // and let Fivetran infer the correct column type from the data
                                    // it receives.
                                    _ => DataType::Unspecified,
                                } as i32,
                                primary_key: column_name == "_id",
                                params: None,
                            }
                        })
                        .collect(),
                })
                .collect(),
        };

        // Here, `WithoutSchema` means that there is no hierarchical level above tables,
        // not that the data is unstructured. Fivetran uses the same meaning of “schema”
        // as Postgres, not the one used in Convex. We do this because the connector is
        // already set up for a particular Convex deployment.
        Ok(SchemaResponse {
            response: Some(schema_response::Response::WithoutSchema(tables)),
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
