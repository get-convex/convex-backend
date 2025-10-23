use std::collections::BTreeMap;

use anyhow::{
    anyhow,
    Context,
};
use async_compression::tokio::bufread::{
    GzipDecoder,
    ZstdDecoder,
};
use chrono::{
    DateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Timelike,
};
use common::async_compat::TokioAsyncReadCompatExt;
use convex_fivetran_common::fivetran_sdk::{
    value_type::Inner as FivetranValue,
    Compression as FivetranFileCompression,
    DataType as FivetranDataType,
    FileParams,
};
use convex_fivetran_destination::api_types::FivetranFieldName;
use futures::{
    stream::BoxStream,
    StreamExt,
};
use prost_types::Timestamp;
use tokio::{
    fs::File,
    io::{
        self,
        BufReader,
    },
};

use crate::{
    aes::{
        Aes256Key,
        AesDecryptor,
    },
    schema::FivetranTableSchema,
};

#[derive(Debug, PartialEq)]
pub enum FivetranFileValue {
    Value(FivetranValue),
    Unmodified,
}

#[derive(Debug, PartialEq)]
pub struct FileRow(pub BTreeMap<FivetranFieldName, FivetranFileValue>);

pub struct FivetranReaderParams {
    unmodified_string: String,
    null_string: String,
}

impl From<FileParams> for FivetranReaderParams {
    fn from(value: FileParams) -> Self {
        Self {
            unmodified_string: value.unmodified_string,
            null_string: value.null_string,
        }
    }
}

/// See https://github.com/fivetran/fivetran_sdk/blob/main/development-guide.md#encryption
pub enum FivetranFileEncryption {
    None,
    Aes { key: Aes256Key },
}

pub async fn create_csv_deserializer(
    file_path: &str,
    compression: FivetranFileCompression,
    encryption: FivetranFileEncryption,
) -> anyhow::Result<csv_async::AsyncDeserializer<impl futures::AsyncRead + Unpin + Send + use<>>> {
    let reader = BufReader::new(File::open(file_path).await.context("Couldn't open file")?);

    // Decryption
    let reader: Box<dyn io::AsyncRead + Unpin + Send> = match encryption {
        FivetranFileEncryption::None => Box::new(reader),
        FivetranFileEncryption::Aes { key } => Box::new(AesDecryptor::new(reader, key).await?),
    };

    // Decompression
    let reader: Box<dyn io::AsyncRead + Unpin + Send> = match compression {
        FivetranFileCompression::Off => reader,
        FivetranFileCompression::Zstd => Box::new(ZstdDecoder::new(BufReader::new(reader))),
        FivetranFileCompression::Gzip => Box::new(GzipDecoder::new(BufReader::new(reader))),
    };

    let deserializer = csv_async::AsyncDeserializer::from_reader(Box::new(reader.compat()));
    if !deserializer.has_headers() {
        anyhow::bail!("Missing file headers");
    }

    Ok(deserializer)
}

pub fn read_rows<'a, R>(
    deserializer: &'a mut csv_async::AsyncDeserializer<R>,
    params: &'a FivetranReaderParams,
    schema: &'a FivetranTableSchema,
) -> BoxStream<'a, anyhow::Result<FileRow>>
where
    R: futures::AsyncRead + Unpin + Send,
{
    deserializer
        .deserialize::<BTreeMap<String, String>>()
        .map(|result| match result {
            Ok(field_map) => Ok(FileRow(
                field_map
                    .into_iter()
                    .map(|(field, value)| -> anyhow::Result<_> {
                        let field: FivetranFieldName =
                            field.parse().context("Couldn't parse field")?;

                        let file_value = if value == params.unmodified_string {
                            FivetranFileValue::Unmodified
                        } else if value == params.null_string {
                            FivetranFileValue::Value(FivetranValue::Null(true))
                        } else {
                            let column = schema
                                .columns
                                .get(&field)
                                .context("Column not in schema")?
                                .to_owned();
                            FivetranFileValue::Value(
                                try_parse_fivetran_value(value.clone(), column.data_type).context(
                                    format!(
                                        "Couldn't parse field {} value {} as {:?} from file",
                                        field, value, column.data_type,
                                    ),
                                )?,
                            )
                        };

                        Ok((field, file_value))
                    })
                    .try_collect()?,
            )),
            Err(csv_error) => Err(anyhow!(csv_error).context("Can’t deserialize the CSV file")),
        })
        .boxed()
}

