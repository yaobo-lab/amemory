//! Edit and create panel for Knowledge items (Skill, Book, Memory, Note)
//! 1:1 replica of Clippi edit panel layout and controls.

use nsh_base::input;
use nsh_core::prelude::*;
use toolkit_rs::chrono::Utc;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::core::find_replace;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::{ItemKind, KnowledgeItem};
use crate::state::app::AppStore;

/// Render Edit / Create panel
pub fn edit_panel_view(
    store: AppStore,
    theme: ClippiTheme,
    on_save: impl Fn(KnowledgeItem, bool) + 'static,
    on_back: impl Fn() + 'static,
) -> impl IntoElement {
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let input_bg = theme.input_bg;
    let input_border = theme.input_border;

    let editing_item = store.editing_item;
    let current_item = editing_item.get();
    let is_new = current_item.is_none();

    let initial_item = current_item.unwrap_or_else(|| KnowledgeItem {
        id: format!("custom_{}", Utc::now().timestamp_millis()),
        kind: ItemKind::Memory,
        title: String::new(),
        content: String::new(),
        description: None,
        source_path: None,
        tags: vec![],
        updated_at: Some(Utc::now().to_rfc3339()),
        favorite: false,
        is_favorite: false,
        ..Default::default()
    });

    let draft_title = signal(initial_item.title.clone());
    let draft_content = signal(initial_item.content.clone());
    let draft_kind = signal(initial_item.kind);
    let draft_tags = signal(initial_item.tags.join(", "));
    let find_open = signal(false);
    let find_text = signal(String::new());
    let replace_text = signal(String::new());

    let header_title = if is_new {
        I18nKey::EditTitleNew.text()
    } else {
        I18nKey::EditTitleEdit.text()
    };

    view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(theme.panel_bg)
        // ── 1. Top 36px Navigation Header ──
        .child(
            view()
                .h(px(36.0))
                .w_full()
                .bg(card_bg)
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(10.0))
                .border_b(px(1.0))
                .border_color(divider)
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .cursor_pointer()
                        .on_press(move || {
                            on_back();
                        })
                        .tooltip(tooltip_box("返回主列表", theme))
                        .child(
                            text(glyphs::BACK)
                                .font_family("iconfont")
                                .text_color(accent)
                                .text_sm(),
                        )
                        .child(
                            text(header_title)
                                .text_color(text_1)
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        ),
                )
                .child(
                    // Save Button
                    view()
                        .h(px(24.0))
                        .px(px(12.0))
                        .rounded(px(5.0))
                        .bg(accent)
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_press(move || {
                            let mut item = initial_item.clone();
                            item.title = draft_title.get();
                            item.content = draft_content.get();
                            item.kind = draft_kind.get();
                            item.tags = draft_tags
                                .get()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            item.updated_at = Some(Utc::now().to_rfc3339());
                            on_save(item, is_new);
                        })
                        .child(
                            text(I18nKey::EditSave.text())
                                .text_color(rgb(0xffffff))
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        ),
                ),
        )
        // ── 2. Type Selector & Toolbar (28px) ──
        .child(
            view()
                .h(px(28.0))
                .w_full()
                .bg(card_bg)
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(10.0))
                .border_b(px(1.0))
                .border_color(divider)
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            text(I18nKey::SettingFilterTypes.text())
                                .text_color(text_3)
                                .text_xs(),
                        )
                        .child(
                            // Kind buttons
                            view()
                                .flex_row()
                                .gap(px(4.0))
                                .child(type_chip(
                                    "Skill",
                                    ItemKind::Skill,
                                    draft_kind,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Book",
                                    ItemKind::Book,
                                    draft_kind,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Memory",
                                    ItemKind::Memory,
                                    draft_kind,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Note",
                                    ItemKind::Note,
                                    draft_kind,
                                    accent,
                                    divider,
                                    text_2,
                                )),
                        ),
                )
                .child(
                    // Find/Replace Toggle
                    view()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(divider)
                        .cursor_pointer()
                        .on_press(move || {
                            find_open.update(|v| *v = !*v);
                        })
                        .child(
                            text("🔍 查找替换")
                                .text_color(text_2)
                                .text_xs(),
                        ),
                ),
        )
        // ── 3. Find & Replace collapsible bar (26px) ──
        .child(
            when(
                move || find_open.get(),
                move || {
                    view()
                        .h(px(28.0))
                        .w_full()
                        .bg(input_bg)
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(10.0))
                        .border_b(px(1.0))
                        .border_color(divider)
                        .child(
                            input()
                                .value(move || find_text.get())
                                .on_change(move |v: &str| find_text.set(v.to_string()))
                                .placeholder(I18nKey::EditFindPlaceholder.text())
                                .w(px(100.0))
                                .h(px(22.0))
                                .rounded(px(4.0))
                                .border(px(1.0))
                                .border_color(input_border)
                                .bg(card_bg)
                                .text_xs()
                                .text_color(text_1),
                        )
                        .child(
                            input()
                                .value(move || replace_text.get())
                                .on_change(move |v: &str| replace_text.set(v.to_string()))
                                .placeholder(I18nKey::EditReplacePlaceholder.text())
                                .w(px(100.0))
                                .h(px(22.0))
                                .rounded(px(4.0))
                                .border(px(1.0))
                                .border_color(input_border)
                                .bg(card_bg)
                                .text_xs()
                                .text_color(text_1),
                        )
                        .child(
                            view()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(accent)
                                .cursor_pointer()
                                .on_press(move || {
                                    let query = find_text.get();
                                    let repl = replace_text.get();
                                    if !query.is_empty() {
                                        draft_content.update(|c| {
                                            let (replaced, ..) = find_replace::replace_all(c, &query, &repl);
                                            *c = replaced;
                                        });
                                    }
                                })
                                .child(
                                    text(I18nKey::EditReplaceAll.text())
                                        .text_color(rgb(0xffffff))
                                        .text_xs(),
                                ),
                        )
                },
            ),
        )
        // ── 4. Title Input Row ──
        .child(
            view()
                .px(px(10.0))
                .py(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text("标题:")
                        .text_color(text_3)
                        .text_xs(),
                )
                .child(
                    input()
                        .value(move || draft_title.get())
                        .on_change(move |v: &str| draft_title.set(v.to_string()))
                        .placeholder("卡片标题 / 知识项名称...")
                        .flex_1()
                        .h(px(24.0))
                        .rounded(px(4.0))
                        .border(px(1.0))
                        .border_color(input_border)
                        .bg(card_bg)
                        .text_xs()
                        .text_color(text_1),
                ),
        )
        // ── 5. Tags Input Row ──
        .child(
            view()
                .px(px(10.0))
                .pb(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text("标签:")
                        .text_color(text_3)
                        .text_xs(),
                )
                .child(
                    input()
                        .value(move || draft_tags.get())
                        .on_change(move |v: &str| draft_tags.set(v.to_string()))
                        .placeholder("多个标签用逗号分隔，如: rust, mcp, prompt")
                        .flex_1()
                        .h(px(24.0))
                        .rounded(px(4.0))
                        .border(px(1.0))
                        .border_color(input_border)
                        .bg(card_bg)
                        .text_xs()
                        .text_color(text_1),
                ),
        )
        // ── 6. Multiline Content Editor & Live Preview ──
        .child(
            view()
                .flex_1()
                .px(px(10.0))
                .pb(px(10.0))
                .flex_col()
                .gap(px(6.0))
                .child(
                    input()
                        .value(move || draft_content.get())
                        .on_change(move |v: &str| draft_content.set(v.to_string()))
                        .placeholder(I18nKey::EditContentPlaceholder.text())
                        .w_full()
                        .flex_1()
                        .rounded(px(6.0))
                        .border(px(1.0))
                        .border_color(input_border)
                        .bg(card_bg)
                        .p(px(8.0))
                        .text_xs()
                        .text_color(text_1),
                ),
        )
}

fn type_chip(
    label: &'static str,
    target_kind: ItemKind,
    current_kind: Signal<ItemKind>,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || if current_kind.get() == target_kind { accent } else { divider })
        .cursor_pointer()
        .on_press(move || {
            current_kind.set(target_kind);
        })
        .child(
            text(label)
                .text_color(move || if current_kind.get() == target_kind { rgb(0xffffff) } else { text_color })
                .text_xs(),
        )
}
