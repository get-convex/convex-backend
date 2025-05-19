//! Allows to select what is included in a streaming export.
//!
//! Fivetran allows users to select which "schemas" (= Convex components),
//! tables and columns they want to include in a streaming export. This module
//! provides the [`StreamingExportSelection`] type, which allows to express such
//! a selection.
//!
//! For instance, here would be how to only include the `_id` and `name` fields
//! of the `users` table and nothing else:
//!
//! ```
//! # #![feature(try_blocks)]
//! # use common::{
//! #     components::ComponentPath,
//! #     document::{CreationTime, DeveloperDocument, InternalId},
//! #     pii::PII,
//! # };
//! # use maplit::btreemap;
//! # use sync_types::Timestamp;
//! # use value::{
//! #     assert_obj,
//! #     DeveloperDocumentId,
//! #     TableNumber,
//! # };
//! # use database::streaming_export_selection::{
//! #     StreamingExportColumnInclusion,
//! #     StreamingExportColumnSelection,
//! #     StreamingExportComponentSelection,
//! #     StreamingExportDocument,
//! #     StreamingExportInclusionDefault,
//! #     StreamingExportSelection,
//! #     StreamingExportTableSelection,
//! # };
//! # fn main() -> anyhow::Result<()> {
//! let selection = StreamingExportSelection {
//!     components: btreemap! {
//!         ComponentPath::root() => StreamingExportComponentSelection::Included {
//!             tables: btreemap! {
//!                 "users".parse()? => StreamingExportTableSelection::Included(
//!                     StreamingExportColumnSelection::new(
//!                         /* columns: */ btreemap! {
//!                             "_id".parse()? => StreamingExportColumnInclusion::Included,
//!                             "name".parse()? => StreamingExportColumnInclusion::Included,
//!                         },
//!                         /* other_columns: */ StreamingExportInclusionDefault::Excluded,
//!                     )?,
//!                 ),
//!             },
//!             other_tables: StreamingExportInclusionDefault::Excluded,
//!         },
//!     },
//!     other_components: StreamingExportInclusionDefault::Excluded,
//! };
//!
//! assert!(selection.is_table_included(&ComponentPath::root(), &"users".parse()?));
//! assert!(!selection.is_table_included(&ComponentPath::root(), &"groups".parse()?));
//! assert!(!selection.is_table_included(&"other_component".parse()?, &"users".parse()?));
//!
//! // This can also be used to filter documents
//! let doc_id = DeveloperDocumentId::new(TableNumber::try_from(1)?, InternalId::MIN);
//! let doc = DeveloperDocument::new(
//!     doc_id.clone(),
//!     CreationTime::try_from(Timestamp::MIN)?,
//!     assert_obj!("_id" => doc_id.encode(), "name" => "Nicolas", "age" => 23),
//! );
//!
//! let column_filter = selection.column_filter(&ComponentPath::root(), &"users".parse()?)?;
//! let filtered = column_filter.filter_document(doc)?;
//! assert_eq!(
//!     filtered,
//!     StreamingExportDocument::new(
//!         doc_id,
//!         PII(assert_obj!("_id" => doc_id.encode(), "name" => "Nicolas")),
//!     )?,
//! );
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;

use anyhow::{
    bail,
    Context,
};
use common::{
    components::ComponentPath,
    document::{
        DeveloperDocument,
        ID_FIELD,
    },
    pii::PII,
};
use convex_fivetran_source::api_types::selection as serialized;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use value::{
    ConvexObject,
    DeveloperDocumentId,
    FieldName,
    TableName,
};

#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug, Arbitrary))]
pub struct StreamingExportSelection {
    /// For each listed component, defines what to do with it in the
    /// streaming export.
    #[cfg_attr(
        test,
        proptest(strategy = "prop::collection::btree_map(any::<ComponentPath>(), \
                             any::<StreamingExportComponentSelection>(), 0..3)")
    )]
    pub components: BTreeMap<ComponentPath, StreamingExportComponentSelection>,

    /// Whether to include components that are not listed in `components`.
    pub other_components: StreamingExportInclusionDefault,
}

