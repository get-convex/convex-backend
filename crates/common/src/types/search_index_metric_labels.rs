use std::borrow::Cow;

use crate::types::IndexId;

/// Labels describing which index/deployment a search request is acting on.
#[derive(Clone, Debug, Default)]
pub struct SearchIndexMetricLabels<'a> {
    pub index_id: Option<IndexId>,
    pub convex_deployment: Option<Cow<'a, str>>,
}

impl<'a> SearchIndexMetricLabels<'a> {
    pub fn new(
        index_id: Option<IndexId>,
        convex_deployment: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            index_id,
            convex_deployment: convex_deployment.map(Into::into),
        }
    }

    pub const fn unknown() -> Self {
        Self {
            index_id: None,
            convex_deployment: None,
        }
    }

    pub fn index_id(&self) -> Option<IndexId> {
        self.index_id
    }

    pub fn convex_deployment(&self) -> Option<&str> {
        self.convex_deployment.as_deref()
    }

    pub fn to_owned(&self) -> SearchIndexMetricLabels<'static> {
        SearchIndexMetricLabels {
            index_id: self.index_id,
            convex_deployment: self
                .convex_deployment
                .as_ref()
                .map(|deployment| Cow::Owned(deployment.to_string())),
        }
    }
}
