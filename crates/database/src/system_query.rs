//! Provides a simplified, more-typed API for querying system metadata tables
//! (as compared to [`common::query::Query`] / [`crate::query::ResolvedQuery`]).

use std::{
    mem,
    ops::{
        Bound,
        RangeBounds,
    },
    sync::Arc,
};

use anyhow::Context as _;
use common::{
    document::{
        ParseDocument as _,
        ParsedDocument,
    },
    interval::{
        BinaryKey,
        End,
        Interval,
        StartIncluded,
    },
    knobs::DEFAULT_QUERY_PREFETCH,
    query::Order,
    runtime::Runtime,
};
use indexing::backend_in_memory_indexes::{
    LazyDocument,
    RangeRequest,
};
use value::{
    serde::ConvexSerializable,
    sorting::write_sort_key,
    walk::ConvexValueWalker,
    DeveloperDocumentId,
    TableNamespace,
    TabletId,
};

use crate::{
    system_tables::{
        SystemIndex,
        SystemTable,
        SystemTableMetadata,
    },
    Transaction,
};

pub struct EqFields {
    prefix: Vec<u8>,
    fields: usize,
}
pub struct Unlimited;

pub struct SystemQueryBuilder<'a, 'b, RT: Runtime, T: SystemTable, R> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
    index: &'b SystemIndex<T>,
    tablet_id: TabletId,
    index_range: R,
    order: Order,
}

pub struct SystemQuery<'a, 'b, RT: Runtime, T: SystemTable> {
    tx: &'a mut Transaction<RT>,
    index: &'b SystemIndex<T>,
    tablet_id: TabletId,
    index_range: Interval,
    order: Order,
}

impl<RT: Runtime> Transaction<RT> {
    /// Start building a query over `index`.
    /// The query will initially range over the entire index. To narrow the
    /// range, use the `.eq()` and `.range()` builder methods.
    pub fn query_system<'a, 'b, T: SystemTable>(
        &'a mut self,
        namespace: TableNamespace,
        index: &'b SystemIndex<T>,
    ) -> anyhow::Result<SystemQueryBuilder<'a, 'b, RT, T, EqFields>>
    where
        T::Metadata: ConvexSerializable,
    {
        // TODO: this should possibly return an empty vector?
        let tablet_id = self
            .table_mapping()
            .namespace(namespace)
            .id_if_exists(T::table_name())
            .with_context(|| format!("Index {:?} not present in {namespace:?}", index.name()))?;
        Ok(SystemQueryBuilder {
            tx: self,
            namespace,
            index,
            tablet_id,
            index_range: EqFields {
                prefix: vec![],
                fields: 0,
            },
            order: Order::Asc,
        })
    }
}

impl<'a, 'b, RT: Runtime, T: SystemTable> SystemQueryBuilder<'a, 'b, RT, T, EqFields> {
    /// Specify equality constraints for the next fields in the index.
    /// This will return an error if the total number of `.eq()` fields exceeds
    /// the number of indexed fields.
    ///
    /// This is not type-safe with respect to the actual field types - providing
    /// an eq() constraint of the wrong type will find nothing.
    pub fn eq<F: ConvexValueWalker + Copy>(mut self, fields: &[F]) -> anyhow::Result<Self> {
        let indexed_fields_len = self.index.fields.len() + 1; // include _id
        self.index_range.push(fields, indexed_fields_len)?;
        Ok(self)
    }

