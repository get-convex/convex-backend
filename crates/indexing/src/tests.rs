use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::Arc,
};

use common::{
    bootstrap_model::index::{
        database_index::{
            DatabaseIndexState,
            DeveloperDatabaseIndexConfig,
            IndexedFields,
        },
        IndexConfig,
        IndexMetadata,
        TabletIndexMetadata,
        INDEX_TABLE,
    },
    document::{
        CreationTime,
        ResolvedDocument,
    },
    index::IndexKey,
    persistence::{
        ConflictStrategy,
        NoopRetentionValidator,
        Persistence,
        RepeatablePersistence,
    },
    testing::{
        TestIdGenerator,
        TestPersistence,
    },
    types::{
        unchecked_repeatable_ts,
        DatabaseIndexUpdate,
        DatabaseIndexValue,
        GenericIndexName,
        PersistenceVersion,
        TableName,
        Timestamp,
    },
};
use must_let::must_let;
use runtime::testing::TestRuntime;
use value::{
    assert_obj,
    FieldPath,
    ResolvedDocumentId,
    TabletId,
};

trait IdGenerator {
    fn system_generate(&mut self, table_name: &TableName) -> ResolvedDocumentId;
}

impl IdGenerator for TestIdGenerator {
    fn system_generate(&mut self, table_name: &TableName) -> ResolvedDocumentId {
        TestIdGenerator::system_generate(self, table_name)
    }
}

struct ConstantId(ResolvedDocumentId);

impl IdGenerator for ConstantId {
    fn system_generate(&mut self, _table_name: &TableName) -> ResolvedDocumentId {
        self.0
    }
}

use crate::{
    backend_in_memory_indexes::BackendInMemoryIndexes,
    index_registry::IndexRegistry,
};

fn next_document_id(
    id_generator: &mut TestIdGenerator,
    table_name: &str,
) -> anyhow::Result<ResolvedDocumentId> {
    Ok(id_generator.user_generate(&TableName::from_str(table_name)?))
}

fn gen_index_document(
    id_generator: &mut dyn IdGenerator,
    metadata: TabletIndexMetadata,
) -> anyhow::Result<ResolvedDocument> {
    let index_id = id_generator.system_generate(&INDEX_TABLE);
    ResolvedDocument::new(index_id, CreationTime::ONE, metadata.try_into()?)
}

fn index_documents(
    id_generator: &mut TestIdGenerator,
    mut indexes: Vec<TabletIndexMetadata>,
) -> anyhow::Result<BTreeMap<ResolvedDocumentId, (Timestamp, ResolvedDocument)>> {
    let mut index_documents = BTreeMap::new();

    let index_table = id_generator.system_table_id(&INDEX_TABLE);
    // Add the _index.by_id index.
    indexes.push(IndexMetadata::new_enabled(
        GenericIndexName::by_id(index_table.tablet_id),
        IndexedFields::by_id(),
    ));
    let ts = Timestamp::must(0);
    for metadata in indexes {
        let doc = gen_index_document(id_generator, metadata.clone())?;
        index_documents.insert(doc.id(), (ts, doc));
    }
    Ok(index_documents)
}

#[test]
fn test_metadata_add_and_drop_index() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_documents = index_documents(&mut id_generator, vec![])?;
    let mut index_registry = IndexRegistry::bootstrap(
        &id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )?;

    assert_eq!(index_registry.all_enabled_indexes().len(), 1);

    let table_id = id_generator.user_table_id(&"messages".parse()?);
    let by_id = GenericIndexName::by_id(table_id.tablet_id);
    let by_name = GenericIndexName::new(table_id.tablet_id, "by_name".parse()?)?;
    // Add `messages.by_id`.
    let by_id = gen_index_document(
        &mut id_generator,
        IndexMetadata::new_enabled(by_id, IndexedFields::by_id()),
    )?;
    let result = index_registry.update(None, Some(&by_id));
    assert!(result.is_ok());
    assert_eq!(index_registry.all_enabled_indexes().len(), 2);

    // Add messages by name.
    let by_name = gen_index_document(
        &mut id_generator,
        IndexMetadata::new_enabled(by_name, vec!["name".parse()?].try_into()?),
    )?;

    let result = index_registry.update(None, Some(&by_name));
    assert!(result.is_ok());
    assert_eq!(index_registry.all_enabled_indexes().len(), 3);

    // Try to drop it. Should succeed.
    let result = index_registry.update(Some(&by_name), None);
    assert!(result.is_ok());
    assert_eq!(index_registry.all_enabled_indexes().len(), 2);

    Ok(())
}

