use anyhow::Context;
use aws_sdk_s3::{
    operation::upload_part::UploadPartOutput,
    types::ChecksumAlgorithm,
};
use aws_utils::are_checksums_disabled;
use serde_json::{
    json,
    Value as JsonValue,
};
use storage::ClientDrivenUploadPartToken;

#[derive(Clone, Copy, Debug)]
pub struct PartNumber(u16);

impl TryFrom<u16> for PartNumber {
    type Error = anyhow::Error;

    fn try_from(v: u16) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=10000).contains(&v),
            "Object part number cannot exceed 10,000 or be 0."
        );
        Ok(Self(v))
    }
}

impl From<PartNumber> for u16 {
    fn from(p: PartNumber) -> u16 {
        p.0
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PartChecksumType {
    Sha256,
    Crc32,
}

impl From<PartChecksumType> for ChecksumAlgorithm {
    fn from(value: PartChecksumType) -> Self {
        match value {
            PartChecksumType::Sha256 => ChecksumAlgorithm::Sha256,
            PartChecksumType::Crc32 => ChecksumAlgorithm::Crc32,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectPart {
    part_number: PartNumber,
    etag: String,
    checksum: String,
    size: u64,
}

impl ObjectPart {
    pub fn new(
        part_number: PartNumber,
        size: u64,
        upload_part_output: UploadPartOutput,
    ) -> anyhow::Result<Self> {
        let checksum = if are_checksums_disabled() {
            "disabled".to_string()
        } else {
            upload_part_output
                .checksum_crc32()
                .ok_or_else(|| anyhow::anyhow!("Object part missing hash! Expected crc32"))?
                .to_string()
        };
        Ok(Self {
            part_number,
            etag: upload_part_output
                .e_tag()
                .ok_or_else(|| anyhow::anyhow!("Object part missing etag"))?
                .to_string(),
            checksum,
            size,
        })
    }

    pub fn part_number(&self) -> PartNumber {
        self.part_number
    }

    pub fn etag(&self) -> &str {
        &self.etag
    }

    pub fn checksum(&self) -> &str {
        &self.checksum
    }
}

impl TryFrom<ObjectPart> for ClientDrivenUploadPartToken {
    type Error = anyhow::Error;

    fn try_from(value: ObjectPart) -> Result<Self, Self::Error> {
        let v = json!({
            "partNumber": u16::from(value.part_number()),
            "etag": value.etag(),
            "checksum": value.checksum(),
            "size": value.size,
        });
        Ok(ClientDrivenUploadPartToken(serde_json::to_string(&v)?))
    }
}

impl TryFrom<ClientDrivenUploadPartToken> for ObjectPart {
    type Error = anyhow::Error;

    fn try_from(value: ClientDrivenUploadPartToken) -> Result<Self, Self::Error> {
        let v: JsonValue = serde_json::from_str(&value.0)?;
        let part_number = (v
            .get("partNumber")
            .context("missing partNumber")?
            .as_u64()
            .context("partNumber should be u16")? as u16)
            .try_into()?;
        let etag = v
            .get("etag")
            .context("missing etag")?
            .as_str()
            .context("etag should be str")?
            .to_string();
        let checksum = v
            .get("checksum")
            .context("missing checksum")?
            .as_str()
            .context("checksum should be str")?
            .to_string();
        let size = v
            .get("size")
            .context("missing partNumber")?
            .as_u64()
            .context("partNumber should be u64")?;
        Ok(Self {
            part_number,
            etag,
            checksum,
            size,
        })
    }
}
