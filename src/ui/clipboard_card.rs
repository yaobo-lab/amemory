//! Reusable single knowledge card item matching Clippi 1:1.

use nsh_core::MouseEvent;
use nsh_core::element::ImageSrc;
use nsh_core::prelude::*;

use super::assets::kind_icons;
use super::hover_toolbar::hover_toolbar_view;
use super::theme::ClippiTheme;
use crate::core::search::highlight_text;
use crate::core::types::KnowledgeItem;
use crate::state::app::AppStore;

/// Width of the left column holding the kind icon tile and label.
const KIND_COLUMN_WIDTH: f32 = 56.0;
/// Side of the tinted rounded tile behind the kind icon.
const KIND_TILE_SIZE: f32 = 32.0;
/// Side of the kind icon inside its tile.
const KIND_ICON_SIZE: f32 = 18.0;

/// Render single knowledge card in virtualized list.
#[allow(clippy::too_many_arguments)]
pub fn knowledge_card_view(
    store: AppStore,
    item_sig: ReadSignal<KnowledgeItem>,
    is_selected: impl Fn() -> bool + Copy + 'static,
    search_query: Signal<String>,
    _theme: ClippiTheme,
    on_select: impl Fn() + 'static,
    on_copy: impl Fn() + 'static,
    on_tag: impl Fn() + 'static,
    on_more: impl Fn() + 'static,
    on_edit: impl Fn() + 'static,
    on_fav: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
) -> impl IntoElement {
    let on_copy = std::sync::Arc::new(on_copy);
    let on_tag = std::sync::Arc::new(on_tag);
    let on_more = std::sync::Arc::new(on_more);
    let on_edit = std::sync::Arc::new(on_edit);
    let on_fav = std::sync::Arc::new(on_fav);
    let on_delete = std::sync::Arc::new(on_delete);

    let item = item_sig.get();
    let is_hovered = signal(false);

    let kind = item.kind;
    let hue = kind_icons::hue(kind);
    let icon_src = ImageSrc::svg_markup(kind_icons::markup(kind));

    let query = search_query.get();
    let snippet_spans = highlight_text(&item.preview, &query);

    view()
        .h(move || px(store.card_height()))
        .w_full()
        .rounded(px(10.0))
        .bg(move || store.theme().surface)
        .border(px(1.0))
        .border_color(move || {
            if is_selected() {
                store.theme().accent
            } else {
                store.theme().divider
            }
        })
        .cursor_pointer()
        .flex_row()
        .overflow_hidden()
        .on_press(on_select)
        .on_mouse_enter(move |_: &MouseEvent| is_hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| is_hovered.set(false))
        // Left favorite indicator stripe (3px)
        .child(view().w(px(3.0)).h_full().bg(move || {
            if item_sig.get().favorite {
                store.theme().fav_color
            } else {
                store.theme().divider
            }
        }))
        // Left type column: tinted rounded icon tile + small kind label
        .child(
            view()
                .w(px(KIND_COLUMN_WIDTH))
                .h_full()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .child(
                    view()
                        .size(px(KIND_TILE_SIZE))
                        .rounded(px(9.0))
                        .border(px(1.0))
                        .border_color(move || {
                            let alpha = if is_selected() { 0x66 } else { 0x2e };
                            rgba((hue << 8) | alpha)
                        })
                        .items_center()
                        .justify_center()
                        .bg(move || {
                            let alpha = if is_selected() { 0x38 } else { 0x1a };
                            rgba((hue << 8) | alpha)
                        })
                        .child(svg(icon_src).size(px(KIND_ICON_SIZE)).text_color(rgb(hue))),
                )
                .child(
                    text(kind_icons::label(kind))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if is_selected() {
                                rgb(hue)
                            } else {
                                store.theme().text_3
                            }
                        }),
                ),
        )
        // Right content column
        .child(
            view()
                .flex_1()
                .h_full()
                .py(px(6.0))
                .pl(px(4.0))
                .pr(px(10.0))
                .justify_between()
                // Top row: Title + score
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            text(move || {
                                let cur = item_sig.get();
                                if cur.title.is_empty() {
                                    cur.uri
                                } else {
                                    cur.title
                                }
                            })
                            .text_color(move || store.theme().text_1)
                            .font_weight(FontWeight::BOLD)
                            .text_xs(),
                        )
                        .child(when(
                            move || item_sig.get().score.is_some(),
                            move || {
                                let s = item_sig.get().score.unwrap_or(0.0);
                                view().child(
                                    text(format!("{s:.2}"))
                                        .text_color(move || store.theme().accent)
                                        .text_xs(),
                                )
                            },
                        )),
                )
                // Middle row: Preview snippet with keyword highlight
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .h(px(32.0))
                        .overflow_hidden()
                        .children(snippet_spans.into_iter().map(move |span| {
                            let is_hl = span.is_highlight;
                            text(span.text)
                                .text_color(move || {
                                    if is_hl {
                                        store.theme().accent
                                    } else {
                                        store.theme().text_2
                                    }
                                })
                                .font_weight(if is_hl {
                                    FontWeight::BOLD
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_xs()
                        })),
                )
                // Bottom row: Info capsules & clickable tag pills
                .child(
                    view()
                        .h(px(18.0))
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            view()
                                .px(px(4.0))
                                .py(px(1.0))
                                .rounded(px(3.0))
                                .bg(move || store.theme().tag_bg)
                                .child(
                                    text(move || {
                                        let o = item_sig.get().owner;
                                        if o.is_empty() {
                                            "default".to_string()
                                        } else {
                                            o
                                        }
                                    })
                                    .text_color(move || store.theme().tag_text)
                                    .text_xs(),
                                ),
                        )
                        .child(if item.part_total > 1 {
                            view()
                                .px(px(4.0))
                                .py(px(1.0))
                                .rounded(px(3.0))
                                .bg(move || store.theme().tag_bg)
                                .child(
                                    text(format!("#{}/{}", item.part_index, item.part_total))
                                        .text_color(move || store.theme().text_3)
                                        .text_xs(),
                                )
                        } else {
                            view()
                        })
                        .children(
                            item.tags
                                .iter()
                                .filter(|t| {
                                    !t.starts_with("skill:")
                                        && !t.starts_with("book:")
                                        && !t.contains('/')
                                })
                                .take(3)
                                .map(move |t_str| {
                                    let click_t = t_str.clone();
                                    view()
                                        .px(px(4.0))
                                        .py(px(1.0))
                                        .rounded(px(3.0))
                                        .bg(move || store.theme().tag_bg)
                                        .cursor_pointer()
                                        .on_press(move || {
                                            store.filters.update(|f| {
                                                if !f.tags.contains(&click_t) {
                                                    f.tags.push(click_t.clone());
                                                }
                                            });
                                        })
                                        .child(
                                            text(format!("#{t_str}"))
                                                .text_color(move || store.theme().accent)
                                                .text_xs(),
                                        )
                                }),
                        ),
                ),
        )
        // Hover toolbar
        .child(when(
            move || is_hovered.get(),
            move || {
                let c_copy = on_copy.clone();
                let c_tag = on_tag.clone();
                let c_more = on_more.clone();
                let c_edit = on_edit.clone();
                let c_fav = on_fav.clone();
                let c_del = on_delete.clone();
                hover_toolbar_view(
                    move || item_sig.get().favorite,
                    store.theme(),
                    move || c_copy(),
                    move || c_tag(),
                    move || c_more(),
                    move || c_edit(),
                    move || c_fav(),
                    move || c_del(),
                )
            },
        ))
}
