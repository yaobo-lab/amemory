//! Skill domain models, frontmatter validation, filesystem management, and index document generation.

use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::DomainError;
use crate::memory::{
    DEFAULT_MAX_PART_CHARS, IndexDocument, IndexMutation, IndexTarget, content_hash_hex,
    skill_doc_id, skill_doc_prefix, split_text_into_parts,
};

const SKILL_FILE_NAME: &str = "SKILL.md";
const ALLOWED_SUBDIRS: [&str; 4] = ["scripts", "references", "assets", "agents"];
const ALLOWED_ROOT_FILES: [&str; 2] = ["LICENSE.txt", "LICENSE"];

/// Complete representation of a Skill directory on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEntry {
    /// Validated skill identifier (`name` in frontmatter and directory name).
    pub name: String,
    /// Validated skill description from frontmatter.
    pub description: String,
    /// Absolute or configured path to the skill directory.
    pub root_dir: PathBuf,
    /// Full text of `SKILL.md`.
    pub skill_markdown: String,
    /// All indexable files inside the skill directory (`SKILL.md` first).
    pub files: Vec<SkillFile>,
}

/// A single text file belonging to a [`SkillEntry`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillFile {
    /// Forward-slash relative path inside the skill directory.
    pub relative_path: String,
    /// Full UTF-8 text content of the file.
    pub content: String,
}

/// Lightweight summary of a Skill for `list_skills`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSummary {
    /// Skill identifier.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Number of text files belonging to this skill.
    pub file_count: usize,
    /// Relative paths of all files in the skill.
    pub files: Vec<String>,
}

/// Ranked skill search match for `find_skills`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMatch {
    /// Skill identifier.
    pub name: String,
    /// Skill description.
    pub description: String,
    /// Relevance score.
    pub score: usize,
    /// Preview snippet.
    pub preview: String,
    /// Relative paths of all files in the skill.
    pub files: Vec<String>,
}

/// Input file specification when creating or updating a Skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillFileInput {
    /// Relative file path under `scripts/`, `references/`, `assets/`, `agents/`, or `LICENSE(.txt)`.
    pub relative_path: String,
    /// UTF-8 file content.
    pub content: String,
}

/// Input parameters for creating a new Skill on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSkillInput {
    /// Optional skill name; if omitted, parsed from `skill_markdown` frontmatter.
    pub name: Option<String>,
    /// Optional description; used if `skill_markdown` is omitted.
    pub description: Option<String>,
    /// Optional Markdown body; used when generating `SKILL.md` from `name` + `description`.
    pub body: Option<String>,
    /// Optional complete `SKILL.md` content.
    pub skill_markdown: Option<String>,
    /// Additional resource files to write into the skill directory.
    pub files: Vec<SkillFileInput>,
}

/// Strategy for handling existing resource files when updating a Skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFileUpdatePolicy {
    /// Keep existing resource files and overwrite or add any specified `files`.
    #[default]
    Merge,
    /// Remove all existing resource files and keep only `SKILL.md` plus specified `files`.
    ReplaceAll,
}

/// Input parameters for updating an existing Skill on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSkillInput {
    /// Target skill name to update.
    pub name: String,
    /// Optional updated description when regenerating `SKILL.md`.
    pub description: Option<String>,
    /// Optional updated Markdown body when regenerating `SKILL.md`.
    pub body: Option<String>,
    /// Optional complete replacement `SKILL.md` content.
    pub skill_markdown: Option<String>,
    /// Resource files to write or overwrite.
    pub files: Vec<SkillFileInput>,
    /// Relative paths of resource files to remove.
    pub remove_files: Vec<String>,
    /// Policy for existing resource files (`Merge` or `ReplaceAll`).
    pub file_policy: SkillFileUpdatePolicy,
}

/// Report returned after creating, updating, or deleting a Skill on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMutationReport {
    /// Action label (`created`, `updated`, `deleted`).
    pub action: String,
    /// Skill name.
    pub skill_name: String,
    /// Skill directory path on disk.
    pub root_dir: String,
    /// Number of files remaining in the skill directory.
    pub file_count: usize,
    /// Number of index chunks generated for this skill.
    pub synced_skill_parts: usize,
    /// Relative paths of files in the skill directory.
    pub files: Vec<String>,
    /// Index mutation to apply to the search store.
    pub index_mutation: IndexMutation,
}

