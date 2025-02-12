use std::{
    collections::BTreeMap,
    io,
    str::FromStr,
    sync::LazyLock,
};

use anyhow::Context;
use async_zip_reader::{
    ZipError,
    ZipFileEntry,
    ZipReader,
};
use bytes::Bytes;
use common::{
    bootstrap_model::tables::TABLES_TABLE,
    components::{
        ComponentName,
        ComponentPath,
    },
    knobs::TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    types::FieldName,
};
use errors::ErrorMetadata;
use futures::{
    pin_mut,
    AsyncBufReadExt,
    AsyncReadExt,
    Future,
    StreamExt,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use model::{
    file_storage::FILE_STORAGE_VIRTUAL_TABLE,
    snapshot_imports::types::ImportFormat,
};
use regex::Regex;
use serde_json::{
    json,
    Value as JsonValue,
};
use shape_inference::{
    export_context::{
        ExportContext,
        GeneratedSchema,
    },
    ProdConfigWithOptionalFields,
    Shape,
    ShapeConfig,
};
use storage::StorageGetStream;
use tokio::io::AsyncBufReadExt as _;
use value::{
    id_v6::DeveloperDocumentId,
    TableName,
};

use crate::snapshot_import::import_error::ImportError;

#[derive(Debug)]
pub enum ImportUnit {
    Object(JsonValue),
    NewTable(ComponentPath, TableName),
    GeneratedSchema(
        ComponentPath,
        TableName,
        GeneratedSchema<ProdConfigWithOptionalFields>,
    ),
    StorageFileChunk(DeveloperDocumentId, Bytes),
}

static COMPONENT_NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*/)?_components/([^/]+)/$").unwrap());
static GENERATED_SCHEMA_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*/)?([^/]+)/generated_schema\.jsonl$").unwrap());
static DOCUMENTS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*/)?([^/]+)/documents\.jsonl$").unwrap());
// _storage/(ID) with optional ignored prefix and extension like
// snapshot/_storage/(ID).png
static STORAGE_FILE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(.*/)?_storage/([^/.]+)(?:\.[^/]+)?$").unwrap());

fn map_zip_error(e: anyhow::Error) -> anyhow::Error {
    if let Some(ZipError::Io(_)) = e.downcast_ref::<ZipError>() {
        e
    } else {
        // Everything else indicates a Zip file that cannot be parsed.
        e.context(ErrorMetadata::bad_request("InvalidZip", "invalid zip file"))
    }
}

fn map_zip_io_error(e: io::Error) -> anyhow::Error {
    if e.kind() == io::ErrorKind::Other {
        // S3 errors get mapped into ErrorKind::Other
        e.into()
    } else {
        anyhow::Error::from(e).context(ErrorMetadata::bad_request("InvalidZip", "invalid zip file"))
    }
}

fn map_csv_error(e: csv_async::Error) -> anyhow::Error {
    let pos_line =
        |pos: &Option<csv_async::Position>| pos.as_ref().map_or(0, |pos| pos.line() as usize);
    match e.kind() {
        csv_async::ErrorKind::Utf8 { pos, .. } => {
            ImportError::CsvInvalidRow(pos_line(pos), e).into()
        },
        csv_async::ErrorKind::UnequalLengths { pos, .. } => {
            ImportError::CsvRowMissingFields(pos_line(pos)).into()
        },
        // IO and Seek are errors from the underlying stream.
        csv_async::ErrorKind::Io(_)
        | csv_async::ErrorKind::Seek
        // We're not using serde for CSV parsing, so these errors are unexpected
        | csv_async::ErrorKind::Serialize(_)
        | csv_async::ErrorKind::Deserialize { .. }
        => e.into(),
        _ => e.into(),
    }
}

