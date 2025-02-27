use anyhow::Context as _;
use futures::{
    Stream,
    TryStreamExt,
};
use futures_async_stream::try_stream;
use value::InternalDocumentId;

use crate::{
    document::ResolvedDocument,
    knobs::DOCUMENTS_IN_MEMORY,
    persistence::{
        DocumentLogEntry,
        DocumentPrevTsQuery,
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
        let queries = read_chunk
            .iter()
            .filter_map(|entry| {
                entry.prev_ts.map(|prev_ts| DocumentPrevTsQuery {
                    id: entry.id,
                    ts: entry.ts,
                    prev_ts,
                })
            })
            .collect();
        let mut prev_revs = reader.previous_revisions_of_documents(queries).await?;
        for DocumentLogEntry {
            ts,
            prev_ts,
            id,
            value: document,
            ..
        } in read_chunk
        {
            let rev = DocumentRevision { ts, document };
            let prev_rev = prev_ts
                .map(|prev_ts| {
                    let entry = prev_revs
                        .remove(&DocumentPrevTsQuery { id, ts, prev_ts })
                        .with_context(|| format!("prev_ts is missing for {id}@{ts}: {prev_ts}"))?;
                    anyhow::Ok(DocumentRevision {
                        ts: entry.ts,
                        document: entry.value,
                    })
                })
                .transpose()?;
            yield RevisionPair { id, rev, prev_rev };
        }
    }
}
