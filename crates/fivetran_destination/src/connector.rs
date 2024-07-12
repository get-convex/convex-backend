use chrono::DateTime;
use convex_fivetran_common::{
    config::{
        AllowAllHosts,
        Config,
    },
    fivetran_sdk::{
        alter_table_response,
        create_table_response,
        describe_table_response,
        destination_server::Destination as FivetranDestination,
        test_response,
        truncate_response,
        write_batch_request::FileParams,
        write_batch_response,
        AlterTableRequest,
        AlterTableResponse,
        ConfigurationFormRequest,
        ConfigurationFormResponse,
        ConfigurationTest,
        CreateTableRequest,
        CreateTableResponse,
        DescribeTableRequest,
        DescribeTableResponse,
        TestRequest,
        TestResponse,
        TruncateRequest,
        TruncateResponse,
        WriteBatchRequest,
        WriteBatchResponse,
    },
};
use convex_fivetran_destination::api_types::DeleteType;
use prost_types::Timestamp;
use tonic::{
    Request,
    Response,
    Status,
};

use crate::{
    application::{
        alter_table,
        create_table,
        describe_table,
        truncate,
        write_batch,
        DescribeTableResponse as _DescribeTableResponse,
    },
    convex_api::{
        ConvexApi,
        Destination,
    },
    log,
};

/// Implements the gRPC server endpoints used by Fivetran.
#[derive(Debug)]
pub struct ConvexFivetranDestination {
    pub allow_all_hosts: AllowAllHosts,
}

type DestinationResult<T> = Result<Response<T>, Status>;

