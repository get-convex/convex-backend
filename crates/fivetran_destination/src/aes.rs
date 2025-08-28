use std::{
    mem::MaybeUninit,
    pin::Pin,
    task::{
        ready,
        Poll,
    },
};

use aes::cipher::{
    block_padding::Pkcs7,
    BlockDecryptMut,
    KeyIvInit,
};
use anyhow::{
    anyhow,
    Context,
};
use tokio::io::{
    self,
    AsyncRead,
    AsyncReadExt,
    ReadBuf,
};

use crate::error::DestinationError;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256Dec>;

const KEY_SIZE_BYTES: usize = 256 / 8;
const IV_SIZE_BYTES: usize = 16;
const READ_SIZE_BYTES: usize = 4096;

pub struct Aes256Key(pub [u8; KEY_SIZE_BYTES]);

impl Default for Aes256Key {
    fn default() -> Self {
        Self([0; KEY_SIZE_BYTES])
    }
}

impl TryFrom<Vec<u8>> for Aes256Key {
    type Error = DestinationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let key: [u8; KEY_SIZE_BYTES] =
            value.try_into().map_err(|_| DestinationError::InvalidKey)?;
        Ok(Aes256Key(key))
    }
}

/// Decrypts the contents of the inner stream using AES-256 in CBC mode
pub struct AesDecryptor<R: AsyncRead> {
    inner: R,
    key: Aes256Key,
    iv: [u8; IV_SIZE_BYTES],
}

impl<R: AsyncRead + Unpin> AesDecryptor<R> {
    pub async fn new(mut inner: R, key: Aes256Key) -> anyhow::Result<Self> {
        let mut iv = [0; IV_SIZE_BYTES];
        inner
            .read_exact(&mut iv)
            .await
            .context("Can’t extract the IV")?;

        Ok(Self { inner, key, iv })
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for AesDecryptor<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        output_buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = &mut *self;

        // Read the encrypted data
        let mut encrypted_data_buf = [MaybeUninit::uninit(); READ_SIZE_BYTES];
        let mut encrypted_data_read_buf = ReadBuf::uninit(&mut encrypted_data_buf);
        let read_result =
            ready!(Pin::new(&mut me.inner).poll_read(cx, &mut encrypted_data_read_buf));
        if read_result.is_err() {
            return Poll::Ready(read_result);
        }

        // Exit on EOF
        let read = encrypted_data_read_buf.filled_mut();
        if read.is_empty() {
            return Poll::Ready(Ok(()));
        }

        // Decrypt
        let cipher = Aes256CbcDec::new(me.key.0.as_slice().into(), &me.iv.into());

        Poll::Ready(
            cipher
                .decrypt_padded_mut::<Pkcs7>(encrypted_data_read_buf.filled_mut())
                .map_err(|e| io::Error::other(anyhow!(e).context("Unpad error")))
                .and_then(|decrypted_data| {
                    if decrypted_data.len() > output_buf.remaining() {
                        Err(io::Error::other(anyhow!("Output buffer too small")))
                    } else {
                        output_buf.put_slice(decrypted_data);
                        Ok(())
                    }
                }),
        )
    }
}
