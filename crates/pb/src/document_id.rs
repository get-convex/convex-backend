use value::{
    ResolvedDocumentId,
    TabletId,
    TabletIdAndTableNumber,
};

use crate::common::{
    ResolvedDocumentId as ResolvedDocumentIdProto,
    TabletIdAndTableNumber as TabletIdAndTableNumberProto,
};

impl From<ResolvedDocumentId> for ResolvedDocumentIdProto {
    fn from(value: ResolvedDocumentId) -> Self {
        let (table, internal_id) = value.into_table_and_id();
        Self {
            table: Some(table.into()),
            internal_id: Some(internal_id.0.to_vec()),
        }
    }
}

impl TryFrom<ResolvedDocumentIdProto> for ResolvedDocumentId {
    type Error = anyhow::Error;

    fn try_from(
        ResolvedDocumentIdProto { table, internal_id }: ResolvedDocumentIdProto,
    ) -> anyhow::Result<Self> {
        let table = table
            .ok_or_else(|| anyhow::anyhow!("Missing table"))?
            .try_into()?;
        let internal_id = internal_id
            .ok_or_else(|| anyhow::anyhow!("Missing internal_id"))?
            .try_into()?;
        Ok(Self::new(table, internal_id))
    }
}

impl From<TabletIdAndTableNumber> for TabletIdAndTableNumberProto {
    fn from(
        TabletIdAndTableNumber {
            table_number,
            tablet_id: table_id,
        }: TabletIdAndTableNumber,
    ) -> Self {
        Self {
            table_number: Some(table_number.into()),
            table_id: Some(table_id.0 .0.to_vec()),
        }
    }
}

impl TryFrom<TabletIdAndTableNumberProto> for TabletIdAndTableNumber {
    type Error = anyhow::Error;

    fn try_from(
        TabletIdAndTableNumberProto {
            table_id,
            table_number,
        }: TabletIdAndTableNumberProto,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            table_number: table_number
                .ok_or_else(|| anyhow::anyhow!("Missing table_number"))?
                .try_into()?,
            tablet_id: TabletId(
                table_id
                    .ok_or_else(|| anyhow::anyhow!("Missing table_id"))?
                    .try_into()?,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::ResolvedDocumentId;
    use crate::common::ResolvedDocumentId as ResolvedDocumentIdProto;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_document_id_roundtrips(left in any::<ResolvedDocumentId>()) {
            assert_roundtrips::<ResolvedDocumentId, ResolvedDocumentIdProto>(left);
        }
    }
}
