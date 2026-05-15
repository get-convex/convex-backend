//! Deploy key encoding and parsing.
//!
//! Format: `env:<deployment_name>|<base64-secret>` — orchestrator-issued
//! keys all carry the `env:` prefix because self-hosted deployments are
//! identified by name (the kind is implied by the deployment record). The
//! parser still accepts the legacy `prod:`/`dev:`/`preview:` prefixes so
//! existing keys minted before this change continue to work.

use anyhow::{
    anyhow,
    bail,
};

pub const DEPLOY_KEY_PREFIX: &str = "env";

#[derive(Debug, Clone)]
pub struct DeployKeyParts<'a> {
    pub kind: &'a str,
    pub deployment_name: &'a str,
    pub secret: &'a str,
}

pub fn parse_deploy_key(input: &str) -> anyhow::Result<DeployKeyParts<'_>> {
    let (kind_and_name, secret) = input
        .split_once('|')
        .ok_or_else(|| anyhow!("deploy key missing pipe separator"))?;
    let (kind, deployment_name) = kind_and_name
        .split_once(':')
        .ok_or_else(|| anyhow!("deploy key missing colon separator"))?;
    if kind.is_empty() || deployment_name.is_empty() || secret.is_empty() {
        bail!("deploy key has empty fields");
    }
    Ok(DeployKeyParts {
        kind,
        deployment_name,
        secret,
    })
}

pub fn encode_deploy_key(_kind: &str, deployment_name: &str, secret: &str) -> String {
    format!("{DEPLOY_KEY_PREFIX}:{deployment_name}|{secret}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_env() {
        let k = parse_deploy_key("env:happy-otter-123|abcdef").unwrap();
        assert_eq!(k.kind, "env");
        assert_eq!(k.deployment_name, "happy-otter-123");
        assert_eq!(k.secret, "abcdef");
    }

    #[test]
    fn parse_legacy_prod() {
        // Pre-`env:` keys must still parse so old tokens keep working.
        let k = parse_deploy_key("prod:happy-otter-123|abcdef").unwrap();
        assert_eq!(k.kind, "prod");
        assert_eq!(k.deployment_name, "happy-otter-123");
    }

    #[test]
    fn encode_uses_env_prefix() {
        // The kind argument is ignored; output always has `env:` so the
        // orchestrator emits a single, uniform format.
        assert_eq!(
            encode_deploy_key("prod", "happy-otter-123", "secret"),
            "env:happy-otter-123|secret",
        );
        assert_eq!(
            encode_deploy_key("preview", "abc-def-1", "s2"),
            "env:abc-def-1|s2",
        );
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_deploy_key("nope").is_err());
        assert!(parse_deploy_key("nope:nopipe").is_err());
        assert!(parse_deploy_key(":blank|secret").is_err());
    }
}