/// Filesystem-backed catalog of Skills.
#[derive(Debug, Clone)]
pub struct SkillCatalog {
    root_dir: PathBuf,
}

impl SkillCatalog {
    /// Create a [`SkillCatalog`] rooted at `root_dir`.
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    /// Return the root directory path of the skill catalog.
    #[must_use]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Scan the catalog directory and return all valid [`SkillEntry`] items sorted by name.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory reading fails or a skill directory is invalid.
    pub fn collect_entries(&self) -> Result<Vec<SkillEntry>, DomainError> {
        if !self.root_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for dir_entry in fs::read_dir(&self.root_dir)? {
            let dir_entry = dir_entry?;
            let path = dir_entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if dir_name.starts_with('.') {
                continue;
            }
            if resolve_skill_entry_file(&path, dir_name).is_none() {
                continue;
            }
            entries.push(load_skill_entry(&path)?);
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// List lightweight summaries for all valid skills in the catalog.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if scanning fails.
    pub fn list_summaries(&self) -> Result<Vec<SkillSummary>, DomainError> {
        let entries = self.collect_entries()?;
        Ok(entries
            .into_iter()
            .map(|entry| SkillSummary {
                name: entry.name,
                description: entry.description,
                file_count: entry.files.len(),
                files: entry.files.into_iter().map(|f| f.relative_path).collect(),
            })
            .collect())
    }

    /// Search skills by lexical query across name, description, file paths, and content.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `query` is empty or catalog scanning fails.
    pub fn find_matches(&self, query: &str, limit: usize) -> Result<Vec<SkillMatch>, DomainError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation("query 不能为空".to_string()));
        }

        let entries = self.collect_entries()?;
        let mut matches: Vec<SkillMatch> = entries
            .into_iter()
            .filter(|entry| !is_model_invocation_disabled(&entry.skill_markdown))
            .filter_map(|entry| {
                let score = score_skill_entry(&entry, trimmed);
                if score == 0 {
                    return None;
                }
                let preview = build_skill_preview(&entry, trimmed);
                let files = entry.files.into_iter().map(|f| f.relative_path).collect();
                Some(SkillMatch {
                    name: entry.name,
                    description: entry.description,
                    score,
                    preview,
                    files,
                })
            })
            .collect();

        matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
        matches.truncate(limit.clamp(1, 50));
        Ok(matches)
    }

    /// Load the full `SKILL.md` content and resource list for `skill_name`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `skill_name` is invalid or does not exist.
    pub fn load_skill(&self, skill_name: &str) -> Result<SkillEntry, DomainError> {
        let normalized = skill_name.trim();
        validate_skill_name(normalized)?;
        let skill_dir = self.root_dir.join(normalized);
        if !skill_dir.is_dir() {
            return Err(DomainError::NotFound(format!(
                "未找到名为 `{normalized}` 的 skill"
            )));
        }
        load_skill_entry(&skill_dir)
    }

    /// Load a specific relative file (`SKILL.md` or resource file) from `skill_name`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `skill_name` or `relative_path` is invalid or missing.
    pub fn load_resource(
        &self,
        skill_name: &str,
        relative_path: &str,
    ) -> Result<SkillFile, DomainError> {
        let entry = self.load_skill(skill_name)?;
        let clean_path = if relative_path.trim().is_empty()
            || relative_path.trim().eq_ignore_ascii_case(SKILL_FILE_NAME)
        {
            SKILL_FILE_NAME.to_string()
        } else {
            normalize_relative_skill_path(relative_path)?
        };

        entry
            .files
            .into_iter()
            .find(|f| f.relative_path == clean_path)
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "在 skill `{}` 中未找到文件 `{clean_path}`",
                    entry.name
                ))
            })
    }

    /// Create a new Skill on disk and build its corresponding [`IndexMutation`].
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if validation fails or the skill directory already exists.
    pub fn create_skill(
        &self,
        input: &CreateSkillInput,
    ) -> Result<SkillMutationReport, DomainError> {
        let skill_markdown = resolve_skill_markdown(
            input.name.as_deref(),
            input.description.as_deref(),
            input.body.as_deref(),
            input.skill_markdown.as_deref(),
        )?;
        let (parsed_name, _) = parse_skill_frontmatter(&skill_markdown)?;
        if let Some(explicit_name) = input
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            validate_skill_name(explicit_name)?;
            if explicit_name != parsed_name {
                return Err(DomainError::Validation(format!(
                    "参数 name `{explicit_name}` 与 SKILL.md frontmatter 中的 name `{parsed_name}` 不一致"
                )));
            }
        }
        validate_skill_document(&parsed_name, &skill_markdown)?;

        let normalized_files = normalize_skill_file_inputs(&input.files)?;
        fs::create_dir_all(&self.root_dir)?;
        let skill_dir = self.root_dir.join(&parsed_name);
        if skill_dir.exists() {
            return Err(DomainError::AlreadyExists(format!(
                "skill `{parsed_name}` 已存在"
            )));
        }

        fs::create_dir_all(&skill_dir)?;
        if let Err(err) = write_skill_files(&skill_dir, &skill_markdown, &normalized_files) {
            let _ = fs::remove_dir_all(&skill_dir);
            return Err(err);
        }

        let entry = load_skill_entry(&skill_dir)?;
        let upserts = build_skill_index_documents(&entry);
        let synced_parts = upserts.len();
        let files: Vec<String> = entry.files.into_iter().map(|f| f.relative_path).collect();

        Ok(SkillMutationReport {
            action: "created".to_string(),
            skill_name: entry.name.clone(),
            root_dir: skill_dir.display().to_string(),
            file_count: files.len(),
            synced_skill_parts: synced_parts,
            files,
            index_mutation: IndexMutation {
                target: IndexTarget::Skills,
                owner_prefix: Some(skill_doc_prefix(&entry.name)),
                upserts,
                delete_doc_ids: Vec::new(),
            },
        })
    }

    /// Update an existing Skill on disk and build its corresponding [`IndexMutation`].
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the skill does not exist or validation fails.
    pub fn update_skill(
        &self,
        input: &UpdateSkillInput,
    ) -> Result<SkillMutationReport, DomainError> {
        let skill_name = input.name.trim();
        validate_skill_name(skill_name)?;
        let skill_dir = self.root_dir.join(skill_name);
        if !skill_dir.is_dir() {
            return Err(DomainError::NotFound(format!(
                "skill `{skill_name}` 不存在，无法更新"
            )));
        }

        let current_entry = load_skill_entry(&skill_dir)?;
        let next_markdown = if input.skill_markdown.is_some()
            || input.description.is_some()
            || input.body.is_some()
        {
            let existing_body = extract_markdown_body(&current_entry.skill_markdown);
            resolve_skill_markdown(
                Some(skill_name),
                input
                    .description
                    .as_deref()
                    .or(Some(current_entry.description.as_str())),
                input.body.as_deref().or(Some(existing_body)),
                input.skill_markdown.as_deref(),
            )?
        } else {
            current_entry.skill_markdown.clone()
        };

        validate_skill_document(skill_name, &next_markdown)?;
        let normalized_files = normalize_skill_file_inputs(&input.files)?;
        let remove_paths: Vec<String> = input
            .remove_files
            .iter()
            .map(|p| normalize_relative_skill_path(p))
            .collect::<Result<Vec<_>, _>>()?;

        if input.file_policy == SkillFileUpdatePolicy::ReplaceAll {
            for existing in &current_entry.files {
                if existing.relative_path != SKILL_FILE_NAME {
                    let target = skill_dir.join(&existing.relative_path);
                    if target.is_file() {
                        fs::remove_file(target)?;
                    }
                }
            }
        } else {
            for rel in &remove_paths {
                let target = skill_dir.join(rel);
                if target.is_file() {
                    fs::remove_file(target)?;
                }
            }
        }

        write_skill_files(&skill_dir, &next_markdown, &normalized_files)?;
        let updated_entry = load_skill_entry(&skill_dir)?;
        let upserts = build_skill_index_documents(&updated_entry);
        let synced_parts = upserts.len();
        let files: Vec<String> = updated_entry
            .files
            .into_iter()
            .map(|f| f.relative_path)
            .collect();

        Ok(SkillMutationReport {
            action: "updated".to_string(),
            skill_name: updated_entry.name.clone(),
            root_dir: skill_dir.display().to_string(),
            file_count: files.len(),
            synced_skill_parts: synced_parts,
            files,
            index_mutation: IndexMutation {
                target: IndexTarget::Skills,
                owner_prefix: Some(skill_doc_prefix(&updated_entry.name)),
                upserts,
                delete_doc_ids: Vec::new(),
            },
        })
    }

    /// Delete a Skill directory from disk and return the [`IndexMutation`] to purge its chunks.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if `skill_name` is invalid or does not exist.
    pub fn delete_skill(&self, skill_name: &str) -> Result<SkillMutationReport, DomainError> {
        let normalized = skill_name.trim();
        validate_skill_name(normalized)?;
        let skill_dir = self.root_dir.join(normalized);
        if !skill_dir.is_dir() {
            return Err(DomainError::NotFound(format!(
                "skill `{normalized}` 不存在，无法删除"
            )));
        }

        let entry = load_skill_entry(&skill_dir)?;
        let old_docs = build_skill_index_documents(&entry);
        let delete_doc_ids: Vec<String> = old_docs.into_iter().map(|d| d.doc_id).collect();
        let removed_files: Vec<String> = entry.files.into_iter().map(|f| f.relative_path).collect();

        fs::remove_dir_all(&skill_dir)?;

        Ok(SkillMutationReport {
            action: "deleted".to_string(),
            skill_name: normalized.to_string(),
            root_dir: skill_dir.display().to_string(),
            file_count: 0,
            synced_skill_parts: 0,
            files: removed_files,
            index_mutation: IndexMutation {
                target: IndexTarget::Skills,
                owner_prefix: Some(skill_doc_prefix(normalized)),
                upserts: Vec::new(),
                delete_doc_ids,
            },
        })
    }

    /// Build all [`IndexDocument`] chunks for every valid Skill currently on disk.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if scanning fails.
    pub fn build_all_index_documents(&self) -> Result<(usize, Vec<IndexDocument>), DomainError> {
        let entries = self.collect_entries()?;
        let count = entries.len();
        let mut docs = Vec::new();
        for entry in &entries {
            docs.extend(build_skill_index_documents(entry));
        }
        Ok((count, docs))
    }
}

