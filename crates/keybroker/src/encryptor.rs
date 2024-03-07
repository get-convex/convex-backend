use std::io::Read;

use byteorder::ReadBytesExt;
use prost::Message;
use sodiumoxide::crypto::secretbox;

use crate::secret::Secret;

#[derive(Clone)]
pub struct Encryptor {
    secret: secretbox::Key,
}
impl Encryptor {
    pub fn new(secret: Secret) -> anyhow::Result<Self> {
        Ok(Self {
            secret: secretbox::Key::from_slice(secret.as_bytes())
                .ok_or_else(|| anyhow::anyhow!("Secret not a valid secretbox key"))?,
        })
    }

    pub fn encode_proto(&self, version: u8, message: impl Message) -> String {
        let nonce = secretbox::gen_nonce();
        let plaintext = message.encode_to_vec();
        let ciphertext = secretbox::seal(&plaintext, &nonce, &self.secret);

        let mut buffer = Vec::with_capacity(1 + nonce.0.len() + ciphertext.len());
        buffer.push(version);
        buffer.extend_from_slice(&nonce.0);
        buffer.extend_from_slice(&ciphertext);
        hex::encode(buffer)
    }

    pub fn decode_proto<M: Default + Message>(
        &self,
        version: u8,
        encoded: &str,
    ) -> anyhow::Result<M> {
        let bytes = hex::decode(encoded)?;
        let mut reader = &bytes[..];

        let message_version = reader.read_u8()?;
        if message_version != version {
            anyhow::bail!("Invalid message version {}", message_version);
        }

        let mut nonce_bytes = [0u8; secretbox::NONCEBYTES];
        reader.read_exact(&mut nonce_bytes)?;
        let nonce = secretbox::Nonce(nonce_bytes);

        let mut ciphertext = Vec::with_capacity(bytes.len() - 1 - secretbox::NONCEBYTES);
        reader.read_to_end(&mut ciphertext)?;

        let plaintext = secretbox::open(&ciphertext, &nonce, &self.secret)
            .map_err(|_| anyhow::anyhow!("Failed to decrypt ciphertext"))?;
        Ok(M::decode(&*plaintext)?)
    }
}
