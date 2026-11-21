//! Edit and create panel for Knowledge items (Skill, Book, Memory, Note)
//! 1:1 replica of Clippi edit panel layout and controls.

use std::collections::BTreeMap;

use nsh_base::input;
use nsh_base::rich_editor::{
    commands::{BoxCommand, command, tiptap},
    extensions::{
        history,
        markdown::{MarkdownOptions, markdown as markdown_plugin},
        starter,
    },
    formats::{DocFormat, MarkdownFormat},
    model::{Attrs, Fragment},
    state::{EditorState, EditorStateConfig},
    view::{EditorSession, NodeViews, RichEditorClasses, rich_editor},
};
use nsh_core::prelude::*;
use serde_json::json;
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
    let editor_session = signal(EditorSession::new(
        build_markdown_state(&initial_content),
        NodeViews::new(),
    ));
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
    let toolbar_rev = signal(0u64);

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
                .h(px(40.0))
                .w_full()
                .bg(card_bg)
                .flex_row()
                .items_center()
                .justify_between()
                .pl(px(10.0))
                .pr(px(12.0))
                .border_b(px(1.0))
                .border_color(divider)
                .child(
                    view()
                        .h(px(30.0))
                        .px(px(8.0))
                        .rounded(px(6.0))
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
                                .text_size(px(18.0)),
                        )
                        .child(
                            text(header_title)
                                .text_color(text_1)
                                .font_weight(FontWeight::BOLD)
                                .text_sm(),
                        ),
                )
                .child(
                    // Save Button
                    view()
                        .h(px(28.0))
                        .px(px(14.0))
                        .rounded(px(6.0))
                        .bg(accent)
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_press(move || {
                            let mut item = initial_item.clone();
                            let current_txt = editor_session
                                .with_untracked(serialize_session_markdown)
                                .unwrap_or_else(|| draft_content.get_untracked());
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
                                            editor_session.with_untracked(|s| {
                                                s.update_state(build_markdown_state(&latest));
                                            });
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
                                        editor_session.with_untracked(|s| {
                                            s.update_state(build_markdown_state(&latest));
                                        });
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
                                                        editor_session.with_untracked(|s| {
                                                            s.update_state(build_markdown_state(&next_txt));
                                                        });
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
                                                            editor_session.with_untracked(|s| {
                                                                s.update_state(build_markdown_state(&next_txt));
                                                            });
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
        // ── 6. NSH Markdown RichEditor or Live Preview ──
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
                        let session_for_view = editor_session.get_untracked();
                        view()
                            .w_full()
                            .flex_1()
                            .flex_col()
                            .rounded(px(6.0))
                            .border(px(1.0))
                            .border_color(input_border)
                            .bg(card_bg)
                            .overflow_hidden()
                            .child(markdown_toolbar(
                                editor_session,
                                draft_content,
                                draft_files,
                                active_file_idx,
                                toolbar_rev,
                                &theme,
                            ))
                            .child(
                                view().w_full().flex_1().overflow_hidden().child(
                                    rich_editor()
                                        .editor(session_for_view)
                                        .placeholder(I18nKey::EditContentPlaceholder.text())
                                        .classes(markdown_editor_classes(theme.bg.r < 0.5))
                                        .w_full()
                                        .h_full()
                                        .text_color(text_1)
                                        .on_change(move |_: &_| {
                                            sync_editor_to_draft(
                                                editor_session,
                                                draft_content,
                                                draft_files,
                                                active_file_idx,
                                            );
                                            toolbar_rev.update(|v| *v = v.wrapping_add(1));
                                        })
                                        .on_selection_change(move |_: &_| {
                                            toolbar_rev.update(|v| *v = v.wrapping_add(1));
                                        }),
                                ),
                            )
                    }),
                ),
        )
}

/// Build an NSH `RichEditor` state initialized with starter + markdown plugins and parsed Markdown.
fn build_markdown_state(markdown_text: &str) -> EditorState {
    let schema = starter::schema();
    let mut plugins = starter::plugins(&schema);
    plugins.extend(markdown_plugin(MarkdownOptions::default()));

    let mut config = EditorStateConfig::new(schema.clone()).plugins(plugins.clone());
    if !markdown_text.is_empty()
        && let Ok(doc) = MarkdownFormat::default().parse(&schema, markdown_text)
    {
        config = config.doc(doc);
    }

    EditorState::create(config).unwrap_or_else(|_| {
        EditorState::create(EditorStateConfig::new(schema).plugins(plugins))
            .expect("empty starter markdown editor state is always valid")
    })
}