/// Parses a Fivetran value in the format
/// [used in Fivetran CSV files](https://github.com/fivetran/fivetran_sdk/blob/main/development-guide.md#examples-of-data-types).
fn try_parse_fivetran_value(
    value: String,
    target_type: FivetranDataType,
) -> anyhow::Result<FivetranValue> {
    Ok(match target_type {
        FivetranDataType::Unspecified => {
            anyhow::bail!("Can’t parse a value to an unspecified type")
        },
        FivetranDataType::Boolean => FivetranValue::Bool(value.parse()?),
        FivetranDataType::Short => FivetranValue::Short(value.parse()?),
        FivetranDataType::Int => FivetranValue::Int(value.parse()?),
        FivetranDataType::Long => FivetranValue::Long(value.parse()?),
        FivetranDataType::Float => FivetranValue::Float(value.parse()?),
        FivetranDataType::Double => FivetranValue::Double(value.parse()?),

        FivetranDataType::NaiveDate => FivetranValue::NaiveDate(Timestamp {
            seconds: NaiveDateTime::new(
                NaiveDate::parse_from_str(&value, "%Y-%m-%d")?,
                NaiveTime::default(),
            )
            .and_utc()
            .timestamp(),
            nanos: 0,
        }),
        FivetranDataType::NaiveTime => {
            let dt = NaiveDateTime::new(
                NaiveDate::default(),
                NaiveTime::parse_from_str(&value, "%H:%M:%S%.f")?,
            )
            .and_utc();
            FivetranValue::NaiveTime(Timestamp {
                seconds: dt.timestamp(),
                nanos: dt.timestamp_subsec_nanos() as i32,
            })
        },
        FivetranDataType::NaiveDatetime => {
            let dt = NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f")?.and_utc();
            FivetranValue::NaiveDatetime(Timestamp {
                seconds: dt.timestamp(),
                nanos: dt.timestamp_subsec_nanos() as i32,
            })
        },
        FivetranDataType::UtcDatetime => {
            let date_time = DateTime::parse_from_rfc3339(&value)?;
            FivetranValue::UtcDatetime(Timestamp {
                seconds: date_time.timestamp(),
                nanos: date_time.nanosecond() as i32,
            })
        },

        FivetranDataType::Binary => FivetranValue::Binary(base64::decode(value)?),

        FivetranDataType::Decimal => FivetranValue::Decimal(value),
        FivetranDataType::Xml => FivetranValue::Xml(value),
        FivetranDataType::String => FivetranValue::String(value),
        FivetranDataType::Json => FivetranValue::Json(value),
    })
}