    /// Apply a one- or two-sided inequality to the index range.
    ///
    /// The inequality is applied to the same number of index fields as the
    /// number of values provided. For example, if the next fields in the index
    /// are `["nextTs", "_creationTime", "_id"]`, then calling
    /// `query.range(..=[a, b])` ranges over `(nextTs, _creationTime) <= (a,
    /// b)` and ignores `_id`.
    pub fn range<F: ConvexValueWalker + Copy, A: AsRef<[F]>>(
        self,
        range: impl RangeBounds<A>,
    ) -> anyhow::Result<SystemQueryBuilder<'a, 'b, RT, T, Interval>> {
        let SystemQueryBuilder {
            tx,
            namespace,
            index,
            tablet_id,
            index_range,
            order,
        } = self;
        let indexed_fields_len = self.index.fields.len() + 1; // include _id
        let index_range = index_range.range(
            range.start_bound().map(AsRef::as_ref),
            range.end_bound().map(AsRef::as_ref),
            indexed_fields_len,
        )?;
        Ok(SystemQueryBuilder {
            tx,
            namespace,
            index,
            tablet_id,
            index_range,
            order,
        })
    }

    /// Sets the iteration order of the query. The query is ascending by
    /// default.
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }
}

impl<'a, 'b, RT: Runtime, T: SystemTable, R: Into<Interval>> SystemQueryBuilder<'a, 'b, RT, T, R> {
    /// Builds the query so that it can be iterated one page at a time with
    /// [`SystemQuery::next_page`].
    pub fn build(self) -> SystemQuery<'a, 'b, RT, T> {
        let Self {
            tx,
            namespace: _,
            index,
            tablet_id,
            index_range,
            order,
        } = self;
        SystemQuery {
            tx,
            index,
            tablet_id,
            index_range: index_range.into(),
            order,
        }
    }

    /// Returns all the documents in the index range.
    pub async fn all(self) -> anyhow::Result<Vec<Arc<ParsedDocument<T::Metadata>>>>
    where
        T::Metadata: ConvexSerializable,
    {
        let mut q = self.build();
        let mut docs = vec![];
        loop {
            let (page, has_more) = q.next_page(*DEFAULT_QUERY_PREFETCH).await?;
            docs.extend(page);
            if !has_more {
                break;
            }
        }
        Ok(docs)
    }

    /// Returns the unique document, if any, in the range. Raises an error if
    /// more than one document is found.
    pub async fn unique(self) -> anyhow::Result<Option<Arc<ParsedDocument<T::Metadata>>>>
    where
        T::Metadata: ConvexSerializable,
    {
        let mut q = self.build();
        let (mut docs, _has_more) = q.next_page(2).await?;
        anyhow::ensure!(
            docs.len() <= 1,
            "expected at most 1 document from index {:?}, got {}",
            q.index.name(),
            docs.len()
        );
        Ok(docs.pop())
    }
}

