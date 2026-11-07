//! Tag filter panel for filtering knowledge items by tag chips.

use nsh_base::input;
use nsh_core::prelude::*;

use super::components::modal_overlay::modal_overlay_view;
use super::theme::ClippiTheme;
use crate::state::app::AppStore;

/// Render tag filter modal dialog.
pub fn tag_filter_view(
    store: AppStore,
    theme: ClippiTheme,
    on_close: impl Fn() + Send + Sync + 'static,
) -> impl IntoElement {
    let on_close = std::sync::Arc::new(on_close);
    let close_btn1 = on_close.clone();
    let close_btn2 = on_close.clone();
    let close_overlay = on_close;
    let card_bg = theme.card_bg;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let input_border = theme.input_border;

    let filters = store.filters;
    let items = store.items;

    // Aggregate unique tags from all items
    let mut all_tags = std::collections::BTreeSet::new();
    for it in items.get_untracked() {
        for t in it.tags {
            all_tags.insert(t);
        }
    }
    // Also add common system tags
    all_tags.insert("rust".to_string());
    all_tags.insert("skills".to_string());
    all_tags.insert("books".to_string());
    all_tags.insert("mcp".to_string());
    all_tags.insert("xerj".to_string());

    let new_tag_name = signal(String::new());

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
            // Header
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        text("🏷️ 标签过滤与管理 (Tags)")
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
            // Create tag row
            .child(
                view()
                    .flex_row()
                    .gap(px(6.0))
                    .items_center()
                    .child(
                        input()
                            .value(move || new_tag_name.get())
                            .on_change(move |v: &str| new_tag_name.set(v.to_string()))
                            .placeholder("新建标签...")
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
                                let tag = new_tag_name.get().trim().to_string();
                                if !tag.is_empty() {
                                    filters.update(|f| {
                                        if !f.tags.contains(&tag) {
                                            f.tags.push(tag.clone());
                                        }
                                    });
                                    new_tag_name.set(String::new());
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
            // Tags Grid
            .child(
                scroll()
                    .w_full()
                    .max_h(px(160.0))
                    .child(
                        view()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(6.0))
                            .children(all_tags.into_iter().map(|tag| {
                                let tag_bg = tag.clone();
                                let tag_tx = tag.clone();
                                let tag_click = tag.clone();
                                view()
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(move || if filters.get().tags.contains(&tag_bg) { accent } else { divider })
                                    .cursor_pointer()
                                    .on_press(move || {
                                        filters.update(|f| {
                                            if let Some(pos) = f.tags.iter().position(|x| x == &tag_click) {
                                                f.tags.remove(pos);
                                            } else {
                                                f.tags.push(tag_click.clone());
                                            }
                                        });
                                    })
                                    .child(
                                        text(format!("#{tag}"))
                                            .text_color(move || if filters.get().tags.contains(&tag_tx) { rgb(0xffffff) } else { text_2 })
                                            .text_xs(),
                                    )
                            })),
                    ),
            )
            // Clear & Done
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .pt(px(4.0))
                    .child(
                        view()
                            .cursor_pointer()
                            .on_press(move || {
                                filters.update(|f| f.tags.clear());
                            })
                            .child(
                                text("清空筛选")
                                    .text_color(text_3)
                                    .text_xs(),
                            ),
                    )
                    .child(
                        view()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(accent)
                            .cursor_pointer()
                            .on_press(move || {
                                close_btn2();
                            })
                            .child(
                                text("完成")
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
