use std::{
    fmt,
    str::FromStr,
    sync::LazyLock,
};

use anyhow::Context;
use sync_types::identifier::{
    check_valid_identifier,
    MIN_IDENTIFIER,
};
use value::{
    heap_size::HeapSize,
    id_v6::VirtualTableNumberMap,
    FieldName,
    InternalId,
    ResolvedDocumentId,
    TableIdentifier,
    TableMapping,
    TableName,
    TabletId,
    VirtualTableMapping,
};

use crate::{
    bootstrap_model::index::{
        index_validation_error,
        IndexMetadata,
        TabletIndexMetadata,
    },
    document::ParsedDocument,
    index::IndexKey,
};

/// Descriptor for an index, e.g., "by_email".
#[derive(Clone, derive_more::Deref, derive_more::Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{_0}")]
pub struct IndexDescriptor(String);

impl IndexDescriptor {
    pub fn min() -> Self {
        MIN_IDENTIFIER
            .parse()
            .expect("Invalid min IndexDescriptor?")
    }

    pub fn is_reserved(&self) -> bool {
        self == &*INDEX_BY_ID_DESCRIPTOR
            || self == &*INDEX_BY_CREATION_TIME_DESCRIPTOR
            || self.0.starts_with('_')
    }
}

impl FromStr for IndexDescriptor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        check_valid_identifier(s).with_context(|| index_validation_error::invalid_index_name(s))?;
        Ok(Self(s.to_owned()))
    }
}

impl fmt::Debug for IndexDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl From<IndexDescriptor> for String {
    fn from(t: IndexDescriptor) -> Self {
        t.0
    }
}

impl HeapSize for IndexDescriptor {
    fn heap_size(&self) -> usize {
        self.0.heap_size()
    }
}

impl From<IndexDescriptor> for FieldName {
    fn from(desc: IndexDescriptor) -> Self {
        desc.0.parse().expect("IndexDescriptor not valid FieldName")
    }
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for IndexDescriptor {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = IndexDescriptor>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        use crate::identifier::arbitrary_regexes::USER_IDENTIFIER_REGEX;
        USER_IDENTIFIER_REGEX.prop_filter_map("Invalid IndexDescriptor", |s| {
            IndexDescriptor::from_str(&s).ok()
        })
    }
}

/// Unique name for an index.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericIndexName<T: TableIdentifier> {
    table: T,
    descriptor: IndexDescriptor,
}

pub type IndexName = GenericIndexName<TableName>;

pub type TabletIndexName = GenericIndexName<TabletId>;

/// Like TabletIndexName in that it refers to a stable underlying index,
/// but it works for virtual tables too.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StableIndexName {
    Physical(TabletIndexName),
    Virtual(IndexName, TabletIndexName),
    Missing,
}

impl StableIndexName {
    pub fn tablet_index_name(&self) -> Option<&TabletIndexName> {
        match self {
            StableIndexName::Physical(tablet_index_name) => Some(tablet_index_name),
            StableIndexName::Virtual(_, tablet_index_name) => Some(tablet_index_name),
            StableIndexName::Missing => None,
        }
    }

    pub fn virtual_table_number_map(
        &self,
        table_mapping: &TableMapping,
        virtual_table_mapping: &VirtualTableMapping,
    ) -> anyhow::Result<Option<VirtualTableNumberMap>> {
        match self {
            StableIndexName::Physical(index_name) => {
                let table_number = table_mapping.tablet_number(*index_name.table())?;
                Ok(Some(VirtualTableNumberMap {
                    virtual_table_number: table_number,
                    physical_table_number: table_number,
                }))
            },
            StableIndexName::Virtual(index_name, tablet_index_name) => {
                Ok(Some(VirtualTableNumberMap {
                    virtual_table_number: virtual_table_mapping.number(index_name.table())?,
                    physical_table_number: table_mapping
                        .tablet_number(*tablet_index_name.table())?,
                }))
            },
            StableIndexName::Missing => Ok(None),
        }
    }
}

impl HeapSize for TabletIndexName {
    fn heap_size(&self) -> usize {
        self.descriptor.heap_size()
    }
}

impl<T: TableIdentifier + FromStr<Err = anyhow::Error>> FromStr for GenericIndexName<T> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let table: T = match parts.next() {
            Some(s) => s
                .parse()
                .with_context(|| index_validation_error::invalid_table_name(s))?,
            None => anyhow::bail!(index_validation_error::not_enough_name_components(s)),
        };
        let descriptor = match parts.next() {
            Some(s) => s
                .parse()
                .with_context(|| index_validation_error::invalid_table_name(s))?,
            None => anyhow::bail!(index_validation_error::not_enough_name_components(s)),
        };
        if parts.next().is_some() {
            anyhow::bail!(index_validation_error::too_many_name_components(s));
        }
        Ok(Self { table, descriptor })
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct IndexDiff {
    pub added: Vec<IndexMetadata<TableName>>,
    /// The set of indexes whose developer configurations (but maybe not
    /// states!) match those in storage
    pub identical: Vec<ParsedDocument<TabletIndexMetadata>>,
    pub dropped: Vec<ParsedDocument<IndexMetadata<TableName>>>,
}

impl<T: TableIdentifier> fmt::Display for GenericIndexName<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.table, self.descriptor)
    }
}

impl<T: TableIdentifier> fmt::Debug for GenericIndexName<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.table, self.descriptor)
    }
}

pub static INDEX_BY_ID_DESCRIPTOR: LazyLock<IndexDescriptor> =
    LazyLock::new(|| "by_id".parse().unwrap());

