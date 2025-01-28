use std::{
    collections::HashSet,
    io::SeekFrom,
    path::{
        Path,
        PathBuf,
    },
};

use async_zip_0_0_9::read::stream::ZipFileReader;
use futures::pin_mut;
use tokio::{
    fs,
    io::{
        AsyncRead,
        AsyncSeekExt,
        AsyncWriteExt,
        BufWriter,
    },
};

pub mod cache;
mod metrics;

/// Extract the archive stream to the specified output directory, which will be
/// created if it doesn't already exist. This function should only be used
/// for trusted ZIP archives; we don't make any attempt to guard against
/// directory traversal attacks nor do we sanitize paths.
pub(crate) async fn extract_zip<P: AsRef<Path>>(
    output_directory: P,
    archive: impl AsyncRead,
) -> anyhow::Result<u64> {
    fs::create_dir_all(&output_directory).await?;
    pin_mut!(archive);
    let mut reader = ZipFileReader::new(archive);
    let mut created_paths: HashSet<PathBuf> = HashSet::new();
    let mut uncompressed_size = 0u64;

    while !reader.finished() {
        if let Some(entry) = reader.entry_reader().await? {
            let path = Path::new(entry.entry().filename());
            // Some ZIP archives contain entries for directories.
            if entry.entry().filename().ends_with('/') {
                if created_paths.contains(path) {
                    continue;
                }
                fs::create_dir_all(output_directory.as_ref().join(path)).await?;
                created_paths.insert(path.to_owned());
                let mut maybe_parent = path.parent();
                while let Some(parent) = maybe_parent {
                    created_paths.insert(parent.to_owned());
                    maybe_parent = parent.parent();
                }
                continue;
            }
            // Others just imply the existence of directories by containing entries with
            // directories in the name.
            if let Some(parent_path) = path.parent()
                && !created_paths.contains(parent_path)
            {
                fs::create_dir_all(output_directory.as_ref().join(parent_path)).await?;
                let mut maybe_parent = Some(parent_path);
                while let Some(parent) = maybe_parent {
                    created_paths.insert(parent.to_owned());
                    maybe_parent = parent.parent();
                }
            }

            // Finally, extract the file.
            {
                let file = fs::File::create(output_directory.as_ref().join(path)).await?;
                let mut file = BufWriter::new(file);
                entry.copy_to_end_crc(&mut file, 2 << 16).await?;
                // Note that `entry.uncompressed_size()` is always zero as we're processing this
                // ZIP file as a stream, and we don't have the necessary metadata upfront.
                // Instead, just use the size of the file after we've written it out.
                uncompressed_size += file.seek(SeekFrom::End(0)).await?;
                file.flush().await?;
            }
        }
    }
    Ok(uncompressed_size)
}