#[test]
fn test_metadata_rename_index() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let index_documents = index_documents(&mut id_generator, vec![])?;
    let mut index_registry = IndexRegistry::bootstrap(
        &id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )?;
    let table = id_generator.user_table_id(&"messages".parse()?);
    let by_id = GenericIndexName::by_id(table.tablet_id);
    let by_first_id = GenericIndexName::new(table.tablet_id, "by_first_id".parse()?)?;
    let by_name = GenericIndexName::new(table.tablet_id, "by_name".parse()?)?;
    let by_first_name = GenericIndexName::new(table.tablet_id, "by_first_name".parse()?)?;

    // Add messages table_scan index.
    let original = gen_index_document(
        &mut id_generator,
        IndexMetadata::new_enabled(by_id.clone(), vec!["name".parse()?].try_into()?),
    )?;
    let result = index_registry.update(None, Some(&original));
    assert!(result.is_ok());
    assert_eq!(index_registry.all_enabled_indexes().len(), 2);
    assert!(index_registry.enabled_index_metadata(&by_id).is_some());

    // Renaming of table scan index is not allowed.
    let rename = ResolvedDocument::new(
        original.id(),
        CreationTime::ONE,
        IndexMetadata::new_enabled(by_first_id.clone(), vec!["name".parse()?].try_into()?)
            .try_into()?,
    )?;

    let result = index_registry.update(Some(&original), Some(&rename));
    let err = result.unwrap_err();
    assert!(
        format!("{:?}", err).contains(&format!(
            "Can't rename system defined index {}.by_id",
            table.tablet_id
        )),
        "{err}"
    );
    assert_eq!(index_registry.all_enabled_indexes().len(), 2);
    assert!(index_registry.enabled_index_metadata(&by_id).is_some());
    assert!(index_registry
        .enabled_index_metadata(&by_first_id)
        .is_none());

    // Add `by_name`
    let original = gen_index_document(
        &mut id_generator,
        IndexMetadata::new_enabled(by_name.clone(), vec!["name".parse()?].try_into()?),
    )?;
    let result = index_registry.update(None, Some(&original));
    assert!(result.is_ok());
    assert_eq!(index_registry.all_enabled_indexes().len(), 3);
    assert!(index_registry.enabled_index_metadata(&by_name).is_some());

    // Rename `by_name` to `by_first_name`.
    let rename = ResolvedDocument::new(
        original.id(),
        CreationTime::ONE,
        IndexMetadata::new_enabled(by_first_name.clone(), vec!["name".parse()?].try_into()?)
            .try_into()?,
    )?;
    let result = index_registry.update(Some(&original), Some(&rename));
    assert!(result.is_ok());
    assert_eq!(index_registry.all_enabled_indexes().len(), 3);
    assert!(index_registry.enabled_index_metadata(&by_name).is_none());
    assert!(index_registry
        .enabled_index_metadata(&by_first_name)
        .is_some());

    Ok(())
}

