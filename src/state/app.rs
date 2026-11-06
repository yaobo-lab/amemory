//! Central reactive application state.

use std::sync::Arc;

use nsh_core::prelude::*;

use crate::config::AppConfig;
use crate::core::filters::FilterCriteria;
use crate::core::settings::GuiSettings;
use crate::core::types::{KnowledgeItem, StatsSnapshot};
use crate::services::amemory_runtime::AmemoryRuntime;
use crate::services::update::{UpdateInfo, UpdatePhase};

/// Primary views in the single-window interface.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    /// Knowledge and memory card list.
    List,
    /// New / Edit panel for Skills, Books, or Memory.
    Edit(Option<KnowledgeItem>),
    /// Settings panel with active tab index.
    Settings(usize),
}

/// Modal confirmation dialog payload.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmDialogData {
    /// Dialog title.
    pub title: String,
    /// Message body.
    pub message: String,
    /// Action callback identifier.
    pub action_id: String,
}

/// Floating Toast notification.
#[derive(Debug, Clone, PartialEq)]
pub struct ToastNotification {
    /// Unique ID.
    pub id: u64,
    /// Toast message text.
    pub message: String,
    /// Whether this is an error alert.
    pub is_error: bool,
}

/// Core reactive app state container.
#[derive(Clone, Copy)]
pub struct AppStore {
    /// Active view signal.
    pub active_view: Signal<ActiveView>,
    /// Search query text signal.
    pub search_query: Signal<String>,
    /// Active filter criteria.
    pub filters: Signal<FilterCriteria>,
    /// Displayed items list.
    pub items: Signal<Vec<KnowledgeItem>>,
    /// Selected item index in the list.
    pub selected_index: Signal<Option<usize>>,
    /// Live system stats.
    pub stats: Signal<StatsSnapshot>,
    /// Whether the home stats overview banner is visible.
    pub show_stats_overview: Signal<bool>,
    /// Whether the main window is pinned (always-on-top).
    pub is_pinned: Signal<bool>,
    /// Left tag sidebar open state.
    pub is_sidebar_open: Signal<bool>,
    /// Active modal confirmation dialog, if any.
    pub confirm_dialog: Signal<Option<ConfirmDialogData>>,
    /// Active toast notification, if any.
    pub active_toast: Signal<Option<ToastNotification>>,
    /// Update progress phase.
    pub update_phase: Signal<UpdatePhase>,
    /// Discovered update info.
    pub update_info: Signal<Option<UpdateInfo>>,
    /// Whether catalog sync is running.
    pub is_syncing: Signal<bool>,
}

impl AppStore {
    /// Create new reactive store with initial defaults.
    #[must_use]
    pub fn new(settings: &GuiSettings) -> Self {
        Self {
            active_view: signal(ActiveView::List),
            search_query: signal(String::new()),
            filters: signal(FilterCriteria::default()),
            items: signal(Vec::new()),
            selected_index: signal(None),
            stats: signal(StatsSnapshot::default()),
            show_stats_overview: signal(settings.show_stats_overview),
            is_pinned: signal(false),
            is_sidebar_open: signal(false),
            confirm_dialog: signal(None),
            active_toast: signal(None),
            update_phase: signal(UpdatePhase::Idle),
            update_info: signal(None),
            is_syncing: signal(false),
        }
    }

    /// Show a transient toast message.
    pub fn show_toast(&self, message: impl Into<String>, is_error: bool) {
        static NEXT_ID: Stored<u64> = stored(1);
        let id = NEXT_ID.with(|n| {
            *n += 1;
            *n
        });
        self.active_toast.set(Some(ToastNotification {
            id,
            message: message.into(),
            is_error,
        }));
    }
}