/// Build deterministic [`IndexDocument`] chunks for a single [`SkillEntry`].
#[must_use]
pub fn build_skill_index_documents(entry: &SkillEntry) -> Vec<IndexDocument> {
    let now = Utc::now().to_rfc3339();
    let mut docs = Vec::new();

    for file in &entry.files {
        let parts = split_text_into_parts(&file.content, DEFAULT_MAX_PART_CHARS);
        let part_total = parts.len();
        for (idx, part_text) in parts.into_iter().enumerate() {
            let part_index = idx + 1;
            let doc_id = skill_doc_id(&entry.name, &file.relative_path, part_index);
            let uri = if part_total == 1 {
                format!("skill://{}/{}", entry.name, file.relative_path)
            } else {
                format!(
                    "skill://{}/{}#part-{part_index}",
                    entry.name, file.relative_path
                )
            };
            let title = if file.relative_path == SKILL_FILE_NAME {
                format!("Skill {}", entry.name)
            } else {
                format!("Skill {} - {}", entry.name, file.relative_path)
            };
            let hash_input = format!(
                "{}\n{}\n{}\n{}",
                entry.name, entry.description, file.relative_path, part_text
            );
            let content_hash = content_hash_hex(hash_input.as_bytes());
            let tags = vec![
                "kind:skill".to_string(),
                format!("skill:{}", entry.name),
                format!("path:{}", file.relative_path),
            ];

            let mut extra = Map::new();
            extra.insert("skill_name".to_string(), Value::String(entry.name.clone()));
            extra.insert(
                "skill_description".to_string(),
                Value::String(entry.description.clone()),
            );

            docs.push(IndexDocument {
                doc_id,
                target: IndexTarget::Skills,
                title,
                uri,
                text: part_text,
                tags,
                source_kind: "skill".to_string(),
                owner_name: entry.name.clone(),
                relative_path: file.relative_path.clone(),
                part_index,
                part_total,
                content_hash,
                updated_at: now.clone(),
                extra,
            });
        }
    }

    docs
}

