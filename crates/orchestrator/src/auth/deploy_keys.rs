//! Deploy key encoding and parsing.
//!
//! Format: `<kind>:<deployment_name>|<secret>` where `<kind>` is the
//! deployment type (`prod` / `dev` / `preview`) or `project` for the
//! project-scoped preview key. The Convex CLI's `isDeploymentKey` check
//! is `/^(dev|prod):.*\|/` — anything else is rejected before a network
//! call ever happens, so the prefix is load-bearing and must match the
//! deployment kind. The parser still accepts the legacy `env:` prefix so
//! tokens minted by older orchestrator builds continue to authenticate
//! against this server (they'll need to be regenerated to actually be
//! usable from the CLI, but the orchestrator won't pretend they're
//! invalid).

use anyhow::{
    anyhow,
    bail,
};

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

pub fn encode_deploy_key(kind: &str, deployment_name: &str, secret: &str) -> String {
    format!("{kind}:{deployment_name}|{secret}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_prod() {
        let k = parse_deploy_key("prod:happy-otter-123|abcdef").unwrap();
        assert_eq!(k.kind, "prod");
        assert_eq!(k.deployment_name, "happy-otter-123");
        assert_eq!(k.secret, "abcdef");
    }

    #[test]
    fn parse_legacy_env() {
        // Tokens minted by older orchestrator builds (which hardcoded the
        // `env:` prefix) must still parse so existing rows authenticate;
        // the CLI won't accept them, but the user can regenerate.
        let k = parse_deploy_key("env:happy-otter-123|abcdef").unwrap();
        assert_eq!(k.kind, "env");
        assert_eq!(k.deployment_name, "happy-otter-123");
    }

    #[test]
    fn encode_carries_kind_prefix() {
        // The CLI keys deploy-key routing off the prefix (`isDeploymentKey`
        // matches `^(dev|prod):`), so we have to emit the real deployment
        // type rather than a placeholder.
        assert_eq!(
            encode_deploy_key("prod", "happy-otter-123", "secret"),
            "prod:happy-otter-123|secret",
        );
        assert_eq!(
            encode_deploy_key("dev", "happy-otter-123", "secret"),
            "dev:happy-otter-123|secret",
        );
        assert_eq!(
            encode_deploy_key("preview", "abc-def-1", "s2"),
            "preview:abc-def-1|s2",
        );
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_deploy_key("nope").is_err());
        assert!(parse_deploy_key("nope:nopipe").is_err());
        assert!(parse_deploy_key(":blank|secret").is_err());
    }
}
