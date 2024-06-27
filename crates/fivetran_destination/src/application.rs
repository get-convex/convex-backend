use std::{
    collections::BTreeMap,
    str::FromStr,
};

use chrono::{
    DateTime,
    Utc,
};
use common::{
    try_chunks::TryChunksExt,
    value::{
        ConvexObject,
        TableName,
    },
};
use convex_fivetran_common::fivetran_sdk::{
    self,
    Compression,
    CsvFileParams,
    Encryption,
};
use convex_fivetran_destination::api_types::{
    BatchWriteOperation,
    BatchWriteRow,
    DeleteType,
    FivetranTableName,
};
use futures::{
    stream::{
        self,
    },
    StreamExt,
};
use futures_async_stream::try_stream;

use crate::{
    convex_api::Destination,
    error::{
        DestinationError,
        SuggestedTable,
    },
    file_reader::{
        create_csv_deserializer,
        read_rows,
        FivetranFileEncryption,
        FivetranReaderParams,
    },
    schema::{
        suggested_convex_table,
        to_fivetran_table,
        validate_destination_schema_table,
        FivetranTableSchema,
    },
};

const ROWS_BY_REQUEST: usize = 500;

pub enum DescribeTableResponse {
    NotFound,
    Table(fivetran_sdk::Table),
}

pub async fn describe_table(
    destination: impl Destination,
    table_name: String,
) -> Result<DescribeTableResponse, DestinationError> {
    let convex_table_name = TableName::from_str(&table_name)
        .map_err(|err| DestinationError::UnsupportedTableName(table_name, err))?;

    let Some(schema) = destination
        .get_schema()
        .await
        .map_err(DestinationError::DeploymentError)?
    else {
        return Ok(DescribeTableResponse::NotFound);
    };

    let Some(convex_table) = schema.tables.get(&convex_table_name) else {
        return Ok(DescribeTableResponse::NotFound);
    };

    Ok(DescribeTableResponse::Table(to_fivetran_table(
        convex_table,
    )?))
}

pub async fn create_table(
    destination: impl Destination,
    table: fivetran_sdk::Table,
) -> Result<(), DestinationError> {
    let convex_table_name = TableName::from_str(&table.name)
        .map_err(|err| DestinationError::UnsupportedTableName(table.name.to_string(), err))?;

    let schema = destination
        .get_schema()
        .await
        .map_err(DestinationError::DeploymentError)?
        .ok_or_else(|| match suggested_convex_table(table.clone()) {
            Ok(suggested_table) => {
                DestinationError::DestinationHasNoSchema(SuggestedTable(suggested_table))
            },
            Err(err) => DestinationError::DestinationHasNoSchemaWithoutSuggestion(Box::new(err)),
        })?;

    let Some(convex_table) = schema.tables.get(&convex_table_name) else {
        return Err(match suggested_convex_table(table) {
            Ok(suggested_table) => {
                DestinationError::MissingTable(convex_table_name, SuggestedTable(suggested_table))
            },
            Err(err) => {
                DestinationError::MissingTableWithoutSuggestion(convex_table_name, Box::new(err))
            },
        });
    };

    validate_destination_schema_table(table, convex_table)?;

    Ok(())
}

pub async fn alter_table(
    destination: impl Destination,
    table: fivetran_sdk::Table,
) -> Result<(), DestinationError> {
    // AlterTable is implemented the same way as CreateTable, as it merely checks
    // that the table in the Convex destination complies to what we expect.
    create_table(destination, table).await
}

pub async fn truncate(
    destination: impl Destination,
    table_name: String,
    delete_before: Option<DateTime<Utc>>,
    delete_type: DeleteType,
) -> Result<(), DestinationError> {
    let convex_table_name = TableName::from_str(&table_name)
        .map_err(|err| DestinationError::UnsupportedTableName(table_name.to_string(), err))?;

    destination
        .truncate_table(convex_table_name, delete_type, delete_before)
        .await
        .map_err(DestinationError::DeploymentError)?;

    Ok(())
}

pub async fn write_batch(
    destination: impl Destination,
    table: fivetran_sdk::Table,
    keys: BTreeMap<String, Vec<u8>>,
    replace_files: Vec<String>,
    update_files: Vec<String>,
    delete_files: Vec<String>,
    csv_file_params: CsvFileParams,
) -> Result<(), DestinationError> {
    let reader_params = FivetranReaderParams::from(csv_file_params.clone());
    let table_name = FivetranTableName::from_str(&table.name)
        .map_err(|err| DestinationError::InvalidTableName(table.name.clone(), err))?;
    let schema = FivetranTableSchema::try_from(table)?;

    let mut streams = vec![];
    for file in replace_files {
        streams.push(row_stream(
            file,
            BatchWriteOperation::Upsert,
            &keys,
            csv_file_params.encryption(),
            csv_file_params.compression(),
            &reader_params,
            &table_name,
            &schema,
        ));
    }
    for file in update_files {
        streams.push(row_stream(
            file,
            BatchWriteOperation::Update,
            &keys,
            csv_file_params.encryption(),
            csv_file_params.compression(),
            &reader_params,
            &table_name,
            &schema,
        ));
    }
    for file in delete_files {
        streams.push(row_stream(
            file,
            BatchWriteOperation::HardDelete,
            &keys,
            csv_file_params.encryption(),
            csv_file_params.compression(),
            &reader_params,
            &table_name,
            &schema,
        ));
    }

    let mut concatenated_stream =
        Box::pin(stream::iter(streams).flatten().try_chunks2(ROWS_BY_REQUEST));
    while let Some(result) = concatenated_stream.next().await {
        destination
            .batch_write(result?)
            .await
            .map_err(DestinationError::DeploymentError)?;
    }

    Ok(())
}

#[try_stream(ok = BatchWriteRow, error = DestinationError)]
async fn row_stream<'a>(
    file: String,
    operation: BatchWriteOperation,
    keys: &'a BTreeMap<String, Vec<u8>>,
    encryption: Encryption,
    compression: Compression,
    reader_params: &'a FivetranReaderParams,
    table_name: &'a FivetranTableName,
    schema: &'a FivetranTableSchema,
) {
    let encryption: FivetranFileEncryption = if encryption == Encryption::Aes {
        let key = keys.get(&file).ok_or(DestinationError::InvalidKey)?;
        FivetranFileEncryption::Aes {
            key: key.clone().try_into()?,
        }
    } else {
        FivetranFileEncryption::None
    };

    let mut deserializer = create_csv_deserializer(&file, compression, encryption)
        .await
        .map_err(|err| DestinationError::FileReadError(file.clone(), err))?;
    let mut reader = read_rows(&mut deserializer, reader_params, schema);

    while let Some(row) = reader.next().await {
        let row: ConvexObject = row
            .map_err(|err| DestinationError::FileReadError(file.clone(), err))?
            .try_into()
            .map_err(DestinationError::InvalidRow)?;

        yield BatchWriteRow {
            table: table_name.to_string(),
            operation,
            row,
        }
    }
}
