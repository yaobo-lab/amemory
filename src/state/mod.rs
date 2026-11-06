//! Reactive state management module.

pub mod app;
pub mod server_state;

pub use app::{ActiveView, AppStore, ConfirmDialogData, ToastNotification};
pub use server_state::ServerRuntimeState;
