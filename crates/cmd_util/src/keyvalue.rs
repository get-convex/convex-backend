use std::str::FromStr;

use anyhow::Context;

pub fn parse_key_value<K, V, Output>(s: &str) -> anyhow::Result<Output>
where
    K: FromStr<Err: std::error::Error + Send + Sync + 'static>,
    V: FromStr<Err: std::error::Error + Send + Sync + 'static>,
    Output: TryFrom<(K, V)>,
    anyhow::Error: From<<Output as TryFrom<(K, V)>>::Error>,
{
    let pos = s
        .find('=')
        .ok_or_else(|| anyhow::anyhow!("invalid key=value: no `=` found in `{s}`"))?;
    let key = &s[..pos];
    let value = &s[pos + 1..];
    Ok((
        key.parse()
            .with_context(|| format!("Failed to parse key {key}"))?,
        value
            .parse()
            .with_context(|| format!("Failed to parse value {value}"))?,
    )
        .try_into()?)
}