#[cfg(test)]
fn to_csv_string_representation(value: &FivetranValue) -> Option<String> {
    match value {
        FivetranValue::Null(_) => None,

        FivetranValue::Bool(v) => Some(v.to_string()),
        FivetranValue::Short(v) => Some(v.to_string()),
        FivetranValue::Int(v) => Some(v.to_string()),
        FivetranValue::Long(v) => Some(v.to_string()),
        FivetranValue::Float(v) => Some(v.to_string()),
        FivetranValue::Double(v) => Some(v.to_string()),

        FivetranValue::NaiveDate(Timestamp { seconds, nanos }) => {
            DateTime::from_timestamp(*seconds, *nanos as u32)
                .map(|dt| dt.naive_utc().date().format("%Y-%m-%d").to_string())
        },
        FivetranValue::NaiveTime(Timestamp { seconds, nanos }) => {
            DateTime::from_timestamp(*seconds, *nanos as u32)
                .map(|dt| dt.time().format("%H:%M:%S%.f").to_string())
        },
        FivetranValue::NaiveDatetime(Timestamp { seconds, nanos }) => {
            DateTime::from_timestamp(*seconds, *nanos as u32)
                .map(|dt| dt.naive_utc().format("%Y-%m-%dT%H:%M:%S%.f").to_string())
        },
        FivetranValue::UtcDatetime(Timestamp { seconds, nanos }) => {
            DateTime::from_timestamp(*seconds, *nanos as u32)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string())
        },

        FivetranValue::Binary(bytes) => Some(base64::encode(bytes)),

        FivetranValue::Decimal(v) => Some(v.clone()),
        FivetranValue::String(v) => Some(v.clone()),
        FivetranValue::Json(v) => Some(v.clone()),
        FivetranValue::Xml(v) => Some(v.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        env,
        str::FromStr,
    };

    use cmd_util::env::env_config;
    use convex_fivetran_common::fivetran_sdk::{
        value_type::Inner as FivetranValue,
        Compression as FivetranFileCompression,
        DataType as FivetranDataType,
    };
    use convex_fivetran_destination::api_types::{
        FivetranFieldName,
        FivetranTableName,
    };
    use futures::StreamExt;
    use maplit::btreemap;
    use proptest::prelude::*;
    use prost_types::Timestamp;
    use tokio::{
        fs::File,
        io::AsyncReadExt,
    };

    use crate::{
        aes::Aes256Key,
        convert::fivetran_data_type,
        file_reader::{
            create_csv_deserializer,
            read_rows,
            to_csv_string_representation,
            try_parse_fivetran_value,
            FileRow,
            FivetranFileEncryption,
            FivetranFileValue,
            FivetranReaderParams,
        },
        schema::{
            FivetranTableColumn,
            FivetranTableSchema,
        },
    };

    fn fixture_path(fixture_name: &str) -> String {
        String::from(
            env::current_dir()
                .unwrap()
                .join("src")
                .join("tests")
                .join("fixtures")
                .join(fixture_name)
                .to_str()
                .unwrap(),
        )
    }

    fn fivetran_table_schema(
        fields: BTreeMap<&'static str, FivetranDataType>,
    ) -> FivetranTableSchema {
        FivetranTableSchema {
            name: FivetranTableName::from_str("my_table").unwrap(),
            columns: fields
                .into_iter()
                .map(|(name, data_type)| {
                    (
                        FivetranFieldName::from_str(name).unwrap(),
                        FivetranTableColumn {
                            data_type,
                            in_primary_key: false,
                        },
                    )
                })
                .collect(),
        }
    }

    #[tokio::test]
    async fn test_parse_a_fivetran_csv_file() -> anyhow::Result<()> {
        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod-NcK9NIjPUutCsz4mjOQQztbnwnE1sY3"),
            null_string: String::from("magic-nullvalue"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Long,
            "title" => FivetranDataType::String,
            "magic_number" => FivetranDataType::Int,
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_update.csv"),
            FivetranFileCompression::Off,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert_eq!(
            rows.collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            vec![
                FileRow(btreemap! {
                    FivetranFieldName::from_str("id")? => FivetranFileValue::Value(FivetranValue::Long(3)),
                    FivetranFieldName::from_str("title")? => FivetranFileValue::Unmodified,
                    FivetranFieldName::from_str("magic_number")? => FivetranFileValue::Value(FivetranValue::Int(15)),
                    FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(false)),
                    FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                        seconds: 1707436799,
                        nanos: 999_999_999,
                    })),
                }),
                FileRow(btreemap! {
                    FivetranFieldName::from_str("id")? => FivetranFileValue::Value(FivetranValue::Long(2)),
                    FivetranFieldName::from_str("title")? => FivetranFileValue::Value(FivetranValue::String("The empire strikes back".into())),
                    FivetranFieldName::from_str("magic_number")? => FivetranFileValue::Unmodified,
                    FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(false)),
                    FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                        seconds: 1707436800,
                        nanos: 0,
                    })),
                }),
                FileRow(btreemap! {
                    FivetranFieldName::from_str("id")? => FivetranFileValue::Value(FivetranValue::Long(99)),
                    FivetranFieldName::from_str("title")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                    FivetranFieldName::from_str("magic_number")? => FivetranFileValue::Value(FivetranValue::Int(99)),
                    FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(false)),
                    FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                        seconds: 1704773419,
                        nanos: 156057706,
                    })),
                }),
            ],
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_backslash_in_csv_files_are_not_escape_chars() -> anyhow::Result<()> {
        // See https://github.com/fivetran/fivetran_sdk/blob/main/development-guide.md#csv

        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod"),
            null_string: String::from("null"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "path" => FivetranDataType::String,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("backslash.csv"),
            FivetranFileCompression::Off,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert_eq!(
            rows.collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            vec![FileRow(btreemap! {
                FivetranFieldName::from_str("path")? => FivetranFileValue::Value(FivetranValue::String("C:\\Program Files\\".to_string())),
            }),],
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_an_uncompressed_fivetran_csv_file() -> anyhow::Result<()> {
        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod-NcK9NIjPUutCsz4mjOQQztbnwnE1sY3"),
            null_string: String::from("null-m8yilkvPsNulehxl2G6pmSQ3G3WWdLP"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Long,
            "title" => FivetranDataType::String,
            "magic_number" => FivetranDataType::Int,
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_delete.csv"),
            FivetranFileCompression::Off,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert_eq!(
            rows.collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            vec![FileRow(btreemap! {
                FivetranFieldName::from_str("id")? => FivetranFileValue::Value(FivetranValue::Long(1)),
                FivetranFieldName::from_str("title")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                FivetranFieldName::from_str("magic_number")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(true)),
                FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                    seconds: 0,
                    nanos: 0,
                })),
            }),],
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_a_zstd_compressed_fivetran_csv_file() -> anyhow::Result<()> {
        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod-NcK9NIjPUutCsz4mjOQQztbnwnE1sY3"),
            null_string: String::from("null-m8yilkvPsNulehxl2G6pmSQ3G3WWdLP"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Long,
            "title" => FivetranDataType::String,
            "magic_number" => FivetranDataType::Int,
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_delete.csv.zst"),
            FivetranFileCompression::Zstd,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert_eq!(
            rows.collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            vec![FileRow(btreemap! {
                FivetranFieldName::from_str("id")? => FivetranFileValue::Value(FivetranValue::Long(1)),
                FivetranFieldName::from_str("title")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                FivetranFieldName::from_str("magic_number")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(true)),
                FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                    seconds: 0,
                    nanos: 0,
                })),
            }),],
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_an_encrypted_file() -> anyhow::Result<()> {
        let mut key = Aes256Key::default();
        File::open(fixture_path("books_batch_1_insert.csv.zst.aes.key"))
            .await?
            .read_exact(&mut key.0)
            .await?;

        let params = FivetranReaderParams {
            unmodified_string: String::from(""),
            null_string: String::from(""),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Long,
            "title" => FivetranDataType::String,
            "magic_number" => FivetranDataType::Int,
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_batch_1_insert.csv.zst.aes"),
            FivetranFileCompression::Zstd,
            FivetranFileEncryption::Aes { key },
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert_eq!(
            rows.collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            vec![
                FileRow(btreemap! {
                        FivetranFieldName::from_str("id")? =>
                FivetranFileValue::Value(FivetranValue::Long(1)),
                        FivetranFieldName::from_str("title")? =>
                FivetranFileValue::Value(FivetranValue::String("The Hitchhiker's Guide to the Galaxy".into())),
                        FivetranFieldName::from_str("magic_number")? =>
                FivetranFileValue::Value(FivetranValue::Int(42)),
                        FivetranFieldName::from_str("_fivetran_deleted")? =>
                FivetranFileValue::Value(FivetranValue::Bool(false)),
                        FivetranFieldName::from_str("_fivetran_synced")? =>
                        FivetranFileValue::Value(try_parse_fivetran_value("2024-01-08T18:52:07.754685370Z".into(), FivetranDataType::UtcDatetime)?),
                }),
                FileRow(btreemap! {
                        FivetranFieldName::from_str("id")? =>
                FivetranFileValue::Value(FivetranValue::Long(2)),
                        FivetranFieldName::from_str("title")? =>
                FivetranFileValue::Value(FivetranValue::String("The Lord of the Rings".into())),
                        FivetranFieldName::from_str("magic_number")? =>
                FivetranFileValue::Value(FivetranValue::Int(1)),
                        FivetranFieldName::from_str("_fivetran_deleted")? =>
                FivetranFileValue::Value(FivetranValue::Bool(false)),
                        FivetranFieldName::from_str("_fivetran_synced")? =>
                        FivetranFileValue::Value(try_parse_fivetran_value("2024-01-08T18:52:07.754685370Z".into(), FivetranDataType::UtcDatetime)?),
                }),
            ],
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_a_gzip_compressed_fivetran_csv_file() -> anyhow::Result<()> {
        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod-NcK9NIjPUutCsz4mjOQQztbnwnE1sY3"),
            null_string: String::from("null-m8yilkvPsNulehxl2G6pmSQ3G3WWdLP"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Long,
            "title" => FivetranDataType::String,
            "magic_number" => FivetranDataType::Int,
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_delete.csv.gz"),
            FivetranFileCompression::Gzip,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert_eq!(
            rows.collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?,
            vec![FileRow(btreemap! {
                FivetranFieldName::from_str("id")? => FivetranFileValue::Value(FivetranValue::Long(1)),
                FivetranFieldName::from_str("title")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                FivetranFieldName::from_str("magic_number")? => FivetranFileValue::Value(FivetranValue::Null(true)),
                FivetranFieldName::from_str("_fivetran_deleted")? => FivetranFileValue::Value(FivetranValue::Bool(true)),
                FivetranFieldName::from_str("_fivetran_synced")? => FivetranFileValue::Value(FivetranValue::UtcDatetime(Timestamp {
                    seconds: 0,
                    nanos: 0,
                })),
            }),],
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_parsing_fails_when_a_column_is_missing_from_the_schema() -> anyhow::Result<()> {
        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod-NcK9NIjPUutCsz4mjOQQztbnwnE1sY3"),
            null_string: String::from("magic-nullvalue"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Short,
            "title" => FivetranDataType::String,
            // magic_number missing
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_update.csv"),
            FivetranFileCompression::Off,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert!(rows
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_parsing_fails_when_a_column_is_incorrect_in_the_schema() -> anyhow::Result<()> {
        let params = FivetranReaderParams {
            unmodified_string: String::from("unmod-NcK9NIjPUutCsz4mjOQQztbnwnE1sY3"),
            null_string: String::from("magic-nullvalue"),
        };
        let schema = fivetran_table_schema(btreemap! {
            "id" => FivetranDataType::Short,
            "title" => FivetranDataType::Int, // !
            "magic_number" => FivetranDataType::Int,
            "_fivetran_deleted" => FivetranDataType::Boolean,
            "_fivetran_synced" => FivetranDataType::UtcDatetime,
        });
        let mut deserializer = create_csv_deserializer(
            &fixture_path("books_update.csv"),
            FivetranFileCompression::Off,
            FivetranFileEncryption::None,
        )
        .await?;
        let rows = read_rows(&mut deserializer, &params, &schema);

        assert!(rows
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .is_err());

        Ok(())
    }

    #[test]
    fn test_parse_boolean() {
        assert_eq!(
            try_parse_fivetran_value("true".into(), FivetranDataType::Boolean).unwrap(),
            FivetranValue::Bool(true)
        );
        assert_eq!(
            try_parse_fivetran_value("false".into(), FivetranDataType::Boolean).unwrap(),
            FivetranValue::Bool(false)
        );
    }

    #[test]
    fn test_parse_short() {
        assert_eq!(
            try_parse_fivetran_value("-32768".into(), FivetranDataType::Short).unwrap(),
            FivetranValue::Short(-32768)
        );
        assert_eq!(
            try_parse_fivetran_value("123".into(), FivetranDataType::Short).unwrap(),
            FivetranValue::Short(123)
        );
        assert_eq!(
            try_parse_fivetran_value("32767".into(), FivetranDataType::Short).unwrap(),
            FivetranValue::Short(32767)
        );
    }

    #[test]
    fn test_parse_int() {
        assert_eq!(
            try_parse_fivetran_value("-2147483648".into(), FivetranDataType::Int).unwrap(),
            FivetranValue::Int(-2147483648)
        );
        assert_eq!(
            try_parse_fivetran_value("456".into(), FivetranDataType::Int).unwrap(),
            FivetranValue::Int(456)
        );
        assert_eq!(
            try_parse_fivetran_value("2147483647".into(), FivetranDataType::Int).unwrap(),
            FivetranValue::Int(2147483647)
        );
    }

    #[test]
    fn test_parse_long() {
        assert_eq!(
            try_parse_fivetran_value("-9223372036854775808".into(), FivetranDataType::Long)
                .unwrap(),
            FivetranValue::Long(i64::MIN)
        );
        assert_eq!(
            try_parse_fivetran_value("789".into(), FivetranDataType::Long).unwrap(),
            FivetranValue::Long(789)
        );
        assert_eq!(
            try_parse_fivetran_value("9223372036854775807".into(), FivetranDataType::Long).unwrap(),
            FivetranValue::Long(i64::MAX)
        );
    }

    #[test]
    fn test_parse_decimal() {
        assert_eq!(
            try_parse_fivetran_value(
                "3.1415926535897932384626433832".into(),
                FivetranDataType::Decimal
            )
            .unwrap(),
            FivetranValue::Decimal("3.1415926535897932384626433832".into())
        );
    }

    #[test]
    fn test_parse_float() {
        assert_eq!(
            try_parse_fivetran_value("1.23".into(), FivetranDataType::Float).unwrap(),
            FivetranValue::Float(1.23)
        );
        assert_eq!(
            try_parse_fivetran_value("3.4028236E+24".into(), FivetranDataType::Float).unwrap(),
            FivetranValue::Float(3.402_823_6E24)
        );
    }

    #[test]
    fn test_parse_double() {
        assert_eq!(
            try_parse_fivetran_value("4.56".into(), FivetranDataType::Double).unwrap(),
            FivetranValue::Double(4.56)
        );
        assert_eq!(
            try_parse_fivetran_value("-2.2250738585072014E-308".into(), FivetranDataType::Double)
                .unwrap(),
            FivetranValue::Double(-2.2250738585072014E-308)
        );
    }

    #[test]
    fn test_parse_naive_date() {
        assert_eq!(
            try_parse_fivetran_value("2007-12-03".into(), FivetranDataType::NaiveDate).unwrap(),
            FivetranValue::NaiveDate(Timestamp {
                seconds: 1196640000,
                nanos: 0,
            })
        );
    }

    #[test]
    fn test_parse_naive_time() {
        assert_eq!(
            try_parse_fivetran_value("19:41:30".into(), FivetranDataType::NaiveTime).unwrap(),
            FivetranValue::NaiveTime(Timestamp {
                seconds: 19 * 60 * 60 + 41 * 60 + 30,
                nanos: 0,
            })
        );
        assert_eq!(
            try_parse_fivetran_value("19:41:30.500".into(), FivetranDataType::NaiveTime).unwrap(),
            FivetranValue::NaiveTime(Timestamp {
                seconds: 19 * 60 * 60 + 41 * 60 + 30,
                nanos: 500_000_000,
            })
        );
    }

    #[test]
    fn test_parse_naive_datetime() {
        assert_eq!(
            try_parse_fivetran_value(
                "2007-12-03T10:15:30".into(),
                FivetranDataType::NaiveDatetime
            )
            .unwrap(),
            FivetranValue::NaiveDatetime(Timestamp {
                seconds: 1196676930,
                nanos: 0,
            })
        );
        assert_eq!(
            try_parse_fivetran_value(
                "1970-01-01T00:00:00".into(),
                FivetranDataType::NaiveDatetime
            )
            .unwrap(),
            FivetranValue::NaiveDatetime(Timestamp {
                seconds: 0,
                nanos: 0,
            })
        );
        assert_eq!(
            try_parse_fivetran_value(
                "1970-01-01T00:00:00.123".into(),
                FivetranDataType::NaiveDatetime
            )
            .unwrap(),
            FivetranValue::NaiveDatetime(Timestamp {
                seconds: 0,
                nanos: 123_000_000,
            })
        );
    }

    #[test]
    fn test_parse_utc_datetime() {
        assert_eq!(
            try_parse_fivetran_value(
                "2007-12-03T10:15:30.123Z".into(),
                FivetranDataType::UtcDatetime
            )
            .unwrap(),
            FivetranValue::UtcDatetime(Timestamp {
                seconds: 1196676930,
                nanos: 123_000_000,
            })
        );
    }

    #[test]
    fn test_parse_binary() {
        assert_eq!(
            try_parse_fivetran_value("SGVsbG8gd29ybGQ=".to_string(), FivetranDataType::Binary)
                .unwrap(),
            FivetranValue::Binary(b"Hello world".to_vec())
        );
    }

    #[test]
    fn test_parse_xml() {
        assert_eq!(
            try_parse_fivetran_value("<tag></tag>".into(), FivetranDataType::Xml).unwrap(),
            FivetranValue::Xml("<tag></tag>".into())
        );
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(
            try_parse_fivetran_value("Hello world".into(), FivetranDataType::String).unwrap(),
            FivetranValue::String("Hello world".into())
        );
    }

    #[test]
    fn test_parse_json() {
        assert_eq!(
            try_parse_fivetran_value("{\"key\": \"value\"}".into(), FivetranDataType::Json)
                .unwrap(),
            FivetranValue::Json("{\"key\": \"value\"}".into())
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn test_fivetran_csv_conversion_roundtrips(value in any::<FivetranValue>()) {
            if let FivetranValue::Null(_) = value {
                return Ok(());
            }
            let value_type = fivetran_data_type(&value).unwrap();

            let as_str = to_csv_string_representation(&value).unwrap();
            prop_assert_eq!(value, try_parse_fivetran_value(as_str, value_type).unwrap());
        }
    }
}