#[test]
fn test_metadata_change_index() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table = id_generator.user_table_id(&"messages".parse()?);
    let by_id = GenericIndexName::by_id(table.tablet_id);
    let by_name = GenericIndexName::new(table.tablet_id, "by_name".parse()?)?;
    let authors_table = id_generator.user_table_id(&"authors".parse()?);
    let authors_by_name = GenericIndexName::new(authors_table.tablet_id, "by_name".parse()?)?;

    let indexes = vec![IndexMetadata::new_enabled(by_id, IndexedFields::by_id())];
    let index_documents = index_documents(&mut id_generator, indexes)?;
    let mut index_registry = IndexRegistry::bootstrap(
        &id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )?;

    let original = gen_index_document(
        &mut id_generator,
        IndexMetadata::new_enabled(by_name.clone(), vec!["name".parse()?].try_into()?),
    )?;
    let result = index_registry.update(None, Some(&original));
    assert!(result.is_ok());

    // Changing fields is not allowed.
    let changed_fields = ResolvedDocument::new(
        original.id(),
        CreationTime::ONE,
        IndexMetadata::new_enabled(by_name.clone(), vec!["first_name".parse()?].try_into()?)
            .try_into()?,
    )?;

    let result = index_registry.update(Some(&original), Some(&changed_fields));
    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err())
        .contains("Can't modify developer index config for existing indexes"));
    let current_metadata = index_registry.enabled_index_metadata(&by_name).unwrap();
    must_let!(let IndexConfig::Database { developer_config, .. } = &current_metadata.config);
    must_let!(let DeveloperDatabaseIndexConfig { fields } = developer_config);
    assert_eq!(*fields, vec!["name".parse()?].try_into()?,);

    // Changing which table the index is indexing is not allowed.
    let changed_table = ResolvedDocument::new(
        original.id(),
        CreationTime::ONE,
        IndexMetadata::new_enabled(authors_by_name.clone(), vec!["name".parse()?].try_into()?)
            .try_into()?,
    )?;

    let result = index_registry.update(Some(&original), Some(&changed_table));
    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err()).contains("Can't change indexed table"));
    assert!(index_registry.enabled_index_metadata(&by_name).is_some());
    assert!(index_registry
        .enabled_index_metadata(&authors_by_name)
        .is_none());

    // Creating a new index with the same name and state is not allowed.
    let name_collision = ResolvedDocument::new(
        id_generator.system_generate(&INDEX_TABLE),
        CreationTime::ONE,
        IndexMetadata::new_enabled(by_name.clone(), vec!["other_field".parse()?].try_into()?)
            .try_into()?,
    )?;

    let result = index_registry.update(None, Some(&name_collision));
    assert!(result.is_err());
    assert!(
        format!("{}", result.unwrap_err().root_cause()).contains(&format!(
            "Cannot create a second enabled index with name {}.by_name",
            table.tablet_id
        ))
    );
    let current_metadata = index_registry.enabled_index_metadata(&by_name).unwrap();
    must_let!(
        let IndexConfig::Database {
            developer_config: DeveloperDatabaseIndexConfig { fields },
            ..
        } = &current_metadata.config
    );
    assert_eq!(*fields, vec!["name".parse()?].try_into()?,);

    Ok(())
}

#[test]
fn test_second_pending_index_for_name_fails() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let indexes = vec![];
    let index_documents = index_documents(&mut id_generator, indexes)?;
    let mut index_registry = IndexRegistry::bootstrap(
        &id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )?;
    let table = id_generator.user_table_id(&"messages".parse()?);

    // Creating a new index with the same name and state is not allowed.
    let by_name = GenericIndexName::new(table.tablet_id, "by_name".parse()?)?;
    let pending = gen_index_document(
        &mut id_generator,
        IndexMetadata::new_backfilling(
            Timestamp::MIN,
            by_name.clone(),
            vec!["name".parse()?].try_into()?,
        ),
    )?;
    let result = index_registry.update(None, Some(&pending));
    assert!(result.is_ok());
    let name_collision = ResolvedDocument::new(
        id_generator.system_generate(&INDEX_TABLE),
        CreationTime::ONE,
        IndexMetadata::new_backfilling(
            Timestamp::MIN,
            by_name.clone(),
            vec!["other_field".parse()?].try_into()?,
        )
        .try_into()?,
    )?;
    let result = index_registry.update(None, Some(&name_collision));
    assert!(result.is_err());
    assert!(
        format!("{}", result.unwrap_err().root_cause()).contains(&format!(
            "Cannot create a second pending index with name {}.by_name",
            table.tablet_id
        ))
    );
    let current_index = index_registry.get_pending(&by_name).unwrap();
    must_let!(let IndexConfig::Database { developer_config, .. } = &current_index.metadata.config);
    must_let!(let DeveloperDatabaseIndexConfig { fields } = developer_config);
    assert_eq!(*fields, vec!["name".parse()?].try_into()?,);

    Ok(())
}

