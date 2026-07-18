use std::sync::LazyLock;

use common::{
    document::{
        ParseDocument,
        ParsedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
};
use database::{
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use keybroker::AdminKeyHash;
use value::{
    ConvexValue,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    admin_keys::types::AdminKeyMetadata,
    SystemIndex,
    SystemTable,
};

pub mod types;

pub const ADMIN_KEYS_TABLE: TableName = TableName::const_new("_admin_keys");

static KEY_HASH_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "keyHash".parse().expect("Invalid built-in field"));

pub static ADMIN_KEYS_BY_HASH_INDEX: LazyLock<SystemIndex<AdminKeysTable>> =
    LazyLock::new(|| SystemIndex::new("by_key_hash", [&KEY_HASH_FIELD]).unwrap());

pub struct AdminKeysTable;
impl SystemTable for AdminKeysTable {
    type Metadata = AdminKeyMetadata;

    const TABLE_NAME: TableName = ADMIN_KEYS_TABLE;

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![ADMIN_KEYS_BY_HASH_INDEX.clone()]
    }
}

pub struct AdminKeysModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> AdminKeysModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn list(&mut self) -> anyhow::Result<Vec<ParsedDocument<AdminKeyMetadata>>> {
        let query = Query::full_table_scan(ADMIN_KEYS_TABLE.clone(), Order::Asc);
        let mut stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let mut out = Vec::new();
        while let Some(doc) = stream.next(self.tx, None).await? {
            out.push(ParseDocument::<AdminKeyMetadata>::parse(doc)?);
        }
        Ok(out)
    }

    pub async fn get_by_hash(
        &mut self,
        hash: &AdminKeyHash,
    ) -> anyhow::Result<Option<ParsedDocument<AdminKeyMetadata>>> {
        let hex_value = hex::encode(hash.as_bytes());
        let range = IndexRange {
            index_name: ADMIN_KEYS_BY_HASH_INDEX.name(),
            range: vec![IndexRangeExpression::Eq(
                KEY_HASH_FIELD.clone(),
                ConvexValue::try_from(hex_value)?.into(),
            )],
            order: Order::Asc,
        };
        let mut stream =
            ResolvedQuery::new(self.tx, TableNamespace::Global, Query::index_range(range))?;
        let Some(doc) = stream.next(self.tx, None).await? else {
            return Ok(None);
        };
        anyhow::ensure!(
            stream.next(self.tx, Some(1)).await?.is_none(),
            "Expected at most one admin key with a given hash"
        );
        Ok(Some(ParseDocument::<AdminKeyMetadata>::parse(doc)?))
    }

    pub async fn insert(
        &mut self,
        hash: AdminKeyHash,
        name: String,
        key_suffix: Option<String>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let doc = AdminKeyMetadata {
            key_hash: hash,
            name,
            revoked_time: None,
            key_suffix,
        };
        SystemMetadataModel::new_global(self.tx)
            .insert(&ADMIN_KEYS_TABLE, doc.try_into()?)
            .await
    }

    /// Idempotent: if a row for `hash` already exists, return it; else insert
    /// a new row using the `(name, key_suffix)` returned by
    /// `metadata_for_insert`.
    ///
    /// The `bool` is `true` only when this call performed the insert. Callers
    /// can use it to gate one-time side effects (audit-log entries, cache
    /// adoption notifications) and to inspect the parsed metadata so that
    /// races with concurrent revocation are honored — a key found revoked
    /// here must still be rejected by the caller.
    pub async fn insert_or_get(
        &mut self,
        hash: AdminKeyHash,
        metadata_for_insert: impl FnOnce() -> (String, Option<String>),
    ) -> anyhow::Result<(ParsedDocument<AdminKeyMetadata>, bool)> {
        if let Some(existing) = self.get_by_hash(&hash).await? {
            return Ok((existing, false));
        }
        let (name, key_suffix) = metadata_for_insert();
        self.insert(hash, name, key_suffix).await?;
        let inserted = self
            .get_by_hash(&hash)
            .await?
            .ok_or_else(|| anyhow::anyhow!("admin key row missing after insert"))?;
        Ok((inserted, true))
    }

    /// Returns the current row plus a flag for whether this call actually
    /// changed the revocation state. If the row was already revoked, the
    /// flag is `false` and no write is performed — callers can use that to
    /// avoid duplicate audit-log entries when the endpoint is hit twice.
    pub async fn revoke(
        &mut self,
        id: ResolvedDocumentId,
    ) -> anyhow::Result<(ParsedDocument<AdminKeyMetadata>, bool)> {
        let existing_doc = SystemMetadataModel::new_global(self.tx)
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Admin key not found"))?;
        let existing: ParsedDocument<AdminKeyMetadata> =
            ParseDocument::<AdminKeyMetadata>::parse(existing_doc)?;
        if existing.revoked_time.is_some() {
            return Ok((existing, false));
        }
        let mut updated = existing.into_value();
        updated.revoked_time = Some(self.tx.runtime().generate_timestamp()?);
        let replaced = SystemMetadataModel::new_global(self.tx)
            .replace(id, updated.try_into()?)
            .await?;
        Ok((ParseDocument::<AdminKeyMetadata>::parse(replaced)?, true))
    }

    pub async fn rename(
        &mut self,
        id: ResolvedDocumentId,
        name: String,
    ) -> anyhow::Result<ParsedDocument<AdminKeyMetadata>> {
        let existing_doc = SystemMetadataModel::new_global(self.tx)
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Admin key not found"))?;
        let existing: ParsedDocument<AdminKeyMetadata> =
            ParseDocument::<AdminKeyMetadata>::parse(existing_doc)?;
        let mut updated = existing.into_value();
        updated.name = name;
        let replaced = SystemMetadataModel::new_global(self.tx)
            .replace(id, updated.try_into()?)
            .await?;
        ParseDocument::<AdminKeyMetadata>::parse(replaced)
    }
}

