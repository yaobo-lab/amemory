//! Tag picker overlay to assign or remove tags on a specific Knowledge Item.

use nsh_base::input;
use nsh_core::prelude::*;

use super::components::modal_overlay::modal_overlay_view;
use super::theme::ClippiTheme;
use crate::core::types::KnowledgeItem;
use crate::state::app::AppStore;

pub fn tag_picker_view(
    _store: AppStore,
    item: KnowledgeItem,
    theme: ClippiTheme,
    on_close: impl Fn() + Send + Sync + 'static,
    on_updated: impl Fn(KnowledgeItem) + Send + Sync + 'static,
) -> impl IntoElement {
    let on_close = std::sync::Arc::new(on_close);
    let close_btn1 = on_close.clone();
    let close_btn2 = on_close.clone();
    let close_overlay = on_close.clone();
    let on_updated = std::sync::Arc::new(on_updated);
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let input_border = theme.input_border;

    let item_tags = signal(item.tags.clone());
    let new_tag_input = signal(String::new());

    let preset_tags = vec![
        "rust", "mcp", "prompt", "xerj", "skills", "books", "docs", "important", "todo",
    ];

    modal_overlay_view(
        view()
            .w(px(280.0))
            .bg(card_bg)
            .rounded(px(10.0))
            .border(px(1.0))
            .border_color(divider)
            .p(px(12.0))
            .flex_col()
            .gap(px(10.0))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        text("🏷️ 为卡片分配标签")
                            .text_color(text_1)
                            .font_weight(FontWeight::BOLD)
                            .text_xs(),
                    )
                    .child(
                        view()
                            .cursor_pointer()
                            .on_press(move || {
                                close_btn1();
                            })
                            .child(
                                text("✕")
                                    .text_color(text_3)
                                    .text_xs(),
                            ),
                    ),
            )
            // Add custom tag input
            .child(
                view()
                    .flex_row()
                    .gap(px(6.0))
                    .items_center()
                    .child(
                        input()
                            .value(move || new_tag_input.get())
                            .on_change(move |v: &str| new_tag_input.set(v.to_string()))
                            .placeholder("输入标签名...")
                            .flex_1()
                            .h(px(24.0))
                            .rounded(px(4.0))
                            .border(px(1.0))
                            .border_color(input_border)
                            .bg(theme.input_bg)
                            .text_xs()
                            .text_color(text_1),
                    )
                    .child(
                        view()
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(4.0))
                            .bg(accent)
                            .cursor_pointer()
                            .on_press(move || {
                                let tag = new_tag_input.get().trim().to_string();
                                if !tag.is_empty() {
                                    item_tags.update(|tags| {
                                        if !tags.contains(&tag) {
                                            tags.push(tag.clone());
                                        }
                                    });
                                    new_tag_input.set(String::new());
                                }
                            })
                            .child(
                                text("+")
                                    .text_color(rgb(0xffffff))
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                            ),
                    ),
            )
            // Tag chips
            .child(
                scroll()
                    .w_full()
                    .max_h(px(160.0))
                    .child(
                        view()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(6.0))
                            .children(preset_tags.into_iter().map(|preset| {
                                let preset_str = preset.to_string();
                                let preset_clone = preset_str.clone();
                                let preset_tc = preset_str.clone();
                                let preset_click = preset_str.clone();

                                view()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(move || if item_tags.get().contains(&preset_clone) { accent } else { divider })
                                    .cursor_pointer()
                                    .on_press(move || {
                                        item_tags.update(|tags| {
                                            if let Some(pos) = tags.iter().position(|t| t == &preset_click) {
                                                tags.remove(pos);
                                            } else {
                                                tags.push(preset_click.clone());
                                            }
                                        });
                                    })
                                    .child(
                                        text(format!("#{}", preset_str))
                                            .text_color(move || if item_tags.get().contains(&preset_tc) { rgb(0xffffff) } else { text_2 })
                                            .text_xs(),
                                    )
                            })),
                    ),
            )
            // Save & Close
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .pt(px(4.0))
                    .child(
                        view()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(accent)
                            .cursor_pointer()
                            .on_press(move || {
                                let mut updated = item.clone();
                                updated.tags = item_tags.get();
                                on_updated(updated);
                                close_btn2();
                            })
                            .child(
                                text("保存")
                                    .text_color(rgb(0xffffff))
                                    .font_weight(FontWeight::BOLD)
                                    .text_xs(),
                            ),
                    ),
            ),
        theme,
        move || close_overlay(),
    )
}
