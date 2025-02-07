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
    async fn new_with_model_and_args(rt: &RT, args: DbFixturesArgs) -> anyhow::Result<Self>;
}

#[async_trait(?Send)]
impl<RT: Runtime> DbFixturesWithModel<RT> for DbFixtures<RT> {
    async fn new_with_model(rt: &RT) -> anyhow::Result<Self> {
        Self::new_with_model_and_args(
            rt,
            DbFixturesArgs {
                virtual_system_mapping: virtual_system_mapping().clone(),
                ..Default::default()
            },
        )
        .await
    }

    async fn new_with_model_and_args(rt: &RT, args: DbFixturesArgs) -> anyhow::Result<Self> {
        let fixture = Self::new_with_args(rt, args).await?;
        initialize_application_system_tables(&fixture.db).await?;
        Ok(fixture)
    }
}
