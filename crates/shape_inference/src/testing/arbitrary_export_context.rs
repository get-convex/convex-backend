use proptest::prelude::*;
use value::{
    id_v6::DeveloperDocumentId,
    FieldName,
};

use crate::{
    export_context::{
        ExportContext,
        GeneratedSchema,
    },
    ShapeConfig,
    StructuralShape,
};

impl Arbitrary for ExportContext {
    type Parameters = ();

    type Strategy = impl Strategy<Value = Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        let leaf = prop_oneof![
            Just(ExportContext::Infer),
            Just(ExportContext::Int64),
            (any::<[u8; 8]>()).prop_map(|nan_le_bytes| ExportContext::Float64NaN { nan_le_bytes }),
            Just(ExportContext::Float64Inf),
            Just(ExportContext::Bytes),
            Just(ExportContext::Set),
            Just(ExportContext::Map),
        ];
        leaf.prop_recursive(2, 8, 4, move |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..=4).prop_map(ExportContext::Array),
                prop::collection::btree_map(any::<FieldName>(), inner.clone(), 0..4)
                    .prop_map(ExportContext::Object),
            ]
        })
    }
}

impl<T: ShapeConfig> Arbitrary for GeneratedSchema<T> {
    type Parameters = ();

    type Strategy = impl Strategy<Value = Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        (
            any::<StructuralShape<T>>(),
            prop::collection::btree_map(
                any::<DeveloperDocumentId>(),
                any::<ExportContext>(),
                0..10,
            ),
        )
            .prop_map(|(inferred_shape, overrides)| Self {
                inferred_shape,
                overrides,
            })
    }
}
