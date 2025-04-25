use serde::{
    Deserialize,
    Serialize,
};
use sync_types::Timestamp;
use value::{
    serde::WithUnknown,
    ConvexObject,
};

use super::segment::{
    FragmentedVectorSegment,
    SerializedFragmentedVectorSegment,
};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct VectorIndexSnapshot {
    pub data: VectorIndexSnapshotData,
    pub ts: Timestamp,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedVectorIndexSnapshot {
    data: WithUnknown<SerializedVectorIndexSnapshotData>,
    ts: i64,
}

impl TryFrom<VectorIndexSnapshot> for SerializedVectorIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(value: VectorIndexSnapshot) -> Result<Self, Self::Error> {
        Ok(SerializedVectorIndexSnapshot {
            ts: value.ts.into(),
            data: WithUnknown::<SerializedVectorIndexSnapshotData>::try_from(value.data)?,
        })
    }
}

impl TryFrom<SerializedVectorIndexSnapshot> for VectorIndexSnapshot {
    type Error = anyhow::Error;

    fn try_from(value: SerializedVectorIndexSnapshot) -> Result<Self, Self::Error> {
        Ok(VectorIndexSnapshot {
            ts: value.ts.try_into()?,
            data: value.data.try_into()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorIndexSnapshotData {
    MultiSegment(Vec<FragmentedVectorSegment>),
    Unknown(ConvexObject),
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::{
        prelude::*,
        sample::size_range,
    };
    use value::{
        proptest::{
            RestrictNaNs,
            ValueBranching,
        },
        ConvexObject,
        ExcludeSetsAndMaps,
        FieldType,
    };

    use super::VectorIndexSnapshotData;
    use crate::bootstrap_model::index::vector_index::FragmentedVectorSegment;

    impl Arbitrary for VectorIndexSnapshotData {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                any::<Vec<FragmentedVectorSegment>>()
                    .prop_map(VectorIndexSnapshotData::MultiSegment),
                any_with::<ConvexObject>((
                    size_range(0..=4),
                    FieldType::User,
                    ValueBranching::default(),
                    ExcludeSetsAndMaps(true),
                    RestrictNaNs(false),
                ))
                .prop_map(VectorIndexSnapshotData::Unknown),
            ]
            .boxed()
        }
    }
}

impl VectorIndexSnapshotData {
    pub fn is_version_current(&self) -> bool {
        let result = matches!(self, VectorIndexSnapshotData::MultiSegment(_));
        if !result {
            tracing::warn!(
                "Vector version mismatch, stored: {:?}, current: MultiSegment",
                self,
            );
        }
        result
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "data_type", rename_all = "PascalCase")]
enum SerializedVectorIndexSnapshotData {
    MultiSegment {
        segments: Vec<SerializedFragmentedVectorSegment>,
    },
}

impl TryFrom<VectorIndexSnapshotData> for WithUnknown<SerializedVectorIndexSnapshotData> {
    type Error = anyhow::Error;

    fn try_from(value: VectorIndexSnapshotData) -> Result<Self, Self::Error> {
        match value {
            VectorIndexSnapshotData::MultiSegment(segments) => {
                let serialized_segments: Vec<SerializedFragmentedVectorSegment> = segments
                    .into_iter()
                    .map(SerializedFragmentedVectorSegment::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(WithUnknown::Known(
                    SerializedVectorIndexSnapshotData::MultiSegment {
                        segments: serialized_segments,
                    },
                ))
            },
            VectorIndexSnapshotData::Unknown(unknown) => Ok(WithUnknown::Unknown(unknown)),
        }
    }
}

impl TryFrom<WithUnknown<SerializedVectorIndexSnapshotData>> for VectorIndexSnapshotData {
    type Error = anyhow::Error;

    fn try_from(
        value: WithUnknown<SerializedVectorIndexSnapshotData>,
    ) -> Result<Self, Self::Error> {
        match value {
            WithUnknown::Known(SerializedVectorIndexSnapshotData::MultiSegment {
                segments: serialized_segments,
            }) => {
                let segments: Vec<FragmentedVectorSegment> = serialized_segments
                    .into_iter()
                    .map(FragmentedVectorSegment::try_from)
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(VectorIndexSnapshotData::MultiSegment(segments))
            },
            WithUnknown::Unknown(unknown) => Ok(VectorIndexSnapshotData::Unknown(unknown)),
        }
    }
}
