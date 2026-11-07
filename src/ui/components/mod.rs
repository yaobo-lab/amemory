//! Common atomic UI components.

pub mod confirm_dialog;
pub mod modal_overlay;
pub mod shortcut_hint;
pub mod slider;
pub mod spinner;
pub mod toast;
pub mod toggle;

pub use confirm_dialog::confirm_dialog_box;
pub use modal_overlay::modal_overlay;
pub use shortcut_hint::shortcut_badge;
pub use slider::stepped_slider;
pub use spinner::loading_spinner;
pub use toast::{toast_banner, tooltip_box};
pub use toggle::toggle_switch;
