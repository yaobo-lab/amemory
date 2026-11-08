//! Edit and create panel for Knowledge items (Skill, Book, Memory, Note)
//! 1:1 replica of Clippi edit panel layout and controls.

use nsh_base::input;
use nsh_core::prelude::*;
use toolkit_rs::chrono::Utc;

use super::assets::glyphs;
use super::components::tooltip_box;
use super::rich_preview::rich_preview_view;
use super::theme::ClippiTheme;
use crate::core::find_replace;
use crate::core::i18n_keys::I18nKey;
use crate::core::types::{ItemKind, KnowledgeItem, SubFileEntry};
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
    let accent_soft = theme.accent_soft;
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

    // Multi-level file list (for Skills & Books with subdirectories like references/, scripts/, chapters/)
    let initial_files: Vec<SubFileEntry> = if initial_item.sub_files.is_empty() {
        let default_rel = if !initial_item.relative_path.is_empty() {
            initial_item.relative_path.clone()
        } else if initial_item.kind == ItemKind::Skill {
            "SKILL.md".to_string()
        } else if initial_item.kind == ItemKind::Book {
            "BOOK.md".to_string()
        } else {
            String::new()
        };
        if default_rel.is_empty() {
            Vec::new()
        } else {
            vec![SubFileEntry {
                relative_path: default_rel,
                content: initial_item.content.clone(),
            }]
        }
    } else {
        initial_item.sub_files.clone()
    };

    let initial_file_idx = initial_files
        .iter()
        .position(|f| {
            !initial_item.relative_path.is_empty() && f.relative_path == initial_item.relative_path
        })
        .unwrap_or(0);

    let initial_content = initial_files
        .get(initial_file_idx)
        .map_or_else(|| initial_item.content.clone(), |f| f.content.clone());

    let draft_title = signal(if initial_item.owner.is_empty() {
        initial_item.title.clone()
    } else {
        initial_item.owner.clone()
    });
    let draft_content = signal(initial_content);
    let draft_files = signal(initial_files);
    let active_file_idx = signal(initial_file_idx);
    let draft_kind = signal(initial_item.kind);
    let draft_tags = signal(initial_item.tags.join(", "));
    let find_open = signal(false);
    let preview_open = signal(false);
    let new_subfile_open = signal(false);
    let new_subfile_path = signal(String::new());
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
                            let current_txt = draft_content.get();
                            let raw_title = draft_title.get();
                            let cur_kind = draft_kind.get();

                            let resolved_title = if raw_title.trim().is_empty() {
                                let first_line = current_txt
                                    .lines()
                                    .find(|l| !l.trim().is_empty())
                                    .unwrap_or("未命名条目");
                                let clean = first_line
                                    .trim_start_matches(['#', ' ', '-'])
                                    .trim();
                                if clean.is_empty() {
                                    format!("新建{}", cur_kind.label())
                                } else {
                                    clean.chars().take(24).collect()
                                }
                            } else {
                                raw_title.trim().to_string()
                            };

                            item.title = resolved_title;
                            let idx = active_file_idx.get();
                            draft_files.update(|files| {
                                if let Some(slot) = files.get_mut(idx) {
                                    slot.content.clone_from(&current_txt);
                                }
                            });
                            let updated_files = draft_files.get();
                            if let Some(primary) = updated_files.first() {
                                item.content.clone_from(&primary.content);
                            } else {
                                item.content = current_txt;
                            }
                            item.sub_files = updated_files;
                            item.kind = cur_kind;
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
                                    draft_files,
                                    draft_content,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Book",
                                    ItemKind::Book,
                                    draft_kind,
                                    draft_files,
                                    draft_content,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Memory",
                                    ItemKind::Memory,
                                    draft_kind,
                                    draft_files,
                                    draft_content,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Note",
                                    ItemKind::Note,
                                    draft_kind,
                                    draft_files,
                                    draft_content,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Script",
                                    ItemKind::Script,
                                    draft_kind,
                                    draft_files,
                                    draft_content,
                                    accent,
                                    divider,
                                    text_2,
                                ))
                                .child(type_chip(
                                    "Doc",
                                    ItemKind::Document,
                                    draft_kind,
                                    draft_files,
                                    draft_content,
                                    accent,
                                    divider,
                                    text_2,
                                )),
                        ),
                )
                .child(
                    // Right toolbar: Preview toggle + Find/Replace Toggle
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            view()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(move || if preview_open.get() { accent } else { divider })
                                .cursor_pointer()
                                .on_press(move || {
                                    preview_open.update(|v| *v = !*v);
                                })
                                .child(
                                    text(I18nKey::EditPreview.text())
                                        .text_color(move || if preview_open.get() { rgb(0xffffff) } else { text_2 })
                                        .text_xs(),
                                ),
                        )
                        .child(
                            view()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(move || if find_open.get() { accent } else { divider })
                                .cursor_pointer()
                                .on_press(move || {
                                    find_open.update(|v| *v = !*v);
                                })
                                .child(
                                    text("🔍 查找替换")
                                        .text_color(move || if find_open.get() { rgb(0xffffff) } else { text_2 })
                                        .text_xs(),
                                ),
                        ),
                ),
        )
        // ── 3. Find & Replace collapsible bar (28px) ──
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
                                .w(px(90.0))
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
                                .w(px(90.0))
                                .h(px(22.0))
                                .rounded(px(4.0))
                                .border(px(1.0))
                                .border_color(input_border)
                                .bg(card_bg)
                                .text_xs()
                                .text_color(text_1),
                        )
                        .child(
                            text(move || {
                                let q = find_text.get();
                                if q.is_empty() {
                                    String::new()
                                } else {
                                    let count = find_replace::find_matches(&draft_content.get(), &q).total;
                                    format!("{count}处")
                                }
                            })
                            .text_color(text_3)
                            .text_xs(),
                        )
                        .child(
                            view()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(divider)
                                .cursor_pointer()
                                .on_press(move || {
                                    let query = find_text.get();
                                    let repl = replace_text.get();
                                    if !query.is_empty() {
                                        let mut did_replace = false;
                                        draft_content.update(|c| {
                                            let (replaced, ok) = find_replace::replace_first(c, &query, &repl);
                                            if ok {
                                                *c = replaced;
                                                did_replace = true;
                                            }
                                        });
                                        if did_replace {
                                            let idx = active_file_idx.get_untracked();
                                            let latest = draft_content.get_untracked();
                                            draft_files.update(|list| {
                                                if let Some(slot) = list.get_mut(idx) {
                                                    slot.content = latest;
                                                }
                                            });
                                            store.show_toast("已替换首处匹配");
                                        } else {
                                            store.show_toast("未找到匹配内容");
                                        }
                                    }
                                })
                                .child(
                                    text(I18nKey::EditReplaceOne.text())
                                        .text_color(text_1)
                                        .text_xs(),
                                ),
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
                                        let mut replaced_count = 0;
                                        draft_content.update(|c| {
                                            let (replaced, count, _) = find_replace::replace_all(c, &query, &repl);
                                            *c = replaced;
                                            replaced_count = count;
                                        });
                                        let idx = active_file_idx.get_untracked();
                                        let latest = draft_content.get_untracked();
                                        draft_files.update(|list| {
                                            if let Some(slot) = list.get_mut(idx) {
                                                slot.content = latest;
                                            }
                                        });
                                        store.show_toast(format!("已全部替换 {replaced_count} 处"));
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
        // ── 5.5 Multi-level File Tree / Tabs (when item has sub-files like SKILL.md, references/*, scripts/*) ──
        .child(
            when(
                move || !draft_files.get().is_empty() || matches!(draft_kind.get(), ItemKind::Skill | ItemKind::Book),
                move || {
                    view()
                        .px(px(10.0))
                        .pb(px(6.0))
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            text(move || format!("多级文件列表 ({} 个文件):", draft_files.get().len()))
                                                .text_color(text_3)
                                                .font_weight(FontWeight::BOLD)
                                                .text_xs(),
                                        )
                                        .child(
                                            view()
                                                .px(px(6.0))
                                                .py(px(1.0))
                                                .rounded(px(4.0))
                                                .bg(accent_soft)
                                                .border(px(1.0))
                                                .border_color(accent)
                                                .cursor_pointer()
                                                .on_press(move || {
                                                    new_subfile_open.update(|v| *v = !*v);
                                                })
                                                .child(
                                                    text("＋添加子文件")
                                                        .text_color(accent)
                                                        .text_xs(),
                                                ),
                                        ),
                                )
                                .child(
                                    text(move || {
                                        let idx = active_file_idx.get();
                                        let list = draft_files.get();
                                        list.get(idx).map_or_else(String::new, |f| {
                                            format!(
                                                "当前: {} ({} 行 · {} B)",
                                                f.relative_path,
                                                draft_content.get().lines().count(),
                                                draft_content.get().len()
                                            )
                                        })
                                    })
                                    .text_color(accent)
                                    .text_xs(),
                                ),
                        )
                        .child(
                            when(
                                move || new_subfile_open.get(),
                                move || {
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            input()
                                                .value(move || new_subfile_path.get())
                                                .on_change(move |v: &str| new_subfile_path.set(v.to_string()))
                                                .placeholder("输入子文件相对路径，如: references/guide.md 或 chapters/01.md")
                                                .flex_1()
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
                                                .px(px(8.0))
                                                .py(px(2.0))
                                                .rounded(px(4.0))
                                                .bg(accent)
                                                .cursor_pointer()
                                                .on_press(move || {
                                                    let raw_p = new_subfile_path.get().trim().replace('\\', "/");
                                                    if !raw_p.is_empty() {
                                                        let prev_idx = active_file_idx.get_untracked();
                                                        let cur_txt = draft_content.get_untracked();
                                                        let mut new_idx = 0;
                                                        draft_files.update(|list| {
                                                            if let Some(slot) = list.get_mut(prev_idx) {
                                                                slot.content = cur_txt;
                                                            }
                                                            if let Some(pos) = list.iter().position(|x| x.relative_path == raw_p) {
                                                                new_idx = pos;
                                                            } else {
                                                                list.push(SubFileEntry {
                                                                    relative_path: raw_p.clone(),
                                                                    content: format!("# {raw_p}\n\n"),
                                                                });
                                                                new_idx = list.len() - 1;
                                                            }
                                                        });
                                                        let next_txt = draft_files
                                                            .get_untracked()
                                                            .get(new_idx)
                                                            .map(|f| f.content.clone())
                                                            .unwrap_or_default();
                                                        active_file_idx.set(new_idx);
                                                        draft_content.set(next_txt);
                                                        new_subfile_path.set(String::new());
                                                        new_subfile_open.set(false);
                                                        store.show_toast(format!("已切换到子文件: {raw_p}"));
                                                    }
                                                })
                                                .child(
                                                    text("确定添加")
                                                        .text_color(rgb(0xffffff))
                                                        .text_xs(),
                                                ),
                                        )
                                },
                            ),
                        )
                        .child(
                            scroll()
                                .horizontal()
                                .w_full()
                                .child(
                                    view()
                                        .flex_row()
                                        .gap(px(4.0))
                                        .child(each(
                                            move || draft_files.get(),
                                            |sf: &SubFileEntry| sf.relative_path.clone(),
                                            move |sf_sig, idx_sig| {
                                                let sf = sf_sig.get();
                                                let idx = idx_sig.get();
                                                let path_label = sf.relative_path.clone();
                                                let path_tip = sf.relative_path;
                                                let file_icon = if path_label.starts_with("scripts/") {
                                                    glyphs::SCRIPT
                                                } else if path_label.starts_with("references/") || path_label.starts_with("notes/") {
                                                    glyphs::NOTE
                                                } else if path_label == "BOOK.md" || path_label.starts_with("chapters/") {
                                                    glyphs::BOOK
                                                } else {
                                                    glyphs::SKILL
                                                };

                                                view()
                                                    .h(px(24.0))
                                                    .px(px(8.0))
                                                    .rounded(px(5.0))
                                                    .bg(move || if active_file_idx.get() == idx_sig.get() { accent_soft } else { card_bg })
                                                    .border(px(1.0))
                                                    .border_color(move || if active_file_idx.get() == idx_sig.get() { accent } else { divider })
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(4.0))
                                                    .cursor_pointer()
                                                    .tooltip(tooltip_box(format!("切换查看/编辑: {path_tip}"), theme))
                                                    .on_press(move || {
                                                        let target_idx = idx_sig.get_untracked();
                                                        let prev_idx = active_file_idx.get_untracked();
                                                        if prev_idx != target_idx {
                                                            let current_txt = draft_content.get_untracked();
                                                            draft_files.update(|list| {
                                                                if let Some(slot) = list.get_mut(prev_idx) {
                                                                    slot.content = current_txt;
                                                                }
                                                            });
                                                            let next_txt = draft_files
                                                                .get_untracked()
                                                                .get(target_idx)
                                                                .map(|f| f.content.clone())
                                                                .unwrap_or_default();
                                                            active_file_idx.set(target_idx);
                                                            draft_content.set(next_txt);
                                                        }
                                                    })
                                                    .child(
                                                        text(file_icon)
                                                            .font_family("iconfont")
                                                            .text_color(move || if active_file_idx.get() == idx { accent } else { text_2 })
                                                            .text_xs(),
                                                    )
                                                    .child(
                                                        text(path_label)
                                                            .text_color(move || if active_file_idx.get() == idx { accent } else { text_1 })
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_xs(),
                                                    )
                                            },
                                        )),
                                ),
                        )
                },
            ),
        )
        // ── 6. Multiline Content Editor or Live Preview ──
        .child(
            view()
                .flex_1()
                .px(px(10.0))
                .pb(px(10.0))
                .flex_col()
                .gap(px(6.0))
                .child(
                    when(
                        move || preview_open.get(),
                        move || rich_preview_view(&draft_content.get(), theme),
                    )
                    .otherwise(move || {
                        input()
                            .multiline(true)
                            .scrollbar(true)
                            .value(move || draft_content.get())
                            .on_change(move |v: &str| {
                                let val = v.to_string();
                                draft_content.set(val.clone());
                                let idx = active_file_idx.get_untracked();
                                draft_files.update(|list| {
                                    if let Some(slot) = list.get_mut(idx) {
                                        slot.content = val;
                                    }
                                });
                            })
                            .placeholder(I18nKey::EditContentPlaceholder.text())
                            .w_full()
                            .flex_1()
                            .rounded(px(6.0))
                            .border(px(1.0))
                            .border_color(input_border)
                            .bg(card_bg)
                            .p(px(8.0))
                            .text_xs()
                            .text_color(text_1)
                    }),
                ),
        )
}

