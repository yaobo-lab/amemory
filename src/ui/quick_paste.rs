//! Quick copy & paste logic for knowledge cards.

use crate::core::types::KnowledgeItem;
use crate::platform::clipboard::write_text;
use crate::services::copy_sound::play_copy_sound;
use crate::state::app::AppStore;

/// Quick paste or copy an item by index or direct item.
pub fn execute_quick_copy(
    store: AppStore,
    item: &KnowledgeItem,
) {
    // 1. Write text to native system clipboard
    write_text(&item.content);

    // 2. Play configured sound effect
    let sound_idx = store.settings.get_untracked().copy_sound_index;
    if store.settings.get_untracked().play_sound_on_copy {
        play_copy_sound(sound_idx);
    }

    // 3. Show toast notification
    store.show_toast(format!("已复制到剪贴板: {}", item.title));

    // 4. Auto hide window if configured
    if store.settings.get_untracked().auto_hide_on_blur {
        // window minimize or hide can be handled by platform
    }
}

/// Execute quick copy for numerical shortcut (1..9).
#[must_use]
pub fn handle_number_shortcut(
    store: AppStore,
    number: usize,
) -> bool {
    let items = store.items.get_untracked();
    if number > 0 && number <= items.len() {
        let item = &items[number - 1];
        execute_quick_copy(store, item);
        true
    } else {
        false
    }
}