/// Validate that `name` is 1..=64 ASCII lowercase alphanumeric characters or single hyphens.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if `name` violates the naming rules.
pub fn validate_skill_name(name: &str) -> Result<(), DomainError> {
    if name.is_empty() || name.len() > 64 {
        return Err(DomainError::Validation(format!(
            "skill 名称 `{name}` 长度必须在 1..=64 之间"
        )));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(DomainError::Validation(format!(
            "skill 名称 `{name}` 不能以连字符开头/结尾或包含连续连字符"
        )));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(DomainError::Validation(format!(
            "skill 名称 `{name}` 仅允许使用小写字母、数字和单连字符"
        )));
    }
    Ok(())
}

/// Parse `(name, description)` from the YAML frontmatter of `SKILL.md`.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if frontmatter is missing or malformed.
pub fn parse_skill_frontmatter(markdown: &str) -> Result<(String, String), DomainError> {
    let normalized = markdown.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Err(DomainError::Validation(
            "SKILL.md 必须以 YAML frontmatter (`---`) 开头".to_string(),
        ));
    };
    let Some((frontmatter, _body)) = rest.split_once("\n---\n") else {
        return Err(DomainError::Validation(
            "SKILL.md YAML frontmatter 缺少结束分隔符 `---`".to_string(),
        ));
    };

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;

    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(DomainError::Validation(format!(
                "SKILL.md frontmatter 行格式非法: `{line}`"
            )));
        };
        let key = key.trim();
        let value = trim_yaml_scalar(value.trim());
        match key {
            "name" => name = Some(value),
            "description" => description = Some(value),
            _ => {}
        }
    }

    let name = name
        .filter(|s| !s.is_empty())
        .ok_or_else(|| DomainError::Validation("SKILL.md frontmatter 缺少 `name`".to_string()))?;
    let description = description.filter(|s| !s.is_empty()).ok_or_else(|| {
        DomainError::Validation("SKILL.md frontmatter 缺少 `description`".to_string())
    })?;

    Ok((name, description))
}

/// Validate `SKILL.md` frontmatter and body constraints against `expected_name`.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if the frontmatter or body is invalid.
pub fn validate_skill_document(expected_name: &str, markdown: &str) -> Result<String, DomainError> {
    validate_skill_name(expected_name)?;
    let (name, description) = parse_skill_frontmatter(markdown)?;
    validate_skill_name(&name)?;
    if name != expected_name {
        return Err(DomainError::Validation(format!(
            "目录名 `{expected_name}` 与 SKILL.md frontmatter 中的 name `{name}` 不一致"
        )));
    }
    if description.chars().count() > 1_024 {
        return Err(DomainError::Validation(
            "SKILL.md description 长度不能超过 1024 个字符".to_string(),
        ));
    }
    if description.contains('<') || description.contains('>') {
        return Err(DomainError::Validation(
            "SKILL.md description 不能包含尖括号 `<` 或 `>`".to_string(),
        ));
    }
    let body = extract_markdown_body(markdown).trim();
    if body.is_empty() {
        return Err(DomainError::Validation(
            "SKILL.md 正文内容不能为空".to_string(),
        ));
    }
    Ok(description)
}

/// Validate and normalize a relative resource file path inside a Skill directory.
///
/// # Errors
///
/// Returns [`DomainError::Validation`] if the path escapes the skill directory or is not in an allowed location.
pub fn normalize_relative_skill_path(raw_path: &str) -> Result<String, DomainError> {
    let trimmed = raw_path.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Err(DomainError::Validation(
            "skill 资源文件路径不能为空".to_string(),
        ));
    }
    let path = Path::new(&trimmed);
    let mut parts = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.starts_with('.') {
                    return Err(DomainError::Validation(format!(
                        "skill 资源路径 `{trimmed}` 不能包含隐藏文件或目录"
                    )));
                }
                parts.push(s.into_owned());
            }
            _ => {
                return Err(DomainError::Validation(format!(
                    "skill 资源路径 `{trimmed}` 必须是安全的相对路径"
                )));
            }
        }
    }

    if parts.is_empty() {
        return Err(DomainError::Validation(
            "skill 资源文件路径不能为空".to_string(),
        ));
    }

    if parts.len() == 1 {
        let single = &parts[0];
        if ALLOWED_ROOT_FILES.contains(&single.as_str()) {
            return Ok(single.clone());
        }
        return Err(DomainError::Validation(format!(
            "skill 根目录仅允许 {SKILL_FILE_NAME} 或 LICENSE.txt，不允许 `{single}`"
        )));
    }

    let top = &parts[0];
    if !ALLOWED_SUBDIRS.contains(&top.as_str()) {
        return Err(DomainError::Validation(format!(
            "skill 资源文件必须位于 scripts/、references/、assets/ 或 agents/ 下，非法路径 `{trimmed}`"
        )));
    }

    Ok(parts.join("/"))
}

