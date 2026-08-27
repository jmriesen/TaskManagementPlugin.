use serde::Deserialize;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

use crate::search_result::SearchResult;

/// Tag-based filter for search results.
#[derive(Debug, Default, Clone, Deserialize)]
#[cfg_attr(feature = "wasm", wasm_bindgen(getter_with_clone))]
pub struct TagFilter {
    /// Result must have all of these tags.
    #[serde(default)]
    pub include: Vec<String>,
    /// Result must have at least one of these tags (ignored if empty).
    #[serde(default)]
    pub any: Vec<String>,
    /// Result must have none of these tags.
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl TagFilter {
    pub fn matches(&self, tags: &[String]) -> bool {
        self.include.iter().all(|t| tags.contains(t))
            && (self.any.is_empty() || self.any.iter().any(|t| tags.contains(t)))
            && self.exclude.iter().all(|t| !tags.contains(t))
    }
}

/// Apply a [`TagFilter`] to a collection of search results, e.g. the combined
/// todos from every file in a vault.
pub fn filter_todos<I: IntoIterator<Item = SearchResult>>(results: I, filter: &TagFilter) -> Vec<SearchResult> {
    results.into_iter().filter(|r| filter.matches(&r.tags)).collect()
}
