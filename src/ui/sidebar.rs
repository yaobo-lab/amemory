//! 64px Attached left tag sidebar matching Clippi 1:1.

use nsh_core::prelude::*;

use super::components::tooltip_box;
use super::theme::ClippiTheme;
use crate::state::app::AppStore;

/// Render 64px left tag sidebar view.
#[must_use]
pub fn sidebar_view(store: AppStore, theme: ClippiTheme) -> impl IntoElement {
    let surface = theme.surface;
    let divider = theme.divider;
    let tag_bg = theme.tag_bg;
    let tag_text = theme.tag_text;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;

    let filters = store.filters;
    let items = store.items;
    let settings = store.settings;
    let show_tag_filter = store.show_tag_filter;

    let get_tag_list = move || {
        let mut set = std::collections::BTreeSet::new();
        for preset in [
            "rust", "mcp", "xerj", "book", "skill", "doc", "async", "tokio",
        ] {
            set.insert(preset.to_string());
        }
        for u in settings.get().user_tags {
            set.insert(u);
        }
        for it in items.get() {
            for t in it.tags {
                if !t.contains('/') && t.len() <= 18 {
                    set.insert(t);
                }
            }
        }
        let mut list = vec!["全部".to_string()];
        list.extend(set);
        list
    };

    view()
        .w(px(68.0))
        .h_full()
        .bg(surface)
        .border_r(px(1.0))
        .border_color(divider)
        .p(px(4.0))
        .flex_col()
        .gap(px(4.0))
        .child(
            view()
                .h(px(22.0))
                .w_full()
                .rounded(px(5.0))
                .bg(accent)
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_press(move || {
                    show_tag_filter.set(true);
                })
                .child(
                    text("＋管理")
                        .text_color(rgb(0xffffff))
                        .font_weight(FontWeight::BOLD)
                        .text_xs(),
                ),
        )
        .child(
            scroll()
                .flex_1()
                .child(view().flex_col().gap(px(3.0)).child(each(
                    get_tag_list,
                    |tag: &String| tag.clone(),
                    move |tag_sig, _| {
                        let tag_name = tag_sig.get();
                        let tag_bg_str = tag_name.clone();
                        let tag_border_str = tag_name.clone();
                        let tag_text_str = tag_name.clone();
                        let tag_click = tag_name.clone();
                        let display_label = tag_name
                            .strip_prefix("skill:")
                            .or_else(|| tag_name.strip_prefix("book:"))
                            .unwrap_or(&tag_name)
                            .to_string();

                        view()
                            .h(px(22.0))
                            .w_full()
                            .px(px(3.0))
                            .rounded(px(5.0))
                            .overflow_hidden()
                            .bg(move || {
                                let active = if tag_bg_str == "全部" {
                                    filters.get().tags.is_empty()
                                } else {
                                    filters.get().tags.contains(&tag_bg_str)
                                };
                                if active { accent_soft } else { tag_bg }
                            })
                            .border(px(1.0))
                            .border_color(move || {
                                let active = if tag_border_str == "全部" {
                                    filters.get().tags.is_empty()
                                } else {
                                    filters.get().tags.contains(&tag_border_str)
                                };
                                if active { accent } else { divider }
                            })
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .tooltip(tooltip_box(tag_name, theme))
                            .on_press(move || {
                                if tag_click == "全部" {
                                    filters.update(|f| f.tags.clear());
                                } else {
                                    filters.update(|f| {
                                        if let Some(pos) =
                                            f.tags.iter().position(|t| t == &tag_click)
                                        {
                                            f.tags.remove(pos);
                                        } else {
                                            f.tags.push(tag_click.clone());
                                        }
                                    });
                                }
                            })
                            .child(
                                text(display_label)
                                    .text_color(move || {
                                        let active = if tag_text_str == "全部" {
                                            filters.get().tags.is_empty()
                                        } else {
                                            filters.get().tags.contains(&tag_text_str)
                                        };
                                        if active { accent } else { tag_text }
                                    })
                                    .text_xs(),
                            )
                    },
                ))),
        )
}