#[test]
fn test_metadata_index_updates() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table = id_generator.user_table_id(&"messages".parse()?);
    let by_id = GenericIndexName::by_id(table.tablet_id);
    let by_author = GenericIndexName::new(table.tablet_id, "by_author".parse()?)?;
    let by_content = GenericIndexName::new(table.tablet_id, "by_content".parse()?)?;
    let indexes = vec![
        IndexMetadata::new_enabled(by_id.clone(), IndexedFields::by_id()),
        IndexMetadata::new_enabled(by_author.clone(), vec!["author".parse()?].try_into()?),
        IndexMetadata::new_enabled(by_content.clone(), vec!["content".parse()?].try_into()?),
    ];
    let index_documents = index_documents(&mut id_generator, indexes)?;
    let mut index_registry = IndexRegistry::bootstrap(
        &id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )?;
    let mut in_memory_indexes =
        BackendInMemoryIndexes::bootstrap(&index_registry, index_documents, Timestamp::MIN)?;

    let by_id = index_registry.get_enabled(&by_id).unwrap().id();
    let by_content = index_registry.get_enabled(&by_content).unwrap().id();
    let by_author = index_registry.get_enabled(&by_author).unwrap().id();

    let doc_id = next_document_id(&mut id_generator, "messages")?;
    let v1 = ResolvedDocument::new(
        doc_id,
        CreationTime::ONE,
        assert_obj!(
            "content" => "hllo",
            "author" => 33,
        ),
    )?;

    index_registry.update(None, Some(&v1))?;
    assert_eq!(
        in_memory_indexes.update(&index_registry, Timestamp::must(1), None, Some(v1.clone())),
        vec![
            DatabaseIndexUpdate {
                index_id: by_id,
                key: IndexKey::new(vec![], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: true,
            },
            DatabaseIndexUpdate {
                index_id: by_author,
                key: IndexKey::new(vec![33.into()], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
            DatabaseIndexUpdate {
                index_id: by_content,
                key: IndexKey::new(vec!["hllo".try_into()?], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
        ]
    );

    let v2 = ResolvedDocument::new(
        doc_id,
        CreationTime::ONE,
        assert_obj!(
            "content" => "hello (edited)",
            "author" => 33,
        ),
    )?;
    index_registry.update(Some(&v1), Some(&v2))?;
    assert_eq!(
        in_memory_indexes.update(
            &index_registry,
            Timestamp::must(2),
            Some(v1),
            Some(v2.clone())
        ),
        vec![
            DatabaseIndexUpdate {
                index_id: by_id,
                key: IndexKey::new(vec![], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: true,
            },
            // We generate an updated index entry even if the field has not
            // changed. Otherwise consistency checking and vacuuming old revisions
            // will become quite complicated. We can reconsider in the long run.
            DatabaseIndexUpdate {
                index_id: by_author,
                key: IndexKey::new(vec![33.into()], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
            DatabaseIndexUpdate {
                index_id: by_content,
                key: IndexKey::new(vec!["hello (edited)".try_into()?], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
            DatabaseIndexUpdate {
                index_id: by_content,
                key: IndexKey::new(vec!["hllo".try_into()?], doc_id.into()),
                value: DatabaseIndexValue::Deleted,
                is_system_index: false,
            },
        ]
    );

    let v3 = ResolvedDocument::new(
        doc_id,
        CreationTime::ONE,
        assert_obj!(
            "author" => 33,
        ),
    )?;
    index_registry.update(Some(&v2), Some(&v3))?;
    assert_eq!(
        in_memory_indexes.update(
            &index_registry,
            Timestamp::must(3),
            Some(v2),
            Some(v3.clone())
        ),
        vec![
            DatabaseIndexUpdate {
                index_id: by_id,
                key: IndexKey::new(vec![], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: true,
            },
            DatabaseIndexUpdate {
                index_id: by_author,
                key: IndexKey::new(vec![33.into()], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
            DatabaseIndexUpdate {
                index_id: by_content,
                key: IndexKey::new_allow_missing(vec![None], doc_id.into()),
                value: DatabaseIndexValue::NonClustered(doc_id),
                is_system_index: false,
            },
            DatabaseIndexUpdate {
                index_id: by_content,
                key: IndexKey::new(vec!["hello (edited)".try_into()?], doc_id.into()),
                value: DatabaseIndexValue::Deleted,
                is_system_index: false,
            },
        ]
    );

    index_registry.update(Some(&v3), None)?;
    assert_eq!(
        in_memory_indexes.update(&index_registry, Timestamp::must(4), Some(v3), None),
        vec![
            DatabaseIndexUpdate {
                index_id: by_id,
                key: IndexKey::new(vec![], doc_id.into()),
                value: DatabaseIndexValue::Deleted,
                is_system_index: true,
            },
            DatabaseIndexUpdate {
                index_id: by_author,
                key: IndexKey::new(vec![33.into()], doc_id.into()),
                value: DatabaseIndexValue::Deleted,
                is_system_index: false,
            },
            DatabaseIndexUpdate {
                index_id: by_content,
                key: IndexKey::new_allow_missing(vec![None], doc_id.into()),
                value: DatabaseIndexValue::Deleted,
                is_system_index: false,
            },
        ]
    );

    Ok(())
}

#[convex_macro::test_runtime]
async fn test_load_into_memory(_rt: TestRuntime) -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let table = id_generator.user_table_id(&"messages".parse()?);
    let by_id = GenericIndexName::by_id(table.tablet_id);
    let by_author = GenericIndexName::new(table.tablet_id, "by_author".parse()?)?;

    let indexes = vec![IndexMetadata::new_enabled(
        by_id.clone(),
        IndexedFields::by_id(),
    )];
    let index_documents = index_documents(&mut id_generator, indexes)?;
    let mut index_registry = IndexRegistry::bootstrap(
        &id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )?;
    let mut in_memory_indexes =
        BackendInMemoryIndexes::bootstrap(&index_registry, index_documents, Timestamp::MIN)?;

    let mut index_metadata =
        IndexMetadata::new_enabled(by_author.clone(), vec!["author".parse()?].try_into()?);
    let index_doc = gen_index_document(&mut id_generator, index_metadata.clone())?;
    index_registry.update(None, Some(&index_doc))?;
    in_memory_indexes.update(
        &index_registry,
        Timestamp::must(1),
        None,
        Some(index_doc.clone()),
    );

    // Add a document to persistence.
    let ps = Arc::new(TestPersistence::new());
    let doc1 = ResolvedDocument::new(
        next_document_id(&mut id_generator, "messages")?,
        CreationTime::ONE,
        assert_obj!(
            "content" => "hello there!",
            "author" => "alice",
        ),
    )?;
    index_registry.update(None, Some(&doc1))?;
    let index_updates = in_memory_indexes.update(
        &index_registry,
        Timestamp::must(2),
        None,
        Some(doc1.clone()),
    );
    ps.write(
        vec![(Timestamp::must(2), doc1.id_with_table_id(), Some(doc1))],
        index_updates
            .into_iter()
            .map(|u| (Timestamp::must(2), u))
            .collect(),
        ConflictStrategy::Error,
    )
    .await?;
    id_generator.write_tables(ps.clone()).await?;
    let retention_validator = Arc::new(NoopRetentionValidator {});

    // Load the index.
    in_memory_indexes
        .load_enabled(
            &index_registry,
            &by_author,
            &RepeatablePersistence::new(
                ps.clone(),
                unchecked_repeatable_ts(Timestamp::must(2)),
                retention_validator,
            )
            .read_snapshot(unchecked_repeatable_ts(Timestamp::must(2)))?,
        )
        .await?;
    assert_eq!(
        in_memory_indexes
            .in_memory_indexes()
            .get(&index_doc.id().internal_id())
            .unwrap()
            .len(),
        1
    );

    // Add another document.
    let doc2 = ResolvedDocument::new(
        next_document_id(&mut id_generator, "messages")?,
        CreationTime::ONE,
        assert_obj!(
            "content" => "hello to you too!",
            "author" => "bob",
        ),
    )?;
    index_registry.update(None, Some(&doc2))?;
    in_memory_indexes.update(&index_registry, Timestamp::must(3), None, Some(doc2));

    // Make sure both documents are loaded in the in_memory index.
    assert_eq!(
        in_memory_indexes
            .in_memory_indexes()
            .get(&index_doc.id().internal_id())
            .unwrap()
            .len(),
        2
    );

    // Change the index state. It should still be loaded into memory.
    must_let!(let IndexConfig::Database { ref mut on_disk_state, ..} = index_metadata.config);
    *on_disk_state = DatabaseIndexState::Enabled;
    let updated_index_doc = ResolvedDocument::new(
        index_doc.id(),
        CreationTime::ONE,
        index_metadata.try_into()?,
    )?;
    index_registry.update(Some(&index_doc), Some(&updated_index_doc))?;
    in_memory_indexes.update(
        &index_registry,
        Timestamp::must(4),
        Some(index_doc.clone()),
        Some(updated_index_doc.clone()),
    );

    // The documents should still be in-memory.
    assert_eq!(
        in_memory_indexes
            .in_memory_indexes()
            .get(&index_doc.id().internal_id())
            .unwrap()
            .len(),
        2
    );

    // Drop the index. It should no longer be in memory.
    index_registry.update(Some(&updated_index_doc), None)?;
    in_memory_indexes.update(
        &index_registry,
        Timestamp::must(4),
        Some(updated_index_doc),
        None,
    );
    assert!(in_memory_indexes
        .in_memory_indexes()
        .get(&index_doc.id().internal_id())
        .is_none());

    Ok(())
}

fn default_registry(id_generator: &mut TestIdGenerator) -> anyhow::Result<IndexRegistry> {
    let index_documents = index_documents(id_generator, vec![])?;
    IndexRegistry::bootstrap(
        id_generator,
        index_documents.values().map(|(_, d)| d),
        PersistenceVersion::default(),
    )
}

#[test]
pub fn same_indexes_empty_registry_are_identical() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();
    let first = default_registry(&mut id_generator)?;
    let second = first.clone();
    assert!(first.same_indexes(&second));
    assert!(second.same_indexes(&first));
    Ok(())
}

fn new_enabled_doc(
    id_generator: &mut dyn IdGenerator,
    tablet_id: TabletId,
    name: &str,
    fields: Vec<&str>,
) -> anyhow::Result<ResolvedDocument> {
    let index_name = GenericIndexName::new(tablet_id, name.parse()?)?;
    let field_paths = fields
        .into_iter()
        .map(|field| field.parse())
        .collect::<anyhow::Result<Vec<FieldPath>>>()?;

    let metadata = IndexMetadata::new_enabled(index_name, field_paths.try_into()?);
    gen_index_document(id_generator, metadata)
}

fn new_pending_doc(
    id_generator: &mut dyn IdGenerator,
    tablet_id: TabletId,
    name: &str,
    fields: Vec<&str>,
) -> anyhow::Result<ResolvedDocument> {
    let index_name = GenericIndexName::new(tablet_id, name.parse()?)?;
    let field_paths = fields
        .into_iter()
        .map(|field| field.parse())
        .collect::<anyhow::Result<Vec<FieldPath>>>()?;

    let metadata =
        IndexMetadata::new_backfilling(Timestamp::MIN, index_name, field_paths.try_into()?);
    gen_index_document(id_generator, metadata)
}

fn tablet_id(id_generator: &mut TestIdGenerator) -> anyhow::Result<TabletId> {
    Ok(id_generator.user_table_id(&"table".parse()?).tablet_id)
}

#[test]
pub fn same_indexes_one_enabled_one_empty_are_not_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let index_doc = new_enabled_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    first.update(None, Some(&index_doc))?;

    assert!(!first.same_indexes(&second));
    assert!(!second.same_indexes(&first));
    Ok(())
}

#[test]
pub fn same_indexes_identical_enabled_doc_are_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let mut second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let index_doc = new_enabled_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    first.update(None, Some(&index_doc))?;
    second.update(None, Some(&index_doc))?;

    assert!(first.same_indexes(&second));
    assert!(second.same_indexes(&first));
    Ok(())
}

#[test]
pub fn same_indexes_different_enabled_doc_id_are_not_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let mut second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let first_doc = new_enabled_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    let second_doc = new_enabled_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    first.update(None, Some(&first_doc))?;
    second.update(None, Some(&second_doc))?;

    assert!(!first.same_indexes(&second));
    assert!(!second.same_indexes(&first));
    Ok(())
}

#[test]
pub fn same_indexes_identical_pending_doc_are_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let mut second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let index_doc = new_pending_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    first.update(None, Some(&index_doc))?;
    second.update(None, Some(&index_doc))?;

    assert!(first.same_indexes(&second));
    assert!(second.same_indexes(&first));
    Ok(())
}

#[test]
pub fn same_indexes_different_pending_docs_are_not_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let mut second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let first_doc = new_pending_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    let second_doc = new_pending_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    first.update(None, Some(&first_doc))?;
    second.update(None, Some(&second_doc))?;

    assert!(!first.same_indexes(&second));
    assert!(!second.same_indexes(&first));
    Ok(())
}

#[test]
pub fn same_indexes_same_docs_different_states_are_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let mut second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let first_doc = new_pending_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    let second_doc = new_enabled_doc(
        &mut ConstantId(first_doc.id()),
        tablet_id,
        "by_author",
        vec!["author"],
    )?;

    first.update(None, Some(&first_doc))?;
    second.update(None, Some(&second_doc))?;

    assert!(first.same_indexes(&second));
    assert!(second.same_indexes(&first));
    Ok(())
}

#[test]
pub fn same_indexes_same_docs_different_update_order_are_same() -> anyhow::Result<()> {
    let mut id_generator = TestIdGenerator::new();

    let mut first = default_registry(&mut id_generator)?;
    let mut second = first.clone();

    let tablet_id = tablet_id(&mut id_generator)?;
    let first_enabled = new_enabled_doc(&mut id_generator, tablet_id, "by_author", vec!["author"])?;
    let second_enabled = new_enabled_doc(&mut id_generator, tablet_id, "by_title", vec!["title"])?;
    let first_pending = new_pending_doc(
        &mut id_generator,
        tablet_id,
        "by_publisher",
        vec!["publisher"],
    )?;
    let second_pending = new_pending_doc(
        &mut id_generator,
        tablet_id,
        "by_subtitle",
        vec!["subtitle"],
    )?;
    first.update(None, Some(&first_enabled))?;
    first.update(None, Some(&second_enabled))?;
    first.update(None, Some(&first_pending))?;
    first.update(None, Some(&second_pending))?;

    second.update(None, Some(&second_pending))?;
    second.update(None, Some(&first_pending))?;
    second.update(None, Some(&second_enabled))?;
    second.update(None, Some(&first_enabled))?;

    assert!(first.same_indexes(&second));
    assert!(second.same_indexes(&first));
    Ok(())
}
