use tantivy::tokenizer::{
    LowerCaser,
    RemoveLongFilter,
    SimpleTokenizer,
    TextAnalyzer,
};

/// How many words (after stemming) can be in a text query?
pub const MAX_QUERY_TERMS: usize = 16;

/// What is the maximum length of a single text term? We will silently drop
/// terms that exceed this length.
///
/// TODO: Or should we truncate these to a prefix?
pub const MAX_TEXT_TERM_LENGTH: usize = 32;

/// What is the maximum number candidate revisions will we load into memory?
pub const MAX_CANDIDATE_REVISIONS: usize = 1024;

/// How many filter conditions can be on a query?
pub const MAX_FILTER_CONDITIONS: usize = 8;

/// Name of the Convex English tokenizer passed to Tantivy.
pub const CONVEX_EN_TOKENIZER: &str = "convex_en";

/// Max word-length in characters for exact search in typo-tolerance
pub const EXACT_SEARCH_MAX_WORD_LENGTH: usize = 4;

/// Max word-length in characters for 1-typo search in typo-tolerance
pub const SINGLE_TYPO_SEARCH_MAX_WORD_LENGTH: usize = 8;

/// The max number of term matches that can occur for each fuzzy query term
pub const MAX_FUZZY_MATCHES_PER_QUERY_TERM: usize = 16;

/// The max number of positions we return for each term matched by a query term
pub const MAX_POSITIONS_PER_MATCHED_TERM: usize = 16;

pub fn convex_en() -> TextAnalyzer {
    TextAnalyzer::from(SimpleTokenizer)
        .filter(RemoveLongFilter::limit(MAX_TEXT_TERM_LENGTH))
        .filter(LowerCaser)
}
