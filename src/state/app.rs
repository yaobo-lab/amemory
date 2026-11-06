//! Central reactive application state.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use nsh_core::prelude::*;

use crate::core::filters::FilterCriteria;
use crate::core::settings::GuiSettings;
use crate::core::types::{KnowledgeItem, StatsSnapshot};
use crate::services::update::{UpdateInfo, UpdatePhase};

/// Primary views in the single-window interface.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    /// Knowledge card list view.
    ClipboardList,
    /// Alias for card list view.
    List,
    /// New / Edit panel for Skills, Books, or Memory.
    Edit,
    /// Edit with specific item payload.
    EditItem(Option<KnowledgeItem>),
    /// Settings panel with active tab index.
    Settings(usize),
}

/// Modal confirmation dialog payload.
#[derive(Clone)]
pub struct ConfirmDialogData {
    /// Dialog title.
    pub title: String,
    /// Dialog description/message text.
    pub message: String,
    /// Action invoked when confirmed.
    pub on_confirm: Arc<dyn Fn() + Send + Sync>,
}

impl std::fmt::Debug for ConfirmDialogData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmDialogData")
            .field("title", &self.title)
            .field("message", &self.message)
            .finish()
    }
}

impl PartialEq for ConfirmDialogData {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.message == other.message
    }
}

/// Floating Toast notification.
#[derive(Debug, Clone, PartialEq)]
pub struct ToastNotification {
    /// Notification ID.
    pub id: u64,
    /// Notification message text.
    pub message: String,
    /// Whether this is an error notification.
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
    /// Toast message string signal for simple toast components.
    pub toast: Signal<Option<String>>,
    /// Update progress phase.
    pub update_phase: Signal<UpdatePhase>,
    /// Discovered update info.
    pub update_available: Signal<Option<UpdateInfo>>,
    /// Whether catalog sync is running.
    pub is_syncing: Signal<bool>,
    /// Current item being edited, if any.
    pub editing_item: Signal<Option<KnowledgeItem>>,
    /// Persisted GUI settings.
    pub settings: Signal<GuiSettings>,
}

impl Default for AppStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AppStore {
    /// Create new reactive store with initial defaults.
    #[must_use]
    pub fn new() -> Self {
        let loaded_settings = GuiSettings::load();
        Self {
            active_view: signal(ActiveView::ClipboardList),
            search_query: signal(String::new()),
            filters: signal(FilterCriteria::default()),
            items: signal(Vec::new()),
            selected_index: signal(None),
            stats: signal(StatsSnapshot::default()),
            show_stats_overview: signal(loaded_settings.show_stats_overview),
            is_pinned: signal(false),
            is_sidebar_open: signal(false),
            confirm_dialog: signal(None),
            active_toast: signal(None),
            toast: signal(None),
            update_phase: signal(UpdatePhase::Idle),
            update_available: signal(None),
            is_syncing: signal(false),
            editing_item: signal(None),
            settings: signal(loaded_settings),
        }
    }

    /// Set displayed items list.
    pub fn set_items(&self, items: Vec<KnowledgeItem>) {
        self.items.set(items);
    }

    /// Set system stats snapshot.
    pub fn set_stats(&self, stats: StatsSnapshot) {
        self.stats.set(stats);
    }

    /// Set syncing indicator flag.
    pub fn set_syncing(&self, syncing: bool) {
        self.is_syncing.set(syncing);
    }

    /// Set update phase.
    pub fn set_update_phase(&self, phase: UpdatePhase) {
        self.update_phase.set(phase);
    }

    /// Set discovered update info.
    pub fn set_update_available(&self, info: Option<UpdateInfo>) {
        self.update_available.set(info);
    }

    /// Open editor to compose a new item.
    pub fn open_create_item(&self) {
        self.editing_item.set(None);
        self.active_view.set(ActiveView::Edit);
    }

    /// Open editor for an existing knowledge item.
    pub fn open_edit_item(&self, item: KnowledgeItem) {
        self.editing_item.set(Some(item));
        self.active_view.set(ActiveView::Edit);
    }

    /// Show a transient toast message.
    pub fn show_toast(&self, message: impl Into<String>) {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let msg = message.into();
        self.toast.set(Some(msg.clone()));
        self.active_toast.set(Some(ToastNotification {
            id,
            message: msg,
            is_error: false,
        }));
    }

    /// Show confirmation dialog.
    pub fn show_confirm_dialog(
        &self,
        title: impl Into<String>,
        message: impl Into<String>,
        on_confirm: impl Fn() + Send + Sync + 'static,
    ) {
        self.confirm_dialog.set(Some(ConfirmDialogData {
            title: title.into(),
            message: message.into(),
            on_confirm: Arc::new(on_confirm),
        }));
    }

    /// Dismiss active confirmation dialog.
    pub fn dismiss_confirm_dialog(&self) {
        self.confirm_dialog.set(None);
    }

    /// Height in logical pixels for card rows in the knowledge list.
    #[must_use]
    pub fn card_height(&self) -> f32 {
        self.settings.get().card_height_mode.height_px()
    }
}