/// Compute a lexical relevance score for `entry` against `query`.
#[must_use]
pub fn score_skill_entry(entry: &SkillEntry, query: &str) -> usize {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }

    let name_lower = entry.name.to_lowercase();
    let desc_lower = entry.description.to_lowercase();
    let body_lower = entry.skill_markdown.to_lowercase();

    let mut score = 0usize;
    if name_lower == q {
        score += 120;
    } else if name_lower.contains(&q) {
        score += 70;
    }
    if desc_lower.contains(&q) {
        score += 45;
    }
    if body_lower.contains(&q) {
        score += 25;
    }

    for token in q.split_whitespace().filter(|t| t.len() >= 2) {
        if name_lower.contains(token) {
            score += 25;
        }
        if desc_lower.contains(token) {
            score += 15;
        }
        if body_lower.contains(token) {
            score += 8;
        }
        for file in &entry.files {
            if file.relative_path.to_lowercase().contains(token) {
                score += 6;
            }
            if file.content.to_lowercase().contains(token) {
                score += 4;
            }
        }
    }

    score
}

fn build_skill_preview(entry: &SkillEntry, query: &str) -> String {
    let q_lower = query.trim().to_lowercase();
    for file in &entry.files {
        for line in file.content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.to_lowercase().contains(&q_lower) {
                return trimmed.chars().take(180).collect();
            }
        }
    }
    entry.description.chars().take(180).collect()
}

fn resolve_skill_entry_file(skill_dir: &Path, dir_name: &str) -> Option<PathBuf> {
    let primary = skill_dir.join(SKILL_FILE_NAME);
    if primary.is_file() {
        return Some(primary);
    }
    let fallback = skill_dir.join(format!("{dir_name}.md"));
    if fallback.is_file() {
        return Some(fallback);
    }
    None
}

fn is_model_invocation_disabled(markdown: &str) -> bool {
    let normalized = markdown.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return false;
    };
    let Some((frontmatter, _)) = rest.split_once("\n---\n") else {
        return false;
    };
    for raw_line in frontmatter.lines() {
        if let Some((key, value)) = raw_line.trim().split_once(':')
            && key.trim() == "disable-model-invocation"
        {
            return trim_yaml_scalar(value.trim()).eq_ignore_ascii_case("true");
        }
    }
    false
}

fn load_skill_entry(skill_dir: &Path) -> Result<SkillEntry, DomainError> {
    let dir_name = skill_dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            DomainError::Validation(format!("无法识别 skill 目录名称: {}", skill_dir.display()))
        })?;

    let skill_md_path = resolve_skill_entry_file(skill_dir, dir_name).ok_or_else(|| {
        DomainError::NotFound(format!(
            "在 skill `{dir_name}` 中未找到 `{SKILL_FILE_NAME}` 或 `{dir_name}.md`"
        ))
    })?;
    let skill_markdown = fs::read_to_string(&skill_md_path)?;
    let description = validate_skill_document(dir_name, &skill_markdown)?;

    let mut resource_files = Vec::new();
    collect_skill_files_recursive(skill_dir, skill_dir, &mut resource_files)?;
    resource_files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut files = Vec::with_capacity(1 + resource_files.len());
    files.push(SkillFile {
        relative_path: SKILL_FILE_NAME.to_string(),
        content: skill_markdown.clone(),
    });
    files.extend(resource_files);

    Ok(SkillEntry {
        name: dir_name.to_string(),
        description,
        root_dir: skill_dir.to_path_buf(),
        skill_markdown,
        files,
    })
}

