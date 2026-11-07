//! Knowledge and memory card replicating Clippi's clipboard card 1:1.

use nsh_core::MouseEvent;
use nsh_core::prelude::*;

use super::assets::glyphs;
use super::hover_toolbar::hover_toolbar_view;
use super::theme::ClippiTheme;
use crate::core::search::highlight_text;
use crate::core::types::{ItemKind, KnowledgeItem};

/// Render single knowledge/memory item card.
#[allow(clippy::needless_pass_by_value)]
pub fn knowledge_card_view(
    item: KnowledgeItem,
    is_selected: bool,
    search_query: String,
    card_height: f32,
    theme: ClippiTheme,
    on_select: impl Fn() + 'static,
    on_copy: impl Fn() + Send + Sync + 'static,
    on_edit: impl Fn() + Send + Sync + 'static,
    on_fav: impl Fn() + Send + Sync + 'static,
    on_delete: impl Fn() + Send + Sync + 'static,
) -> impl IntoElement {
    let on_copy = std::sync::Arc::new(on_copy);
    let on_edit = std::sync::Arc::new(on_edit);
    let on_fav = std::sync::Arc::new(on_fav);
    let on_delete = std::sync::Arc::new(on_delete);

    let is_fav = item.favorite;

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

    let title_display = if item.title.is_empty() {
        item.uri.clone()
    } else {
        item.title.clone()
    };

    let query = search_query;
    let snippet_spans = highlight_text(&item.preview, &query);

    view()
        .h(px(card_height))
        .w_full()
        .rounded(px(10.0))
        .bg(surface)
        .border(px(1.0))
        .border_color(if is_selected { accent } else { divider })
        .cursor_pointer()
        .flex_row()
        .overflow_hidden()
        .on_press(on_select)
        .on_mouse_enter(move |_: &MouseEvent| is_hovered.set(true))
        .on_mouse_leave(move |_: &MouseEvent| is_hovered.set(false))
        // Left favorite indicator stripe (3px)
        .child(
            view()
                .w(px(3.0))
                .h_full()
                .bg(if item.favorite { fav_color } else { divider }),
        )
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
                        .text_color(if is_selected { accent } else { text_2 })
                        .text_sm(),
                )
                .child(
                    text(kind_name)
                        .text_color(text_3)
                        .text_xs(),
                ),
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
                            text(title_display)
                                .text_color(text_1)
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        )
                        .child(
                            item.score.map_or_else(view, |s| {
                                view().child(text(format!("{s:.1}")).text_color(accent).text_xs())
                            }),
                        ),
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
                                .font_weight(if span.is_highlight { FontWeight::BOLD } else { FontWeight::NORMAL })
                                .text_xs()
                        })),
                )
                // Bottom row: Info capsules
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
                                .child(text(item.owner.clone()).text_color(tag_text).text_xs()),
                        )
                        .child(
                            if item.part_total > 1 {
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
                            },
                        ),
                ),
        )
        // Hover toolbar
        .child(
            when(
                move || is_hovered.get(),
                move || {
                    let c_copy = on_copy.clone();
                    let c_edit = on_edit.clone();
                    let c_fav = on_fav.clone();
                    let c_del = on_delete.clone();
                    hover_toolbar_view(
                        is_fav,
                        theme,
                        move || c_copy(),
                        move || c_edit(),
                        move || c_fav(),
                        move || c_del(),
                    )
                },
            ),
        )
}
