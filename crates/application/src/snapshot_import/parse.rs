use std::{
    collections::BTreeMap,
    io,
    str::FromStr,
    sync::{
        Arc,
        LazyLock,
    },
};

use anyhow::Context;
use bytes::Bytes;
use common::{
    bootstrap_model::tables::TABLES_TABLE,
    components::{
        ComponentName,
        ComponentPath,
    },
    knobs::TRANSACTION_MAX_USER_WRITE_SIZE_BYTES,
    types::{
        FieldName,
        FullyQualifiedObjectKey,
    },
};
use errors::ErrorMetadata;
use futures::{
    stream::{
        self,
        BoxStream,
    },
    AsyncBufReadExt,
    AsyncReadExt,
    StreamExt,
    TryStreamExt,
};
use futures_async_stream::{
    try_stream,
    try_stream_block,
};
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
    ProdConfig,
    Shape,
    ShapeConfig,
};
use storage::{
    Storage,
    StorageExt,
};
use storage_zip_reader::StorageZipArchive;
use tokio::io::{
    AsyncBufReadExt as _,
    AsyncRead,
    BufReader,
};
use tokio_util::io::ReaderStream;
use value::{
    id_v6::DeveloperDocumentId,
    TableName,
};

use crate::snapshot_import::import_error::ImportError;

pub type ImportDocumentStream = BoxStream<'static, anyhow::Result<JsonValue>>;
pub type ImportStorageFileStream = BoxStream<'static, anyhow::Result<Bytes>>;
pub struct ParsedImport {
    pub generated_schemas: Vec<(ComponentPath, TableName, GeneratedSchema<ProdConfig>)>,
    pub documents: Vec<(ComponentPath, TableName, ImportDocumentStream)>,
    pub storage_files: Vec<(ComponentPath, DeveloperDocumentId, ImportStorageFileStream)>,
}