#[allow(clippy::too_many_arguments)]
fn type_chip(
    label: &'static str,
    target_kind: ItemKind,
    current_kind: Signal<ItemKind>,
    draft_files: Signal<Vec<SubFileEntry>>,
    draft_content: Signal<String>,
    accent: nsh_core::prelude::Rgba,
    divider: nsh_core::prelude::Rgba,
    text_color: nsh_core::prelude::Rgba,
) -> impl IntoElement {
    view()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(move || {
            if current_kind.get() == target_kind {
                accent
            } else {
                divider
            }
        })
        .cursor_pointer()
        .on_press(move || {
            current_kind.set(target_kind);
            if matches!(target_kind, ItemKind::Skill | ItemKind::Book)
                && draft_files.get_untracked().is_empty()
            {
                let primary_name = if target_kind == ItemKind::Skill {
                    "SKILL.md"
                } else {
                    "BOOK.md"
                };
                draft_files.set(vec![SubFileEntry {
                    relative_path: primary_name.to_string(),
                    content: draft_content.get_untracked(),
                }]);
            }
        })
        .child(
            text(label)
                .text_color(move || {
                    if current_kind.get() == target_kind {
                        rgb(0xffffff)
                    } else {
                        text_color
                    }
                })
                .text_xs(),
        )
}
