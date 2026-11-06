//! Core layer: pure Rust models and logic shared by the GUI and services.
//!
//! Nothing in this module depends on a UI framework or on platform APIs, so it
//! can be unit tested headlessly and reused by any frontend.

pub mod filters;
pub mod find_replace;
pub mod frontend;
pub mod i18n;
pub mod i18n_keys;
pub mod paths;
pub mod search;
pub mod settings;
pub mod types;
