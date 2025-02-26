use std::sync::LazyLock;

use anyhow::Context;
use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        Expression,
        Order,
        Query,
    },
    runtime::Runtime,
};
use database::{
    ResolvedQuery,
    SystemMetadataModel,
    Transaction,
};
use value::{
    val,
    TableName,
    TableNamespace,
};

use self::types::AwsLambdaVersion;
use crate::{
    backend_info::BackendInfoModel,
    source_packages::types::SourcePackageId,
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static AWS_LAMBDA_VERSIONS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_aws_lambda_versions"
        .parse()
        .expect("invalid built-in aws_lambda_versions table")
});

pub struct AwsLambdaVersionsTable;
impl SystemTable for AwsLambdaVersionsTable {
    fn table_name(&self) -> &'static TableName {
        &AWS_LAMBDA_VERSIONS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<AwsLambdaVersion>::try_from(document).map(|_| ())
    }
}

pub struct AwsLambdaVersionsModel<'a, RT: Runtime> {
    lambda_name: String,
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> AwsLambdaVersionsModel<'a, RT> {
    pub fn new(lambda_name: String, tx: &'a mut Transaction<RT>) -> Self {
        Self { lambda_name, tx }
    }

    #[allow(unused)]
    pub async fn requested_provision_concurrency(&mut self) -> anyhow::Result<i32> {
        Ok(BackendInfoModel::new(self.tx)
            .get()
            .await
            .context("Get Backend Info failed")?
            .map(|bi| bi.provision_concurrency)
            .unwrap_or(0))
    }

    pub async fn mark_deployed(&mut self, new_version: AwsLambdaVersion) -> anyhow::Result<()> {
        // AWS' sentinel is "$LATEST". Make sure we never write this by accident.
        anyhow::ensure!(new_version.lambda_version != "$LATEST");
        anyhow::ensure!(new_version.lambda_version != "");
        if let Some(latest) = self.latest_version_document().await? {
            SystemMetadataModel::new_global(self.tx)
                .replace(latest.id(), new_version.try_into()?)
                .await?;
        } else {
            SystemMetadataModel::new_global(self.tx)
                .insert(&AWS_LAMBDA_VERSIONS_TABLE, new_version.try_into()?)
                .await?;
        }
        Ok(())
    }

    pub async fn latest_version(&mut self) -> anyhow::Result<Option<AwsLambdaVersion>> {
        Ok(self
            .latest_version_document()
            .await?
            .map(|v| v.into_value()))
    }

    pub async fn latest_version_document(
        &mut self,
    ) -> anyhow::Result<Option<ParsedDocument<AwsLambdaVersion>>> {
        // Do a full table scan on lambda versions table. Good enough
        // because table is small. If it were bigger, we'd want index on lambda_name
        let mut query = ResolvedQuery::new(
            self.tx,
            TableNamespace::Global,
            Query::full_table_scan(AWS_LAMBDA_VERSIONS_TABLE.clone(), Order::Desc).filter(
                Expression::field_eq_literal("lambdaName".parse()?, val!(self.lambda_name.clone())),
            ),
        )?;
        let document = query.expect_at_most_one(self.tx).await?;
        document
            .map(ParsedDocument::<AwsLambdaVersion>::try_from)
            .transpose()
    }

    /// Determines if we should route execute requests to static lambda or not.
    /// Decides by fetching the latest source package and see if it matches
    /// the current latest deployed static Lambda. If not, should route to
    /// dynamic Lambda.
    ///
    /// Requires
    /// - Lambda name this model is constructed with is that of the static
    ///   Lambda.
    pub async fn static_lambda_ready(
        &mut self,
        source_pkg_id: &SourcePackageId,
    ) -> anyhow::Result<Option<AwsLambdaVersion>> {
        // Fetch static lambda version
        let static_version = self.latest_version_document().await?;

        // Check if static version source package ID matches latest source package
        if let Some(static_version) = static_version {
            let source_package_id = static_version.package_desc.as_static()?;
            let should_use_static = source_package_id.as_ref() == Some(source_pkg_id);

            Ok(should_use_static.then(|| static_version.into_value()))
        } else {
            Ok(None)
        }
    }
}
