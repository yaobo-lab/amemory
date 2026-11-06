//! amemory GUI module built on Native Shell (NSH).
//! 1:1 pixel-for-pixel replica of Clippi UI styling and dimensions.

pub mod assets;
pub mod bottom_bar;
pub mod clipboard_card;
pub mod clipboard_list;
pub mod components;
pub mod context_menu;
pub mod edit_panel;
pub mod filter_bar;
pub mod font;
pub mod hover_toolbar;
pub mod quick_paste;
pub mod rich_preview;
pub mod root;
pub mod search_box;
pub mod search_highlight;
pub mod settings;
pub mod sidebar;
pub mod sidebar_window;
pub mod stats_overview;
pub mod tag_filter;
pub mod tag_picker;
pub mod theme;
pub mod titlebar;
pub mod type_filter_config;
pub mod window_manager;

pub use window_manager::run_gui;
