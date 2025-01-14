use std::str::FromStr;

use common::types::PersistenceVersion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbDriverTag {
    Sqlite,
    Postgres(PersistenceVersion),
    PostgresAwsIam(PersistenceVersion),
    MySql(PersistenceVersion),
    MySqlAwsIam(PersistenceVersion),
}

impl clap::ValueEnum for DbDriverTag {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            DbDriverTag::Sqlite,
            DbDriverTag::MySql(PersistenceVersion::V5),
            DbDriverTag::MySqlAwsIam(PersistenceVersion::V5),
            DbDriverTag::Postgres(PersistenceVersion::V5),
            DbDriverTag::PostgresAwsIam(PersistenceVersion::V5),
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

impl DbDriverTag {
    pub fn persistence_version(&self) -> anyhow::Result<PersistenceVersion> {
        match self {
            Self::Postgres(version)
            | Self::PostgresAwsIam(version)
            | Self::MySql(version)
            | Self::MySqlAwsIam(version) => Ok(*version),
            Self::Sqlite => anyhow::bail!("sqlite has no persistence version"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DbDriverTag::Sqlite => "sqlite",
            DbDriverTag::Postgres(PersistenceVersion::V5) => "postgres-v5",
            DbDriverTag::PostgresAwsIam(PersistenceVersion::V5) => "postgres-v5-aws-iam",
            DbDriverTag::MySql(PersistenceVersion::V5) => "mysql-v5",
            DbDriverTag::MySqlAwsIam(PersistenceVersion::V5) => "mysql-v5-aws-iam",
        }
    }
}

impl FromStr for DbDriverTag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sqlite" => Ok(Self::Sqlite),
            "postgres-v5" => Ok(DbDriverTag::Postgres(PersistenceVersion::V5)),
            "postgres-v5-aws-iam" => Ok(DbDriverTag::PostgresAwsIam(PersistenceVersion::V5)),
            "mysql-v5" => Ok(DbDriverTag::MySql(PersistenceVersion::V5)),
            "mysql-v5-aws-iam" => Ok(DbDriverTag::MySqlAwsIam(PersistenceVersion::V5)),
            _ => anyhow::bail!("unrecognized db_driver {s}"),
        }
    }
}
