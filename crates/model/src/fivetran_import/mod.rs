use std::{
    collections::BTreeMap,
    str::FromStr,
};

use anyhow::Context;
use chrono::{
    DateTime,
    Utc,
};
use common::{
    bootstrap_model::index::{
        database_index::DeveloperDatabaseIndexConfig,
        DeveloperIndexMetadata,
        IndexConfig,
    },
    document::{
        ResolvedDocument,
        CREATION_TIME_FIELD_PATH,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
};
use convex_fivetran_destination::{
    api_types::{
        BatchWriteOperation,
        BatchWriteRow,
        DeleteType,
    },
    constants::{
        FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
        FIVETRAN_SYNCED_INDEX_DESCRIPTOR,
        FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS,
        FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS,
        METADATA_CONVEX_FIELD_NAME,
        SOFT_DELETE_CONVEX_FIELD_NAME,
        SOFT_DELETE_FIELD_PATH,
        SYNCED_FIELD_PATH,
    },
};
use database::{
    IndexModel,
    PatchValue,
    ResolvedQuery,
    Transaction,
    UserFacingModel,
};
use errors::ErrorMetadata;
use value::{
    ConvexObject,
    ConvexValue,
    FieldName,
    TableName,
    TableNamespace,
};

pub struct FivetranImportModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> FivetranImportModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn apply_operation(&mut self, row: BatchWriteRow) -> anyhow::Result<()> {
        let table_name = TableName::from_str(&row.table)?;
        let existing_document = self
            .primary_key_query(&table_name, &row.row)
            .await?
            .expect_at_most_one(self.tx)
            .await?;

        match row.operation {
            BatchWriteOperation::Upsert => {
                if let Some(existing_document) = existing_document {
                    UserFacingModel::new(self.tx, TableNamespace::Global)
                        .replace(existing_document.developer_id(), row.row)
                        .await?;
                } else {
                    UserFacingModel::new(self.tx, TableNamespace::Global)
                        .insert(table_name.clone(), row.row)
                        .await?;
                }
            },
            BatchWriteOperation::Update => {
                let Some(existing_document) = existing_document else {
                    anyhow::bail!(ErrorMetadata::not_found(
                        "FivetranMissingUpdatedRow",
                        format!(
                            "Fivetran is trying to update a row that doesn’t exist in the Convex \
                             destination.",
                        ),
                    ));
                };

                UserFacingModel::new(self.tx, TableNamespace::Global)
                    .patch(
                        existing_document.developer_id(),
                        fivetran_patch_value(existing_document.into_value().into_value(), row.row),
                    )
                    .await?;
            },
            BatchWriteOperation::HardDelete => {
                if let Some(existing_document) = existing_document {
                    UserFacingModel::new(self.tx, TableNamespace::Global)
                        .delete(existing_document.developer_id())
                        .await?;
                }
            },
        }

        Ok(())
    }

    pub async fn truncate_document(
        &mut self,
        doc: ResolvedDocument,
        delete_type: DeleteType,
    ) -> anyhow::Result<()> {
        match delete_type {
            DeleteType::HardDelete => {
                UserFacingModel::new(self.tx, TableNamespace::Global)
                    .delete(doc.developer_id())
                    .await?;
            },
            DeleteType::SoftDelete => {
                UserFacingModel::new(self.tx, TableNamespace::Global)
                    .replace(
                        doc.developer_id(),
                        mark_as_soft_deleted(doc.into_value().0)?,
                    )
                    .await?;
            },
        }
        Ok(())
    }

    async fn primary_key_query(
        &mut self,
        table_name: &TableName,
        object: &ConvexObject,
    ) -> anyhow::Result<ResolvedQuery<RT>> {
        let mut model = IndexModel::new(self.tx);
        let indexes = model
            .get_system_indexes(TableNamespace::root_component())
            .await?;
        let index = indexes
            .into_iter()
            .map(|index| index.into_value())
            .find(|index: &DeveloperIndexMetadata| {
                index.name.table() == table_name
                    && *index.name.descriptor() == *FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR
                    && index.is_database_index()
            })
            .ok_or_else(|| {
                ErrorMetadata::bad_request(
                    "MissingFivetranPrimaryKeyIndex",
                    format!(
                        "The `{}` index on the `{table_name}` table is missing. Please edit your \
                         `schema.ts` file and add the following index to the `{table_name}` \
                         table: .index(\"{}\", [/* …attributes in the primary key… */]) (or \
                         .index(\"sync_index\", [\"fivetran.deleted\", /* …attributes in the \
                         primary key… */]).",
                        *FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
                        *FIVETRAN_PRIMARY_KEY_INDEX_DESCRIPTOR,
                    ),
                )
            })?;

        let IndexConfig::Database {
            developer_config: DeveloperDatabaseIndexConfig { fields },
            ..
        } = index.config
        else {
            anyhow::bail!("Unexpected index type");
        };

        let mut range: Vec<IndexRangeExpression> = Vec::new();
        for field_path in fields.iter() {
            range.push(if *field_path == *SOFT_DELETE_FIELD_PATH {
                IndexRangeExpression::Eq(field_path.clone(), ConvexValue::Boolean(false).into())
            } else if *field_path == *CREATION_TIME_FIELD_PATH {
                continue;
            } else {
                let value = object
                    .get_path(field_path)
                    .context(ErrorMetadata::bad_request(
                        "MissingValueForFivetranPrimaryKeyIndex",
                        format!(
                            "Missing value for field {field_path:?} of primary key index {}.",
                            index.name
                        ),
                    ))?;
                IndexRangeExpression::Eq(field_path.clone(), value.clone().into())
            })
        }

        ResolvedQuery::new(
            self.tx,
            TableNamespace::Global,
            Query::index_range(IndexRange {
                index_name: index.name,
                range,
                order: Order::Asc,
            }),
        )
    }

    /// Queries all the documents in the given table that were not soft deleted.
    ///
    /// If `delete_before` is set, only returns documents that have a
    /// `fivetran.synced` field smaller than `delete_before`.
    pub async fn synced_query(
        &mut self,
        table_name: &TableName,
        delete_before: &Option<DateTime<Utc>>,
    ) -> anyhow::Result<ResolvedQuery<RT>> {
        let mut model = IndexModel::new(self.tx);
        let indexes = model
            .get_system_indexes(TableNamespace::root_component())
            .await?;

        let index = indexes
            .into_iter()
            .map(|index| index.into_value())
            .find(|index| {
                *index.name.table() == *table_name
                    && *index.name.descriptor() == *FIVETRAN_SYNCED_INDEX_DESCRIPTOR
            })
            .ok_or_else(|| {
                ErrorMetadata::bad_request(
                    "MissingFivetranSyncedIndex",
                    format!(
                        "The Fivetran synchronization index on the `{table_name}` table is \
                         missing. Something went wrong with the Fivetran sync.",
                    ),
                )
            })?;

        if !index.is_database_index() {
            anyhow::bail!("Unexpected index type");
        }

        let IndexConfig::Database {
            developer_config: DeveloperDatabaseIndexConfig { fields },
            ..
        } = index.config
        else {
            anyhow::bail!("Unexpected index type");
        };

        if fields != *FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS
            && fields != *FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS
        {
            anyhow::bail!(ErrorMetadata::bad_request(
                "WrongFieldsInFivetranSyncedIndex",
                format!(
                    "The Fivetran synchronization index on the `{table_name}` has the wrong \
                     fields {fields:?}. Expected {} or {} Something went wrong with the Fivetran \
                     sync.",
                    *FIVETRAN_SYNC_INDEX_WITH_SOFT_DELETE_FIELDS,
                    *FIVETRAN_SYNC_INDEX_WITHOUT_SOFT_DELETE_FIELDS,
                ),
            ))
        }

        let mut range: Vec<IndexRangeExpression> = Vec::new();
        for field_path in fields.iter() {
            if *field_path == *SYNCED_FIELD_PATH {
                if let Some(delete_before) = delete_before {
                    range.push(IndexRangeExpression::Lt(
                        field_path.clone(),
                        ConvexValue::Float64(delete_before.timestamp_millis() as f64),
                    ));
                }
            } else if *field_path == *SOFT_DELETE_FIELD_PATH {
                range.push(IndexRangeExpression::Eq(
                    field_path.clone(),
                    ConvexValue::Boolean(false).into(),
                ));
            } else if *field_path == *CREATION_TIME_FIELD_PATH {
                continue;
            } else {
                anyhow::bail!("Unexpected field in sync index");
            }
        }

        ResolvedQuery::new(
            self.tx,
            TableNamespace::Global,
            Query::index_range(IndexRange {
                index_name: index.name,
                range,
                order: Order::Asc,
            }),
        )
    }
}