#[cfg(test)]
mod tests {
    use keybroker::AdminKeyHash;
    use sync_types::Timestamp;
    use value::ConvexObject;

    use super::types::AdminKeyMetadata;

    #[test]
    fn round_trip_without_revoked_time() -> anyhow::Result<()> {
        let original = AdminKeyMetadata {
            key_hash: AdminKeyHash([7u8; 32]),
            name: "laptop".to_string(),
            revoked_time: None,
            key_suffix: Some("Ab12Cd34".to_string()),
        };
        let obj: ConvexObject = original.clone().try_into()?;
        let decoded: AdminKeyMetadata = obj.try_into()?;
        assert_eq!(original, decoded);
        Ok(())
    }

    #[test]
    fn round_trip_with_revoked_time() -> anyhow::Result<()> {
        let original = AdminKeyMetadata {
            key_hash: AdminKeyHash([0xABu8; 32]),
            name: "ci".to_string(),
            revoked_time: Some(Timestamp::try_from(1_700_000_000_000_000_000u64)?),
            key_suffix: None,
        };
        let obj: ConvexObject = original.clone().try_into()?;
        let decoded: AdminKeyMetadata = obj.try_into()?;
        assert_eq!(original, decoded);
        Ok(())
    }

    #[test]
    fn hash_bytes_are_preserved_via_hex_encoding() -> anyhow::Result<()> {
        // Ensure every byte is exercised (not just repeated values).
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        let original = AdminKeyMetadata {
            key_hash: AdminKeyHash(bytes),
            name: "".to_string(),
            revoked_time: None,
            key_suffix: None,
        };
        let obj: ConvexObject = original.clone().try_into()?;
        let decoded: AdminKeyMetadata = obj.try_into()?;
        assert_eq!(original, decoded);
        assert_eq!(decoded.key_hash.as_bytes(), &bytes);
        Ok(())
    }
}
