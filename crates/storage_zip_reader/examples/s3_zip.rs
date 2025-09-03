use std::sync::Arc;

use anyhow::Context as _;
use aws_s3::storage::S3Storage;
use runtime::prod::ProdRuntime;
use storage_zip_reader::StorageZipArchive;

const USAGE: &str = "usage: s3_zip BUCKET KEY [entry_name]";
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let bucket = args.next().context(USAGE)?;
    let key = args.next().context(USAGE)?.try_into()?;
    let entry_name = args.next();
    let tokio_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let rt = ProdRuntime::new(&tokio_rt);
    let archive = tokio_rt.block_on(async {
        StorageZipArchive::open(
            Arc::new(S3Storage::new_with_prefix(bucket, "".into(), rt.clone()).await?),
            &key,
        )
        .await
    })?;
    eprintln!("entries:");
    for entry in archive.entries() {
        eprintln!(
            "- {}: {} compressed, {} uncompressed",
            entry.name, entry.compressed_size, entry.uncompressed_size
        );
    }
    if let Some(entry_name) = entry_name {
        let entry = archive
            .by_name(&entry_name)
            .context("entry does not exist")?;
        let bytes = tokio_rt.block_on(tokio::io::copy(
            &mut archive.read_entry(entry.clone()),
            &mut tokio::io::stdout(),
        ))?;
        eprintln!("read {bytes} bytes");
    }
    Ok(())
}