/// Serialize the current `EditorSession` document back to Markdown text.
fn serialize_session_markdown(session: &EditorSession) -> Option<String> {
    MarkdownFormat::default()
        .serialize(session.state().doc())
        .ok()
}

/// Sync the serialized Markdown from `editor_session` into `draft_content` and active sub-file.
fn sync_editor_to_draft(
    editor_session: Signal<EditorSession>,
    draft_content: Signal<String>,
    draft_files: Signal<Vec<SubFileEntry>>,
    active_file_idx: Signal<usize>,
) {
    if let Some(val) = editor_session.with_untracked(serialize_session_markdown) {
        draft_content.set(val.clone());
        let idx = active_file_idx.get_untracked();
        draft_files.update(|list| {
            if let Some(slot) = list.get_mut(idx) {
                slot.content = val;
            }
        });
    }
}

/// Formatting and editing actions exposed by the Markdown editor header toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolbarAction {
    Undo,
    Redo,
    Heading(u64),
    Bold,
    Italic,
    Strike,
    InlineCode,
    BulletList,
    OrderedList,
    TaskList,
    Blockquote,
    CodeBlock,
    HorizontalRule,
}

impl ToolbarAction {
    fn build_command(self, session: &EditorSession) -> Option<BoxCommand> {
        let state = session.state();
        let schema = state.schema();
        match self {
            Self::Undo => Some(command(history::undo)),
            Self::Redo => Some(command(history::redo)),
            Self::Heading(level) => {
                let heading = schema.node_type("heading")?;
                let paragraph = schema.node_type("paragraph")?;
                let attrs = Attrs::from_iter([("level".to_owned(), json!(level))]);
                Some(tiptap::toggle_node(heading, paragraph, attrs))
            }
            Self::Bold => {
                let mark = schema.mark_type("bold")?;
                Some(tiptap::toggle_mark(mark, Attrs::new(), false))
            }
            Self::Italic => {
                let mark = schema.mark_type("italic")?;
                Some(tiptap::toggle_mark(mark, Attrs::new(), false))
            }
            Self::Strike => {
                let mark = schema.mark_type("strike")?;
                Some(tiptap::toggle_mark(mark, Attrs::new(), false))
            }
            Self::InlineCode => {
                let mark = schema.mark_type("code")?;
                Some(tiptap::toggle_mark(mark, Attrs::new(), false))
            }
            Self::BulletList => {
                let list = schema.node_type("bulletList")?;
                let item = schema.node_type("listItem")?;
                Some(tiptap::toggle_list(list, item, false, Attrs::new()))
            }
            Self::OrderedList => {
                let list = schema.node_type("orderedList")?;
                let item = schema.node_type("listItem")?;
                Some(tiptap::toggle_list(list, item, false, Attrs::new()))
            }
            Self::TaskList => {
                let list = schema.node_type("taskList")?;
                let item = schema.node_type("taskItem")?;
                Some(tiptap::toggle_list(list, item, false, Attrs::new()))
            }
            Self::Blockquote => {
                let quote = schema.node_type("blockquote")?;
                Some(tiptap::toggle_wrap(quote, Attrs::new()))
            }
            Self::CodeBlock => {
                let code_block = schema.node_type("codeBlock")?;
                let paragraph = schema.node_type("paragraph")?;
                Some(tiptap::toggle_node(code_block, paragraph, Attrs::new()))
            }
            Self::HorizontalRule => {
                let hr = schema.node_type("horizontalRule")?;
                Some(command(move |st, dispatch| {
                    let Ok(node) = hr.create(None, Fragment::empty(), Vec::new()) else {
                        return false;
                    };
                    if let Some(dispatch) = dispatch {
                        let mut tr = st.tr();
                        tr.replace_selection_with(node, false);
                        dispatch(tr);
                    }
                    true
                }))
            }
        }
    }

    fn is_active(self, session: &EditorSession) -> bool {
        match self {
            Self::Heading(level) => session.is_active(
                Some("heading"),
                Some(Attrs::from_iter([("level".to_owned(), json!(level))])),
            ),
            Self::Bold => session.is_active(Some("bold"), None),
            Self::Italic => session.is_active(Some("italic"), None),
            Self::Strike => session.is_active(Some("strike"), None),
            Self::InlineCode => session.is_active(Some("code"), None),
            Self::BulletList => session.is_active(Some("bulletList"), None),
            Self::OrderedList => session.is_active(Some("orderedList"), None),
            Self::TaskList => session.is_active(Some("taskList"), None),
            Self::Blockquote => session.is_active(Some("blockquote"), None),
            Self::CodeBlock => session.is_active(Some("codeBlock"), None),
            Self::Undo | Self::Redo | Self::HorizontalRule => false,
        }
    }