/// Parse and stream units from the imported file, starting with a NewTable
/// for each table and then Objects for each object to import into the table.
/// stream_body returns the file as streamed bytes. stream_body() can be called
/// multiple times to read the file multiple times, for cases where the file
/// must be read out of order, e.g. because the _tables table must be imported
/// first.
/// Objects are yielded with the following guarantees:
/// 1. When an Object is yielded, it is in the table corresponding to the most
///    recently yielded NewTable.
/// 2. When a StorageFileChunk is yielded, it is in the _storage table
///    corresponding to the most recently yielded NewTable.
/// 3. All StorageFileChunks for a single file are yielded contiguously, in
///    order.
/// 4. If a table has a GeneratedSchema, the GeneratedSchema will be yielded
///    before any Objects in that table.
#[try_stream(ok = ImportUnit, error = anyhow::Error)]
pub async fn parse_objects<'a, Fut>(
    format: ImportFormat,
    component_path: ComponentPath,
    stream_body: impl Fn() -> Fut + 'a,
) where
    Fut: Future<Output = anyhow::Result<StorageGetStream>> + 'a,
{
    match format {
        ImportFormat::Csv(table_name) => {
            let reader = stream_body().await?;
            yield ImportUnit::NewTable(component_path, table_name);
            let mut reader = csv_async::AsyncReader::from_reader(reader.into_reader());
            if !reader.has_headers() {
                anyhow::bail!(ImportError::CsvMissingHeaders);
            }
            let field_names = {
                let headers = reader.headers().await.map_err(map_csv_error)?;
                headers
                    .iter()
                    .map(|s| {
                        let trimmed = s.trim_matches(' ');
                        let field_name = FieldName::from_str(trimmed)
                            .map_err(|e| ImportError::CsvInvalidHeader(trimmed.to_string(), e))?;
                        Ok(field_name)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?
            };
            let mut enumerate_rows = reader.records().enumerate();
            while let Some((i, row_r)) = enumerate_rows.next().await {
                let lineno = i + 1;
                let parsed_row = row_r
                    .map_err(map_csv_error)?
                    .iter()
                    .map(parse_csv_cell)
                    .collect::<Vec<JsonValue>>();
                let mut obj = BTreeMap::new();
                if field_names.len() != parsed_row.len() {
                    anyhow::bail!(ImportError::CsvRowMissingFields(lineno));
                }
                for (field_name, value) in field_names.iter().zip(parsed_row.into_iter()) {
                    obj.insert(field_name.to_string(), value);
                }
                yield ImportUnit::Object(serde_json::to_value(obj)?);
            }
        },
        ImportFormat::JsonLines(table_name) => {
            let mut reader = stream_body().await?.into_reader();
            yield ImportUnit::NewTable(component_path, table_name);
            let mut line = String::new();
            let mut lineno = 1;
            while reader
                .read_line(&mut line)
                .await
                .map_err(ImportError::NotUtf8)?
                > 0
            {
                let v: serde_json::Value = serde_json::from_str(&line)
                    .map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
                yield ImportUnit::Object(v);
                line.clear();
                lineno += 1;
            }
        },
        ImportFormat::JsonArray(table_name) => {
            let reader = stream_body().await?;
            yield ImportUnit::NewTable(component_path, table_name);
            let mut buf = Vec::new();
            let mut truncated_reader = reader
                .into_reader()
                .take((*TRANSACTION_MAX_USER_WRITE_SIZE_BYTES as u64) + 1);
            truncated_reader.read_to_end(&mut buf).await?;
            if buf.len() > *TRANSACTION_MAX_USER_WRITE_SIZE_BYTES {
                anyhow::bail!(ImportError::JsonArrayTooLarge(buf.len()));
            }
            let v: serde_json::Value =
                serde_json::from_slice(&buf).map_err(ImportError::NotJson)?;
            let array = v.as_array().ok_or(ImportError::NotJsonArray)?;
            for value in array.iter() {
                yield ImportUnit::Object(value.clone());
            }
        },
        ImportFormat::Zip => {
            let base_component_path = component_path;
            let reader = stream_body().await?;
            let temp_file = copy_to_temp_file(reader).await?;
            let mut zip_reader = ZipReader::new(std::io::BufReader::new(temp_file))
                .await
                .map_err(map_zip_error)?;
            let filenames: Vec<_> = zip_reader.file_names().await?;
            {
                // First pass, all the things we can store in memory:
                // a. _tables/documents.jsonl
                // b. _storage/documents.jsonl
                // c. user_table/generated_schema.jsonl
                // _tables needs to be imported before user tables so we can
                // pick table numbers correctly for schema validation.
                // Each generated schema must be parsed before the corresponding
                // table/documents.jsonl file, so we correctly infer types from
                // export-formatted JsonValues.
                let mut table_metadata: BTreeMap<_, Vec<_>> = BTreeMap::new();
                let mut storage_metadata: BTreeMap<_, Vec<_>> = BTreeMap::new();
                let mut generated_schemas: BTreeMap<_, Vec<_>> = BTreeMap::new();
                for (i, filename) in filenames.iter().enumerate() {
                    let documents_table_name =
                        parse_documents_jsonl_table_name(filename, &base_component_path)?;
                    if let Some((component_path, table_name)) = documents_table_name.clone()
                        && table_name == *TABLES_TABLE
                    {
                        let entry_reader = zip_reader.by_index(i).await.map_err(map_zip_error)?;
                        table_metadata.insert(
                            component_path,
                            parse_documents_jsonl(entry_reader, &base_component_path)
                                .try_collect()
                                .await?,
                        );
                    } else if let Some((component_path, table_name)) = documents_table_name
                        && table_name == *FILE_STORAGE_VIRTUAL_TABLE
                    {
                        let entry_reader = zip_reader.by_index(i).await.map_err(map_zip_error)?;
                        storage_metadata.insert(
                            component_path,
                            parse_documents_jsonl(entry_reader, &base_component_path)
                                .try_collect()
                                .await?,
                        );
                    } else if let Some((component_path, table_name)) = parse_table_filename(
                        filename,
                        &base_component_path,
                        &GENERATED_SCHEMA_PATTERN,
                    )? {
                        let entry_reader = zip_reader.by_index(i).await.map_err(map_zip_error)?;
                        tracing::info!(
                            "importing zip file containing generated_schema {table_name}"
                        );
                        let generated_schema =
                            parse_generated_schema(filename, entry_reader.read()).await?;
                        generated_schemas
                            .entry(component_path.clone())
                            .or_default()
                            .push(ImportUnit::GeneratedSchema(
                                component_path,
                                table_name,
                                generated_schema,
                            ));
                    }
                }
                for table_unit in table_metadata.into_values().flatten() {
                    yield table_unit;
                }
                for generated_schema_unit in generated_schemas.into_values().flatten() {
                    yield generated_schema_unit;
                }
                for (component_path, storage_metadata) in storage_metadata {
                    if !storage_metadata.is_empty() {
                        // Yield NewTable for _storage and Object for each storage file's metadata.
                        for storage_unit in storage_metadata {
                            yield storage_unit;
                        }
                        // Yield StorageFileChunk for each file in this component.
                        for (i, filename) in filenames.iter().enumerate() {
                            if let Some((file_component_path, storage_id)) =
                                parse_storage_filename(filename, &base_component_path)?
                                && file_component_path == component_path
                            {
                                let mut entry_reader =
                                    zip_reader.by_index(i).await.map_err(map_zip_error)?.read();
                                tracing::info!(
                                    "importing zip file containing storage file {}",
                                    storage_id.encode()
                                );
                                while let buf =
                                    entry_reader.fill_buf().await.map_err(map_zip_io_error)?
                                    && !buf.is_empty()
                                {
                                    yield ImportUnit::StorageFileChunk(
                                        storage_id,
                                        Bytes::copy_from_slice(buf),
                                    );
                                    let len = buf.len();
                                    entry_reader.consume(len);
                                }
                                // In case it's an empty file, make sure we send at
                                // least one chunk.
                                yield ImportUnit::StorageFileChunk(storage_id, Bytes::new());
                            }
                        }
                    }
                }
            }

            // Second pass: user tables.
            for (i, filename) in filenames.iter().enumerate() {
                if let Some((_, table_name)) =
                    parse_documents_jsonl_table_name(filename, &base_component_path)?
                    && !table_name.is_system()
                {
                    let entry_reader = zip_reader.by_index(i).await.map_err(map_zip_error)?;
                    let stream = parse_documents_jsonl(entry_reader, &base_component_path);
                    pin_mut!(stream);
                    while let Some(unit) = stream.try_next().await? {
                        yield unit;
                    }
                }
            }
        },
    }
}

// Copy an object to disk so that we can more efficiently seek through the file.
// TODO: write something that can efficiently seek through storage objects
async fn copy_to_temp_file(reader: StorageGetStream) -> anyhow::Result<std::fs::File> {
    let size = reader.content_length;
    let file = common::runtime::block_in_place(|| {
        let file = tempfile::tempfile().context("Failed to create temp file")?;
        #[cfg(target_os = "linux")]
        if size > 0 {
            unsafe {
                use std::os::fd::AsRawFd;
                if libc::fallocate64(
                    file.as_raw_fd(),
                    0, /* mode */
                    0, /* offset */
                    size,
                ) < 0
                {
                    return Err(anyhow::Error::from(std::io::Error::last_os_error())
                        .context(format!("Failed to fallocate {size} bytes")));
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            _ = size;
        }
        anyhow::Ok(file)
    })?;
    let mut tokio_file = tokio::fs::File::from_std(file);
    tokio::io::copy_buf(&mut reader.into_tokio_reader(), &mut tokio_file)
        .await
        .context("Failed to copy snapshot to temp file")?;
    // N.B.: it's ok that this file is seeked to the end because the ZipReader is
    // immediately going to seek it anyway.
    Ok(tokio_file.into_std().await)
}

pub fn parse_component_path(
    mut filename: &str,
    base_component_path: &ComponentPath,
) -> anyhow::Result<ComponentPath> {
    let mut component_names = Vec::new();
    while let Some(captures) = COMPONENT_NAME_PATTERN.captures(filename) {
        filename = captures.get(1).map_or("", |c| c.as_str());
        let component_name_str = captures
            .get(2)
            .expect("regex has two capture groups")
            .as_str();
        let component_name: ComponentName = component_name_str.parse().map_err(|e| {
            ErrorMetadata::bad_request(
                "InvalidComponentName",
                format!("component name '{component_name_str}' invalid: {e}"),
            )
        })?;
        component_names.push(component_name);
    }
    component_names.reverse();
    let mut component_path = base_component_path.clone();
    for component_name in component_names {
        component_path = component_path.push(component_name);
    }
    Ok(component_path)
}

fn parse_table_filename(
    filename: &str,
    base_component_path: &ComponentPath,
    regex: &Regex,
) -> anyhow::Result<Option<(ComponentPath, TableName)>> {
    match regex.captures(filename) {
        None => Ok(None),
        Some(captures) => {
            let table_name_str = captures
                .get(2)
                .expect("regex has two capture groups")
                .as_str();
            let table_name = table_name_str.parse().map_err(|e| {
                ErrorMetadata::bad_request(
                    "InvalidTableName",
                    format!("table name '{table_name_str}' invalid: {e}"),
                )
            })?;
            let prefix = captures.get(1).map_or("", |c| c.as_str());
            let component_path = parse_component_path(prefix, base_component_path)?;
            Ok(Some((component_path, table_name)))
        },
    }
}

fn parse_storage_filename(
    filename: &str,
    base_component_path: &ComponentPath,
) -> anyhow::Result<Option<(ComponentPath, DeveloperDocumentId)>> {
    match STORAGE_FILE_PATTERN.captures(filename) {
        None => Ok(None),
        Some(captures) => {
            let storage_id_str = captures
                .get(2)
                .expect("regex has two capture groups")
                .as_str();
            if storage_id_str == "documents" {
                return Ok(None);
            }
            let storage_id = DeveloperDocumentId::decode(storage_id_str).map_err(|e| {
                ErrorMetadata::bad_request(
                    "InvalidStorageId",
                    format!("_storage id '{storage_id_str}' invalid: {e}"),
                )
            })?;
            let prefix = captures.get(1).map_or("", |c| c.as_str());
            let component_path = parse_component_path(prefix, base_component_path)?;
            Ok(Some((component_path, storage_id)))
        },
    }
}

fn parse_documents_jsonl_table_name(
    filename: &str,
    base_component_path: &ComponentPath,
) -> anyhow::Result<Option<(ComponentPath, TableName)>> {
    parse_table_filename(filename, base_component_path, &DOCUMENTS_PATTERN)
}

#[try_stream(ok = ImportUnit, error = anyhow::Error)]
async fn parse_documents_jsonl<'a>(
    entry_reader: ZipFileEntry<'a>,
    base_component_path: &'a ComponentPath,
) {
    let (component_path, table_name) =
        parse_documents_jsonl_table_name(entry_reader.name(), base_component_path)?
            .context("expected documents.jsonl file")?;
    tracing::info!("importing zip file containing table {table_name}");
    yield ImportUnit::NewTable(component_path, table_name);
    let mut reader = entry_reader.read();
    let mut line = String::new();
    let mut lineno = 1;
    while reader
        .read_line(&mut line)
        .await
        .map_err(map_zip_io_error)?
        > 0
    {
        let v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
        yield ImportUnit::Object(v);
        line.clear();
        lineno += 1;
    }
}

async fn parse_generated_schema<T: ShapeConfig, R: tokio::io::AsyncBufRead + Unpin>(
    filename: &str,
    mut entry_reader: R,
) -> anyhow::Result<GeneratedSchema<T>> {
    let mut line = String::new();
    let mut lineno = 1;
    entry_reader
        .read_line(&mut line)
        .await
        .map_err(ImportError::NotUtf8)?;
    let inferred_type_json: serde_json::Value =
        serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
    let inferred_type = Shape::from_str(inferred_type_json.as_str().with_context(|| {
        ImportError::InvalidConvexValue(
            lineno,
            anyhow::anyhow!("first line of generated_schema must be a string"),
        )
    })?)
    .with_context(|| {
        ErrorMetadata::bad_request("InvalidGeneratedSchema", format!("cannot parse {filename}"))
    })?;
    line.clear();
    lineno += 1;
    let mut overrides = BTreeMap::new();
    while entry_reader
        .read_line(&mut line)
        .await
        .map_err(ImportError::NotUtf8)?
        > 0
    {
        let mut v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
        let o = v.as_object_mut().with_context(|| {
            ImportError::InvalidConvexValue(lineno, anyhow::anyhow!("overrides should be object"))
        })?;
        if o.len() != 1 {
            anyhow::bail!(ImportError::InvalidConvexValue(
                lineno,
                anyhow::anyhow!("override object should have one item")
            ));
        }
        let (key, value) = o.into_iter().next().context("must have one item")?;
        let export_context = ExportContext::try_from(value.clone())
            .map_err(|e| ImportError::InvalidConvexValue(lineno, e))?;
        overrides.insert(
            DeveloperDocumentId::decode(key)
                .map_err(|e| ImportError::InvalidConvexValue(lineno, e.into()))?,
            export_context,
        );

        line.clear();
        lineno += 1;
    }
    let generated_schema = GeneratedSchema {
        inferred_shape: inferred_type,
        overrides,
    };
    Ok(generated_schema)
}

// For now, we only parse out floats and strings in CSV files.
pub fn parse_csv_cell(s: &str) -> JsonValue {
    if let Ok(r) = s.parse::<f64>() {
        return json!(r);
    }
    json!(s)
}

#[cfg(test)]
mod tests {
    use common::components::ComponentPath;

    use crate::snapshot_import::parse::{
        parse_documents_jsonl_table_name,
        parse_storage_filename,
        parse_table_filename,
        GENERATED_SCHEMA_PATTERN,
    };

    #[test]
    fn test_filename_regex() -> anyhow::Result<()> {
        let (_, table_name) =
            parse_documents_jsonl_table_name("users/documents.jsonl", &ComponentPath::root())?
                .unwrap();
        assert_eq!(table_name, "users".parse()?);
        // Regression test, checking that the '.' is escaped.
        assert!(
            parse_documents_jsonl_table_name("users/documentsxjsonl", &ComponentPath::root())?
                .is_none()
        );
        // When an export is unzipped and re-zipped, sometimes there's a prefix.
        let (_, table_name) = parse_documents_jsonl_table_name(
            "snapshot/users/documents.jsonl",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(table_name, "users".parse()?);
        let (_, table_name) = parse_table_filename(
            "users/generated_schema.jsonl",
            &ComponentPath::root(),
            &GENERATED_SCHEMA_PATTERN,
        )?
        .unwrap();
        assert_eq!(table_name, "users".parse()?);
        let (_, storage_id) = parse_storage_filename(
            "_storage/kg2ah8mk1xtg35g7zyexyc96e96yr74f.gif",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(&storage_id.to_string(), "kg2ah8mk1xtg35g7zyexyc96e96yr74f");
        let (_, storage_id) = parse_storage_filename(
            "snapshot/_storage/kg2ah8mk1xtg35g7zyexyc96e96yr74f.gif",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(&storage_id.to_string(), "kg2ah8mk1xtg35g7zyexyc96e96yr74f");
        // No file extension.
        let (_, storage_id) = parse_storage_filename(
            "_storage/kg2ah8mk1xtg35g7zyexyc96e96yr74f",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(&storage_id.to_string(), "kg2ah8mk1xtg35g7zyexyc96e96yr74f");
        Ok(())
    }

    #[test]
    fn test_component_path_regex() -> anyhow::Result<()> {
        let (component_path, table_name) = parse_documents_jsonl_table_name(
            "_components/waitlist/tbl/documents.jsonl",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(&String::from(component_path), "waitlist");
        assert_eq!(&table_name.to_string(), "tbl");

        let (component_path, table_name) = parse_documents_jsonl_table_name(
            "some/parentdir/_components/waitlist/tbl/documents.jsonl",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(&String::from(component_path), "waitlist");
        assert_eq!(&table_name.to_string(), "tbl");

        let (component_path, table_name) = parse_documents_jsonl_table_name(
            "_components/waitlist/_components/ratelimit/tbl/documents.jsonl",
            &ComponentPath::root(),
        )?
        .unwrap();
        assert_eq!(&String::from(component_path), "waitlist/ratelimit");
        assert_eq!(&table_name.to_string(), "tbl");

        let (component_path, table_name) = parse_documents_jsonl_table_name(
            "_components/waitlist/_components/ratelimit/tbl/documents.jsonl",
            &"friendship".parse()?,
        )?
        .unwrap();
        assert_eq!(
            &String::from(component_path),
            "friendship/waitlist/ratelimit"
        );
        assert_eq!(&table_name.to_string(), "tbl");

        let (component_path, table_name) = parse_documents_jsonl_table_name(
            "tbl/documents.jsonl",
            &"waitlist/ratelimit".parse()?,
        )?
        .unwrap();
        assert_eq!(&String::from(component_path), "waitlist/ratelimit");
        assert_eq!(&table_name.to_string(), "tbl");

        Ok(())
    }
}
