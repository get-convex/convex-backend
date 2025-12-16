use std::collections::HashSet;

use indexmap::IndexSet;
use itertools::Itertools as _;
use serde::{
    Deserialize,
    Serialize,
};

use super::KeyUsage;
use crate::convert_v8::{
    DOMException,
    DOMExceptionName,
};

#[derive(Deserialize, Serialize, Default)]
pub(super) struct JsonWebKey {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#use: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_ops: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oth: Option<Vec<RsaOtherPrimesInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct RsaOtherPrimesInfo {
    // The following fields are defined in Section 6.3.2.7 of JSON Web Algorithms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

impl JsonWebKey {
    pub(super) fn check_kty(&self, kty: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.kty.as_deref() == Some(kty),
            DOMException::new(
                format!("JWK \"kty\" must be {kty:?}"),
                DOMExceptionName::DataError
            ),
        );
        Ok(())
    }

    pub(super) fn check_crv(&self, crv: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.crv.as_deref() == Some(crv),
            DOMException::new(
                format!("JWK \"crv\" must be {crv:?}"),
                DOMExceptionName::DataError
            ),
        );
        Ok(())
    }

    pub(super) fn check_ext(&self, extractable: bool) -> anyhow::Result<()> {
        if let Some(false) = self.ext
            && extractable
        {
            anyhow::bail!(DOMException::new(
                "JWK \"ext\" must be true",
                DOMExceptionName::DataError
            ));
        }
        Ok(())
    }

    pub(super) fn check_key_ops_and_use(
        &self,
        key_usages: &IndexSet<KeyUsage>,
        expected_use: &str,
    ) -> anyhow::Result<()> {
        if !key_usages.is_empty()
            && let Some(r#use) = &self.r#use
        {
            anyhow::ensure!(
                r#use == expected_use,
                DOMException::new(
                    format!("JWK \"use\" must be {expected_use:?}"),
                    DOMExceptionName::DataError
                ),
            );
        }
        if let Some(key_ops) = &self.key_ops {
            let allowed_usages: HashSet<_> = key_ops
                .iter()
                .filter_map(|s| s.parse::<KeyUsage>().ok())
                .collect();
            for usage in key_usages {
                anyhow::ensure!(
                    allowed_usages.contains(usage),
                    DOMException::new(
                        format!("JWK \"key_ops\" does not contain {usage:?}"),
                        DOMExceptionName::DataError
                    ),
                );
            }
        }
        Ok(())
    }

    pub(super) fn check_alg_oneof(&self, expected: &[&str]) -> anyhow::Result<()> {
        if let Some(alg) = &self.alg {
            anyhow::ensure!(
                expected.contains(&alg.as_str()),
                DOMException::new(
                    format!(
                        "JWK \"alg\" must be {}",
                        expected.iter().map(|e| format!("{e:?}")).join(" or ")
                    ),
                    DOMExceptionName::DataError
                ),
            );
        }
        Ok(())
    }

    pub(super) fn check_alg(&self, expected: &str) -> anyhow::Result<()> {
        self.check_alg_oneof(&[expected])
    }
}