/// Converts `patch` to the corresponding `PatchValue`, including the
/// preexisting Fivetran metadata fields if they exist and are not overridden.
///
/// We need this because patching a document will replace all fields with their
/// new value. But for Fivetran’s point of view, the metadata fields are all
/// separate, which means we must make sure that we’re not overriding the entire
/// `fivetran` field if it contains attributes that are not overridden.
fn fivetran_patch_value(existing_document: ConvexObject, patch: ConvexObject) -> PatchValue {
    let metadata_field_name = FieldName::from(METADATA_CONVEX_FIELD_NAME.clone());

    let mut existing_document: BTreeMap<FieldName, ConvexValue> = existing_document.into();
    let Some(ConvexValue::Object(existing_metadata)) =
        existing_document.remove(&metadata_field_name)
    else {
        return PatchValue::from(patch);
    };

    let mut patch_contents: BTreeMap<FieldName, ConvexValue> = patch.clone().into();
    let Some(ConvexValue::Object(new_metadata)) = patch_contents.remove(&metadata_field_name)
    else {
        return PatchValue::from(patch);
    };

    PatchValue::from(
        patch
            .shallow_merge(
                ConvexObject::for_value(
                    metadata_field_name,
                    ConvexValue::Object(existing_metadata.shallow_merge(new_metadata).expect(
                        "The number of metadata fields should always be under the field count \
                         limit",
                    )),
                )
                .expect("Putting the merged object in a new object is always valid"),
            )
            .expect(
                "Adding a field to an object that previously had the same field keeps the object \
                 under the field count limit",
            ),
    )
}

