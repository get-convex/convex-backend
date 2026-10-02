use std::collections::BTreeMap;

use bytes::Bytes;
use imbl::OrdMap;
use value::ResolvedDocumentId;

use crate::{
    writes::PendingWrites,
    FileStorageWriteSize,
};

/// A file stored in a transaction whose `_file_storage` entry is
/// written but whose contents haven't been uploaded yet.
#[derive(Clone, Debug)]
pub struct PendingFileUpload {
    pub bytes: Bytes,
}

/// A file stored in a transaction that has been uploaded.
#[derive(Clone, Debug, PartialEq)]
pub struct UploadedFileUpload {
    /// Size of the file in bytes.
    pub size: usize,
}

/// The files a transaction stored. All files are pending until we upload them
/// after function execution finishes.
#[derive(Clone, Debug)]
pub enum FileUploads {
    /// Files stored while the transaction is running.
    Pending(OrdMap<ResolvedDocumentId, PendingFileUpload>),
    /// Uploaded files after the transaction execution has finished.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "nothing uploads the pending files yet")
    )]
    Uploaded(BTreeMap<ResolvedDocumentId, UploadedFileUpload>),
}

impl Default for FileUploads {
    fn default() -> Self {
        Self::Pending(OrdMap::new())
    }
}

impl PendingWrites for FileUploads {}

impl FileUploads {
    pub(crate) fn add_pending(
        &mut self,
        id: ResolvedDocumentId,
        upload: PendingFileUpload,
    ) -> anyhow::Result<()> {
        match self {
            Self::Pending(uploads) => {
                uploads.insert(id, upload);
                Ok(())
            },
            Self::Uploaded(_) => anyhow::bail!(
                "Can't store a file in a transaction whose files were already uploaded"
            ),
        }
    }

    /// Remove the pending file with the given `_file_storage` id, if it exists.
    pub(crate) fn remove_pending(&mut self, id: &ResolvedDocumentId) -> anyhow::Result<()> {
        match self {
            Self::Pending(uploads) => {
                uploads.remove(id);
                Ok(())
            },
            Self::Uploaded(_) => anyhow::bail!(
                "Can't remove a file in a transaction whose files were already uploaded"
            ),
        }
    }

    pub(crate) fn num_files(&self) -> usize {
        match self {
            Self::Pending(uploads) => uploads.len(),
            Self::Uploaded(uploads) => uploads.len(),
        }
    }

    pub(crate) fn total_bytes(&self) -> usize {
        match self {
            Self::Pending(uploads) => uploads.values().map(|upload| upload.bytes.len()).sum(),
            Self::Uploaded(uploads) => uploads.values().map(|upload| upload.size).sum(),
        }
    }

    pub(crate) fn size(&self) -> FileStorageWriteSize {
        FileStorageWriteSize {
            num_writes: self.num_files(),
            size: self.total_bytes(),
        }
    }
}