/// Defines what to do in streaming export for the components/tables/columns
/// that are not explicitly listed.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum StreamingExportInclusionDefault {
    /// Exclude these items in streaming export.
    Excluded,
    /// Include these items in streaming export, including all of their
    /// descendants. For instance, if applied to a component, all tables
    /// and columns in that component will be included.
    Included,
}

impl Default for StreamingExportSelection {
    fn default() -> Self {
        // By default, includes the entire deployment.
        Self {
            components: BTreeMap::new(),
            other_components: StreamingExportInclusionDefault::Included,
        }
    }
}

impl StreamingExportSelection {
    /// Know whether to include a given table in the streaming export.
    pub fn is_table_included(&self, component: &ComponentPath, table: &TableName) -> bool {
        match self.components.get(component) {
            Some(component_selection) => component_selection.is_table_included(table),
            None => self.other_components == StreamingExportInclusionDefault::Included,
        }
    }

    /// Get the [`StreamingExportColumnSelection`] for a given table.
    ///
    /// This should only be called for a table that is included in the
    /// streaming export. Otherwise, an error is returned.
    pub fn column_filter(
        &self,
        component: &ComponentPath,
        table: &TableName,
    ) -> anyhow::Result<&StreamingExportColumnSelection> {
        match (self.components.get(component), self.other_components) {
            (Some(component_selection), _) => component_selection.column_filter(table),
            (None, StreamingExportInclusionDefault::Included) => Ok(&ALL_COLUMNS),
            (None, StreamingExportInclusionDefault::Excluded) => {
                anyhow::bail!("Getting column filter for an implicitly excluded component")
            },
        }
    }

    /// Create a [`StreamingExportSelection`] that only includes a single
    /// table.
    #[cfg(test)]
    pub fn single_table(component: ComponentPath, table: TableName) -> Self {
        use maplit::btreemap;

        Self {
            components: btreemap! {
                component => StreamingExportComponentSelection::Included {
                    tables: btreemap! {
                        table => StreamingExportTableSelection::Included(
                            StreamingExportColumnSelection::all_columns(),
                        ),
                    },
                    other_tables: StreamingExportInclusionDefault::Excluded,
                },
            },
            other_components: StreamingExportInclusionDefault::Excluded,
        }
    }
}

/// What to do during streaming export for a particular component.
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug, Arbitrary))]
pub enum StreamingExportComponentSelection {
    Excluded,
    Included {
        #[cfg_attr(
            test,
            proptest(strategy = "prop::collection::btree_map(any::<TableName>(), \
                                 any::<StreamingExportTableSelection>(), 0..3)")
        )]
        tables: BTreeMap<TableName, StreamingExportTableSelection>,
        other_tables: StreamingExportInclusionDefault,
    },
}

impl StreamingExportComponentSelection {
    fn is_table_included(&self, table: &TableName) -> bool {
        match self {
            Self::Excluded => false,
            Self::Included {
                tables,
                other_tables,
            } => match tables.get(table) {
                Some(StreamingExportTableSelection::Excluded) => false,
                Some(StreamingExportTableSelection::Included { .. }) => true,
                None => other_tables == &StreamingExportInclusionDefault::Included,
            },
        }
    }

    fn column_filter(&self, table: &TableName) -> anyhow::Result<&StreamingExportColumnSelection> {
        Ok(match self {
            StreamingExportComponentSelection::Included {
                tables,
                other_tables,
            } => match (tables.get(table), other_tables) {
                (Some(StreamingExportTableSelection::Included(filter)), _) => filter,
                (None, StreamingExportInclusionDefault::Included) => &ALL_COLUMNS,
                _ => bail!("Getting column filter for an excluded table"),
            },
            StreamingExportComponentSelection::Excluded => {
                bail!("Getting column filter for an explicitly excluded component")
            },
        })
    }
}

/// What to do in streaming export for a particular table.
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug, Arbitrary))]
pub enum StreamingExportTableSelection {
    Excluded,
    Included(StreamingExportColumnSelection),
}

