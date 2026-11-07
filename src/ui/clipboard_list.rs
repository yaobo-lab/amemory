//! Virtualized / scrollable knowledge item list view.

use nsh_core::prelude::*;

use super::assets::glyphs;
use super::clipboard_card::knowledge_card_view;
use super::theme::ClippiTheme;
use crate::core::filters::{FilterCriteria, SearchMode};
use crate::core::i18n_keys::I18nKey;
use crate::core::types::KnowledgeItem;
use crate::state::app::AppStore;

fn compute_query_score(item: &KnowledgeItem, query: &str, mode: SearchMode) -> Option<f32> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return item.score;
    }

    let terms: Vec<&str> = q.split_whitespace().filter(|t| !t.is_empty()).collect();
    let title_lower = item.title.to_lowercase();
    let preview_lower = item.preview.to_lowercase();
    let content_lower = item.content.to_lowercase();
    let uri_lower = item.uri.to_lowercase();

    let mut lexical_score = 0.0_f32;
    let mut matched_any = false;

    if title_lower.contains(&q) {
        lexical_score += 3.5;
        matched_any = true;
    }
    if preview_lower.contains(&q) {
        lexical_score += 2.0;
        matched_any = true;
    }
    if content_lower.contains(&q) {
        lexical_score += 1.5;
        matched_any = true;
    }
    if uri_lower.contains(&q) {
        lexical_score += 1.2;
        matched_any = true;
    }
    if item.tags.iter().any(|t| t.to_lowercase().contains(&q)) {
        lexical_score += 2.0;
        matched_any = true;
    }

    // Token-level overlap for multi-term and semantic/hybrid queries
    let mut term_hits = 0usize;
    for term in &terms {
        if title_lower.contains(term)
            || preview_lower.contains(term)
            || content_lower.contains(term)
            || uri_lower.contains(term)
            || item.tags.iter().any(|t| t.to_lowercase().contains(term))
        {
            term_hits += 1;
            lexical_score += 0.8;
        }
    }

    // Character bigram overlap for semantic mode fuzzy recall
    let mut semantic_score = 0.0_f32;
    if matches!(mode, SearchMode::Semantic | SearchMode::Hybrid) {
        let q_chars: Vec<char> = q.chars().filter(|c| !c.is_whitespace()).collect();
        if q_chars.len() >= 2 {
            let mut bigram_hits = 0usize;
            let total_bigrams = q_chars.len() - 1;
            for window in q_chars.windows(2) {
                let bg: String = window.iter().collect();
                if title_lower.contains(&bg) || preview_lower.contains(&bg) || content_lower.contains(&bg) {
                    bigram_hits += 1;
                }
            }
            let ratio = bigram_hits as f32 / total_bigrams as f32;
            if ratio >= 0.5 {
                semantic_score = ratio * 3.0;
            }
        } else if term_hits > 0 {
            semantic_score = 1.5;
        }
    }

    let final_score = match mode {
        SearchMode::Lexical => {
            if !matched_any && term_hits < terms.len().max(1) {
                return None;
            }
            item.score.unwrap_or(lexical_score)
        }
        SearchMode::Semantic => {
            if !matched_any && semantic_score <= 0.0 && term_hits == 0 {
                return None;
            }
            item.score.unwrap_or_else(|| (semantic_score + lexical_score * 0.4).max(0.5))
        }
        SearchMode::Hybrid => {
            if !matched_any && semantic_score <= 0.0 && term_hits == 0 {
                return None;
            }
            item.score.unwrap_or_else(|| lexical_score + semantic_score * 0.6)
        }
    };

    Some(final_score)
}

fn filter_items(
    items: &[KnowledgeItem],
    filters: &FilterCriteria,
    query: &str,
) -> Vec<KnowledgeItem> {
    let q = query.trim();
    let mut results: Vec<KnowledgeItem> = items
        .iter()
        .filter_map(|item| {
            if filters.favorites_only && !item.favorite && !item.is_favorite {
                return None;
            }
            if let Some(kind) = filters.kind {
                if item.kind != kind {
                    return None;
                }
            } else if q.is_empty() && filters.tags.is_empty() {
                // In default "All" view without search query, show primary Skill/Book/Memory entries
                // rather than flooding the list with every sub-file; sub-files appear in Script/Note/Doc tabs or search
                let is_sub_file = !item.relative_path.is_empty()
                    && item.relative_path != "SKILL.md"
                    && item.relative_path != "BOOK.md";
                if is_sub_file {
                    return None;
                }
            }
            if let Some(scope) = &filters.scope
                && !item.uri.starts_with(scope)
            {
                return None;
            }
            if !filters.tags.is_empty()
                && !filters.tags.iter().any(|t| {
                    item.tags.iter().any(|it_tag| {
                        it_tag.eq_ignore_ascii_case(t)
                            || it_tag.strip_prefix("skill:") == Some(t.as_str())
                            || it_tag.strip_prefix("book:") == Some(t.as_str())
                    })
                })
            {
                return None;
            }
            if !q.is_empty() {
                let score = compute_query_score(item, q, filters.search_mode)?;
                let mut cloned = item.clone();
                cloned.score = Some(score);
                return Some(cloned);
            }
            Some(item.clone())
        })
        .collect();

    if !q.is_empty() {
        results.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .total_cmp(&a.score.unwrap_or(0.0))
                .then_with(|| a.title.cmp(&b.title))
        });
    }
    results
}

/// Render knowledge item list view.
pub fn clipboard_list_view(
    store: AppStore,
    _card_height: f32,
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
                                    let is_selected = move || selected_index_signal.get() == Some(index.get());

                                    let copy_for_select = copy_fn.clone();
                                    let edit_for_select = edit_fn.clone();
                                    let on_select = move || {
                                        let idx = index.get_untracked();
                                        let cur_item = item.get_untracked();
                                        if selected_index_signal.get_untracked() == Some(idx) {
                                            edit_for_select(cur_item);
                                        } else {
                                            selected_index_signal.set(Some(idx));
                                            if store.settings.get_untracked().paste_on_select {
                                                copy_for_select(cur_item);
                                            } else {
                                                store.show_toast(format!(
                                                    "已选中「{}」，再次点击可打开编辑",
                                                    cur_item.title
                                                ));
                                            }
                                        }
                                    };

                                    let on_copy_cb = copy_fn.clone();
                                    let on_copy = move || on_copy_cb(item.get_untracked());

                                    let on_tag = move || {
                                        store.tag_picking_item.set(Some(item.get_untracked()));
                                    };

                                    let on_more = move || {
                                        store.context_menu_item.set(Some(item.get_untracked()));
                                    };

                                    let on_edit_cb = edit_fn.clone();
                                    let on_edit = move || on_edit_cb(item.get_untracked());

                                    let on_fav_cb = fav_fn.clone();
                                    let on_fav = move || on_fav_cb(item.get_untracked());

                                    let on_del_cb = del_fn.clone();
                                    let on_del = move || on_del_cb(item.get_untracked());

                                    knowledge_card_view(
                                        store,
                                        item,
                                        is_selected,
                                        search_query,
                                        theme,
                                        on_select,
                                        on_copy,
                                        on_tag,
                                        on_more,
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
