use tantivy::{
    query::Scorer,
    DocId,
    DocSet,
    Score,
};

/// Creates a `DocSet` that iterate through the intersection of two `DocSet`s
/// using only one of them for scoring.
///
/// This is similar to tantivy's Intersection but it only uses one scorer to
/// score.
pub struct Intersection<TDocSet: DocSet> {
    left: TDocSet,
    right: TDocSet,
    score_left: bool,
}

pub fn go_to_first_doc<TDocSet: DocSet>(docsets: &mut [TDocSet]) -> DocId {
    assert!(!docsets.is_empty());
    let mut candidate = docsets.iter().map(TDocSet::doc).max().unwrap();
    'outer: loop {
        for docset in docsets.iter_mut() {
            let seek_doc = docset.seek(candidate);
            if seek_doc > candidate {
                candidate = docset.doc();
                continue 'outer;
            }
        }
        return candidate;
    }
}

impl<TDocSet: DocSet> DocSet for Intersection<TDocSet> {
    fn advance(&mut self) -> DocId {
        let (left, right) = (&mut self.left, &mut self.right);
        let mut candidate = left.advance();

        loop {
            let right_doc = right.seek(candidate);
            candidate = left.seek(right_doc);
            if candidate == right_doc {
                break;
            }
        }

        debug_assert_eq!(candidate, self.left.doc());
        debug_assert_eq!(candidate, self.right.doc());
        candidate
    }

    fn seek(&mut self, target: DocId) -> DocId {
        self.left.seek(target);
        let mut docsets: Vec<&mut dyn DocSet> = vec![&mut self.left, &mut self.right];
        let doc = go_to_first_doc(&mut docsets[..]);
        debug_assert!(docsets.iter().all(|docset| docset.doc() == doc));
        debug_assert!(doc >= target);
        doc
    }

    fn doc(&self) -> DocId {
        self.left.doc()
    }

    fn size_hint(&self) -> u32 {
        self.left.size_hint()
    }
}

impl<TScorer> Scorer for Intersection<TScorer>
where
    TScorer: Scorer,
{
    fn score(&mut self) -> Score {
        if self.score_left {
            self.left.score()
        } else {
            self.right.score()
        }
    }
}