impl ParsedImport {
    fn single_table(
        component_path: ComponentPath,
        table_name: TableName,
        documents: ImportDocumentStream,
    ) -> Self {
        Self {
            generated_schemas: vec![],
            documents: vec![(component_path, table_name, documents)],
            storage_files: vec![],
        }
    }
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

fn map_zip_io_error(e: io::Error) -> anyhow::Error {
    if e.kind() == io::ErrorKind::InvalidData {
        // Content errors become InvalidData errors
        anyhow::Error::from(e).context(ErrorMetadata::bad_request("InvalidZip", "invalid zip file"))
    } else {
        // S3 errors get mapped into ErrorKind::Other
        e.into()
    }
}

fn map_csv_error(e: csv_async::Error) -> anyhow::Error {
    let pos_line = |pos: &Option<csv_async::Position>| pos.as_ref().map_or(0, |pos| pos.line());
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

/// Parse the imported file, returning separate streams for each table or
/// storage file.
pub async fn parse_import_file(
    format: ImportFormat,
    component_path: ComponentPath,
    storage: Arc<dyn Storage>,
    fq_object_key: FullyQualifiedObjectKey,
) -> anyhow::Result<ParsedImport> {
    let stream_body = || async {
        storage
            .get_fq_object(&fq_object_key)
            .await?
            .with_context(|| format!("Missing import object {fq_object_key:?}"))
    };
    match format {
        ImportFormat::Csv(table_name) => Ok(ParsedImport::single_table(
            component_path,
            table_name,
            parse_csv_import(stream_body().await?).boxed(),
        )),
        ImportFormat::JsonLines(table_name) => {
            let mut reader = stream_body().await?.into_reader();
            Ok(ParsedImport::single_table(
                component_path,
                table_name,
                try_stream_block!({
                    let mut line = String::new();
                    let mut lineno = 1;
                    while reader
                        .read_line(&mut line)
                        .await
                        .map_err(ImportError::NotUtf8)?
                        > 0
                    {
                        // Check for UTF-8 BOM at the start of the first line
                        if lineno == 1 && line.as_bytes().starts_with(&[0xEF, 0xBB, 0xBF]) {
                            anyhow::bail!(ImportError::Utf8BomNotSupported);
                        }
                        let v: serde_json::Value = serde_json::from_str(&line)
                            .map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
                        yield v;
                        line.clear();
                        lineno += 1;
                    }
                })
                .boxed(),
            ))
        },
        ImportFormat::JsonArray(table_name) => {
            let reader = stream_body().await?;
            let mut buf = Vec::new();
            let mut truncated_reader = reader
                .into_reader()
                .take((*TRANSACTION_MAX_USER_WRITE_SIZE_BYTES as u64) + 1);
            truncated_reader.read_to_end(&mut buf).await?;
            if buf.len() > *TRANSACTION_MAX_USER_WRITE_SIZE_BYTES {
                anyhow::bail!(ImportError::JsonArrayTooLarge(buf.len()));
            }
            let v: serde_json::Value = {
                // Check for UTF-8 BOM and reject it
                if buf.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    anyhow::bail!(ImportError::Utf8BomNotSupported);
                }
                serde_json::from_slice(&buf).map_err(ImportError::NotJson)?
            };
            let JsonValue::Array(array) = v else {
                anyhow::bail!(ImportError::NotJsonArray)
            };
            Ok(ParsedImport::single_table(
                component_path,
                table_name,
                stream::iter(array.into_iter().map(Ok)).boxed(),
            ))
        },
        ImportFormat::Zip => {
            let base_component_path = component_path;
            let zip_reader = StorageZipArchive::open_fq(storage, fq_object_key).await?;

            let mut import = ParsedImport {
                generated_schemas: vec![],
                documents: vec![],
                storage_files: vec![],
            };
            for entry in zip_reader.entries() {
                if let Some((component_path, table_name)) =
                    parse_documents_jsonl_table_name(&entry.name, &base_component_path)?
                {
                    if table_name.is_system()
                        && table_name != *TABLES_TABLE
                        && table_name != *FILE_STORAGE_VIRTUAL_TABLE
                    {
                        tracing::info!("Skipping system table entry {}", entry.name);
                        continue;
                    }
                    let entry_reader = zip_reader.read_entry(entry.clone());
                    tracing::info!(
                        "importing zip file containing table {component_path}:{table_name}"
                    );
                    import.documents.push((
                        component_path,
                        table_name,
                        parse_documents_jsonl(entry_reader).boxed(),
                    ));
                } else if let Some((component_path, table_name)) = parse_table_filename(
                    &entry.name,
                    &base_component_path,
                    &GENERATED_SCHEMA_PATTERN,
                )? {
                    let entry_reader = zip_reader.read_entry(entry.clone());
                    tracing::info!("importing zip file containing generated_schema {table_name}");
                    let generated_schema =
                        parse_generated_schema(&entry.name, entry_reader).await?;
                    import
                        .generated_schemas
                        .push((component_path, table_name, generated_schema));
                } else if let Some((component_path, storage_id)) =
                    parse_storage_filename(&entry.name, &base_component_path)?
                {
                    let entry_reader = zip_reader.read_entry(entry.clone());
                    import.storage_files.push((
                        component_path,
                        storage_id,
                        ReaderStream::new(entry_reader)
                            .map_err(anyhow::Error::from)
                            .boxed(),
                    ));
                }
            }
            Ok(import)
        },
    }
}

#[try_stream(ok = JsonValue, error = anyhow::Error)]
async fn parse_csv_import(reader: storage::StorageGetStream) {
    let mut reader = csv_async::AsyncReader::from_reader(reader.into_reader());
    if !reader.has_headers() {
        // TODO: this will never happen.
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
    let mut lineno = 0;
    let mut rows = reader.records();
    while let Some(row_r) = rows.next().await {
        lineno += 1;
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
        yield serde_json::to_value(obj)?;
    }
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

#[try_stream(ok = JsonValue, error = anyhow::Error)]
async fn parse_documents_jsonl(reader: impl AsyncRead + Unpin) {
    let mut line = String::new();
    let mut lineno = 1;
    let mut reader = BufReader::new(reader);
    while reader
        .read_line(&mut line)
        .await
        .map_err(map_zip_io_error)?
        > 0
    {
        let v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| ImportError::JsonInvalidRow(lineno, e))?;
        yield v;
        line.clear();
        lineno += 1;
    }
}

async fn parse_generated_schema<T: ShapeConfig>(
    filename: &str,
    entry_reader: impl tokio::io::AsyncRead + Unpin,
) -> anyhow::Result<GeneratedSchema<T>> {
    let mut line = String::new();
    let mut lineno = 1;
    let mut entry_reader = BufReader::new(entry_reader);
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
    .map_err(|e| {
        ErrorMetadata::bad_request(
            "InvalidGeneratedSchema",
            format!("cannot parse {filename}: {e:#}"),
        )
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