/// For a table in the streaming export, defines what to do for each of its
/// columns.
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug, Arbitrary))]
pub struct StreamingExportColumnSelection {
    #[cfg_attr(
        test,
        proptest(strategy = "prop::collection::btree_map(any::<FieldName>(), \
                             any::<StreamingExportColumnInclusion>(), 0..3)")
    )]
    columns: BTreeMap<FieldName, StreamingExportColumnInclusion>,
    other_columns: StreamingExportInclusionDefault,
}

/// Defines what to do for a particular column in the streaming export.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum StreamingExportColumnInclusion {
    Excluded,
    Included,
}

static ALL_COLUMNS: StreamingExportColumnSelection = StreamingExportColumnSelection::all_columns();

impl StreamingExportColumnSelection {
    pub fn new(
        columns: BTreeMap<FieldName, StreamingExportColumnInclusion>,
        other_columns: StreamingExportInclusionDefault,
    ) -> anyhow::Result<Self> {
        let column_selection = Self {
            columns,
            other_columns,
        };

        anyhow::ensure!(
            column_selection.includes(&FieldName::from(ID_FIELD.clone())),
            "`_id` must be included in the column selection"
        );

        Ok(column_selection)
    }

    /// Create a [`StreamingExportColumnSelection`] that includes all columns.
    pub const fn all_columns() -> Self {
        Self {
            columns: BTreeMap::new(),
            other_columns: StreamingExportInclusionDefault::Included,
        }
    }

    /// Filter a [`DeveloperDocument`] to only include the columns that are
    /// included in this column selection.
    pub fn filter_document(
        &self,
        document: DeveloperDocument,
    ) -> anyhow::Result<StreamingExportDocument> {
        let id = document.id();
        let value = document.into_value().0;

        let filtered_value = value.filter_fields(|field| self.includes(field));

        StreamingExportDocument::new(id, PII(filtered_value))
    }

    fn includes(&self, column: &FieldName) -> bool {
        self.columns
            .get(column)
            .cloned()
            .map(|c| c == StreamingExportColumnInclusion::Included)
            .unwrap_or(self.other_columns == StreamingExportInclusionDefault::Included)
    }
}

/// Similar to [`DeveloperDocument`], but `_creationTime` is allowed to be
/// omitted.
#[derive(Eq, PartialEq, Debug)]
#[cfg_attr(test, derive(Clone))]
pub struct StreamingExportDocument {
    id: DeveloperDocumentId,
    value: PII<ConvexObject>,
}

impl StreamingExportDocument {
    pub fn new(id: DeveloperDocumentId, value: PII<ConvexObject>) -> anyhow::Result<Self> {
        // Verify that `value` contains `_id`
        anyhow::ensure!(
            value.0.get(&FieldName::from(ID_FIELD.clone()))
                == Some(
                    &id.encode()
                        .try_into()
                        .context("Can't serialize the ID as a Convex string")?
                ),
            "`_id` must be included in the value"
        );

        Ok(Self { id, value })
    }
}

#[cfg(test)]
impl StreamingExportDocument {
    /// Create a [`StreamingExportDocument`] from a [`ResolvedDocument`],
    /// including all fields.
    pub fn with_all_fields(document: ::common::document::ResolvedDocument) -> Self {
        let document = document.to_developer();
        Self {
            id: document.id(),
            value: document.into_value(),
        }
    }

    pub fn id(&self) -> DeveloperDocumentId {
        self.id
    }
}

#[cfg(test)]
mod tests_is_table_included {
    use maplit::btreemap;

    use super::*;

    #[test]
    fn test_uses_other_components_when_specific_table_information_not_available() {
        let component: ComponentPath = "test_component".parse().unwrap();
        let table: TableName = "test_table".parse().unwrap();

        let selection_included = StreamingExportSelection {
            components: BTreeMap::new(),
            other_components: StreamingExportInclusionDefault::Included,
        };
        assert!(selection_included.is_table_included(&component, &table));

        let selection_excluded = StreamingExportSelection {
            components: BTreeMap::new(),
            other_components: StreamingExportInclusionDefault::Excluded,
        };
        assert!(!selection_excluded.is_table_included(&component, &table));
    }

