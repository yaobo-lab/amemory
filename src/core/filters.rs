//! Filter criteria for knowledge items and search queries.

pub use domain::memory::SearchMode;
use serde::{Deserialize, Serialize};

use super::types::ItemKind;

/// Active filter state for the knowledge list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterCriteria {
    /// Active category filter (`None` means All).
    pub kind: Option<ItemKind>,
    /// Search retrieval mode.
    pub search_mode: SearchMode,
    /// URI scope prefix filter (e.g. `"skill://"`, `"book://"`).
    pub scope: Option<String>,
    /// Selected tag filters.
    pub tags: Vec<String>,
    /// Show favorite items only.
    pub favorites_only: bool,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            kind: None,
            search_mode: SearchMode::Hybrid,
            scope: None,
            tags: Vec::new(),
            favorites_only: false,
        }
    }
}

impl FilterCriteria {
    /// Reset all filters to default.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether any non-default filter is active.
    #[must_use]
    pub fn has_active_filters(&self) -> bool {
        self.kind.is_some()
            || self.scope.is_some()
            || !self.tags.is_empty()
            || self.favorites_only
            || self.search_mode != SearchMode::Hybrid
    }
}
