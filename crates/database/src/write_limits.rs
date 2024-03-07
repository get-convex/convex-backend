use value::DeveloperDocumentId;

/// Metrics related to document writes
pub struct BiggestDocumentWrites {
    pub max_size: (DeveloperDocumentId, usize),
    pub max_nesting: (DeveloperDocumentId, usize),
}
