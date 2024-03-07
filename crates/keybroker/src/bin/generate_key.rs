use std::{
    convert::TryFrom,
    env,
};

use common::types::MemberId;
use keybroker::{
    InstanceSecret,
    KeyBroker,
};

const USAGE: &str = "USAGE: ./generate_key <instance_name> <instance_secret> [member_id]";

fn main() -> anyhow::Result<()> {
    let instance_name = env::args().nth(1).ok_or_else(|| anyhow::anyhow!(USAGE))?;
    let instance_secret_s = env::args().nth(2).ok_or_else(|| anyhow::anyhow!(USAGE))?;
    let member_id = env::args()
        .nth(3)
        .unwrap_or_else(|| "0".to_owned())
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!(USAGE))?;
    let instance_secret = InstanceSecret::try_from(&instance_secret_s[..])?;

    let broker = KeyBroker::new(&instance_name[..], instance_secret)?;
    let admin_key = broker.issue_admin_key(MemberId(member_id));
    println!("{}", admin_key);
    let system_key = broker.issue_system_key();
    println!("{}", system_key);
    Ok(())
}
