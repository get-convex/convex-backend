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
    persistence::RepeatablePersistence,
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

type RevisionStreamEntry =
    anyhow::Result<(Timestamp, InternalDocumentId, Option<ResolvedDocument>)>;

#[allow(clippy::needless_lifetimes)]
#[try_stream(ok = RevisionPair, error = anyhow::Error)]
pub async fn stream_revision_pairs<'a>(
    documents: impl Stream<Item = RevisionStreamEntry> + 'a,
    reader: &'a RepeatablePersistence,
) {
    let documents = documents.try_chunks(*DOCUMENTS_IN_MEMORY);
    futures::pin_mut!(documents);

    while let Some(read_chunk) = documents.try_next().await? {
        let ids = read_chunk.iter().map(|(ts, id, _)| (*id, *ts)).collect();
        let mut prev_revs = reader.previous_revisions(ids).await?;
        for (ts, id, document) in read_chunk {
            let rev = DocumentRevision { ts, document };
            let prev_rev =
                prev_revs
                    .remove((&id, &ts).as_comparator())
                    .map(|(prev_ts, prev_document)| DocumentRevision {
                        ts: prev_ts,
                        document: prev_document,
                    });
            yield RevisionPair { id, rev, prev_rev };
        }
    }
}
