//! Knowledge and memory card replicating Clippi's clipboard card 1:1.

use nsh_core::MouseEvent;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::hover_toolbar::hover_toolbar_view;
use super::theme::ClippiTheme;
use crate::core::search::highlight_text;
use crate::core::types::{ItemKind, KnowledgeItem};
use crate::state::app::AppStore;

/// Render single knowledge/memory item card with reactive selection and favorite state.
#[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
pub fn knowledge_card_view(
    store: AppStore,
    item_sig: ReadSignal<KnowledgeItem>,
    is_selected: impl Fn() -> bool + Copy + 'static,
    search_query: Signal<String>,
    theme: ClippiTheme,
    on_select: impl Fn() + 'static,
    on_copy: impl Fn() + Send + Sync + 'static,
    on_tag: impl Fn() + Send + Sync + 'static,
    on_more: impl Fn() + Send + Sync + 'static,
    on_edit: impl Fn() + Send + Sync + 'static,
    on_fav: impl Fn() + Send + Sync + 'static,
    on_delete: impl Fn() + Send + Sync + 'static,
) -> impl IntoElement {
    let on_copy = std::sync::Arc::new(on_copy);
    let on_tag = std::sync::Arc::new(on_tag);
    let on_more = std::sync::Arc::new(on_more);
    let on_edit = std::sync::Arc::new(on_edit);
    let on_fav = std::sync::Arc::new(on_fav);
    let on_delete = std::sync::Arc::new(on_delete);

    let item = item_sig.get();

    let surface = theme.surface;
    let divider = theme.divider;
    let text_1 = theme.text_1;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;
    let accent = theme.accent;
    let fav_color = theme.fav_color;
    let tag_bg = theme.tag_bg;
    let tag_text = theme.tag_text;

    let is_hovered = signal(false);

    let (icon, kind_name) = match item.kind {
        ItemKind::Skill => (glyphs::SKILL, "SKILL"),
        ItemKind::Book => (glyphs::BOOK, "BOOK"),
        ItemKind::Document => (glyphs::DOCUMENT, "DOC"),
        ItemKind::Memory => (glyphs::MEMORY, "MEM"),
        ItemKind::Script => (glyphs::SCRIPT, "CODE"),
        ItemKind::Note => (glyphs::NOTE, "NOTE"),
    };

    let query = search_query.get();
    let snippet_spans = highlight_text(&item.preview, &query);

    view()
        .h(move || px(store.card_height()))
        .w_full()
        .rounded(px(10.0))
        .bg(surface)
        .border(px(1.0))
        .border_color(move || if is_selected() { accent } else { divider })
        .cursor_pointer()
        .flex_row()
        .overflow_hidden()
        .on_press(on_select)
        .on_mouse_enter(move |_: &MouseEvent| is_hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| is_hovered.set(false))
        // Left favorite indicator stripe (3px)
        .child(view().w(px(3.0)).h_full().bg(move || {
            if item_sig.get().favorite {
                fav_color
            } else {
                divider
            }
        }))
        // Left type column (36px)
        .child(
            view()
                .w(px(36.0))
                .h_full()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .child(
                    text(icon)
                        .font_family("iconfont")
                        .text_color(move || if is_selected() { accent } else { text_2 })
                        .text_sm(),
                )
                .child(text(kind_name).text_color(text_3).text_xs()),
        )
        // Right content column
        .child(
            view()
                .flex_1()
                .h_full()
                .py(px(6.0))
                .pr(px(8.0))
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
                            .text_color(text_1)
                            .font_weight(FontWeight::BOLD)
                            .text_xs(),
                        )
                        .child(when(
                            move || item_sig.get().score.is_some(),
                            move || {
                                let s = item_sig.get().score.unwrap_or(0.0);
                                view().child(text(format!("{s:.2}")).text_color(accent).text_xs())
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
                            text(span.text)
                                .text_color(if span.is_highlight { accent } else { text_2 })
                                .font_weight(if span.is_highlight {
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
                                .bg(tag_bg)
                                .child(
                                    text(move || {
                                        let o = item_sig.get().owner;
                                        if o.is_empty() {
                                            "default".to_string()
                                        } else {
                                            o
                                        }
                                    })
                                    .text_color(tag_text)
                                    .text_xs(),
                                ),
                        )
                        .child(if item.part_total > 1 {
                            view()
                                .px(px(4.0))
                                .py(px(1.0))
                                .rounded(px(3.0))
                                .bg(tag_bg)
                                .child(
                                    text(format!("#{}/{}", item.part_index, item.part_total))
                                        .text_color(text_3)
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
                                        .bg(tag_bg)
                                        .cursor_pointer()
                                        .on_press(move || {
                                            store.filters.update(|f| {
                                                if !f.tags.contains(&click_t) {
                                                    f.tags.push(click_t.clone());
                                                }
                                            });
                                        })
                                        .child(
                                            text(format!("#{t_str}")).text_color(accent).text_xs(),
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
                    theme,
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