impl<RT: Runtime, T: SystemTable> SystemQuery<'_, '_, RT, T> {
    /// Returns the next (up to) `n` documents and a boolean indicating if more
    /// results are expected. Note that this could return `true` even if there
    /// aren't actually any more documents, if there's still an unexplored index
    /// range that happens to have no documents in it.
    pub async fn next_page(
        &mut self,
        n: usize,
    ) -> anyhow::Result<(Vec<Arc<ParsedDocument<T::Metadata>>>, bool)>
    where
        T::Metadata: ConvexSerializable,
    {
        if self.index_range.is_empty() {
            return Ok((vec![], false));
        }

        let Ok(tablet_index_id) = self
            .index
            .name
            .clone()
            .map_table(&|_| Ok::<_, !>(self.tablet_id));

        let request = RangeRequest {
            index_name: tablet_index_id,
            printable_index_name: self.index.name(),
            order: self.order,
            interval: mem::replace(&mut self.index_range, Interval::empty()),
            max_size: n,
        };
        let [result] = self
            .tx
            .index
            .range_no_deps(&[&request])
            .await
            .try_into()
            .map_err(|_| anyhow::anyhow!("returned wrong batch size"))?;
        let (page, cursor) = result?;
        let (page_range, remaining) = request.interval.split(cursor, self.order);
        self.index_range = remaining;
        self.tx.reads.record_indexed_directly(
            request.index_name,
            self.index.fields.clone(),
            page_range,
        )?;
        for (_, doc, _) in &page {
            // NOTE: since this is a system read, we don't bother tracking usage;
            // we only update `system_tx_size` for stats
            self.tx
                .reads
                .record_read_system_document(doc.approximate_size());
        }

        Ok((
            page.into_iter()
                .map(|(_index_key, doc, _ts)| {
                    match doc {
                        LazyDocument::Resolved(doc) => SystemTableMetadata::parse_from_doc(doc),
                        LazyDocument::Packed(doc) => doc.parse(),
                    }
                    .map(Arc::new)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            !self.index_range.is_empty(),
        ))
    }
}

impl<RT: Runtime> Transaction<RT> {
    /// Queries the document with the given `id` from table `T`.
    ///
    /// Note that if `id` actually belongs to a different table, this will
    /// return `Ok(None)` (and not raise any error).
    pub async fn get_system<T: SystemTable>(
        &mut self,
        namespace: TableNamespace,
        id: DeveloperDocumentId,
    ) -> anyhow::Result<Option<Arc<ParsedDocument<T::Metadata>>>>
    where
        T::Metadata: ConvexSerializable,
    {
        self.query_system(namespace, &SystemIndex::<T>::by_id())?
            .eq(&[id.encode_into(&mut Default::default())])?
            .unique()
            .await
    }
}

impl EqFields {
    fn push<F: ConvexValueWalker + Copy>(
        &mut self,
        fields: &[F],
        indexed_fields_len: usize,
    ) -> anyhow::Result<()> {
        for &value in fields {
            write_sort_key(value, &mut self.prefix).map_err(Into::into)?;
            self.fields += 1;
        }
        // Sanity checks against developer errors
        anyhow::ensure!(
            self.fields <= indexed_fields_len,
            "invalid system query: provided {} eq() fields but index only has {} fields",
            self.fields,
            indexed_fields_len
        );
        Ok(())
    }

    fn range<F: ConvexValueWalker + Copy>(
        self,
        lower_bound: Bound<&[F]>,
        upper_bound: Bound<&[F]>,
        indexed_fields_len: usize,
    ) -> anyhow::Result<Interval> {
        for bound in [lower_bound, lower_bound] {
            if let Bound::Excluded(b) | Bound::Included(b) = bound {
                anyhow::ensure!(
                    self.fields + b.len() <= indexed_fields_len,
                    "invalid system query: provided {} eq() fields and {} bound fields but index \
                     only has {} fields",
                    self.fields,
                    b.len(),
                    indexed_fields_len
                );
            }
        }

        let start = match lower_bound {
            bound @ (Bound::Included(fields) | Bound::Excluded(fields)) => {
                let mut start = self.prefix.clone();
                for &value in fields {
                    write_sort_key(value, &mut start).map_err(Into::into)?;
                }
                if let Bound::Excluded(_) = bound {
                    let Some(key) = BinaryKey::from(start).increment() else {
                        return Ok(Interval::empty());
                    };
                    key
                } else {
                    BinaryKey::from(start)
                }
            },
            Bound::Unbounded => BinaryKey::from(self.prefix.clone()),
        };

        let end = match upper_bound {
            bound @ (Bound::Included(fields) | Bound::Excluded(fields)) => {
                let mut end = self.prefix;
                for &value in fields {
                    write_sort_key(value, &mut end).map_err(Into::into)?;
                }
                if let Bound::Included(_) = bound {
                    End::after_prefix(&end.into())
                } else {
                    End::Excluded(end.into())
                }
            },
            Bound::Unbounded => End::after_prefix(&self.prefix.into()),
        };

        Ok(Interval {
            start: StartIncluded(start),
            end,
        })
    }
}

impl From<EqFields> for Interval {
    fn from(value: EqFields) -> Self {
        Interval::prefix(value.prefix.into())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::Debug,
        ops::Bound,
        sync::{
            Arc,
            LazyLock,
        },
    };

    use common::{
        document::ParsedDocument,
        interval::{
            BinaryKey,
            End,
            Interval,
            IntervalSet,
            StartIncluded,
        },
        testing::assert_contains,
    };
    use proptest_derive::Arbitrary;
    use runtime::testing::TestRuntime;
    use serde::{
        Deserialize,
        Serialize,
    };
    use value::{
        assert_obj,
        codegen_convex_serialization,
        values_to_bytes,
        ConvexValue,
        DeveloperDocumentId,
        ResolvedDocumentId,
        TableName,
        TableNamespace,
    };

    use crate::{
        system_tables::{
            SystemIndex,
            SystemTable,
        },
        test_helpers::DbFixtures,
        Database,
        TestFacingModel,
        Transaction,
    };

    static TEST_TABLE_NAME: LazyLock<TableName> = LazyLock::new(|| "test".parse().unwrap());
    static TEST_INDEX: LazyLock<SystemIndex<TestTable>> = LazyLock::new(|| {
        SystemIndex::new("by_a_b", [&"a".parse().unwrap(), &"b".parse().unwrap()]).unwrap()
    });
    struct TestTable;
    impl SystemTable for TestTable {
        type Metadata = TestMetadata;

        fn table_name() -> &'static TableName {
            &TEST_TABLE_NAME
        }

        fn indexes() -> Vec<SystemIndex<Self>> {
            vec![TEST_INDEX.clone()]
        }
    }
    #[derive(Serialize, Deserialize, Clone, Arbitrary, Debug, PartialEq)]
    struct TestMetadata {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        a: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        b: Option<i64>,
    }
    codegen_convex_serialization!(TestMetadata, TestMetadata);

    async fn setup_db(rt: &TestRuntime) -> anyhow::Result<Database<TestRuntime>> {
        let DbFixtures { db, tp, .. } = DbFixtures::new(rt).await?;
        db.create_backfilled_index_for_test(
            tp,
            TableNamespace::test_user(),
            TEST_INDEX.name(),
            TEST_INDEX.fields.clone(),
        )
        .await?;
        Ok(db)
    }

    fn index_reads<T: SystemTable>(
        tx: &Transaction<TestRuntime>,
        test_index: &SystemIndex<T>,
        namespace: TableNamespace,
    ) -> IntervalSet {
        let name = test_index
            .name()
            .map_table(
                &tx.metadata
                    .table_mapping()
                    .namespace(namespace)
                    .name_to_tablet(),
            )
            .unwrap();
        tx.reads.read_set().index_reads_for_test(&name)
    }

    fn i(start: BinaryKey, end: End) -> Interval {
        Interval {
            start: StartIncluded(start),
            end,
        }
    }

    fn intervals<const N: usize>(intervals: [Interval; N]) -> IntervalSet {
        let mut set = IntervalSet::new();
        for interval in intervals {
            set.add(interval);
        }
        set
    }

    fn k<T: TryInto<ConvexValue, Error: Debug>, const N: usize>(values: [T; N]) -> BinaryKey {
        BinaryKey::from(values_to_bytes(
            &values
                .into_iter()
                .map(|v| Some(v.try_into().unwrap()))
                .collect::<Vec<_>>(),
        ))
    }

    #[convex_macro::test_runtime]
    async fn test_system_query(rt: TestRuntime) -> anyhow::Result<()> {
        let db = setup_db(&rt).await?;
        let namespace = TableNamespace::test_user();
        let mut tx = db.begin_system().await?;
        let doc1 = TestFacingModel::new(&mut tx)
            .insert_and_get(TEST_TABLE_NAME.clone(), assert_obj! {})
            .await?;
        let doc2 = TestFacingModel::new(&mut tx)
            .insert_and_get(TEST_TABLE_NAME.clone(), assert_obj! { "a" => 1i64, })
            .await?;
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let docs = tx.query_system(namespace, &TEST_INDEX)?.all().await?;
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id(), doc1.id());
        assert_eq!(**docs[0], TestMetadata { a: None, b: None });
        assert_eq!(docs[1].id(), doc2.id());
        assert_eq!(
            **docs[1],
            TestMetadata {
                a: Some(1),
                b: None,
            }
        );
        assert_eq!(index_reads(&tx, &TEST_INDEX, namespace), IntervalSet::All);

        // Step through a query one page at a time
        let creation_time_index = SystemIndex::<TestTable>::by_creation_time();
        let mut query = tx.query_system(namespace, &creation_time_index)?.build();
        let (page, has_more) = query.next_page(1).await?;
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id(), doc1.id());
        assert!(has_more);
        // Check that the read set is updated incrementally for each document read
        assert_eq!(
            index_reads(query.tx, &creation_time_index, namespace),
            intervals([i(
                BinaryKey::min(),
                End::after_prefix(&k::<ConvexValue, 2>([
                    doc1.creation_time().into(),
                    doc1.developer_id().encode().try_into().unwrap()
                ]))
            )])
        );
        let (page, has_more) = query.next_page(1).await?;
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id(), doc2.id());
        assert!(has_more);
        assert_eq!(
            index_reads(query.tx, &creation_time_index, namespace),
            intervals([i(
                BinaryKey::min(),
                End::after_prefix(&k::<ConvexValue, 2>([
                    doc2.creation_time().into(),
                    doc2.developer_id().encode().try_into().unwrap()
                ]))
            )])
        );
        let (page, has_more) = query.next_page(1).await?;
        assert_eq!(page, vec![]);
        assert!(!has_more);
        assert_eq!(
            index_reads(&tx, &creation_time_index, namespace),
            IntervalSet::All
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_system_query_eq(rt: TestRuntime) -> anyhow::Result<()> {
        let db = setup_db(&rt).await?;
        let namespace = TableNamespace::test_user();
        let mut tx = db.begin_system().await?;
        let doc = TestFacingModel::new(&mut tx)
            .insert_and_get(
                TEST_TABLE_NAME.clone(),
                assert_obj! { "a" => 1i64, "b" => 2i64 },
            )
            .await?;
        db.commit(tx).await?;

        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .eq(&[1i64, 2])?
            .unique()
            .await?;
        assert_eq!(queried.as_ref().unwrap().id(), doc.id());
        assert_eq!(
            index_reads(&tx, &TEST_INDEX, namespace),
            intervals([Interval::prefix(k([1i64, 2i64]))])
        );
        assert_eq!(
            tx.query_system(namespace, &TEST_INDEX)?
                .eq(&[1i64])?
                .unique()
                .await?,
            queried
        );
        assert_eq!(
            tx.query_system(namespace, &TEST_INDEX)?
                .eq(&[2i64])?
                .unique()
                .await?,
            None
        );
        // Too many fields provided for the index
        assert_contains(
            &tx.query_system(namespace, &TEST_INDEX)?
                .eq(&[1i64; 5])
                .err()
                .unwrap(),
            "invalid system query",
        );
        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_range(rt: TestRuntime) -> anyhow::Result<()> {
        let db = setup_db(&rt).await?;
        let namespace = TableNamespace::test_user();
        let mut tx = db.begin_system().await?;
        let doc1 = TestFacingModel::new(&mut tx)
            .insert_and_get(
                TEST_TABLE_NAME.clone(),
                assert_obj! { "a" => 1i64, "b" => 2i64 },
            )
            .await?;
        let doc2 = TestFacingModel::new(&mut tx)
            .insert_and_get(
                TEST_TABLE_NAME.clone(),
                assert_obj! { "a" => 1i64, "b" => 3i64 },
            )
            .await?;
        let doc3 = TestFacingModel::new(&mut tx)
            .insert_and_get(
                TEST_TABLE_NAME.clone(),
                assert_obj! { "a" => 2i64, "b" => 1i64 },
            )
            .await?;
        db.commit(tx).await?;

        fn ids(docs: Vec<Arc<ParsedDocument<TestMetadata>>>) -> Vec<ResolvedDocumentId> {
            docs.into_iter().map(|d| d.id()).collect()
        }

        // half-exclusive range on `a`
        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .range([1i64]..[2i64])?
            .all()
            .await?;
        assert_eq!(ids(queried), [doc1.id(), doc2.id()]);
        assert_eq!(
            index_reads(&tx, &TEST_INDEX, namespace),
            intervals([i(k([1i64]), End::Excluded(k([2i64])))])
        );
        drop(tx);

        // inclusive range on `a`
        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .range([1i64]..=[2i64])?
            .all()
            .await?;
        assert_eq!(ids(queried), [doc1.id(), doc2.id(), doc3.id()]);
        assert_eq!(
            index_reads(&tx, &TEST_INDEX, namespace),
            intervals([i(k([1i64]), End::after_prefix(&k([2i64])))])
        );
        drop(tx);

        // range bounds do not have to be the same length
        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .range([1i64, 3].as_slice()..=[2i64].as_slice())?
            .all()
            .await?;
        assert_eq!(ids(queried), [doc2.id(), doc3.id()]);
        assert_eq!(
            index_reads(&tx, &TEST_INDEX, namespace),
            intervals([i(k([1i64, 3i64]), End::after_prefix(&k([2i64])))])
        );
        drop(tx);

        // can use an exclusive start bound (with ugly syntax)
        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .range((Bound::Excluded([1i64]), Bound::Unbounded))?
            .all()
            .await?;
        assert_eq!(ids(queried), [doc3.id()]);
        assert_eq!(
            index_reads(&tx, &TEST_INDEX, namespace),
            intervals([i(k([1i64]).increment().unwrap(), End::Unbounded)])
        );
        drop(tx);

        // an empty range works and returns nothing
        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .range([1i64]..[1i64])?
            .all()
            .await?;
        assert_eq!(queried, []);
        assert_eq!(index_reads(&tx, &TEST_INDEX, namespace), IntervalSet::new());
        drop(tx);

        // can use .eq() then .range()
        let mut tx = db.begin_system().await?;
        let queried = tx
            .query_system(namespace, &TEST_INDEX)?
            .eq(&[1i64])?
            .range([2i64]..[3i64])?
            .all()
            .await?;
        assert_eq!(ids(queried), [doc1.id()]);
        // this is the same as [1, 2]..[1, 3]
        assert_eq!(
            index_reads(&tx, &TEST_INDEX, namespace),
            intervals([i(k([1i64, 2]), End::Excluded(k([1i64, 3])))])
        );
        drop(tx);

        // too many fields in range() returns an error
        let mut tx = db.begin_system().await?;
        assert_contains(
            &tx.query_system(namespace, &TEST_INDEX)?
                .eq(&[1i64])?
                .range([2i64; 4]..)
                .err()
                .unwrap(),
            "invalid system query",
        );
        drop(tx);

        Ok(())
    }

    #[convex_macro::test_runtime]
    async fn test_get_system(rt: TestRuntime) -> anyhow::Result<()> {
        let db = setup_db(&rt).await?;
        let namespace = TableNamespace::test_user();
        let mut tx = db.begin_system().await?;
        let doc = TestFacingModel::new(&mut tx)
            .insert_and_get(TEST_TABLE_NAME.clone(), assert_obj! {})
            .await?;
        db.commit(tx).await?;
        let mut tx = db.begin_system().await?;
        assert_eq!(
            tx.get_system::<TestTable>(namespace, doc.developer_id())
                .await?
                .unwrap()
                .id(),
            doc.id()
        );
        assert_eq!(
            tx.get_system::<TestTable>(namespace, DeveloperDocumentId::MIN)
                .await?,
            None
        );
        let doc_key = k([doc.developer_id().encode()]);
        let missing_key = k([DeveloperDocumentId::MIN.encode()]);
        assert_eq!(
            index_reads(&tx, &SystemIndex::<TestTable>::by_id(), namespace),
            intervals([Interval::prefix(missing_key), Interval::prefix(doc_key),])
        );

        Ok(())
    }
}
