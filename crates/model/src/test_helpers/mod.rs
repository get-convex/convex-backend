use async_trait::async_trait;
use common::runtime::Runtime;
use database::test_helpers::{
    DbFixtures,
    DbFixturesArgs,
};

use crate::{
    initialize_application_system_tables,
    virtual_system_mapping,
};

#[async_trait(?Send)]
pub trait DbFixturesWithModel<RT: Runtime>: Sized {
    async fn new_with_model(rt: &RT) -> anyhow::Result<Self>;
    async fn with_model(self) -> anyhow::Result<Self>;
}

#[async_trait(?Send)]
impl<RT: Runtime> DbFixturesWithModel<RT> for DbFixtures<RT> {
    async fn new_with_model(rt: &RT) -> anyhow::Result<Self> {
        Self::new_with_args(
            rt,
            DbFixturesArgs {
                virtual_system_mapping: virtual_system_mapping(),
                ..Default::default()
            },
        )
        .await?
        .with_model()
        .await
    }

    async fn with_model(mut self) -> anyhow::Result<Self> {
        initialize_application_system_tables(&self.db).await?;
        Ok(self)
    }
}