fn mark_as_soft_deleted(object: ConvexObject) -> anyhow::Result<ConvexObject> {
    let metadata_key = FieldName::from(METADATA_CONVEX_FIELD_NAME.clone());

    let mut new_value: BTreeMap<FieldName, ConvexValue> = object.into();
    let metadata_object = match new_value.remove(&metadata_key) {
        Some(ConvexValue::Object(object)) => object,
        _ => ConvexObject::empty(),
    };

    new_value.insert(
        metadata_key,
        ConvexValue::Object(metadata_object.shallow_merge(ConvexObject::for_value(
            FieldName::from(SOFT_DELETE_CONVEX_FIELD_NAME.clone()),
            ConvexValue::Boolean(true),
        )?)?),
    );
    new_value.try_into()
}

#[cfg(test)]
mod tests {
    use database::PatchValue;
    use value::{
        assert_obj,
        ConvexValue,
    };

    use crate::fivetran_import::{
        fivetran_patch_value,
        mark_as_soft_deleted,
    };

    #[test]
    fn test_mark_as_soft_deleted_for_object_with_fivetran_field() -> anyhow::Result<()> {
        assert_eq!(
            mark_as_soft_deleted(assert_obj!(
                "top_level_field" => ConvexValue::Int64(42),
                "fivetran" => assert_obj!(
                    "id" => ConvexValue::Int64(1),
                    "synced" => ConvexValue::Float64(1715336497241.0),
                    "deleted" => ConvexValue::Boolean(false),
                ),
            ))?,
            assert_obj!(
                "top_level_field" => ConvexValue::Int64(42),
                "fivetran" => assert_obj!(
                    "id" => ConvexValue::Int64(1),
                    "synced" => ConvexValue::Float64(1715336497241.0),
                    "deleted" => ConvexValue::Boolean(true),
                ),
            )
        );
        Ok(())
    }

    #[test]
    fn test_mark_as_soft_deleted_for_object_without_fivetran_field() -> anyhow::Result<()> {
        assert_eq!(
            mark_as_soft_deleted(assert_obj!(
                "top_level_field" => ConvexValue::Int64(42),
            ))?,
            assert_obj!(
                "top_level_field" => ConvexValue::Int64(42),
                "fivetran" => assert_obj!(
                    "deleted" => ConvexValue::Boolean(true),
                ),
            )
        );
        Ok(())
    }

    #[test]
    fn test_fivetran_patch_value_with_no_metadata_field() {
        assert_eq!(
            fivetran_patch_value(
                assert_obj!(
                    "id" => 42,
                    "name" => "Nathan",
                ),
                assert_obj!("name" => "Nicolas")
            ),
            PatchValue::from(assert_obj!("name" => "Nicolas"))
        );
    }

    #[test]
    fn test_fivetran_patch_value_with_metadata_field_only_on_existing() {
        assert_eq!(
            fivetran_patch_value(
                assert_obj!(
                    "id" => 42,
                    "name" => "Nathan",
                    "fivetran" => assert_obj!("synced" => 1716672331152),
                ),
                assert_obj!("name" => "Nicolas")
            ),
            PatchValue::from(assert_obj!("name" => "Nicolas"))
        );
    }

    #[test]
    fn test_fivetran_patch_value_with_metadata_field_only_on_patch() {
        assert_eq!(
            fivetran_patch_value(
                assert_obj!(
                    "id" => 42,
                    "name" => "Nathan",
                ),
                assert_obj!(
                    "name" => "Nicolas",
                    "fivetran" => assert_obj!("synced" => 1716672331152),
                )
            ),
            PatchValue::from(assert_obj!(
                "name" => "Nicolas",
                "fivetran" => assert_obj!("synced" => 1716672331152),
            ))
        );
    }

    #[test]
    fn test_fivetran_patch_value_merge() {
        assert_eq!(
            fivetran_patch_value(
                assert_obj!(
                    "id" => 42,
                    "name" => "Nathan",
                    "fivetran" => assert_obj!(
                        "id" => 42,
                        "deleted" => false,
                        "synced" => 1716672331152,
                    ),
                ),
                assert_obj!(
                    "name" => "Nicolas",
                    "fivetran" => assert_obj!(
                        "id" => 42,
                        "synced" => 1716672354859,
                    ),
                )
            ),
            PatchValue::from(assert_obj!(
                "name" => "Nicolas",
                "fivetran" => assert_obj!(
                    "id" => 42,
                    "deleted" => false,
                    "synced" => 1716672354859,
                ),
            ))
        );
    }
}
