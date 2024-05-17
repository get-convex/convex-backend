use std::{
    ops::Deref,
    sync::Arc,
};

use async_lru::async_lru::{
    AsyncLru,
    SizedValue,
};
use common::{
    bounded_thread_pool::BoundedThreadPool,
    runtime::Runtime,
};
use futures::FutureExt;
use qdrant_segment::segment::Segment;
use vector::qdrant_segments::{
    load_disk_segment,
    UntarredVectorDiskSegmentPaths,
};

#[derive(Clone)]
struct SegmentGenerator<RT: Runtime> {
    thread_pool: BoundedThreadPool<RT>,
}

pub struct SizedSegment(pub Segment);

impl SizedValue for SizedSegment {
    fn size(&self) -> u64 {
        1
    }
}

impl Deref for SizedSegment {
    type Target = Segment;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<RT: Runtime> SegmentGenerator<RT> {
    async fn generate_value(
        self,
        paths: UntarredVectorDiskSegmentPaths,
    ) -> anyhow::Result<SizedSegment> {
        let segment = self
            .thread_pool
            .execute(move || load_disk_segment(paths))
            .await??;
        Ok(SizedSegment(segment))
    }
}

pub(crate) struct SegmentCache<RT: Runtime> {
    lru: AsyncLru<RT, UntarredVectorDiskSegmentPaths, SizedSegment>,
    segment_generator: SegmentGenerator<RT>,
}

impl<RT: Runtime> SegmentCache<RT> {
    pub fn new(
        rt: RT,
        size: u64,
        thread_pool: BoundedThreadPool<RT>,
        max_concurrent_searches: usize,
    ) -> Self {
        Self {
            lru: AsyncLru::new(rt, size, max_concurrent_searches, "segment_cache"),
            segment_generator: SegmentGenerator { thread_pool },
        }
    }

    pub async fn get(
        &self,
        paths: UntarredVectorDiskSegmentPaths,
    ) -> anyhow::Result<Arc<SizedSegment>> {
        self.lru
            .get(
                paths.clone(),
                self.segment_generator.clone().generate_value(paths).boxed(),
            )
            .await
    }
}
