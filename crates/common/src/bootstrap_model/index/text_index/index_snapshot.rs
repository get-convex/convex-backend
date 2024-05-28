use serde::{
    Deserialize,
    Serialize,
};
use value::{
    serde::WithUnknown,
    ConvexObject,
};

use crate::types::{
    ObjectKey,
    PersistenceVersion,
    Timestamp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct TextIndexSnapshot {
    pub data: TextIndexSnapshotData,
    pub ts: Timestamp,
    pub version: TextSnapshotVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextIndexSnapshotData {
    /// The "legacy" (aka current) single segment format that must be built by
    /// reading the entire table for each set of incremental updates.
    SingleSegment(ObjectKey),
    /// The new (currently unused) multi segment format that can be built
    /// incrementally.
    MultiSegment(Vec<FragmentedTextSegment>),
    /// An unrecognized format that can be round tripped without being modified.
    /// Same as a proto with unknown fields.
    /// Used because we don't want to delete / recreate index metadata
    /// unintentionally when changing versions and rolling services
    /// backwards/forwards.
    Unknown(ConvexObject),
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::{
        prelude::*,
        sample::size_range,
    };
    use value::{
        ConvexObject,
        ExcludeSetsAndMaps,
        FieldType,
    };

    use super::{
        FragmentedTextSegment,
        TextIndexSnapshotData,
    };
    use crate::types::ObjectKey;

    impl Arbitrary for TextIndexSnapshotData {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                any::<ObjectKey>().prop_map(TextIndexSnapshotData::SingleSegment),
                any::<Vec<FragmentedTextSegment>>().prop_map(TextIndexSnapshotData::MultiSegment),
                any_with::<ConvexObject>((
                    size_range(0..=4),
                    FieldType::User,
                    ExcludeSetsAndMaps(true)
                ))
                .prop_map(TextIndexSnapshotData::Unknown),
            ]
            .boxed()
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "data_type", rename_all = "PascalCase")]
enum SerializedTextIndexSnapshotData {
    SingleSegment {
        segment: String,
    },
    MultiSegment {
        segments: Vec<SerializedFragmentedTextSegment>,
    },
}

impl TryFrom<WithUnknown<SerializedTextIndexSnapshotData>> for TextIndexSnapshotData {
    type Error = anyhow::Error;

    fn try_from(value: WithUnknown<SerializedTextIndexSnapshotData>) -> Result<Self, Self::Error> {
        match value {
            WithUnknown::Known(SerializedTextIndexSnapshotData::SingleSegment { segment }) => Ok(
                TextIndexSnapshotData::SingleSegment(ObjectKey::try_from(segment)?),
            ),
            WithUnknown::Known(SerializedTextIndexSnapshotData::MultiSegment {
                segments: serialized_segments,
            }) => {
                let segments: Vec<FragmentedTextSegment> = serialized_segments
                    .into_iter()
                    .map(FragmentedTextSegment::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(TextIndexSnapshotData::MultiSegment(segments))
            },
            WithUnknown::Unknown(unknown) => Ok(TextIndexSnapshotData::Unknown(unknown)),
        }
    }
}

impl TryFrom<TextIndexSnapshotData> for WithUnknown<SerializedTextIndexSnapshotData> {
    type Error = anyhow::Error;

    fn try_from(value: TextIndexSnapshotData) -> Result<Self, Self::Error> {
        match value {
            TextIndexSnapshotData::SingleSegment(segment) => {
                let serialized_segment = segment.try_into()?;
                Ok(WithUnknown::Known(
                    SerializedTextIndexSnapshotData::SingleSegment {
                        segment: serialized_segment,
                    },
                ))
            },
            TextIndexSnapshotData::MultiSegment(segments) => {
                let serialized_segments: Vec<SerializedFragmentedTextSegment> = segments
                    .into_iter()
                    .map(SerializedFragmentedTextSegment::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(WithUnknown::Known(
                    SerializedTextIndexSnapshotData::MultiSegment {
                        segments: serialized_segments,
                    },
                ))
            },
            TextIndexSnapshotData::Unknown(unknown) => Ok(WithUnknown::Unknown(unknown)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializedFragmentedTextSegment {
    pub segment_key: String,
    pub id_tracker_key: String,
    pub deleted_terms_table_key: String,
    pub alive_bitset_key: String,
    pub num_indexed_documents: u64,
    pub id: String,
}

impl TryFrom<FragmentedTextSegment> for SerializedFragmentedTextSegment {
    type Error = anyhow::Error;

    fn try_from(value: FragmentedTextSegment) -> anyhow::Result<Self> {
        Ok(Self {
            segment_key: value.segment_key.to_string(),
            id_tracker_key: value.id_tracker_key.to_string(),
            deleted_terms_table_key: value.deleted_terms_table_key.to_string(),
            alive_bitset_key: value.alive_bitset_key.to_string(),
            num_indexed_documents: value.num_indexed_documents,
            id: value.id,
        })
    }
}

impl TryFrom<SerializedFragmentedTextSegment> for FragmentedTextSegment {
    type Error = anyhow::Error;

    fn try_from(value: SerializedFragmentedTextSegment) -> Result<Self, Self::Error> {
        Ok(Self {
            segment_key: value.segment_key.try_into()?,
            id_tracker_key: value.id_tracker_key.try_into()?,
            deleted_terms_table_key: value.deleted_terms_table_key.try_into()?,
            alive_bitset_key: value.alive_bitset_key.try_into()?,
            num_indexed_documents: value.num_indexed_documents,
            id: value.id,
        })
    }
}

#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentedTextSegment {
    pub segment_key: ObjectKey,
    pub id_tracker_key: ObjectKey,
    pub deleted_terms_table_key: ObjectKey,
    pub alive_bitset_key: ObjectKey,
    // 2^63 ~= 9.2 * 10^18. We only support i64 in Convex.
    #[cfg_attr(
        any(test, feature = "testing"),
        proptest(strategy = "1u64..9223372000000000000")
    )]
    pub num_indexed_documents: u64,
    // A random UUID that can be used to identify a segment to determine if the
    // segment has changed during non-transactional index changes (compaction).
    pub id: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum TextSnapshotVersion {
    /// V0 is the original version for search snapshots.
    /// In particular, it interprets missing fields as null.
    V0,
    /// V1 interprets missing fields as undefined.
    V1MissingAsUndefined,
    /// V2 uses string IDs
    V2UseStringIds,
}

impl TextSnapshotVersion {
    pub fn new(persistence_version: PersistenceVersion) -> Self {
        // Add a new TextSnapshotVersion if the index key format changes between
        // different persistence versions.
        match persistence_version {
            PersistenceVersion::V5 => Self::V2UseStringIds,
        }
    }

    pub fn to_code(&self) -> i64 {
        match self {
            Self::V0 => 0,
            Self::V1MissingAsUndefined => 1,
            Self::V2UseStringIds => 2,
        }
    }

    pub fn from_code(code: i64) -> anyhow::Result<Self> {
        match code {
            0 => Ok(Self::V0),
            1 => Ok(Self::V1MissingAsUndefined),
            2 => Ok(Self::V2UseStringIds),
            _ => anyhow::bail!("unrecognized search snapshot version {code:?}"),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedTextIndexSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<WithUnknown<SerializedTextIndexSnapshotData>>,
    ts: i64,
    version: i64,
}

impl TryFrom<TextIndexSnapshot> for SerializedTextIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(snapshot: TextIndexSnapshot) -> Result<Self, Self::Error> {
        let (index, data) = if let TextIndexSnapshotData::SingleSegment(index) = snapshot.data {
            (Some(index.to_string()), None)
        } else {
            (None, Some(snapshot.data.try_into()?))
        };
        Ok(Self {
            index,
            data,
            ts: snapshot.ts.into(),
            version: snapshot.version.to_code(),
        })
    }
}

impl TryFrom<SerializedTextIndexSnapshot> for TextIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(serialized: SerializedTextIndexSnapshot) -> Result<Self, Self::Error> {
        let data: TextIndexSnapshotData = if let Some(index) = serialized.index {
            TextIndexSnapshotData::SingleSegment(index.try_into()?)
        } else if let Some(serialized_data) = serialized.data {
            TextIndexSnapshotData::try_from(serialized_data)?
        } else {
            anyhow::bail!("Both data and index are missing!");
        };
        Ok(Self {
            data,
            ts: serialized.ts.try_into()?,
            version: TextSnapshotVersion::from_code(serialized.version)?,
        })
    }
}

#[cfg(test)]
pub mod test {
    use must_let::must_let;
    use proptest::{
        prelude::{
            any,
            Arbitrary,
        },
        prop_compose,
        proptest,
        strategy::Strategy,
    };
    use serde::{
        Deserialize,
        Serialize,
    };
    use sync_types::Timestamp;

    use crate::{
        bootstrap_model::index::text_index::{
            index_snapshot::SerializedTextIndexSnapshot,
            TextIndexSnapshot,
            TextIndexSnapshotData,
            TextSnapshotVersion,
        },
        types::ObjectKey,
    };

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OldSerializedSearchIndexSnapshot {
        index: String,
        ts: i64,
        version: i64,
    }

    impl Arbitrary for OldSerializedSearchIndexSnapshot {
        type Parameters = ();

        type Strategy = impl Strategy<Value = OldSerializedSearchIndexSnapshot>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_compose! {
            fn inner()(
                    key in any::<ObjectKey>(),
                    ts in any::<Timestamp>(),
                    version in any::<TextSnapshotVersion>()
                ) -> OldSerializedSearchIndexSnapshot {
                    OldSerializedSearchIndexSnapshot {
                        index: key.to_string(),
                        ts: ts.into(),
                        version: version.to_code(),
                    }
                }
            }
            inner()
        }
    }

    proptest! {
        // Make sure new backends can parse the old serialization format. This can't be removed
        // until we're sure we've migrated every search index (which may never happen).
        #[test]
        fn test_parse_from_old_snapshot(snapshot in any::<OldSerializedSearchIndexSnapshot>()) {
            let serialized = serde_json::to_string(&snapshot).unwrap();
            let deserialize: SerializedTextIndexSnapshot =
                serde_json::from_str(&serialized).unwrap();
            let deserialized_snapshot =
                TextIndexSnapshot::try_from(deserialize).unwrap();
            must_let!(let TextIndexSnapshotData::SingleSegment(key) = deserialized_snapshot.data);
            assert_eq!(key, ObjectKey::try_from(snapshot.index).unwrap())
        }

        // Make sure that an old backend can parse our new index format. This can be removed once
        // we know we won't roll back to a version that doesn't recognize the new format.
        #[test]
        fn test_parse_old_snapshot_from_new(snapshot in any::<TextIndexSnapshot>()
            .prop_filter(
                "only single segment is backwards compatible",
                |snapshot| matches!(snapshot.data, TextIndexSnapshotData::SingleSegment(_))
            )
        ) {
            must_let!(let TextIndexSnapshotData::SingleSegment(ref index) = &snapshot.data);
            let index = index.clone();

            let serialized_data = SerializedTextIndexSnapshot::try_from(snapshot).unwrap();
            let serialized = serde_json::to_string(&serialized_data).unwrap();
            let deserialized: OldSerializedSearchIndexSnapshot =
                serde_json::from_str(&serialized).unwrap();
            assert_eq!(index.to_string(), deserialized.index);
        }
    }
}
