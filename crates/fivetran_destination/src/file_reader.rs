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
use fivetran_common::fivetran_sdk::{
    value_type::Inner as FivetranValue,
    Compression as FivetranFileCompression,
    DataType as FivetranDataType,
    FileParams,
};
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
    api_types::FivetranFieldName,
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