fn collect_skill_files_recursive(
    root: &Path,
    current: &Path,
    out: &mut Vec<SkillFile>,
) -> Result<(), DomainError> {
    let root_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let fallback_entry_name = format!("{root_name}.md");
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with('.') || name == "__pycache__" {
            continue;
        }
        if path.is_dir() {
            if current == root && !ALLOWED_SUBDIRS.contains(&name) {
                continue;
            }
            collect_skill_files_recursive(root, &path, out)?;
        } else if path.is_file() {
            if current == root && (name == SKILL_FILE_NAME || name == fallback_entry_name) {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if normalize_relative_skill_path(&rel_str).is_ok()
                    && let Ok(content) = fs::read_to_string(&path)
                    && !content.trim().is_empty()
                {
                    out.push(SkillFile {
                        relative_path: rel_str,
                        content,
                    });
                }
            }
        }
    }
    Ok(())
}

fn resolve_skill_markdown(
    name: Option<&str>,
    description: Option<&str>,
    body: Option<&str>,
    skill_markdown: Option<&str>,
) -> Result<String, DomainError> {
    if let Some(raw_md) = skill_markdown.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(format!("{raw_md}\n"));
    }

    let name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            DomainError::Validation(
                "未提供 skill_markdown 时，必须同时提供 name 和 description".to_string(),
            )
        })?;
    let description = description
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            DomainError::Validation(
                "未提供 skill_markdown 时，必须同时提供 name 和 description".to_string(),
            )
        })?;
    validate_skill_name(name)?;

    let body_content = body
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("# {name}\n\n{description}"), str::to_string);

    let mut rendered = String::new();
    let _ = writeln!(rendered, "---");
    let _ = writeln!(rendered, "name: {name}");
    let _ = writeln!(rendered, "description: {description}");
    let _ = writeln!(rendered, "---");
    let _ = writeln!(rendered);
    let _ = writeln!(rendered, "{body_content}");
    Ok(rendered)
}

fn normalize_skill_file_inputs(
    files: &[SkillFileInput],
) -> Result<Vec<SkillFileInput>, DomainError> {
    let mut normalized = Vec::with_capacity(files.len());
    for file in files {
        let clean_path = normalize_relative_skill_path(&file.relative_path)?;
        if file.content.trim().is_empty() {
            return Err(DomainError::Validation(format!(
                "文件 `{clean_path}` 的内容不能为空"
            )));
        }
        normalized.push(SkillFileInput {
            relative_path: clean_path,
            content: file.content.clone(),
        });
    }
    Ok(normalized)
}

fn write_skill_files(
    skill_dir: &Path,
    skill_markdown: &str,
    files: &[SkillFileInput],
) -> Result<(), DomainError> {
    fs::write(skill_dir.join(SKILL_FILE_NAME), skill_markdown)?;
    for file in files {
        let target = skill_dir.join(&file.relative_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, &file.content)?;
    }
    Ok(())
}

fn extract_markdown_body(markdown: &str) -> &str {
    let normalized = markdown.trim_start();
    if let Some(rest) = normalized.strip_prefix("---\n")
        && let Some((_fm, body)) = rest.split_once("\n---\n")
    {
        return body;
    }
    if let Some(rest) = normalized.strip_prefix("---\r\n")
        && let Some((_fm, body)) = rest.split_once("\r\n---\r\n")
    {
        return body;
    }
    markdown
}

fn trim_yaml_scalar(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn skill_lifecycle_and_chunk_generation_work() {
        let dir = TempDir::new().expect("tempdir");
        let catalog = SkillCatalog::new(dir.path());

        let created = catalog
            .create_skill(&CreateSkillInput {
                name: Some("rust-refactor".to_string()),
                description: Some("Idiomatic Rust refactoring guide".to_string()),
                body: Some("# Rust Refactor\n\nUse strong types.".to_string()),
                skill_markdown: None,
                files: vec![SkillFileInput {
                    relative_path: "references/checklist.md".to_string(),
                    content: "Check clippy and rustfmt.".to_string(),
                }],
            })
            .expect("create skill");

        assert_eq!(created.skill_name, "rust-refactor");
        assert_eq!(created.file_count, 2);
        assert_eq!(created.index_mutation.upserts.len(), 2);

        let matches = catalog.find_matches("clippy", 5).expect("find matches");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "rust-refactor");

        let deleted = catalog.delete_skill("rust-refactor").expect("delete skill");
        assert_eq!(deleted.index_mutation.delete_doc_ids.len(), 2);
    }
}