    fn is_enabled(self, session: &EditorSession) -> bool {
        match self {
            Self::Undo => history::undo_depth(&session.state()) > 0,
            Self::Redo => history::redo_depth(&session.state()) > 0,
            _ => true,
        }
    }
}

/// Render the Markdown formatting toolbar docked at the top of the editor card.
fn markdown_toolbar(
    editor_session: Signal<EditorSession>,
    draft_content: Signal<String>,
    draft_files: Signal<Vec<SubFileEntry>>,
    active_file_idx: Signal<usize>,
    toolbar_rev: Signal<u64>,
    theme: &ClippiTheme,
) -> impl IntoElement {
    let ctx = ToolbarCtx {
        editor_session,
        draft_content,
        draft_files,
        active_file_idx,
        toolbar_rev,
        theme: *theme,
    };

    view()
        .w_full()
        .h(px(32.0))
        .bg(theme.input_bg)
        .border_b(px(1.0))
        .border_color(theme.divider)
        .px(px(6.0))
        .flex_row()
        .items_center()
        .child(
            scroll().horizontal().w_full().child(
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(2.0))
                    .child(toolbar_btn("↶", "撤销 (Ctrl+Z)", ToolbarAction::Undo, &ctx))
                    .child(toolbar_btn("↷", "重做 (Ctrl+Y)", ToolbarAction::Redo, &ctx))
                    .child(toolbar_divider(theme.divider))
                    .child(toolbar_btn(
                        "H1",
                        "一级标题 (# )",
                        ToolbarAction::Heading(1),
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "H2",
                        "二级标题 (## )",
                        ToolbarAction::Heading(2),
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "H3",
                        "三级标题 (### )",
                        ToolbarAction::Heading(3),
                        &ctx,
                    ))
                    .child(toolbar_divider(theme.divider))
                    .child(toolbar_btn("B", "加粗 (Ctrl+B)", ToolbarAction::Bold, &ctx))
                    .child(toolbar_btn(
                        "I",
                        "斜体 (Ctrl+I)",
                        ToolbarAction::Italic,
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "S",
                        "删除线 (~~text~~)",
                        ToolbarAction::Strike,
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "</>",
                        "行内代码 (`code`)",
                        ToolbarAction::InlineCode,
                        &ctx,
                    ))
                    .child(toolbar_divider(theme.divider))
                    .child(toolbar_btn(
                        "• 列表",
                        "无序列表 (- )",
                        ToolbarAction::BulletList,
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "1. 列表",
                        "有序列表 (1. )",
                        ToolbarAction::OrderedList,
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "☑ 任务",
                        "任务列表 ([ ] )",
                        ToolbarAction::TaskList,
                        &ctx,
                    ))
                    .child(toolbar_divider(theme.divider))
                    .child(toolbar_btn(
                        "❝ 引用",
                        "引用块 (> )",
                        ToolbarAction::Blockquote,
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "{ } 代码块",
                        "代码块 (```)",
                        ToolbarAction::CodeBlock,
                        &ctx,
                    ))
                    .child(toolbar_btn(
                        "―",
                        "插入分割线 (---)",
                        ToolbarAction::HorizontalRule,
                        &ctx,
                    )),
            ),
        )
}

#[derive(Clone, Copy)]
struct ToolbarCtx {
    editor_session: Signal<EditorSession>,
    draft_content: Signal<String>,
    draft_files: Signal<Vec<SubFileEntry>>,
    active_file_idx: Signal<usize>,
    toolbar_rev: Signal<u64>,
    theme: ClippiTheme,
}

fn toolbar_divider(color: Rgba) -> impl IntoElement {
    view().w(px(1.0)).h(px(14.0)).mx(px(3.0)).bg(color)
}

