#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::Arc,
    };

    use isolate::test_helpers::TEST_SOURCE;
    use model::source_packages::upload_download::{
        download_package,
        upload_package,
    };
    use runtime::prod::ProdRuntime;
    use storage::LocalDirStorage;

    #[convex_macro::prod_rt_test]
    async fn test_upload_download_roundtrip(rt: ProdRuntime) -> anyhow::Result<()> {
        let storage = Arc::new(LocalDirStorage::new(rt)?);
        let modules: BTreeMap<_, _> = TEST_SOURCE
            .iter()
            .map(|m| (m.path.clone().canonicalize(), m))
            .collect();
        let (key, digest, _) = upload_package(modules.clone(), storage.clone(), None).await?;
        let downloaded = download_package(storage, key, digest).await?;
        let original: BTreeMap<_, _> = modules.into_iter().map(|(k, v)| (k, v.clone())).collect();
        assert_eq!(downloaded, original);

        Ok(())
    }
}
