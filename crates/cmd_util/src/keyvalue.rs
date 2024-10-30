use std::str::FromStr;

use anyhow::Context;

pub fn parse_key_value<K, V, Output>(s: &str) -> anyhow::Result<Output>
where
    K: FromStr<Err: Send + Sync + 'static>,
    V: FromStr<Err: Send + Sync + 'static>,
    Output: TryFrom<(K, V)>,
    anyhow::Error: From<<Output as TryFrom<(K, V)>>::Error>
        + From<<K as FromStr>::Err>
        + From<<V as FromStr>::Err>,
{
    let (key, value) = s
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("invalid key=value: no `=` found in `{s}`"))?;
    Ok((
        key.parse()
            .map_err(anyhow::Error::from)
            .with_context(|| format!("Failed to parse key {key}"))?,
        value
            .parse()
            .map_err(anyhow::Error::from)
            .with_context(|| format!("Failed to parse value {value}"))?,
    )
        .try_into()?)
}