fn toolbar_btn(
    label: &'static str,
    tip: &'static str,
    action: ToolbarAction,
    ctx: &ToolbarCtx,
) -> impl IntoElement {
    let editor_session = ctx.editor_session;
    let draft_content = ctx.draft_content;
    let draft_files = ctx.draft_files;
    let active_file_idx = ctx.active_file_idx;
    let toolbar_rev = ctx.toolbar_rev;
    let theme = ctx.theme;
    let accent = theme.accent;
    let accent_soft = theme.accent_soft;
    let text_1 = theme.text_1;
    let text_3 = theme.text_3;
    let btn_hover = theme.btn_hover;

    let label_el = text(label)
        .text_xs()
        .font_weight(
            if matches!(action, ToolbarAction::Bold | ToolbarAction::Heading(_)) {
                FontWeight::BOLD
            } else {
                FontWeight::MEDIUM
            },
        )
        .text_color(move || {
            let _ = toolbar_rev.get();
            editor_session.with_untracked(|s| {
                if !action.is_enabled(s) {
                    text_3
                } else if action.is_active(s) {
                    accent
                } else {
                    text_1
                }
            })
        });

    view()
        .h(px(24.0))
        .px(px(6.0))
        .rounded(px(4.0))
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(move || {
            let _ = toolbar_rev.get();
            editor_session.with_untracked(|s| {
                if action.is_active(s) {
                    accent_soft
                } else {
                    rgba(0x0000_0000)
                }
            })
        })
        .hover(move |s| s.bg(btn_hover))
        .tooltip(tooltip_box(tip, theme))
        .on_press(move || {
            editor_session.with_untracked(|session| {
                if let Some(cmd) = action.build_command(session) {
                    let _ = session.run(&cmd);
                    session.focus();
                }
            });
            sync_editor_to_draft(editor_session, draft_content, draft_files, active_file_idx);
            toolbar_rev.update(|v| *v = v.wrapping_add(1));
        })
        .child(label_el)
}

/// Tailwind part classes styling NSH's headless `RichEditor` for Markdown editing.
fn markdown_editor_classes(is_dark: bool) -> RichEditorClasses {
    let mut classes = RichEditorClasses::default();
    classes.root = Some("w-full h-full".into());
    classes.content = Some("p-3 gap-1.5 text-sm leading-relaxed".into());
    classes.placeholder = Some("opacity-50".into());
    classes.list_marker = Some("opacity-70 font-medium".into());
    classes.task_checkbox = Some("w-3.5 h-3.5 rounded border".into());

    let code_block_cls = if is_dark {
        "bg-zinc-900 text-zinc-100 font-mono text-xs rounded-md px-3 py-2 my-1 border border-zinc-700"
    } else {
        "bg-zinc-100 text-zinc-800 font-mono text-xs rounded-md px-3 py-2 my-1 border border-zinc-200"
    };
    let inline_code_cls = if is_dark {
        "bg-zinc-800 text-emerald-300 font-mono text-xs px-1 rounded"
    } else {
        "bg-zinc-100 text-emerald-700 font-mono text-xs px-1 rounded"
    };
    let quote_cls = if is_dark {
        "border-l-2 border-zinc-500 pl-3 py-0.5 text-zinc-300 italic my-1"
    } else {
        "border-l-2 border-zinc-300 pl-3 py-0.5 text-zinc-600 italic my-1"
    };
    let hr_cls = if is_dark {
        "border-t border-zinc-700 my-2"
    } else {
        "border-t border-zinc-200 my-2"
    };

    classes.nodes = Some(BTreeMap::from([
        ("paragraph".to_owned(), "text-sm leading-relaxed".to_owned()),
        ("heading".to_owned(), "font-bold mt-2 mb-1".to_owned()),
        (
            "heading[level=1]".to_owned(),
            "text-xl font-bold".to_owned(),
        ),
        (
            "heading[level=2]".to_owned(),
            "text-lg font-bold".to_owned(),
        ),
        (
            "heading[level=3]".to_owned(),
            "text-base font-semibold".to_owned(),
        ),
        ("blockquote".to_owned(), quote_cls.to_owned()),
        ("codeBlock".to_owned(), code_block_cls.to_owned()),
        ("bulletList".to_owned(), "pl-5 gap-0.5".to_owned()),
        ("orderedList".to_owned(), "pl-5 gap-0.5".to_owned()),
        ("taskList".to_owned(), "pl-5 gap-0.5".to_owned()),
        ("horizontalRule".to_owned(), hr_cls.to_owned()),
    ]));
    classes.marks = Some(BTreeMap::from([
        ("bold".to_owned(), "font-bold".to_owned()),
        ("italic".to_owned(), "italic".to_owned()),
        ("strike".to_owned(), "line-through opacity-75".to_owned()),
        ("code".to_owned(), inline_code_cls.to_owned()),
        ("link".to_owned(), "underline text-emerald-600".to_owned()),
    ]));
    classes
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
