use std::{
    collections::HashSet,
    io::{
        Seek,
        SeekFrom,
    },
    path::{
        Path,
        PathBuf,
    },
};

use async_zip::base::read::stream::ZipFileReader;
use futures::pin_mut;
use tokio::io::AsyncBufRead;

pub mod cache;
mod metrics;

/// Extract the archive stream to the specified output directory, which will be
/// created if it doesn't already exist. This function should only be used
/// for trusted ZIP archives; we don't make any attempt to guard against
/// directory traversal attacks nor do we sanitize paths.
pub(crate) async fn extract_zip<P: AsRef<Path>>(
    output_directory: P,
    archive: impl AsyncBufRead,
) -> anyhow::Result<u64> {
    std::fs::create_dir_all(&output_directory)?;
    pin_mut!(archive);
    let mut zip = ZipFileReader::with_tokio(archive);
    let mut created_paths: HashSet<PathBuf> = HashSet::new();
    let mut uncompressed_size = 0u64;

    while let Some(mut entry) = zip.next_with_entry().await? {
        let path = Path::new(entry.reader().entry().filename().as_str()?);
        // Some ZIP archives contain entries for directories.
        if entry.reader().entry().filename().as_str()?.ends_with('/') {
            if created_paths.contains(path) {
                zip = entry.skip().await?;
                continue;
            }
            std::fs::create_dir_all(output_directory.as_ref().join(path))?;
            created_paths.insert(path.to_owned());
            let mut maybe_parent = path.parent();
            while let Some(parent) = maybe_parent {
                created_paths.insert(parent.to_owned());
                maybe_parent = parent.parent();
            }
            zip = entry.skip().await?;
            continue;
        }
        // Others just imply the existence of directories by containing entries with
        // directories in the name.
        if let Some(parent_path) = path.parent()
            && !created_paths.contains(parent_path)
        {
            std::fs::create_dir_all(output_directory.as_ref().join(parent_path))?;
            let mut maybe_parent = Some(parent_path);
            while let Some(parent) = maybe_parent {
                created_paths.insert(parent.to_owned());
                maybe_parent = parent.parent();
            }
        }

        // Finally, extract the file.
        let std_file = std::fs::File::create(output_directory.as_ref().join(path))?;
        let mut file = futures::io::AllowStdIo::new(std_file);
        futures::io::copy(entry.reader_mut(), &mut file).await?;
        let mut std_file = file.into_inner();
        // Note that `entry.uncompressed_size()` is always zero as we're processing this
        // ZIP file as a stream, and we don't have the necessary metadata upfront.
        // Instead, just use the size of the file after we've written it out.
        uncompressed_size += std_file.seek(SeekFrom::End(0))?;

        zip = entry.done().await?;
    }
    Ok(uncompressed_size)
}