    #[test]
    fn test_does_not_include_tables_from_excluded_components() {
        let component: ComponentPath = "test_component".parse().unwrap();
        let table: TableName = "test_table".parse().unwrap();

        let selection = StreamingExportSelection {
            components: btreemap! {
                component.clone() => StreamingExportComponentSelection::Excluded,
            },
            other_components: StreamingExportInclusionDefault::Included,
        };

        assert!(!selection.is_table_included(&component, &table));
    }

    #[test]
    fn test_uses_specific_table_information_when_available() {
        let component: ComponentPath = "test_component".parse().unwrap();
        let table: TableName = "test_table".parse().unwrap();

        let selection_excluded = StreamingExportSelection {
            components: btreemap! {
                component.clone() => StreamingExportComponentSelection::Included {
                    tables: btreemap! {
                        table.clone() => StreamingExportTableSelection::Excluded,
                    },
                    other_tables: StreamingExportInclusionDefault::Included,
                },
            },
            other_components: StreamingExportInclusionDefault::Included,
        };
        assert!(!selection_excluded.is_table_included(&component, &table));

        let selection_included = StreamingExportSelection {
            components: btreemap! {
                component.clone() => StreamingExportComponentSelection::Included {
                    tables: btreemap! {
                        table.clone() => StreamingExportTableSelection::Included(
                            StreamingExportColumnSelection::all_columns(),
                        ),
                    },
                    other_tables: StreamingExportInclusionDefault::Excluded,
                },
            },
            other_components: StreamingExportInclusionDefault::Excluded,
        };
        assert!(selection_included.is_table_included(&component, &table));
    }

    #[test]
    fn test_uses_default_table_selection_when_necessary() {
        let component: ComponentPath = "test_component".parse().unwrap();
        let table: TableName = "test_table".parse().unwrap();

        let selection_excluded = StreamingExportSelection {
            components: btreemap! {
                component.clone() => StreamingExportComponentSelection::Included {
                    tables: BTreeMap::new(),
                    other_tables: StreamingExportInclusionDefault::Excluded,
                },
            },
            other_components: StreamingExportInclusionDefault::Included,
        };
        assert!(!selection_excluded.is_table_included(&component, &table));

        let selection_included = StreamingExportSelection {
            components: btreemap! {
                component.clone() => StreamingExportComponentSelection::Included {
                    tables: BTreeMap::new(),
                    other_tables: StreamingExportInclusionDefault::Included,
                },
            },
            other_components: StreamingExportInclusionDefault::Included,
        };
        assert!(selection_included.is_table_included(&component, &table));
    }
}

impl TryFrom<serialized::Selection> for StreamingExportSelection {
    type Error = anyhow::Error;

    fn try_from(
        serialized::Selection {
            components,
            other_components,
        }: serialized::Selection,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            components: components
                .into_iter()
                .map(|(k, v)| -> anyhow::Result<_> { Ok((k.parse()?, v.try_into()?)) })
                .try_collect()?,
            other_components: other_components.into(),
        })
    }
}

impl From<serialized::ColumnInclusion> for StreamingExportColumnInclusion {
    fn from(value: serialized::ColumnInclusion) -> Self {
        match value {
            serialized::ColumnInclusion::Excluded => Self::Excluded,
            serialized::ColumnInclusion::Included => Self::Included,
        }
    }
}

impl TryFrom<serialized::ComponentSelection> for StreamingExportComponentSelection {
    type Error = anyhow::Error;

    fn try_from(value: serialized::ComponentSelection) -> Result<Self, Self::Error> {
        Ok(match value {
            serialized::ComponentSelection::Excluded => Self::Excluded,
            serialized::ComponentSelection::Included {
                tables,
                other_tables,
            } => Self::Included {
                tables: tables
                    .into_iter()
                    .map(|(k, v)| -> anyhow::Result<_> { Ok((k.parse()?, v.try_into()?)) })
                    .try_collect()?,
                other_tables: other_tables.into(),
            },
        })
    }
}