#[tonic::async_trait]
impl FivetranDestination for ConvexFivetranDestination {
    async fn configuration_form(
        &self,
        _: Request<ConfigurationFormRequest>,
    ) -> DestinationResult<ConfigurationFormResponse> {
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

    async fn test(&self, request: Request<TestRequest>) -> DestinationResult<TestResponse> {
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
        Ok(Response::new(TestResponse {
            response: Some(match source.test_streaming_import_connection().await {
                Ok(_) => {
                    log("Successful test request");
                    test_response::Response::Success(true)
                },
                Err(e) => {
                    log(&format!("Test error: {e}"));
                    test_response::Response::Failure(e.to_string())
                },
            }),
        }))
    }

    async fn describe_table(
        &self,
        request: Request<DescribeTableRequest>,
    ) -> DestinationResult<DescribeTableResponse> {
        log(&format!("describe table request"));
        let DescribeTableRequest {
            configuration,
            schema_name,
            table_name,
        } = request.into_inner();
        let config = match Config::from_parameters(configuration, self.allow_all_hosts) {
            Ok(config) => config,
            Err(error) => {
                return Ok(Response::new(DescribeTableResponse {
                    response: Some(describe_table_response::Response::Failure(
                        error.to_string(),
                    )),
                }));
            },
        };
        let table_name = fivetran_req_to_table_name(schema_name, table_name);
        log(&format!("describe table request for {}", config.deploy_url));
        let destination = ConvexApi { config };

        Ok(Response::new(DescribeTableResponse {
            response: Some(match describe_table(destination, table_name).await {
                Ok(_DescribeTableResponse::NotFound) => {
                    log("Successful describe table request (table not found)");
                    describe_table_response::Response::NotFound(true)
                },
                Ok(_DescribeTableResponse::Table(table)) => {
                    log("Successful describe table request (table found)");
                    describe_table_response::Response::Table(table)
                },
                Err(err) => {
                    log(&format!("Describe table error: {err}"));
                    describe_table_response::Response::Failure(err.to_string())
                },
            }),
        }))
    }

    async fn create_table(
        &self,
        request: Request<CreateTableRequest>,
    ) -> DestinationResult<CreateTableResponse> {
        log(&format!("create table request"));
        let CreateTableRequest {
            configuration,
            schema_name,
            table,
        } = request.into_inner();
        let config = match Config::from_parameters(configuration, self.allow_all_hosts) {
            Ok(config) => config,
            Err(error) => {
                return Ok(Response::new(CreateTableResponse {
                    response: Some(create_table_response::Response::Failure(error.to_string())),
                }));
            },
        };
        log(&format!("create table request for {}", config.deploy_url));
        let destination = ConvexApi { config };

        let Some(mut table) = table else {
            return Ok(Response::new(CreateTableResponse {
                response: Some(create_table_response::Response::Failure(
                    "Missing table argument".to_string(),
                )),
            }));
        };
        table.name = fivetran_req_to_table_name(schema_name, table.name);

        Ok(Response::new(CreateTableResponse {
            response: Some(match create_table(destination, table).await {
                Ok(_) => {
                    log("Successful create table request");
                    create_table_response::Response::Success(true)
                },
                Err(e) => {
                    log(&format!("Create table error: {e}"));
                    create_table_response::Response::Failure(e.to_string())
                },
            }),
        }))
    }

    async fn alter_table(
        &self,
        request: Request<AlterTableRequest>,
    ) -> DestinationResult<AlterTableResponse> {
        log(&format!("alter table request"));
        let AlterTableRequest {
            configuration,
            schema_name,
            table,
        } = request.into_inner();
        let config = match Config::from_parameters(configuration, self.allow_all_hosts) {
            Ok(config) => config,
            Err(error) => {
                return Ok(Response::new(AlterTableResponse {
                    response: Some(alter_table_response::Response::Failure(error.to_string())),
                }));
            },
        };
        log(&format!("alter table request for {}", config.deploy_url));
        let destination = ConvexApi { config };

        let Some(mut table) = table else {
            return Ok(Response::new(AlterTableResponse {
                response: Some(alter_table_response::Response::Failure(
                    "Missing table argument".to_string(),
                )),
            }));
        };
        table.name = fivetran_req_to_table_name(schema_name, table.name);

        Ok(Response::new(AlterTableResponse {
            response: Some(match alter_table(destination, table).await {
                Ok(_) => {
                    log("Successful alter table request");
                    alter_table_response::Response::Success(true)
                },
                Err(e) => {
                    log(&format!("Alter table error: {e}"));
                    alter_table_response::Response::Failure(e.to_string())
                },
            }),
        }))
    }

    async fn truncate(
        &self,
        request: Request<TruncateRequest>,
    ) -> DestinationResult<TruncateResponse> {
        log(&format!("truncate request"));
        let TruncateRequest {
            configuration,
            schema_name,
            table_name,
            synced_column: _,
            utc_delete_before,
            soft,
        } = request.into_inner();
        let config = match Config::from_parameters(configuration, self.allow_all_hosts) {
            Ok(config) => config,
            Err(error) => {
                return Ok(Response::new(TruncateResponse {
                    response: Some(truncate_response::Response::Failure(error.to_string())),
                }));
            },
        };
        let table_name = fivetran_req_to_table_name(schema_name, table_name);
        log(&format!("truncate request for {}", config.deploy_url));
        let destination = ConvexApi { config };

        Ok(Response::new(TruncateResponse {
            response: Some(
                match truncate(
                    destination,
                    table_name,
                    utc_delete_before.map(|Timestamp { seconds, nanos }| {
                        DateTime::from_timestamp(seconds, nanos as u32).expect("Invalid timestamp")
                    }),
                    match soft {
                        Some(_) => DeleteType::SoftDelete,
                        None => DeleteType::HardDelete,
                    },
                )
                .await
                {
                    Ok(_) => {
                        log("Successful truncate request");
                        truncate_response::Response::Success(true)
                    },
                    Err(e) => {
                        log(&format!("Truncate error: {e}"));
                        truncate_response::Response::Failure(e.to_string())
                    },
                },
            ),
        }))
    }

    async fn write_batch(
        &self,
        request: Request<WriteBatchRequest>,
    ) -> DestinationResult<WriteBatchResponse> {
        log(&format!("write batch request"));
        let WriteBatchRequest {
            configuration,
            schema_name,
            table,
            keys,
            replace_files,
            update_files,
            delete_files,
            file_params,
        } = request.into_inner();
        let config = match Config::from_parameters(configuration, self.allow_all_hosts) {
            Ok(config) => config,
            Err(error) => {
                return Ok(Response::new(WriteBatchResponse {
                    response: Some(write_batch_response::Response::Failure(error.to_string())),
                }));
            },
        };
        log(&format!("write batch request for {}", config.deploy_url));
        let destination = ConvexApi { config };

        let Some(mut table) = table else {
            return Ok(Response::new(WriteBatchResponse {
                response: Some(write_batch_response::Response::Failure(
                    "Missing table argument".to_string(),
                )),
            }));
        };
        table.name = fivetran_req_to_table_name(schema_name, table.name);

        let Some(FileParams::Csv(csv_file_params)) = file_params else {
            return Ok(Response::new(WriteBatchResponse {
                response: Some(write_batch_response::Response::Failure(
                    "Missing file_params argument".to_string(),
                )),
            }));
        };

        Ok(Response::new(WriteBatchResponse {
            response: Some(
                match write_batch(
                    destination,
                    table,
                    keys,
                    replace_files,
                    update_files,
                    delete_files,
                    csv_file_params,
                )
                .await
                {
                    Ok(_) => {
                        log("Successful batch write request");
                        write_batch_response::Response::Success(true)
                    },
                    Err(e) => {
                        log(&format!("Batch write error: {e}"));
                        write_batch_response::Response::Failure(e.to_string())
                    },
                },
            ),
        }))
    }
}

fn fivetran_req_to_table_name(fivetran_schema_name: String, fivetran_table_name: String) -> String {
    format!("{fivetran_schema_name}_{fivetran_table_name}")
}
