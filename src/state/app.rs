//! Central reactive application state.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use nsh_core::prelude::*;

use crate::config::AppConfig;
use crate::core::filters::FilterCriteria;
use crate::core::settings::GuiSettings;
use crate::core::types::{KnowledgeItem, StatsSnapshot};
use crate::services::update::{UpdateInfo, UpdatePhase};

/// Primary views in the single-window interface.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
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
            .finish_non_exhaustive()
    }
}

impl PartialEq for ConfirmDialogData {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.message == other.message
    }
}

/// Floating Toast notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastNotification {
    /// Notification ID.
    pub id: u64,
    /// Notification message text.
    pub message: String,
    /// Whether this is an error notification.
    pub is_error: bool,
    /// Creation timestamp for auto-dismissal.
    pub created_at: std::time::Instant,
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
    /// Runtime application config (`etc/config.toml`).
    pub app_config: Signal<AppConfig>,
    /// Whether the tag filter & management modal is visible.
    pub show_tag_filter: Signal<bool>,
    /// Whether the category filter type config modal is visible.
    pub show_type_config: Signal<bool>,
    /// Current item whose tags are being edited in the Tag Picker modal.
    pub tag_picking_item: Signal<Option<KnowledgeItem>>,
    /// Current item whose context action menu is open.
    pub context_menu_item: Signal<Option<KnowledgeItem>>,
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
        let loaded_config = AppConfig::load(None).unwrap_or_default();
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
            app_config: signal(loaded_config),
            show_tag_filter: signal(false),
            show_type_config: signal(false),
            tag_picking_item: signal(None),
            context_menu_item: signal(None),
        }
    }

    /// Set displayed items list, enriching items with persisted favorites and custom tags.
    pub fn set_items(&self, mut items: Vec<KnowledgeItem>) {
        let settings = self.settings.get_untracked();
        for item in &mut items {
            let fav =
                settings.is_favorite(&item.id) || settings.is_favorite(&item.uri) || item.favorite;
            item.favorite = fav;
            item.is_favorite = fav;
            item.tags = settings.merged_item_tags(&item.id, &item.tags);
        }
        self.items.set(items);
    }

    /// Update GUI settings and persist to `etc/gui_settings.json`.
    pub fn update_settings(&self, f: impl FnOnce(&mut GuiSettings)) {
        let mut snapshot = None;
        self.settings.update(|s| {
            f(s);
            snapshot = Some(s.clone());
        });
        if let Some(s) = snapshot {
            crate::core::i18n::set_language(&s.language);
            let _ = s.save();
        }
    }

    /// Update application configuration and persist to `etc/config.toml`.
    pub fn update_app_config(&self, f: impl FnOnce(&mut AppConfig)) {
        let mut snapshot = None;
        self.app_config.update(|cfg| {
            f(cfg);
            snapshot = Some(cfg.clone());
        });
        if let Some(cfg) = snapshot {
            let _ = cfg.save(None);
        }
    }

    /// Toggle favorite status for an item and persist to `etc/gui_settings.json`.
    #[must_use]
    pub fn toggle_item_favorite(&self, item_id: &str) -> bool {
        let mut next_fav = false;
        self.update_settings(|s| {
            next_fav = s.toggle_favorite(item_id);
        });
        let target_id = item_id.to_string();
        self.items.update(|list| {
            if let Some(it) = list.iter_mut().find(|x| x.id == target_id) {
                it.favorite = next_fav;
                it.is_favorite = next_fav;
            }
        });
        next_fav
    }

    /// Build the current MCP HTTP endpoint URL from `app_config`.
    #[must_use]
    pub fn mcp_endpoint_url(&self) -> String {
        let cfg = self.app_config.get();
        let host = if cfg.server.host == "0.0.0.0" {
            "127.0.0.1"
        } else {
            cfg.server.host.trim()
        };
        let path = if cfg.server.endpoint_path.starts_with('/') {
            cfg.server.endpoint_path
        } else {
            format!("/{}", cfg.server.endpoint_path)
        };
        format!("http://{host}:{}{path}", cfg.server.port)
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

    /// Show a transient toast message (auto-dismisses after ~2.2 seconds).
    pub fn show_toast(&self, message: impl Into<String>) {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let msg = message.into();
        self.toast.set(Some(msg.clone()));
        self.active_toast.set(Some(ToastNotification {
            id,
            message: msg,
            is_error: false,
            created_at: std::time::Instant::now(),
        }));
    }

    /// Check and clear expired toast notifications.
    pub fn check_toast_expiry(&self) {
        let should_clear = self.active_toast.with(|t| {
            t.as_ref()
                .is_some_and(|n| n.created_at.elapsed() >= std::time::Duration::from_millis(2200))
        });
        if should_clear {
            self.toast.set(None);
            self.active_toast.set(None);
        }
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
        let s = self.settings.get();
        (s.card_height_mode.height_px() * s.font_scale.clamp(0.8, 1.4)).round()
    }

    /// Resolved reactive theme matching current user settings.
    #[must_use]
    pub fn theme(&self) -> crate::ui::theme::ClippiTheme {
        let s = self.settings.get();
        crate::ui::theme::ClippiTheme::resolve_with_accent(s.theme_mode, s.accent_color.as_deref())
    }

    /// Return translated string for the given i18n key and track language setting reactively.
    #[must_use]
    pub fn tr(&self, key: crate::core::i18n_keys::I18nKey) -> &'static str {
        let _ = self.settings.get();
        key.text()
    }
}
