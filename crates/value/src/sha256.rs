//! Ergonomic wrappers on top of the `sha2` crate, which is a bit too generic to
//! be nice to use. (For example, they use `GenericArray` with type level
//! integer lengths for their digest type.)
use std::{
    fmt,
    hash::Hasher,
    io::{
        self,
        Write,
    },
    ops::{
        BitXorAssign,
        Deref,
    },
};

use anyhow::Context;
use sha2::Digest;

use crate::ConvexValue;

#[must_use]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub fn as_base64(&self) -> String {
        base64::encode(self)
    }

    pub fn from_base64(v: &str) -> anyhow::Result<Self> {
        let bytes = base64::decode(v)?;
        let arr: [u8; 32] = bytes.try_into().ok().context("sha256 not 32 bytes")?;
        Ok(Sha256Digest::from(arr))
    }

    pub fn as_hex(&self) -> String {
        hex::encode(self)
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Digest({})", hex::encode(self.0))
    }
}

impl Deref for Sha256Digest {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for Sha256Digest {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Sha256Digest {
    fn from(d: [u8; 32]) -> Self {
        Self(d)
    }
}

impl TryFrom<Vec<u8>> for Sha256Digest {
    type Error = anyhow::Error;

    fn try_from(sha256: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Sha256Digest(
            (*sha256).try_into().context("Sha256 wasn't 32 bytes.")?,
        ))
    }
}

impl BitXorAssign for Sha256Digest {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (i, x) in rhs.iter().enumerate() {
            self.0[i] ^= *x;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sha256 {
    inner: sha2::Sha256,
}

impl Write for Sha256 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Self {
            inner: sha2::Sha256::new(),
        }
    }

    pub fn hash(buf: &[u8]) -> Sha256Digest {
        let mut hasher = Self::new();
        hasher.update(buf);
        hasher.finalize()
    }

    pub fn update(&mut self, buf: &[u8]) {
        self.inner.update(buf)
    }

    pub fn finalize(self) -> Sha256Digest {
        Sha256Digest(self.inner.finalize().into())
    }
}

impl Hasher for Sha256 {
    // Prefer using `finalize` which returns the full 256 bits.
    fn finish(&self) -> u64 {
        let digest = self.clone().finalize();
        let mut hash = 0;
        // Compress the 32 byte digest into 8 bytes:
        // Interpret the [u8; 32] as [u64 little endian; 4]
        // and compute hash = XOR(the u64s).
        for (i, x) in digest.iter().enumerate() {
            hash ^= (*x as u64) << ((i % 8) * 8);
        }
        hash
    }

    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
}

impl TryFrom<Sha256Digest> for ConvexValue {
    type Error = anyhow::Error;

    fn try_from(sha256: Sha256Digest) -> Result<Self, Self::Error> {
        ConvexValue::try_from(sha256.to_vec())
    }
}

impl TryFrom<ConvexValue> for Sha256Digest {
    type Error = anyhow::Error;

    fn try_from(value: ConvexValue) -> Result<Self, Self::Error> {
        match value {
            ConvexValue::Bytes(sha256) => Vec::<u8>::from(sha256).try_into(),
            _ => anyhow::bail!("Unexpected non-bytes value in sha256digest"),
        }
    }
}
