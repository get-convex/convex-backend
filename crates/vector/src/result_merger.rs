use std::{
    cmp::Reverse,
    collections::BinaryHeap,
};

use futures::TryStreamExt;

use crate::VectorSearchQueryResult;

fn push_results_to_heap(
    heap: &mut BinaryHeap<Reverse<VectorSearchQueryResult>>,
    results: Vec<VectorSearchQueryResult>,
    capacity: usize,
) {
    for result in results {
        // Store Reverse(result) in the heap so that the heap becomes a min-heap
        // instead of the default max-heap. This way, we can evict the smallest
        // element in the heap efficiently once we've reached capacity,
        // leaving us with the top K results.
        heap.push(Reverse(result));
        if heap.len() > capacity {
            heap.pop();
        }
    }
}

fn heap_to_sorted_results(
    heap: BinaryHeap<Reverse<VectorSearchQueryResult>>,
) -> Vec<VectorSearchQueryResult> {
    // BinaryHeap::into_sorted_vec returns results in ascending order of score,
    // but this is a Vec<Reverse<_>>, so the order is already descending, as
    // desired.
    heap.into_sorted_vec().into_iter().map(|r| r.0).collect()
}

/// Merges multiple vectors of VectorSearchQueryResult into top-K results
/// using a min-heap approach.
///
/// The min-heap keeps track of the K best results seen so far. When a new
/// result comes in, it's added to the heap. If the heap exceeds capacity,
/// the smallest element (lowest score) is evicted.
///
/// Results are returned in descending order by score.
pub fn merge_vector_results(
    results_iter: impl Iterator<Item = Vec<VectorSearchQueryResult>>,
    capacity: usize,
) -> Vec<VectorSearchQueryResult> {
    let mut heap: BinaryHeap<Reverse<VectorSearchQueryResult>> =
        BinaryHeap::with_capacity(capacity + 1);

    for results in results_iter {
        push_results_to_heap(&mut heap, results, capacity);
    }

    heap_to_sorted_results(heap)
}

/// Async streaming version of merge_vector_results for use with
/// futures::Stream.
///
/// This is used server-side where segments are fetched and queried as a stream.
/// It uses try_fold to accumulate results into the min-heap as they arrive.
///
/// Results are returned in descending order by score.
pub async fn merge_vector_results_stream<S, E>(
    stream: S,
    capacity: usize,
) -> Result<Vec<VectorSearchQueryResult>, E>
where
    S: futures::Stream<Item = Result<Vec<VectorSearchQueryResult>, E>>,
{
    let heap = stream
        .try_fold(
            BinaryHeap::with_capacity(capacity + 1),
            |mut acc, results| async move {
                push_results_to_heap(&mut acc, results, capacity);
                Ok(acc)
            },
        )
        .await?;

    Ok(heap_to_sorted_results(heap))
}

#[cfg(test)]
mod tests {
    use common::types::WriteTimestamp;
    use value::InternalId;

    use super::*;

    fn make_id(id: u8) -> InternalId {
        let mut bytes = [0u8; 16];
        bytes[15] = id;
        InternalId::from(bytes)
    }

    fn make_result(score: f32, id: u8) -> VectorSearchQueryResult {
        VectorSearchQueryResult {
            score,
            id: make_id(id),
            ts: WriteTimestamp::Pending,
        }
    }

    #[test]
    fn test_merge_empty() {
        let results: Vec<Vec<VectorSearchQueryResult>> = vec![];
        let merged = merge_vector_results(results.into_iter(), 10);
        assert!(merged.is_empty());
    }

    #[test]
    fn test_merge_single_batch() {
        let results = vec![vec![
            make_result(0.9, 1),
            make_result(0.7, 2),
            make_result(0.8, 3),
        ]];
        let merged = merge_vector_results(results.into_iter(), 10);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].score, 0.9);
        assert_eq!(merged[1].score, 0.8);
        assert_eq!(merged[2].score, 0.7);
    }

    #[test]
    fn test_merge_multiple_batches() {
        let results = vec![
            vec![make_result(0.9, 1), make_result(0.5, 2)],
            vec![make_result(0.8, 3), make_result(0.95, 4)],
            vec![make_result(0.7, 5)],
        ];
        let merged = merge_vector_results(results.into_iter(), 10);
        assert_eq!(merged.len(), 5);
        assert_eq!(merged[0].score, 0.95);
        assert_eq!(merged[1].score, 0.9);
        assert_eq!(merged[2].score, 0.8);
        assert_eq!(merged[3].score, 0.7);
        assert_eq!(merged[4].score, 0.5);
    }

    #[test]
    fn test_merge_with_capacity_limit() {
        let results = vec![
            vec![make_result(0.9, 1), make_result(0.5, 2)],
            vec![make_result(0.8, 3), make_result(0.95, 4)],
            vec![make_result(0.7, 5)],
        ];
        let merged = merge_vector_results(results.into_iter(), 3);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].score, 0.95);
        assert_eq!(merged[1].score, 0.9);
        assert_eq!(merged[2].score, 0.8);
    }

    #[test]
    fn test_merge_preserves_order_for_equal_scores() {
        let results = vec![vec![
            make_result(0.5, 3),
            make_result(0.5, 1),
            make_result(0.5, 2),
        ]];
        let merged = merge_vector_results(results.into_iter(), 10);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].id, make_id(3));
        assert_eq!(merged[1].id, make_id(2));
        assert_eq!(merged[2].id, make_id(1));
    }
}
