use async_zip::{
    write::{
        EntryStreamWriter,
        ZipFileWriter,
    },
    Compression,
    ZipEntryBuilder,
    ZipEntryBuilderExt,
};
use bytes::Bytes;
use common::{
    self,
    async_compat::TokioAsyncWriteCompatExt,
    document::ResolvedDocument,
    types::TableName,
};
use futures::{
    pin_mut,
    AsyncWriteExt,
};
use serde_json::{
    json,
    Value as JsonValue,
};
use shape_inference::{
    export_context::GeneratedSchema,
    ShapeConfig,
};
use storage::ChannelWriter;
use tokio::io::AsyncBufRead;
use value::export::ValueFormat;

static AFTER_DOCUMENTS_CLEAN: Bytes = Bytes::from_static("\n".as_bytes());

// 0o644 => read-write for owner, read for everyone else.
const ZIP_ENTRY_PERMISSIONS: u16 = 0o644;

pub(super) static README_MD_CONTENTS: &str = r#"# Welcome to your Convex snapshot export!

This ZIP file contains a snapshot of the tables in your Convex deployment.

Documents for each table are listed as lines of JSON in
<table_name>/documents.jsonl files.

For details on the format and how to use this snapshot with npx convex import,
check out [the docs](https://docs.convex.dev/database/import-export/export) or
ask us in [Discord](http://convex.dev/community).
"#;

// 'a is lifetime of entire zip file writer.
// 'b is lifetime of entry writer for a single table.
pub struct ZipSnapshotTableUpload<'a, 'b> {
    entry_writer: EntryStreamWriter<'b, &'a mut ChannelWriter>,
}

impl<'a, 'b> ZipSnapshotTableUpload<'a, 'b> {
    async fn new(
        zip_writer: &'b mut ZipFileWriter<&'a mut ChannelWriter>,
        path_prefix: &str,
        table_name: TableName,
    ) -> anyhow::Result<Self> {
        let source_path = format!("{path_prefix}{table_name}/documents.jsonl");
        let builder = ZipEntryBuilder::new(source_path.clone(), Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let entry_writer = zip_writer.write_entry_stream(builder.build()).await?;
        Ok(Self { entry_writer })
    }

    pub async fn write(&mut self, doc: ResolvedDocument) -> anyhow::Result<()> {
        let json = doc.export(ValueFormat::ConvexCleanJSON);
        self.write_json_line(json).await
    }

    pub async fn write_json_line(&mut self, json: JsonValue) -> anyhow::Result<()> {
        let buf = serde_json::to_vec(&json)?;
        self.entry_writer.compat_mut_write().write_all(&buf).await?;
        self.entry_writer
            .compat_mut_write()
            .write_all(&AFTER_DOCUMENTS_CLEAN)
            .await?;
        Ok(())
    }

    pub async fn complete(self) -> anyhow::Result<()> {
        self.entry_writer.close().await?;
        Ok(())
    }
}

pub struct ZipSnapshotUpload<'a> {
    writer: ZipFileWriter<&'a mut ChannelWriter>,
}

impl<'a> ZipSnapshotUpload<'a> {
    pub async fn new(out: &'a mut ChannelWriter) -> anyhow::Result<Self> {
        let writer = ZipFileWriter::new(out);
        let mut zip_snapshot_upload = Self { writer };
        zip_snapshot_upload
            .stream_full_file("README.md".to_owned(), README_MD_CONTENTS.as_bytes())
            .await?;
        Ok(zip_snapshot_upload)
    }

    #[fastrace::trace]
    pub async fn stream_full_file(
        &mut self,
        path: String,
        contents: impl AsyncBufRead,
    ) -> anyhow::Result<()> {
        let builder = ZipEntryBuilder::new(path, Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let mut entry_writer = self.writer.write_entry_stream(builder.build()).await?;
        pin_mut!(contents);
        tokio::io::copy_buf(&mut contents, &mut entry_writer).await?;
        entry_writer.close().await?;
        Ok(())
    }

    pub async fn start_table(
        &mut self,
        path_prefix: &str,
        table_name: TableName,
    ) -> anyhow::Result<ZipSnapshotTableUpload<'a, '_>> {
        ZipSnapshotTableUpload::new(&mut self.writer, path_prefix, table_name).await
    }

    /// System tables have known shape, so we don't need to serialize it.
    pub async fn start_system_table(
        &mut self,
        path_prefix: &str,
        table_name: TableName,
    ) -> anyhow::Result<ZipSnapshotTableUpload<'a, '_>> {
        anyhow::ensure!(table_name.is_system());
        ZipSnapshotTableUpload::new(&mut self.writer, path_prefix, table_name).await
    }

    pub async fn write_generated_schema<T: ShapeConfig>(
        &mut self,
        path_prefix: &str,
        table_name: &TableName,
        generated_schema: GeneratedSchema<T>,
    ) -> anyhow::Result<()> {
        let generated_schema_path = format!("{path_prefix}{table_name}/generated_schema.jsonl");
        let builder = ZipEntryBuilder::new(generated_schema_path.clone(), Compression::Deflate)
            .unix_permissions(ZIP_ENTRY_PERMISSIONS);
        let mut entry_writer = self.writer.write_entry_stream(builder.build()).await?;
        let generated_schema_str = generated_schema.inferred_shape.to_string();
        entry_writer
            .compat_mut_write()
            .write_all(serde_json::to_string(&generated_schema_str)?.as_bytes())
            .await?;
        entry_writer.compat_mut_write().write_all(b"\n").await?;
        for (override_id, override_export_context) in generated_schema.overrides.into_iter() {
            let override_json =
                json!({override_id.encode(): JsonValue::from(override_export_context)});
            entry_writer
                .compat_mut_write()
                .write_all(serde_json::to_string(&override_json)?.as_bytes())
                .await?;
            entry_writer.compat_mut_write().write_all(b"\n").await?;
        }
        entry_writer.close().await?;
        Ok(())
    }

    pub async fn complete(self) -> anyhow::Result<()> {
        self.writer.close().await?;
        Ok(())
    }
}
