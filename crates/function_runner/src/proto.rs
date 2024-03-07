use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    time::SystemTime,
};

use anyhow::Context;
use common::document::DocumentUpdate;
use pb::funrun::{
    FunrunFinalTransaction as FunrunFinalTransactionProto,
    FunrunReads as FunrunReadsProto,
    Writes as FunrunWritesProto,
};
use value::{
    ResolvedDocumentId,
    TableNumber,
};

use super::{
    FunctionFinalTransaction,
    FunctionReads,
    FunctionWrites,
};
impl TryFrom<FunctionFinalTransaction> for FunrunFinalTransactionProto {
    type Error = anyhow::Error;

    fn try_from(
        FunctionFinalTransaction {
            begin_timestamp,
            reads,
            writes,
            rows_read,
        }: FunctionFinalTransaction,
    ) -> anyhow::Result<Self> {
        let ts = SystemTime::from(begin_timestamp).into();
        Ok(Self {
            begin_timestamp: Some(ts),
            reads: Some(reads.into()),
            writes: Some(writes.try_into()?),
            rows_read: rows_read
                .into_iter()
                .map(|(table, rows_read)| Ok((table.into(), rows_read)))
                .collect::<anyhow::Result<HashMap<u32, u64>>>()?,
        })
    }
}

impl TryFrom<FunrunFinalTransactionProto> for FunctionFinalTransaction {
    type Error = anyhow::Error;

    fn try_from(
        FunrunFinalTransactionProto {
            begin_timestamp,
            reads,
            writes,
            rows_read,
        }: FunrunFinalTransactionProto,
    ) -> anyhow::Result<Self> {
        let system_timestamp: SystemTime = begin_timestamp
            .context("Missing begin_timestamp")?
            .try_into()?;
        let begin_timestamp = system_timestamp.try_into()?;
        Ok(Self {
            begin_timestamp,
            reads: reads.context("Missing reads")?.try_into()?,
            writes: writes.context("Missing writes")?.try_into()?,
            rows_read: rows_read
                .into_iter()
                .map(|(table, rows_read)| Ok((table.try_into()?, rows_read)))
                .collect::<anyhow::Result<BTreeMap<TableNumber, u64>>>()?,
        })
    }
}
impl From<FunctionReads> for FunrunReadsProto {
    fn from(
        FunctionReads {
            reads,
            num_intervals,
            user_tx_size,
            system_tx_size,
        }: FunctionReads,
    ) -> Self {
        Self {
            reads: Some(reads.into()),
            num_intervals: Some(num_intervals as u64),
            user_tx_size: Some(user_tx_size.into()),
            system_tx_size: Some(system_tx_size.into()),
        }
    }
}

impl TryFrom<FunrunReadsProto> for FunctionReads {
    type Error = anyhow::Error;

    fn try_from(
        FunrunReadsProto {
            reads,
            num_intervals,
            user_tx_size,
            system_tx_size,
        }: FunrunReadsProto,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            reads: reads
                .ok_or_else(|| anyhow::anyhow!("Missing reads"))?
                .try_into()?,
            num_intervals: num_intervals.ok_or_else(|| anyhow::anyhow!("Missing num_intervals"))?
                as usize,
            user_tx_size: user_tx_size
                .ok_or_else(|| anyhow::anyhow!("Missing user_tx_size"))?
                .try_into()?,
            system_tx_size: system_tx_size
                .ok_or_else(|| anyhow::anyhow!("Missing system_tx_size"))?
                .try_into()?,
        })
    }
}

impl TryFrom<FunctionWrites> for FunrunWritesProto {
    type Error = anyhow::Error;

    fn try_from(
        FunctionWrites {
            updates,
            generated_ids,
        }: FunctionWrites,
    ) -> anyhow::Result<Self> {
        let updates = updates
            .into_values()
            .map(|update| update.try_into())
            .try_collect()?;
        Ok(Self {
            updates,
            generated_ids: generated_ids.into_iter().map(|id| id.into()).collect(),
        })
    }
}

impl TryFrom<FunrunWritesProto> for FunctionWrites {
    type Error = anyhow::Error;

    fn try_from(
        FunrunWritesProto {
            updates,
            generated_ids,
        }: FunrunWritesProto,
    ) -> anyhow::Result<Self> {
        let updates = updates
            .into_iter()
            .map(|update| {
                let update: DocumentUpdate = update.try_into()?;
                anyhow::Ok::<(ResolvedDocumentId, DocumentUpdate)>((update.id, update))
            })
            .try_collect()?;
        let generated_ids = generated_ids
            .into_iter()
            .map(|id| id.try_into())
            .try_collect()?;
        Ok(Self {
            updates,
            generated_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use pb::funrun::FunrunFinalTransaction as FunrunFinalTransactionProto;
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use crate::FunctionFinalTransaction;

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_funrun_final_transaction_proto_roundtrips(
            left in any::<FunctionFinalTransaction>()
        ) {
            assert_roundtrips::<FunctionFinalTransaction, FunrunFinalTransactionProto>(left);
        }
    }
}
