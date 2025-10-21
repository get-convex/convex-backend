use common::{
    run_persistence_test_suite,
    testing::persistence_test_suite,
};
use sqlite::SqlitePersistence;
use tempfile::TempDir;

run_persistence_test_suite!(
    db,
    TempDir::new()?,
    SqlitePersistence::new(
        db.path()
            .join("convex_local_backend.sqlite3")
            .to_str()
            .unwrap(),
    )?
);
