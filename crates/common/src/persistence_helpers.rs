use futures::{
    Stream,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use value::InternalDocumentId;

use crate::{
    comparators::AsComparator,
    document::ResolvedDocument,
    knobs::DOCUMENTS_IN_MEMORY,
    persistence::{
        DocumentLogEntry,
        RepeatablePersistence,
    },
    try_chunks::TryChunksExt,
    types::Timestamp,
};

#[derive(Debug)]
pub struct DocumentRevision {
    pub ts: Timestamp,
    pub document: Option<ResolvedDocument>,
}

#[derive(Debug)]
pub struct RevisionPair {
    pub id: InternalDocumentId,
    pub rev: DocumentRevision,
    pub prev_rev: Option<DocumentRevision>,
}

impl RevisionPair {
    pub fn ts(&self) -> Timestamp {
        self.rev.ts
    }

    pub fn document(&self) -> Option<&ResolvedDocument> {
        self.rev.document.as_ref()
    }

    pub fn prev_document(&self) -> Option<&ResolvedDocument> {
        self.prev_rev.as_ref().and_then(|r| r.document.as_ref())
    }
}

type RevisionStreamEntry = anyhow::Result<DocumentLogEntry>;

#[allow(clippy::needless_lifetimes)]
#[try_stream(ok = RevisionPair, error = anyhow::Error)]
pub async fn stream_revision_pairs<'a>(
    documents: impl Stream<Item = RevisionStreamEntry> + 'a,
    reader: &'a RepeatablePersistence,
) {
    let documents = documents.try_chunks2(*DOCUMENTS_IN_MEMORY);
    futures::pin_mut!(documents);

    while let Some(read_chunk) = documents.try_next().await? {
        // TODO: use prev_ts when it is available
        let ids = read_chunk
            .iter()
            .map(|entry| (entry.id, entry.ts))
            .collect();
        let mut prev_revs = reader.previous_revisions(ids).await?;
        for DocumentLogEntry {
            ts,
            id,
            value: document,
            ..
        } in read_chunk
        {
            let rev = DocumentRevision { ts, document };
            let prev_rev =
                prev_revs
                    .remove((&id, &ts).as_comparator())
                    .map(|entry| DocumentRevision {
                        ts: entry.ts,
                        document: entry.value,
                    });
            yield RevisionPair { id, rev, prev_rev };
        }
    }
}
