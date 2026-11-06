//! Compile-time-safe i18n engine (ported from Clippi).
//!
//! A global atomic flag stores the current language so any code path can
//! resolve strings without threading settings through call chains.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static IS_ENGLISH: AtomicBool = AtomicBool::new(false);
static LANG_VERSION: AtomicU64 = AtomicU64::new(0);

/// Set the current language (`"en"` or anything else for Chinese).
pub fn set_language(lang: &str) {
    IS_ENGLISH.store(lang.starts_with("en"), Ordering::Relaxed);
    LANG_VERSION.fetch_add(1, Ordering::Release);
}

/// Monotonic counter bumped on every language switch.
#[inline]
#[must_use]
pub fn lang_version() -> u64 {
    LANG_VERSION.load(Ordering::Acquire)
}

/// Whether the current language is English.
#[inline]
#[must_use]
pub fn is_en() -> bool {
    IS_ENGLISH.load(Ordering::Relaxed)
}

/// Define the `I18nKey` enum with `text()` and `fmt()` methods.
#[macro_export]
macro_rules! define_i18n {
    ($($key:ident: ($zh:literal, $en:literal)),* $(,)?) => {
        /// Every user-visible string key.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[allow(missing_docs)]
        pub enum I18nKey {
            $($key,)*
        }

        impl I18nKey {
            /// Translation for the current language.
            #[must_use]
            pub fn text(self) -> &'static str {
                if $crate::core::i18n::is_en() {
                    match self { $(Self::$key => $en,)* }
                } else {
                    match self { $(Self::$key => $zh,)* }
                }
            }

            /// Format with positional placeholders `{0}`, `{1}`, ...
            #[must_use]
            pub fn fmt(self, args: &[&str]) -> String {
                let mut result = self.text().to_string();
                for (i, arg) in args.iter().enumerate() {
                    result = result.replace(&format!("{{{i}}}"), arg);
                }
                result
            }
        }
    };
}
