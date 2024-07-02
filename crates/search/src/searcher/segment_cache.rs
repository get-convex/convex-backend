use std::{
    ops::Deref,
    path::PathBuf,
    sync::Arc,
};

use async_lru::async_lru::{
    AsyncLru,
    SizedValue,
};
use common::{
    bounded_thread_pool::BoundedThreadPool,
    id_tracker::StaticIdTracker,
    runtime::Runtime,
};
use futures::FutureExt;
use qdrant_segment::segment::Segment;
use tantivy::Searcher;
use text_search::tracker::{
    load_alive_bitset,
    StaticDeletionTracker,
};
use vector::qdrant_segments::{
    load_disk_segment,
    UntarredVectorDiskSegmentPaths,
};

use crate::disk_index::index_reader_for_directory;

#[derive(Clone)]
struct VectorSegmentGenerator<RT: Runtime> {
    thread_pool: BoundedThreadPool<RT>,
}

pub struct SizedVectorSegment(pub Segment);

impl SizedValue for SizedVectorSegment {
    fn size(&self) -> u64 {
        1
    }
}

impl Deref for SizedVectorSegment {
    type Target = Segment;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<RT: Runtime> VectorSegmentGenerator<RT> {
    async fn generate_value(
        self,
        paths: UntarredVectorDiskSegmentPaths,
    ) -> anyhow::Result<SizedVectorSegment> {
        let segment = self
            .thread_pool
            .execute(move || load_disk_segment(paths))
            .await??;
        Ok(SizedVectorSegment(segment))
    }
}

pub(crate) struct VectorSegmentCache<RT: Runtime> {
    lru: AsyncLru<RT, UntarredVectorDiskSegmentPaths, SizedVectorSegment>,
    segment_generator: VectorSegmentGenerator<RT>,
}

impl<RT: Runtime> VectorSegmentCache<RT> {
    pub fn new(
        rt: RT,
        size: u64,
        thread_pool: BoundedThreadPool<RT>,
        max_concurrent_searches: usize,
    ) -> Self {
        Self {
            lru: AsyncLru::new(rt, size, max_concurrent_searches, "vector_segment_cache"),
            segment_generator: VectorSegmentGenerator { thread_pool },
        }
    }

    pub async fn get(
        &self,
        paths: UntarredVectorDiskSegmentPaths,
    ) -> anyhow::Result<Arc<SizedVectorSegment>> {
        self.lru
            .get(
                paths.clone(),
                self.segment_generator.clone().generate_value(paths).boxed(),
            )
            .await
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct TextDiskSegmentPaths {
    pub index_path: PathBuf,
    pub alive_bitset_path: PathBuf,
    pub deleted_terms_table_path: PathBuf,
    pub id_tracker_path: PathBuf,
}

pub enum TextSegment {
    Empty,
    Segment {
        searcher: Searcher,
        deletion_tracker: StaticDeletionTracker,
        id_tracker: StaticIdTracker,
        segment_ord: u32,
    },
}

impl SizedValue for TextSegment {
    fn size(&self) -> u64 {
        1
    }
}

#[derive(Clone)]
pub struct TextSegmentGenerator<RT: Runtime> {
    thread_pool: BoundedThreadPool<RT>,
}

impl<RT: Runtime> TextSegmentGenerator<RT> {
    async fn generate_value(
        self,
        TextDiskSegmentPaths {
            index_path,
            alive_bitset_path,
            deleted_terms_table_path,
            id_tracker_path,
        }: TextDiskSegmentPaths,
    ) -> anyhow::Result<TextSegment> {
        let load_text_segment = move || -> anyhow::Result<TextSegment> {
            let reader = index_reader_for_directory(index_path)?;
            let searcher = reader.searcher();
            if searcher.segment_readers().is_empty() {
                return Ok(TextSegment::Empty);
            }
            anyhow::ensure!(searcher.segment_readers().len() == 1);
            let alive_bitset = load_alive_bitset(&alive_bitset_path)?;
            let deletion_tracker =
                StaticDeletionTracker::load(alive_bitset, &deleted_terms_table_path)?;
            let id_tracker = StaticIdTracker::load_from_path(id_tracker_path)?;
            let text_segment_reader = TextSegment::Segment {
                searcher,
                deletion_tracker,
                id_tracker,
                segment_ord: 0,
            };
            Ok(text_segment_reader)
        };
        self.thread_pool.execute(load_text_segment).await?
    }
}

pub(crate) struct TextSegmentCache<RT: Runtime> {
    lru: AsyncLru<RT, TextDiskSegmentPaths, TextSegment>,
    text_segment_generator: TextSegmentGenerator<RT>,
}

impl<RT: Runtime> TextSegmentCache<RT> {
    pub fn new(
        rt: RT,
        size: u64,
        thread_pool: BoundedThreadPool<RT>,
        max_concurrent_searches: usize,
    ) -> Self {
        Self {
            lru: AsyncLru::new(rt, size, max_concurrent_searches, "text_segment_cache"),
            text_segment_generator: TextSegmentGenerator { thread_pool },
        }
    }

    pub async fn get(&self, paths: TextDiskSegmentPaths) -> anyhow::Result<Arc<TextSegment>> {
        self.lru
            .get(
                paths.clone(),
                self.text_segment_generator
                    .clone()
                    .generate_value(paths)
                    .boxed(),
            )
            .await
    }
}
