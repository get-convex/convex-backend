use anyhow::Context;
use common::version::Version;
use errors::ErrorMetadata;

pub fn parse_version(version: Option<String>) -> anyhow::Result<Option<Version>> {
    match version {
        Some(version) => {
            let version = Version::parse(&version).context(ErrorMetadata::bad_request(
                "InvalidClientVersion",
                format!("Invalid client version {version}"),
            ))?;
            Ok(Some(version))
        },
        None => Ok(None),
    }
}
