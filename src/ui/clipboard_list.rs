//! Virtualized / scrollable knowledge item list view.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::clipboard_card::knowledge_card_view;
use super::theme::ClippiTheme;
use crate::core::filters::FilterCriteria;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::KnowledgeItem;
use crate::state::app::AppStore;

fn filter_items(
    items: &[KnowledgeItem],
    filters: &FilterCriteria,
    query: &str,
) -> Vec<KnowledgeItem> {
    let q = query.trim().to_lowercase();
    items
        .iter()
        .filter(|item| {
            if filters.favorites_only && !item.favorite && !item.is_favorite {
                return false;
            }
            if let Some(kind) = filters.kind
                && item.kind != kind
            {
                return false;
            }
            if let Some(scope) = &filters.scope
                && !item.uri.starts_with(scope)
            {
                return false;
            }
            if !filters.tags.is_empty() && !filters.tags.iter().any(|t| item.tags.contains(t)) {
                return false;
            }
            if !q.is_empty() {
                let matches_title = item.title.to_lowercase().contains(&q);
                let matches_content = item.content.to_lowercase().contains(&q);
                let matches_uri = item.uri.to_lowercase().contains(&q);
                let matches_tags = item.tags.iter().any(|t| t.to_lowercase().contains(&q));
                if !matches_title && !matches_content && !matches_uri && !matches_tags {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect()
}

/// Render knowledge item list view.
pub fn clipboard_list_view(
    store: AppStore,
    card_height: f32,
    theme: ClippiTheme,
    on_copy_item: impl Fn(KnowledgeItem) + Send + Sync + 'static,
    on_edit_item: impl Fn(KnowledgeItem) + Send + Sync + 'static,
    on_fav_item: impl Fn(KnowledgeItem) + Send + Sync + 'static,
    on_delete_item: impl Fn(KnowledgeItem) + Send + Sync + 'static,
) -> impl IntoElement {
    let on_copy_item = std::sync::Arc::new(on_copy_item);
    let on_edit_item = std::sync::Arc::new(on_edit_item);
    let on_fav_item = std::sync::Arc::new(on_fav_item);
    let on_delete_item = std::sync::Arc::new(on_delete_item);

    let items_signal = store.items;
    let filters_signal = store.filters;
    let query_signal = store.search_query;
    let selected_index_signal = store.selected_index;
    let search_query = store.search_query;
    let text_2 = theme.text_2;
    let text_3 = theme.text_3;

    let get_filtered = move || {
        filter_items(
            &items_signal.get(),
            &filters_signal.get(),
            &query_signal.get(),
        )
    };

    view()
        .flex_1()
        .w_full()
        .px(px(8.0))
        .child(
            when(
                move || get_filtered().is_empty(),
                move || {
                    // Empty placeholder
                    view()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .py(px(40.0))
                        .child(text(glyphs::SEARCH).font_family("iconfont").text_color(text_3).text_2xl())
                        .child(
                            text(I18nKey::CardEmptyTitle.text())
                                .text_color(text_2)
                                .font_weight(FontWeight::BOLD)
                                .text_xs(),
                        )
                        .child(
                            text(I18nKey::CardEmptyDesc.text())
                                .text_color(text_3)
                                .text_xs(),
                        )
                },
            )
            .otherwise(move || {
                let copy_fn = on_copy_item.clone();
                let edit_fn = on_edit_item.clone();
                let fav_fn = on_fav_item.clone();
                let del_fn = on_delete_item.clone();

                scroll()
                    .flex_1()
                    .child(
                        view()
                            .flex_col()
                            .gap(px(6.0))
                            .pb(px(8.0))
                            .child(each(
                                get_filtered,
                                |item: &KnowledgeItem| item.id.clone(),
                                move |item, index| {
                                    let item_val = item.get();
                                    let idx = index.get();
                                    let is_selected = selected_index_signal.with(|s| *s == Some(idx));

                                    let on_select = move || {
                                        selected_index_signal.set(Some(idx));
                                    };

                                    let item_copy = item_val.clone();
                                    let on_copy_cb = copy_fn.clone();
                                    let on_copy = move || on_copy_cb(item_copy.clone());

                                    let item_edit = item_val.clone();
                                    let on_edit_cb = edit_fn.clone();
                                    let on_edit = move || on_edit_cb(item_edit.clone());

                                    let item_fav = item_val.clone();
                                    let on_fav_cb = fav_fn.clone();
                                    let on_fav = move || on_fav_cb(item_fav.clone());

                                    let item_del = item_val.clone();
                                    let on_del_cb = del_fn.clone();
                                    let on_del = move || on_del_cb(item_del.clone());

                                    knowledge_card_view(
                                        item_val,
                                        is_selected,
                                        search_query.get(),
                                        card_height,
                                        theme,
                                        on_select,
                                        on_copy,
                                        on_edit,
                                        on_fav,
                                        on_del,
                                    )
                                },
                            )),
                    )
            }),
        )
}