impl TryFrom<serialized::TableSelection> for StreamingExportTableSelection {
    type Error = anyhow::Error;

    fn try_from(value: serialized::TableSelection) -> Result<Self, Self::Error> {
        Ok(match value {
            serialized::TableSelection::Excluded => Self::Excluded,
            serialized::TableSelection::Included {
                columns,
                other_columns,
            } => {
                let column_selection = StreamingExportColumnSelection {
                    columns: columns
                        .into_iter()
                        .map(|(k, v)| -> anyhow::Result<_> { Ok((k.parse()?, v.into())) })
                        .try_collect()?,
                    other_columns: other_columns.into(),
                };

                Self::Included(column_selection)
            },
        })
    }
}

impl From<serialized::InclusionDefault> for StreamingExportInclusionDefault {
    fn from(value: serialized::InclusionDefault) -> Self {
        match value {
            serialized::InclusionDefault::Excluded => Self::Excluded,
            serialized::InclusionDefault::Included => Self::Included,
        }
    }
}

#[cfg(test)]
mod tests_column_filtering {
    use common::document::CreationTime;
    use maplit::btreemap;
    use sync_types::Timestamp;
    use value::{
        assert_obj,
        InternalId,
        TableNumber,
    };

    use super::*;

    #[test]
    fn test_uses_specific_column_information_when_available() -> anyhow::Result<()> {
        let column: FieldName = "test_column".parse().unwrap();

        let selection_excluded = StreamingExportColumnSelection::new(
            btreemap! {
                "_id".parse()? => StreamingExportColumnInclusion::Included,
                column.clone() => StreamingExportColumnInclusion::Excluded,
            },
            StreamingExportInclusionDefault::Included,
        )?;
        assert!(!selection_excluded.includes(&column));

        let selection_included = StreamingExportColumnSelection::new(
            btreemap! {
                "_id".parse()? => StreamingExportColumnInclusion::Included,
                column.clone() => StreamingExportColumnInclusion::Included,
            },
            StreamingExportInclusionDefault::Excluded,
        )?;
        assert!(selection_included.includes(&column));

        Ok(())
    }

    #[test]
    fn test_uses_other_columns_when_specific_information_does_not_exist() -> anyhow::Result<()> {
        let column: FieldName = "test_column".parse().unwrap();

        let selection_excluded = StreamingExportColumnSelection::new(
            btreemap! {
                "_id".parse()? => StreamingExportColumnInclusion::Included,
            },
            StreamingExportInclusionDefault::Excluded,
        )?;
        assert!(!selection_excluded.includes(&column));

        let selection_included = StreamingExportColumnSelection::new(
            BTreeMap::new(),
            StreamingExportInclusionDefault::Included,
        )?;
        assert!(selection_included.includes(&column));

        Ok(())
    }

    #[test]
    fn test_cant_create_filter_with_id_column_excluded() {
        let id_column: FieldName = "_id".parse().unwrap();

        StreamingExportColumnSelection::new(
            btreemap! {
                id_column.clone() => StreamingExportColumnInclusion::Excluded,
            },
            StreamingExportInclusionDefault::Included,
        )
        .unwrap_err();

        StreamingExportColumnSelection::new(
            btreemap! {
                id_column.clone() => StreamingExportColumnInclusion::Included,
            },
            StreamingExportInclusionDefault::Excluded,
        )
        .unwrap();

        StreamingExportColumnSelection::new(
            btreemap! {},
            StreamingExportInclusionDefault::Excluded,
        )
        .unwrap_err();
    }

