use std::time::SystemTime;

use common::{
    document::InternalId,
    runtime::Runtime,
    value::{
        GenericDocumentId,
        TableIdentifier,
    },
};
use rand::{
    Rng,
    RngCore,
    SeedableRng,
};
use rand_chacha::ChaCha12Rng;
use value::{
    ResolvedDocumentId,
    TabletIdAndTableNumber,
};

/// A production ID generator scoped to a single transaction.
///
/// This creates InternalIds with 14 bytes of randomness followed by the day in
/// 2 bytes.
///
/// The time is pinned on construction, so only use for a single transaction!
pub struct TransactionIdGenerator {
    rng: ChaCha12Rng,
    day_bytes: [u8; 2],
}

impl TransactionIdGenerator {
    pub fn new<RT: Runtime>(runtime: &RT) -> anyhow::Result<Self> {
        let rng_seed = runtime.with_rng(|rng| rng.gen());
        let rng = ChaCha12Rng::from_seed(rng_seed);

        // Get the current day as 2 bytes.
        let duration = runtime
            .system_time()
            .duration_since(SystemTime::UNIX_EPOCH)?;
        let days = duration.as_secs() / 86400;
        let day_bytes = days.to_be_bytes();
        // First 6 bytes should always be 0 (this only works until 2149).
        anyhow::ensure!(day_bytes[..6] == [0u8; 6]);

        Ok(Self {
            rng,
            day_bytes: day_bytes[6..].try_into()?,
        })
    }

    pub fn generate_internal(&mut self) -> InternalId {
        let mut id_bytes = [0u8; 16];
        self.rng.fill_bytes(&mut id_bytes[..14]);
        id_bytes[14..].clone_from_slice(&self.day_bytes);
        InternalId(id_bytes)
    }

    pub fn generate<T: TableIdentifier>(&mut self, table_name: &T) -> GenericDocumentId<T> {
        table_name.id(self.generate_internal())
    }

    pub fn generate_resolved(
        &mut self,
        tablet_id_and_number: TabletIdAndTableNumber,
    ) -> ResolvedDocumentId {
        ResolvedDocumentId::new(
            tablet_id_and_number.tablet_id,
            tablet_id_and_number
                .table_number
                .id(self.generate_internal()),
        )
    }
}

#[cfg(test)]
mod tests {

    use std::time::SystemTime;

    use common::{
        runtime::Runtime,
        types::TableName,
    };
    use runtime::testing::TestDriver;

    use crate::transaction_id_generator::TransactionIdGenerator;

    // Make sure that our production generated IDs really do contain the day in the
    // last two bytes.
    #[test]
    fn generated_ids_include_day() -> anyhow::Result<()> {
        let td = TestDriver::new();
        let table: TableName = str::parse("table")?;

        let mut id_generator = TransactionIdGenerator::new(&td.rt())?;
        let id = id_generator.generate(&table);
        let bytes = &id.internal_id()[..];
        let mut day_bytes = [0u8; 8];
        day_bytes[6..].copy_from_slice(&bytes[14..]);
        let day_from_id = u64::from_be_bytes(day_bytes);

        let duration = td
            .rt()
            .system_time()
            .duration_since(SystemTime::UNIX_EPOCH)?;
        let day_from_system_time = (duration.as_secs()) / 86400;

        assert_eq!(day_from_id, day_from_system_time);
        Ok(())
    }
}
