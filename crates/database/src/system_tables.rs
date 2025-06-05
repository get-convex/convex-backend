use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{
        Debug,
        Display,
    },
    marker::PhantomData,
    sync::Arc,
};

use common::{
    bootstrap_model::index::database_index::IndexedFields,
    document::{
        ParseDocument,
        ParsedDocument,
        ResolvedDocument,
    },
    types::{
        GenericIndexName,
        IndexDescriptor,
        IndexName,
        IndexTableIdentifier,
    },
    virtual_system_mapping::VirtualSystemDocMapper,
};
use value::{
    heap_size::HeapSize,
    FieldPath,
    TableName,
};

/// Represents a system table.
///
/// This trait is not dyn-compatible because it has a `Metadata` associated
/// type.
pub trait SystemTable: Send + Sync + Sized + 'static {
    /// Table name for this system table. Must begin with `_`
    fn table_name() -> &'static TableName;
    /// List of indexes for the system table
    fn indexes() -> Vec<SystemIndex<Self>>;
    fn virtual_table() -> Option<(
        &'static TableName,
        BTreeMap<IndexName, IndexName>,
        Arc<dyn VirtualSystemDocMapper>,
    )> {
        None
    }

    type Metadata: SystemTableMetadata;
}

pub trait SystemTableMetadata: Sized {
    fn parse_from_doc(doc: ResolvedDocument) -> anyhow::Result<ParsedDocument<Self>>;
}

impl<T> SystemTableMetadata for T
where
    ResolvedDocument: ParseDocument<T>,
{
    fn parse_from_doc(doc: ResolvedDocument) -> anyhow::Result<ParsedDocument<Self>> {
        ParseDocument::parse(doc)
    }
}

/// dyn-compatible form of [`SystemTable`]. This is automatically implemented
/// for any `SystemTable`.
pub trait ErasedSystemTable: Send + Sync {
    fn table_name(&self) -> &'static TableName;
    fn indexes(&self) -> Vec<ErasedSystemIndex>;
    fn virtual_table(
        &self,
    ) -> Option<(
        &'static TableName,
        BTreeMap<IndexName, IndexName>,
        Arc<dyn VirtualSystemDocMapper>,
    )>;

    /// Check that a document is valid for this system table.
    /// We can't return the parsed document struct because its type might not
    /// be accessible from db-verifier.
    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()>;
}

impl<T: SystemTable> ErasedSystemTable for T {
    fn table_name(&self) -> &'static TableName {
        T::table_name()
    }

    fn indexes(&self) -> Vec<ErasedSystemIndex> {
        T::indexes().into_iter().map(SystemIndex::erase).collect()
    }

    fn virtual_table(
        &self,
    ) -> Option<(
        &'static TableName,
        BTreeMap<IndexName, IndexName>,
        Arc<dyn VirtualSystemDocMapper>,
    )> {
        T::virtual_table()
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        <T::Metadata>::parse_from_doc(document)?;
        Ok(())
    }
}

/// Wraps a `SystemTable` so that it can be used with a `GenericIndexName`
pub struct SystemTableName<T: SystemTable>(PhantomData<T>);
impl<T: SystemTable> SystemTableName<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: SystemTable> Copy for SystemTableName<T> {}
impl<T: SystemTable> Clone for SystemTableName<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: SystemTable> Debug for SystemTableName<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemTableName")
            .field("table_name", T::table_name())
            .finish()
    }
}
impl<T: SystemTable> Display for SystemTableName<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(T::table_name(), f)
    }
}
impl<T: SystemTable> HeapSize for SystemTableName<T> {
    fn heap_size(&self) -> usize {
        0
    }
}
impl<T: SystemTable> PartialEq for SystemTableName<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
impl<T: SystemTable> Eq for SystemTableName<T> {}
impl<T: SystemTable> PartialOrd for SystemTableName<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: SystemTable> Ord for SystemTableName<T> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}
impl<T: SystemTable> IndexTableIdentifier for SystemTableName<T> {}

pub struct SystemIndex<T: SystemTable> {
    pub name: GenericIndexName<SystemTableName<T>>,
    pub fields: IndexedFields,
}

impl<T: SystemTable> Clone for SystemIndex<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            fields: self.fields.clone(),
        }
    }
}

impl<T: SystemTable> SystemIndex<T> {
    pub fn by_id() -> Self {
        SystemIndex {
            name: GenericIndexName::by_id(SystemTableName::new()),
            fields: IndexedFields::by_id(),
        }
    }

    pub fn new<const N: usize>(
        descriptor: &'static str,
        fields: [&FieldPath; N],
    ) -> anyhow::Result<Self> {
        Ok(SystemIndex {
            name: GenericIndexName::new(SystemTableName::new(), IndexDescriptor::new(descriptor)?)?,
            fields: fields
                .into_iter()
                .cloned()
                .collect::<Vec<FieldPath>>()
                .try_into()?,
        })
    }

    pub fn name(&self) -> IndexName {
        let Ok(name) = self
            .name
            .clone()
            .map_table(&|_| Ok::<_, !>(T::table_name().clone()));
        name
    }

    pub fn descriptor(&self) -> &IndexDescriptor {
        self.name.descriptor()
    }

    pub fn erase(self) -> ErasedSystemIndex {
        ErasedSystemIndex {
            name: self.name(),
            fields: self.fields,
        }
    }
}

/// Like `SystemIndex`, but not generic over a concrete `SystemTable` type
pub struct ErasedSystemIndex {
    pub name: IndexName,
    pub fields: IndexedFields,
}
