//! Database client implementation for the Rust runner
//!
//! This module implements the `DatabaseClient` trait from `rust_runner`,
//! providing synchronous database access that wraps the async Convex
//! transaction API.

use std::{
    cell::RefCell,
    str::FromStr,
};

use anyhow::Context;
use common::query::{
    Order,
    Query,
};
use database::{
    bootstrap_model::user_facing::UserFacingModel,
    query::{
        DeveloperQuery,
        TableFilter,
    },
    Transaction,
};
use rust_runner::DatabaseClient;
use value::{
    id_v6::DeveloperDocumentId,
    ConvexObject,
    TableName,
    TableNamespace,
};

/// A database client that wraps a Convex transaction.
///
/// This struct implements the `DatabaseClient` trait required by the Rust
/// runner, allowing WASM functions to perform database operations. Since the
/// trait methods are synchronous but the underlying Convex database API is
/// async, we use tokio's `block_in_place` capability to bridge the gap.
///
/// The transaction is stored in a `RefCell` to allow interior mutability,
/// as the DatabaseClient trait methods take `&self` but database operations
/// require mutable access to the transaction.
pub struct TransactionDatabaseClient<RT> {
    /// The transaction wrapped in a RefCell for interior mutability
    transaction: RefCell<Transaction<RT>>,
    /// The namespace for table lookups (typically Global for root component)
    namespace: TableNamespace,
}

impl<RT> TransactionDatabaseClient<RT> {
    /// Create a new database client from a transaction.
    ///
    /// # Arguments
    /// * `transaction` - The database transaction to use for operations
    pub fn new(transaction: Transaction<RT>) -> Self {
        Self {
            transaction: RefCell::new(transaction),
            namespace: TableNamespace::root_component(),
        }
    }

    /// Create a new database client with a specific table namespace.
    ///
    /// # Arguments
    /// * `transaction` - The database transaction to use for operations
    /// * `namespace` - The table namespace to use for operations
    pub fn with_namespace(transaction: Transaction<RT>, namespace: TableNamespace) -> Self {
        Self {
            transaction: RefCell::new(transaction),
            namespace,
        }
    }

    /// Execute an async database operation using the runtime.
    ///
    /// This helper method blocks on the async operation using tokio's
    /// block_in_place.
    fn block_on<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: std::future::Future<Output = anyhow::Result<T>>,
    {
        common::runtime::block_in_place(|| tokio::runtime::Handle::current().block_on(f))
    }
}

impl<RT> DatabaseClient for TransactionDatabaseClient<RT> {
    fn query(&self, table: String) -> anyhow::Result<Vec<(String, serde_json::Value)>> {
        let table_name = TableName::from_str(&table)
            .with_context(|| format!("Invalid table name: {}", table))?;

        self.block_on(async {
            let mut tx = self.transaction.borrow_mut();

            // Create a full table scan query
            let query = Query::full_table_scan(table_name.clone(), Order::Asc);

            // Create a developer query
            let mut dev_query = DeveloperQuery::new(
                &mut *tx,
                self.namespace,
                query,
                TableFilter::ExcludePrivateSystemTables,
            )?;

            // Collect all results
            let mut results = Vec::new();
            loop {
                match dev_query.next(&mut *tx, None).await? {
                    Some(doc) => {
                        let id = doc.id().encode();
                        let value = doc.into_value().0.to_internal_json();
                        results.push((id, value));
                    },
                    None => break,
                }
            }

            Ok(results)
        })
    }

    fn get(&self, id: String) -> anyhow::Result<Option<serde_json::Value>> {
        let doc_id = DeveloperDocumentId::from_str(&id)
            .with_context(|| format!("Invalid document ID: {}", id))?;

        self.block_on(async {
            let mut tx = self.transaction.borrow_mut();
            let mut model = UserFacingModel::new(&mut *tx, self.namespace);
            let doc = model.get(doc_id, None).await?;
            Ok(doc.map(|d| d.into_value().0.to_internal_json()))
        })
    }

    fn insert(&self, table: String, value: serde_json::Value) -> anyhow::Result<String> {
        let table_name = TableName::from_str(&table)
            .with_context(|| format!("Invalid table name: {}", table))?;

        let convex_obj = ConvexObject::try_from(value)
            .context("Failed to convert JSON value to Convex object")?;

        self.block_on(async {
            let mut tx = self.transaction.borrow_mut();
            let mut model = UserFacingModel::new(&mut *tx, self.namespace);
            let doc_id = model.insert(table_name, convex_obj).await?;
            Ok(doc_id.encode())
        })
    }

    fn patch(&self, id: String, value: serde_json::Value) -> anyhow::Result<()> {
        let doc_id = DeveloperDocumentId::from_str(&id)
            .with_context(|| format!("Invalid document ID: {}", id))?;

        let convex_obj = ConvexObject::try_from(value)
            .context("Failed to convert JSON value to Convex object")?;

        // Convert the ConvexObject to a PatchValue
        let patch_value = database::PatchValue::from(convex_obj);

        self.block_on(async {
            let mut tx = self.transaction.borrow_mut();
            let mut model = UserFacingModel::new(&mut *tx, self.namespace);
            model.patch(doc_id, patch_value).await?;
            Ok(())
        })
    }

    fn delete(&self, id: String) -> anyhow::Result<()> {
        let doc_id = DeveloperDocumentId::from_str(&id)
            .with_context(|| format!("Invalid document ID: {}", id))?;

        self.block_on(async {
            let mut tx = self.transaction.borrow_mut();
            let mut model = UserFacingModel::new(&mut *tx, self.namespace);
            model.delete(doc_id).await?;
            Ok(())
        })
    }

    fn count(&self, table: String) -> anyhow::Result<u64> {
        let table_name = TableName::from_str(&table)
            .with_context(|| format!("Invalid table name: {}", table))?;

        self.block_on(async {
            let mut tx = self.transaction.borrow_mut();

            // Use UserFacingModel to count documents
            // Since there's no direct count method, we iterate and count
            let query = Query::full_table_scan(table_name.clone(), Order::Asc);
            let mut dev_query = DeveloperQuery::new(
                &mut *tx,
                self.namespace,
                query,
                TableFilter::ExcludePrivateSystemTables,
            )?;

            let mut count = 0u64;
            loop {
                match dev_query.next(&mut *tx, None).await? {
                    Some(_) => count += 1,
                    None => break,
                }
            }

            Ok(count)
        })
    }
}

#[cfg(test)]
mod tests {
    use common::testing::TestRuntime;

    use super::*;

    #[test]
    fn test_database_client_trait() {
        // This test verifies the TransactionDatabaseClient implements DatabaseClient
        fn assert_implements_trait<T: DatabaseClient>() {}
        assert_implements_trait::<TransactionDatabaseClient<TestRuntime>>();
    }
}
