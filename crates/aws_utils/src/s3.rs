use std::fmt::Debug;

use anyhow::Context;
use aws_config::retry::RetryConfig;
use aws_sdk_s3::{
    types::Object,
    Client,
};
use futures_async_stream::try_stream;

use crate::must_config_from_env;

#[derive(Clone, Debug)]
pub struct S3Client(pub Client);

static S3_TRASH_FOLDER: &str = ".trash/";

impl S3Client {
    pub async fn new(enable_retries: bool) -> anyhow::Result<Self> {
        let retry_config = match enable_retries {
            true => RetryConfig::standard(),
            false => RetryConfig::disabled(),
        };
        let config = must_config_from_env()
            .context("AWS env variables are required when using AWS Lambda")?
            .retry_config(retry_config)
            .load()
            .await;

        let s3_client = Client::new(&config);

        Ok(Self(s3_client))
    }

    #[try_stream(ok = Vec<Object>, error = anyhow::Error)]
    pub async fn list_all_s3_files_from_bucket(&self, bucket: String) {
        let mut documents_stream = self
            .0
            .list_objects_v2()
            .bucket(bucket)
            .into_paginator()
            .send();

        tracing::debug!("Starting API calls to AWS");

        loop {
            let output = documents_stream.next().await;

            match output {
                None => break,
                Some(result) => {
                    if let Err(ref e) = result {
                        if let Some(r) = e.raw_response() {
                            tracing::error!("{r:?}");
                        }
                    }
                    let result = result.context("Error listing files")?;
                    let current_documents = result.contents.unwrap_or_default();
                    tracing::debug!(
                        "Fetched a page of {} files from AWS",
                        current_documents.len()
                    );

                    yield current_documents
                        .into_iter()
                        .filter(|obj| !obj.key.clone().unwrap().starts_with(S3_TRASH_FOLDER))
                        .collect();
                },
            }
        }

        tracing::debug!("Fetched all user file metadata from AWS");
    }

    pub async fn delete_s3_file(
        &self,
        bucket: String,
        key: String,
        version_id: Option<String>,
    ) -> anyhow::Result<()> {
        // Deleting is safe because our S3 buckets have versioning turned on and only
        // perform a soft delete when deleteObject is called
        let mut builder = self.0.delete_object().bucket(bucket).key(key.clone());
        builder = match version_id {
            Some(id) => builder.version_id(id),
            None => builder,
        };
        let result = builder.send().await;

        result.with_context(|| format!("Failed to delete S3 file with key {}", key))?;

        tracing::info!("Delete of S3 file with key {} was successful", key);

        Ok(())
    }

    pub async fn recover_s3_files_from_bucket(
        &self,
        bucket: String,
        instance_name: String,
        for_real: bool,
    ) -> anyhow::Result<()> {
        tracing::debug!("Making API call to AWS");
        let files = self
            .0
            .list_object_versions()
            .bucket(bucket.clone())
            .prefix(instance_name.clone())
            .send()
            .await?;

        tracing::info!("Retrieved files from AWS. Starting undelete");

        let mut deleted_objects = 0;
        for marker in files.delete_markers() {
            let key = marker
                .key
                .clone()
                .context("Expected delete marker to have a key")?;
            if for_real {
                self.delete_s3_file(
                    bucket.clone(),
                    key,
                    Some(
                        marker
                            .version_id
                            .clone()
                            .context("Expected delete marker to have a version id")?,
                    ),
                )
                .await?;
            } else {
                println!(
                    "DRY RUN: Would have deleted S3 delete marker with key {}",
                    key
                );
            }
            deleted_objects += 1;
        }

        tracing::info!("Recovered {deleted_objects} deleted files for instance {instance_name}");

        Ok(())
    }
}
