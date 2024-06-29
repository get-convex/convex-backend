use async_zip::base::read::{
    WithEntry,
    ZipEntryReader,
};
use futures::{
    AsyncBufRead,
    AsyncReadExt,
};

// https://github.com/Majored/rs-async-zip/issues/141
const ASYNC_ZIP_FIXED_THEIR_CRC_BUG: bool = false;

#[async_trait::async_trait]
pub trait ZipEntryReaderExt: Send {
    async fn read_to_end_checked_bypass_async_zip_crc_bug(&mut self) -> anyhow::Result<Vec<u8>>;
    async fn read_to_string_checked_bypass_async_zip_crc_bug(&mut self) -> anyhow::Result<String>;
}

#[async_trait::async_trait]
impl<'a, R: AsyncBufRead + Unpin + Send> ZipEntryReaderExt
    for ZipEntryReader<'a, R, WithEntry<'a>>
{
    async fn read_to_end_checked_bypass_async_zip_crc_bug(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut contents = Vec::new();
        if ASYNC_ZIP_FIXED_THEIR_CRC_BUG {
            self.read_to_end_checked(&mut contents).await?;
        } else {
            self.read_to_end(&mut contents).await?;
            let hash_after = self.compute_hash();
            let expected_crc = self.entry().crc32();
            if expected_crc != 0 {
                // Only do CRC check if the expected crc is nonzero. The zero
                // case means that the data descriptor header holds the crc, triggering
                // the bug in async-zip mentioned above. Could also mean a 1/2^32
                // chance of a normal local header.
                assert_eq!(hash_after, self.entry().crc32());
            }
        }
        Ok(contents)
    }

    async fn read_to_string_checked_bypass_async_zip_crc_bug(&mut self) -> anyhow::Result<String> {
        let mut contents = String::new();
        if ASYNC_ZIP_FIXED_THEIR_CRC_BUG {
            self.read_to_string_checked(&mut contents).await?;
        } else {
            self.read_to_string(&mut contents).await?;
            let hash_after = self.compute_hash();
            let expected_crc = self.entry().crc32();
            if expected_crc != 0 {
                // Only do CRC check if the expected crc is nonzero. The zero
                // case means that the data descriptor header holds the crc, triggering
                // the bug in async-zip mentioned above. Could also mean a 1/2^32
                // chance of a normal local header.
                assert_eq!(hash_after, self.entry().crc32());
            }
        }
        Ok(contents)
    }
}