pub static INDEX_BY_CREATION_TIME_DESCRIPTOR: LazyLock<IndexDescriptor> =
    LazyLock::new(|| "by_creation_time".parse().unwrap());

impl<T: TableIdentifier> GenericIndexName<T> {
    /// Create a new index name for the table and given descriptor,
    /// e.g., "users.by_email".
    pub fn new(table: T, descriptor: IndexDescriptor) -> anyhow::Result<Self> {
        check_valid_identifier(&descriptor)?;
        anyhow::ensure!(
            !descriptor.is_reserved(),
            index_validation_error::name_reserved(&table, &descriptor)
        );
        Ok(Self { table, descriptor })
    }

    pub fn new_reserved(table: T, descriptor: IndexDescriptor) -> anyhow::Result<Self> {
        check_valid_identifier(&descriptor)?;
        anyhow::ensure!(
            descriptor.is_reserved(),
            "Index descriptor is not reserved: {descriptor}"
        );
        Ok(Self { table, descriptor })
    }

    /// The index that exists for all tables which indexes no fields except the
    /// implicitly-included `_id`.
    pub fn by_id(table: T) -> Self {
        Self {
            table,
            descriptor: INDEX_BY_ID_DESCRIPTOR.clone(),
        }
    }

    /// The index that exists for all tables which indexes `_creationTime`.
    pub fn by_creation_time(table: T) -> Self {
        Self {
            table,
            descriptor: INDEX_BY_CREATION_TIME_DESCRIPTOR.clone(),
        }
    }

    /// The table this index is over.
    pub fn table(&self) -> &T {
        &self.table
    }

    /// The descriptor for the index, e.g., "by_email".
    pub fn descriptor(&self) -> &IndexDescriptor {
        &self.descriptor
    }

    /// Is the index name for the by_id index?
    pub fn is_by_id(&self) -> bool {
        self.descriptor == *INDEX_BY_ID_DESCRIPTOR
    }

    /// Is the index name for the creation time index?
    pub fn is_creation_time(&self) -> bool {
        self.descriptor == *INDEX_BY_CREATION_TIME_DESCRIPTOR
    }

    /// Is this index reserved? The system automatically defines these indexes
    /// on all tables and therefore allows querying them if the underlying table
    /// doesn't exist.
    ///
    /// Note - this is subtly different than IndexName.is_system_owned because
    /// this method only checks the index name, but IndexName.is_system_owned
    /// checks both the table name and the index name. This method is not
    /// sufficient to determine if an index can be mutated safely, you must
    /// resolve the table id and check the table id.
    pub fn is_by_id_or_creation_time(&self) -> bool {
        self.is_by_id() || self.is_creation_time()
    }

    pub fn map_table<U: TableIdentifier>(
        self,
        f: &impl Fn(T) -> anyhow::Result<U>,
    ) -> anyhow::Result<GenericIndexName<U>> {
        Ok(GenericIndexName {
            table: f(self.table)?,
            descriptor: self.descriptor,
        })
    }
}

impl IndexName {
    /// Is this index either an index on a system table or a system-defined
    /// index? These indexes do not count towards user quota.
    pub fn is_system_owned(&self) -> bool {
        // Table scan and all indexes on system tables do not count towards
        // user defined indexes.
        self.table.is_system() || self.descriptor.is_reserved()
    }

    pub fn to_resolved(
        self,
        f: impl Fn(TableName) -> anyhow::Result<TabletId>,
    ) -> anyhow::Result<TabletIndexName> {
        Ok(GenericIndexName {
            table: f(self.table)?,
            descriptor: self.descriptor,
        })
    }
}

#[cfg(any(test, feature = "testing"))]
impl<T: TableIdentifier + proptest::arbitrary::Arbitrary> proptest::arbitrary::Arbitrary
    for GenericIndexName<T>
{
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = GenericIndexName<T>>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        any::<(T, IndexDescriptor)>().prop_filter_map("Invalid IndexName", |(t, d)| {
            GenericIndexName::new(t, d).ok()
        })
    }
}

pub type IndexId = InternalId;

#[derive(Eq, PartialEq, Clone, Debug, Ord, PartialOrd)]
pub struct DatabaseIndexUpdate {
    // id of the index document where the index is defined.
    pub index_id: IndexId,

    pub key: IndexKey,
    pub value: DatabaseIndexValue,

    pub is_system_index: bool,
}

#[derive(Eq, PartialEq, Clone, Debug, Ord, PartialOrd)]
pub enum DatabaseIndexValue {
    // We don't need the documentId in case of index entry delete.
    Deleted,
    // Non-clustered index only stores the documentId.
    NonClustered(ResolvedDocumentId),
}

impl DatabaseIndexValue {
    pub fn is_delete(&self) -> bool {
        matches!(self, DatabaseIndexValue::Deleted)
    }
}

#[cfg(test)]
mod tests {
    mod test_min_index_descriptor {
        use std::str::FromStr;

        use proptest::prelude::*;

        use super::super::IndexDescriptor;

        proptest! {
            #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

            #[test]
            fn proptest(index_name in any::<IndexDescriptor>()) {
                assert!(IndexDescriptor::min() <= index_name);
            }
        }

        #[test]
        fn proptest_trophies() {
            // #2716: `IndexDescriptor::min` was "a", where "A" < "a".
            assert!(IndexDescriptor::min() <= IndexDescriptor::from_str("B").unwrap());
        }
    }
}
