use std::collections::BTreeMap;

use serde::{
    Deserialize,
    Serialize,
};

/// Defines the components, tables, and columns to export in a deployment.
///
/// This is the serializable version of `StreamingExportSelection` in the
/// database crate.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Eq, PartialEq, Debug, Clone))]
pub struct Selection {
    #[serde(flatten)]
    pub components: BTreeMap<String, ComponentSelection>,
    #[serde(rename = "_other")]
    pub other_components: InclusionDefault,
}

/// Serializable version of `StreamingExportInclusionDefault`
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum InclusionDefault {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(rename = "incl")]
    Included,
}

impl Default for Selection {
    fn default() -> Self {
        // By default, include everything.
        Self {
            components: BTreeMap::new(),
            other_components: InclusionDefault::Included,
        }
    }
}

/// Serializable version of `StreamingExportComponentSelection`.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug))]
pub enum ComponentSelection {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(untagged)]
    Included {
        #[serde(flatten)]
        tables: BTreeMap<String, TableSelection>,
        #[serde(rename = "_other")]
        other_tables: InclusionDefault,
    },
}

/// Serializable version of
/// `StreamingExportTableSelection` + `StreamingExportColumnSelection`
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Eq, PartialEq, Debug))]
pub enum TableSelection {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(untagged)]
    Included {
        #[serde(flatten)]
        columns: BTreeMap<String, ColumnInclusion>,
        #[serde(rename = "_other")]
        other_columns: InclusionDefault,
    },
}

/// Serializable version of `StreamingExportColumnInclusion`.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ColumnInclusion {
    #[serde(rename = "excl")]
    Excluded,
    #[serde(rename = "incl")]
    Included,
}

#[cfg(test)]
mod tests {
    use maplit::btreemap;
    use serde::{
        de::DeserializeOwned,
        Serialize,
    };
    use serde_json::{
        json,
        Value as JsonValue,
    };

    use super::*;

    #[test]
    pub fn test_selection_serde() {
        assert_serde(
            json!({
                "": json!({
                    "users": json!({
                        "name": json!("incl"),
                        "_creationTime": json!("incl"),
                        "_deleted": json!("incl"),
                        "password": json!("excl"),
                        "_other": json!("excl"),
                    }),
                    "_other": json!("excl"),
                }),
                "waitlist": json!("excl"),
                "_other": json!("incl"),
            }),
            Selection {
                components: btreemap! {
                    "".to_string() => ComponentSelection::Included {
                        tables: btreemap! {
                            "users".to_string() => TableSelection::Included {
                                columns: btreemap! {
                                    "name".to_string() => ColumnInclusion::Included,
                                    "_creationTime".to_string() => ColumnInclusion::Included,
                                    "_deleted".to_string() => ColumnInclusion::Included,
                                    "password".to_string() => ColumnInclusion::Excluded,
                                },
                                other_columns: InclusionDefault::Excluded,
                            },
                        },
                        other_tables: InclusionDefault::Excluded,
                    },
                    "waitlist".to_string() => ComponentSelection::Excluded,
                },
                other_components: InclusionDefault::Included,
            },
        );
    }

    #[test]
    fn test_serde_inclusion() {
        assert_serde(json!("incl"), InclusionDefault::Included);
        assert_serde(json!("excl"), InclusionDefault::Excluded);
    }

    #[test]
    fn test_serde_table() {
        assert_serde(json!("excl"), TableSelection::Excluded);
        assert_serde(
            json!({ "_other": "incl" }),
            TableSelection::Included {
                columns: BTreeMap::new(),
                other_columns: InclusionDefault::Included,
            },
        );
    }

    #[test]
    fn test_serde_component() {
        assert_serde(json!("excl"), ComponentSelection::Excluded);
        assert_serde(
            json!({ "users": json!({ "_other": "incl" }), "_other": "excl" }),
            ComponentSelection::Included {
                tables: btreemap! {
                    "users".to_string() => TableSelection::Included {
                        columns: BTreeMap::new(),
                        other_columns: InclusionDefault::Included,
                    },
                },
                other_tables: InclusionDefault::Excluded,
            },
        );
    }

    fn assert_serde<T: DeserializeOwned + Serialize + Eq + std::fmt::Debug + Clone>(
        json: JsonValue,
        value: T,
    ) {
        let serialized = serde_json::to_value(&value).expect("can’t serialize to JSON");
        assert_eq!(
            json, serialized,
            "Incorrect serialization\n➡️ Input: {value:?}\n✅ Expected: {json:?}\n❌ Got:      \
             {serialized:?}"
        );

        let deserialized: T = serde_json::from_value(json.clone()).expect("can’t deserialize to T");
        assert_eq!(
            value, deserialized,
            "Incorrect deserialization\n➡️ Input: {json:?}\n✅ Expected: {value:?}\n❌ Got:      \
             {deserialized:?}"
        );
    }
}