    #[test]
    fn test_filter_document_with_creation_time() -> anyhow::Result<()> {
        let id = DeveloperDocumentId::new(TableNumber::try_from(1)?, InternalId::MIN);
        let creation_time = CreationTime::try_from(Timestamp::MIN)?;
        let value = assert_obj!(
            "_id" => id.encode(),
            "_creationTime" => f64::from(creation_time),
            "name" => "test",
            "password" => "hunter2",
        );
        let doc = DeveloperDocument::new(id, creation_time, value);

        let selection = StreamingExportColumnSelection::new(
            btreemap! {
                "_id".parse()? => StreamingExportColumnInclusion::Included,
                "_creationTime".parse()? => StreamingExportColumnInclusion::Included,
                "name".parse()? => StreamingExportColumnInclusion::Included,
            },
            StreamingExportInclusionDefault::Excluded,
        )?;

        let filtered = selection.filter_document(doc.clone())?;
        assert_eq!(
            filtered,
            StreamingExportDocument::new(
                id,
                PII(assert_obj!(
                    "_id" => id.encode(),
                    "_creationTime" => f64::from(creation_time),
                    "name" => "test"
                )),
            )?
        );

        Ok(())
    }

    #[test]
    fn test_filter_document_without_creation_time() -> anyhow::Result<()> {
        let id = DeveloperDocumentId::new(TableNumber::try_from(1)?, InternalId::MIN);
        let creation_time = CreationTime::try_from(Timestamp::MIN)?;
        let value = assert_obj!(
            "_id" => id.encode(),
            "_creationTime" => f64::from(creation_time),
            "name" => "test",
            "password" => "hunter2",
        );
        let doc = DeveloperDocument::new(id, creation_time, value);

        let selection = StreamingExportColumnSelection::new(
            btreemap! {
                "_creationTime".parse()? => StreamingExportColumnInclusion::Excluded,
                "password".parse()? => StreamingExportColumnInclusion::Excluded,
            },
            StreamingExportInclusionDefault::Included,
        )?;

        let filtered = selection.filter_document(doc)?;
        assert_eq!(
            filtered,
            StreamingExportDocument::new(
                id,
                PII(assert_obj!(
                    "_id" => id.encode(),
                    "name" => "test"
                )),
            )?
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests_serialization {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::*;

    impl From<StreamingExportSelection> for serialized::Selection {
        fn from(
            StreamingExportSelection {
                components,
                other_components,
            }: StreamingExportSelection,
        ) -> Self {
            Self {
                components: components
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.into()))
                    .collect(),
                other_components: other_components.into(),
            }
        }
    }

    impl From<StreamingExportColumnInclusion> for serialized::ColumnInclusion {
        fn from(value: StreamingExportColumnInclusion) -> Self {
            match value {
                StreamingExportColumnInclusion::Excluded => Self::Excluded,
                StreamingExportColumnInclusion::Included => Self::Included,
            }
        }
    }

    impl From<StreamingExportComponentSelection> for serialized::ComponentSelection {
        fn from(value: StreamingExportComponentSelection) -> Self {
            match value {
                StreamingExportComponentSelection::Excluded => Self::Excluded,
                StreamingExportComponentSelection::Included {
                    tables,
                    other_tables,
                } => Self::Included {
                    tables: tables
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.into()))
                        .collect(),
                    other_tables: other_tables.into(),
                },
            }
        }
    }

    impl From<StreamingExportTableSelection> for serialized::TableSelection {
        fn from(value: StreamingExportTableSelection) -> Self {
            match value {
                StreamingExportTableSelection::Excluded => Self::Excluded,
                StreamingExportTableSelection::Included(StreamingExportColumnSelection {
                    columns,
                    other_columns,
                }) => Self::Included {
                    columns: columns
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.into()))
                        .collect(),
                    other_columns: other_columns.into(),
                },
            }
        }
    }

    impl From<StreamingExportInclusionDefault> for serialized::InclusionDefault {
        fn from(value: StreamingExportInclusionDefault) -> Self {
            match value {
                StreamingExportInclusionDefault::Excluded => Self::Excluded,
                StreamingExportInclusionDefault::Included => Self::Included,
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1),
            failure_persistence: None, ..ProptestConfig::default()
        })]
        #[test]
        fn test_streaming_export_selection_roundtrip(value in any::<StreamingExportSelection>()) {
            let serialized: serialized::Selection = value.clone().into();
            let deserialized: StreamingExportSelection = serialized.try_into().expect("Can't roundtrip");
            prop_assert_eq!(value, deserialized);
        }
    }
}
