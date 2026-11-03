//! Skill 目录扫描、Memvid 同步与精确资源读取。

use std::{
    collections::BTreeMap,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use eyre::Context;
use memvid_core::{DocMetadata, FrameStatus, Memvid, PutOptions};
use serde::{Deserialize, Serialize};

const CATALOG_URI: &str = "skills://catalog";
const SKILL_TRACK: &str = "skills";
const MAX_PART_CHARS: usize = 1_800;
const MAX_SKILL_DOCUMENT_BYTES: usize = 1024 * 1024;

/// 当前 Skill 目录的内存目录。
#[derive(Clone, Debug)]
pub struct SkillCatalog {
    root: PathBuf,
    skills: BTreeMap<String, SkillDefinition>,
}

/// 一个可通过 MCP 加载的 Skill。
#[derive(Clone, Debug)]
pub struct SkillDefinition {
    name: String,
    description: String,
    entry_file: String,
    automatic: bool,
    files: BTreeMap<String, SkillFile>,
}

#[derive(Clone, Debug)]
struct SkillFile {
    content: String,
    part_count: usize,
}

/// 将目录内容同步到 Memvid 后的统计信息。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SyncReport {
    /// 新增或更新的资源分块数。
    pub written_parts: usize,
    /// 内容未变化的资源分块数。
    pub unchanged_parts: usize,
    /// 因源文件或 Skill 已删除而移除的分块数。
    pub deleted_parts: usize,
    /// 当前可用的 Skill 数量。
    pub skill_count: usize,
}

/// 创建、更新或删除磁盘 Skill 时发生的错误。
#[derive(Debug)]
pub struct SkillMutationError {
    kind: SkillMutationErrorKind,
    message: String,
    source: Option<io::Error>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SkillMutationErrorKind {
    InvalidRequest,
    FileSystem,
}

impl SkillMutationError {
    /// 返回该错误是否由调用参数或目标 Skill 状态引起。
    #[must_use]
    pub const fn is_request_error(&self) -> bool {
        matches!(self.kind, SkillMutationErrorKind::InvalidRequest)
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self {
            kind: SkillMutationErrorKind::InvalidRequest,
            message: message.into(),
            source: None,
        }
    }

    fn file_system(operation: &str, path: &Path, source: io::Error) -> Self {
        Self {
            kind: SkillMutationErrorKind::FileSystem,
            message: format!("{operation} {}", path.display()),
            source: Some(source),
        }
    }
}

impl fmt::Display for SkillMutationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = &self.source {
            write!(formatter, "{}: {source}", self.message)
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl Error for SkillMutationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|source| source as _)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct StoredCatalog {
    version: u32,
    skills: BTreeMap<String, StoredSkill>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredSkill {
    description: String,
    entry_file: String,
    automatic: bool,
    files: BTreeMap<String, usize>,
}

impl SkillCatalog {
    /// 扫描一个目录的直接子目录，并将每个子目录解释为一个 Skill。
    ///
    /// # Errors
    ///
    /// 目录不可读取、Skill 缺少 Markdown 入口文件、入口文件不是 UTF-8，或存在重名时返回错误。
    pub fn scan(root: impl AsRef<Path>) -> eyre::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut directories = fs::read_dir(&root)
            .wrap_err_with(|| format!("reading skills directory {}", root.display()))?
            .collect::<Result<Vec<_>, _>>()?;
        directories.sort_by_key(fs::DirEntry::file_name);

        let mut skills = BTreeMap::new();
        for directory in directories {
            if !directory.file_type()?.is_dir() || is_ignored_directory(&directory.path()) {
                continue;
            }

            let definition = scan_skill_directory(&directory.path())?;
            let name = definition.name.clone();
            if skills.insert(name.clone(), definition).is_some() {
                eyre::bail!("duplicate skill name: {name}");
            }
        }

        Ok(Self { root, skills })
    }

    /// 返回 Skill 根目录。
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 按名称查找 Skill。
    #[must_use]
    pub fn skill(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name).or_else(|| {
            self.skills
                .values()
                .find(|skill| skill.name.eq_ignore_ascii_case(name))
        })
    }

    /// 按名称排序迭代所有 Skill。
    pub fn iter(&self) -> impl Iterator<Item = &SkillDefinition> {
        self.skills.values()
    }
}

impl SkillDefinition {
    /// Skill 的稳定名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 用于自动匹配开发任务的简短说明。
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// 主说明文件相对于 Skill 目录的路径。
    #[must_use]
    pub fn entry_file(&self) -> &str {
        &self.entry_file
    }

    /// 是否允许模型根据任务自动选择此 Skill。
    #[must_use]
    pub const fn automatic(&self) -> bool {
        self.automatic
    }

    /// 按路径排序迭代 Skill 中的文本资源。
    pub fn file_paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// 返回资源的分块数量。
    #[must_use]
    pub fn part_count(&self, path: &str) -> Option<usize> {
        self.files.get(path).map(|file| file.part_count)
    }
}

/// 在 Skill 根目录中创建一个新的 `SKILL.md`。
///
/// # Errors
///
/// 名称或文档不符合 Zed Skill 约束、目标目录已存在，或目录和文件无法创建时返回错误。
pub fn create_skill(root: &Path, name: &str, content: &str) -> Result<PathBuf, SkillMutationError> {
    validate_skill_name(name)?;
    validate_skill_document(name, content)?;

    let directory = root.join(name);
    match fs::symlink_metadata(&directory) {
        Ok(_) => {
            return Err(SkillMutationError::invalid(format!(
                "skill already exists: {name}"
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(SkillMutationError::file_system(
                "checking skill path",
                &directory,
                error,
            ));
        }
    }

    fs::create_dir(&directory).map_err(|error| {
        SkillMutationError::file_system("creating skill directory", &directory, error)
    })?;
    let entry = directory.join("SKILL.md");
    if let Err(error) = fs::write(&entry, content) {
        if let Err(cleanup_error) = fs::remove_dir(&directory) {
            log::warn!(
                "failed to clean up incomplete skill directory: path={}, error={cleanup_error}",
                directory.display()
            );
        }
        return Err(SkillMutationError::file_system(
            "writing skill entry",
            &entry,
            error,
        ));
    }

    Ok(directory)
}

/// 完整替换现有 Skill 的 `SKILL.md`，并保留其他资源文件。
///
/// # Errors
///
/// 名称或文档不符合 Zed Skill 约束、目标 Skill 不存在，或主文件无法写入时返回错误。
pub fn update_skill(root: &Path, name: &str, content: &str) -> Result<PathBuf, SkillMutationError> {
    validate_skill_name(name)?;
    validate_skill_document(name, content)?;

    let directory = existing_skill_directory(root, name)?;
    let entry = directory.join("SKILL.md");
    let metadata = fs::symlink_metadata(&entry).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            SkillMutationError::invalid(format!("skill has no SKILL.md entry: {name}"))
        } else {
            SkillMutationError::file_system("checking skill entry", &entry, error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SkillMutationError::invalid(format!(
            "skill entry is not a regular file: {}",
            entry.display()
        )));
    }

    fs::write(&entry, content)
        .map_err(|error| SkillMutationError::file_system("writing skill entry", &entry, error))?;
    Ok(directory)
}

/// 永久删除一个 Skill 目录及其全部资源文件。
///
/// # Errors
///
/// 名称无效、目标 Skill 不存在或目录无法删除时返回错误。
pub fn delete_skill(root: &Path, name: &str) -> Result<PathBuf, SkillMutationError> {
    validate_skill_name(name)?;
    let directory = existing_skill_directory(root, name)?;
    fs::remove_dir_all(&directory).map_err(|error| {
        SkillMutationError::file_system("deleting skill directory", &directory, error)
    })?;
    Ok(directory)
}

fn existing_skill_directory(root: &Path, name: &str) -> Result<PathBuf, SkillMutationError> {
    let directory = root.join(name);
    let metadata = fs::symlink_metadata(&directory).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            SkillMutationError::invalid(format!("unknown skill: {name}"))
        } else {
            SkillMutationError::file_system("checking skill directory", &directory, error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(SkillMutationError::invalid(format!(
            "skill path is not a regular directory: {}",
            directory.display()
        )));
    }
    Ok(directory)
}

fn validate_skill_name(name: &str) -> Result<(), SkillMutationError> {
    let bytes = name.as_bytes();
    let valid_edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !(1..=64).contains(&bytes.len())
        || !bytes.first().copied().is_some_and(valid_edge)
        || !bytes.last().copied().is_some_and(valid_edge)
        || bytes.iter().any(|byte| !valid_edge(*byte) && *byte != b'-')
        || name.contains("--")
    {
        return Err(SkillMutationError::invalid(
            "name must contain 1-64 lowercase letters or digits separated by single hyphens",
        ));
    }
    Ok(())
}

fn validate_skill_document(name: &str, content: &str) -> Result<(), SkillMutationError> {
    if content.len() > MAX_SKILL_DOCUMENT_BYTES {
        return Err(SkillMutationError::invalid(format!(
            "content exceeds the {MAX_SKILL_DOCUMENT_BYTES}-byte limit"
        )));
    }
    if content.contains('\0') {
        return Err(SkillMutationError::invalid(
            "content must not contain NUL characters",
        ));
    }

    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err(SkillMutationError::invalid(
            "content must start with a YAML frontmatter delimiter",
        ));
    }

    let mut metadata = BTreeMap::new();
    let mut closed = false;
    for line in &mut lines {
        let line = line.trim();
        if line == "---" {
            closed = true;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            metadata.insert(
                key.trim().to_owned(),
                value.trim().trim_matches(['\'', '"']).to_owned(),
            );
        }
    }
    if !closed {
        return Err(SkillMutationError::invalid(
            "content must close its YAML frontmatter with ---",
        ));
    }

    let document_name = metadata
        .get("name")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SkillMutationError::invalid("frontmatter.name is required"))?;
    if document_name != name {
        return Err(SkillMutationError::invalid(format!(
            "frontmatter.name must match the requested name '{name}'"
        )));
    }

    let description = metadata
        .get("description")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| SkillMutationError::invalid("frontmatter.description is required"))?;
    if description.chars().count() > 1024 {
        return Err(SkillMutationError::invalid(
            "frontmatter.description must not exceed 1024 characters",
        ));
    }
    if metadata
        .get("disable-model-invocation")
        .is_some_and(|value| !matches!(value.as_str(), "true" | "false"))
    {
        return Err(SkillMutationError::invalid(
            "frontmatter.disable-model-invocation must be true or false",
        ));
    }
    if !lines.any(|line| !line.trim().is_empty()) {
        return Err(SkillMutationError::invalid(
            "content must include a non-empty Markdown body",
        ));
    }

    Ok(())
}

/// 把目录快照增量同步到 Memvid。
///
/// # Errors
///
/// Memvid 帧读取、写入、删除、提交或内部目录反序列化失败时返回错误。
pub fn sync_to_memvid(memvid: &mut Memvid, catalog: &SkillCatalog) -> eyre::Result<SyncReport> {
    let previous = read_stored_catalog(memvid)?;
    let current = StoredCatalog::from(catalog);
    let mut report = SyncReport {
        skill_count: catalog.skills.len(),
        ..SyncReport::default()
    };

    for skill in catalog.skills.values() {
        let description_changed = previous.skills.get(&skill.name).is_some_and(|old| {
            old.description != skill.description
                || old.entry_file != skill.entry_file
                || old.automatic != skill.automatic
        });

        for (path, file) in &skill.files {
            for (index, part) in split_text(&file.content).iter().enumerate() {
                let uri = part_uri(&skill.name, path, index);
                let options = skill_part_options(skill, path, part);
                if upsert_frame(memvid, &uri, part.as_bytes(), options, description_changed)? {
                    report.written_parts += 1;
                } else {
                    report.unchanged_parts += 1;
                }
            }
        }
    }

    for (old_name, old_skill) in &previous.skills {
        let current_skill = current.skills.get(old_name);
        for (old_path, old_part_count) in &old_skill.files {
            let current_part_count = current_skill
                .and_then(|skill| skill.files.get(old_path))
                .copied()
                .unwrap_or(0);
            for index in current_part_count..*old_part_count {
                if delete_active_frame(memvid, &part_uri(old_name, old_path, index))? {
                    report.deleted_parts += 1;
                }
            }
        }
    }

    let serialized = serde_json::to_vec_pretty(&current)?;
    let catalog_options = PutOptions::builder()
        .track(SKILL_TRACK)
        .kind("skill-catalog")
        .uri(CATALOG_URI)
        .title("Skill catalog")
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .build();
    let catalog_changed = upsert_frame(memvid, CATALOG_URI, &serialized, catalog_options, false)?;

    if report.written_parts > 0 || report.deleted_parts > 0 || catalog_changed {
        memvid.commit().wrap_err("committing skill catalog")?;
    }

    Ok(report)
}

/// 从 Memvid 中读取并拼接一个 Skill 资源的完整内容。
///
/// # Errors
///
/// Skill 或资源不存在、对应帧无法读取，或帧内容不是 UTF-8 时返回错误。
pub fn load_resource(
    memvid: &mut Memvid,
    catalog: &SkillCatalog,
    skill_name: &str,
    resource: Option<&str>,
) -> eyre::Result<(String, String)> {
    let skill = catalog
        .skill(skill_name)
        .ok_or_else(|| eyre::eyre!("unknown skill: {skill_name}"))?;
    let path = resource.unwrap_or_else(|| skill.entry_file());
    let part_count = skill
        .part_count(path)
        .ok_or_else(|| eyre::eyre!("skill '{}' has no resource '{path}'", skill.name()))?;

    let mut content = String::new();
    for index in 0..part_count {
        let uri = part_uri(skill.name(), path, index);
        let frame = memvid
            .frame_by_uri(&uri)
            .wrap_err_with(|| format!("finding Memvid frame {uri}"))?;
        let bytes = memvid
            .frame_canonical_payload(frame.id)
            .wrap_err_with(|| format!("reading Memvid frame {uri}"))?;
        let part = String::from_utf8(bytes)
            .wrap_err_with(|| format!("skill resource is not UTF-8: {uri}"))?;
        content.push_str(&part);
    }

    Ok((path.to_owned(), content))
}

fn scan_skill_directory(directory: &Path) -> eyre::Result<SkillDefinition> {
    let directory_name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| eyre::eyre!("invalid skill directory name: {}", directory.display()))?;
    let mut paths = Vec::new();
    collect_files(directory, directory, &mut paths)?;
    paths.sort();

    let entry_file = select_entry_file(directory_name, &paths)
        .ok_or_else(|| eyre::eyre!("skill '{directory_name}' has no Markdown entry file"))?;
    let entry_path = directory.join(Path::new(&entry_file));
    let entry_content = fs::read_to_string(&entry_path)
        .wrap_err_with(|| format!("reading skill entry {}", entry_path.display()))?;
    let metadata = parse_frontmatter(&entry_content);
    let name = metadata
        .get("name")
        .cloned()
        .unwrap_or_else(|| directory_name.to_owned());
    let description = metadata
        .get("description")
        .cloned()
        .unwrap_or_else(|| first_heading(&entry_content).unwrap_or_else(|| name.clone()));
    let automatic = !metadata
        .get("disable-model-invocation")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));

    let mut files = BTreeMap::new();
    for relative_path in paths {
        let full_path = directory.join(Path::new(&relative_path));
        let bytes = fs::read(&full_path)
            .wrap_err_with(|| format!("reading skill resource {}", full_path.display()))?;
        let Ok(content) = String::from_utf8(bytes) else {
            log::warn!(
                "skipping non-UTF-8 skill resource: path={}",
                full_path.display()
            );
            continue;
        };
        let part_count = split_text(&content).len();
        files.insert(
            relative_path,
            SkillFile {
                content,
                part_count,
            },
        );
    }

    if !files.contains_key(&entry_file) {
        eyre::bail!(
            "skill entry is not a UTF-8 text file: {}",
            entry_path.display()
        );
    }

    Ok(SkillDefinition {
        name,
        description,
        entry_file,
        automatic,
        files,
    })
}

fn collect_files(root: &Path, directory: &Path, output: &mut Vec<String>) -> eyre::Result<()> {
    let mut entries = fs::read_dir(directory)
        .wrap_err_with(|| format!("reading skill directory {}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !is_ignored_directory(&path) {
                collect_files(root, &path, output)?;
            }
        } else if file_type.is_file() && !is_ignored_file(&path) {
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            output.push(relative);
        }
    }
    Ok(())
}

fn select_entry_file(directory_name: &str, paths: &[String]) -> Option<String> {
    if paths.iter().any(|path| path == "SKILL.md") {
        return Some("SKILL.md".to_owned());
    }

    let named_entry = format!("{directory_name}.md");
    if paths.iter().any(|path| path == &named_entry) {
        return Some(named_entry);
    }

    paths
        .iter()
        .find(|path| Path::new(path).extension().is_some_and(|ext| ext == "md"))
        .cloned()
}

fn parse_frontmatter(content: &str) -> BTreeMap<String, String> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return BTreeMap::new();
    }

    let mut metadata = BTreeMap::new();
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            metadata.insert(
                key.trim().to_owned(),
                value.trim().trim_matches(['\'', '"']).to_owned(),
            );
        }
    }
    metadata
}

fn first_heading(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::to_owned))
}

fn split_text(content: &str) -> Vec<String> {
    if content.is_empty() {
        return vec![String::new()];
    }

    let mut parts = Vec::new();
    let mut start = 0;
    let mut char_count = 0;
    for (index, _) in content.char_indices() {
        if char_count == MAX_PART_CHARS {
            parts.push(content[start..index].to_owned());
            start = index;
            char_count = 0;
        }
        char_count += 1;
    }
    parts.push(content[start..].to_owned());
    parts
}

fn skill_part_options(skill: &SkillDefinition, path: &str, part: &str) -> PutOptions {
    let is_markdown = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
    let metadata = DocMetadata {
        mime: Some(
            if is_markdown {
                "text/markdown"
            } else {
                "text/plain"
            }
            .to_owned(),
        ),
        ..DocMetadata::default()
    };

    let search_text = format!(
        "skill: {}\ndescription: {}\nresource: {path}\n{part}",
        skill.name, skill.description
    );
    PutOptions::builder()
        .track(SKILL_TRACK)
        .kind("skill-resource")
        .uri(part_uri(&skill.name, path, 0))
        .title(format!("{} — {path}", skill.name))
        .metadata(metadata)
        .search_text(search_text)
        .push_tag("skill")
        .push_tag(skill.name.clone())
        .label(if skill.automatic {
            "automatic"
        } else {
            "manual-only"
        })
        .auto_tag(false)
        .extract_dates(false)
        .extract_triplets(false)
        .instant_index(false)
        .build()
}

fn upsert_frame(
    memvid: &mut Memvid,
    uri: &str,
    bytes: &[u8],
    mut options: PutOptions,
    force: bool,
) -> eyre::Result<bool> {
    options.uri = Some(uri.to_owned());
    if let Ok(existing) = memvid.frame_by_uri(uri)
        && existing.status == FrameStatus::Active
    {
        let current = memvid.frame_canonical_payload(existing.id)?;
        let content_matches = current == bytes;
        if !force && content_matches {
            return Ok(false);
        }
        memvid.update_frame(existing.id, Some(bytes.to_vec()), options, None)?;
        return Ok(true);
    }

    memvid.put_bytes_with_options(bytes, options)?;
    Ok(true)
}

fn delete_active_frame(memvid: &mut Memvid, uri: &str) -> eyre::Result<bool> {
    let Ok(frame) = memvid.frame_by_uri(uri) else {
        return Ok(false);
    };
    if frame.status != FrameStatus::Active {
        return Ok(false);
    }
    memvid.delete_frame(frame.id)?;
    Ok(true)
}

fn read_stored_catalog(memvid: &mut Memvid) -> eyre::Result<StoredCatalog> {
    let Ok(frame) = memvid.frame_by_uri(CATALOG_URI) else {
        return Ok(StoredCatalog::default());
    };
    if frame.status != FrameStatus::Active {
        return Ok(StoredCatalog::default());
    }

    let bytes = memvid.frame_canonical_payload(frame.id)?;
    serde_json::from_slice(&bytes).wrap_err("parsing stored skill catalog")
}

impl From<&SkillCatalog> for StoredCatalog {
    fn from(catalog: &SkillCatalog) -> Self {
        let skills = catalog
            .skills
            .iter()
            .map(|(name, skill)| {
                let files = skill
                    .files
                    .iter()
                    .map(|(path, file)| (path.clone(), file.part_count))
                    .collect();
                (
                    name.clone(),
                    StoredSkill {
                        description: skill.description.clone(),
                        entry_file: skill.entry_file.clone(),
                        automatic: skill.automatic,
                        files,
                    },
                )
            })
            .collect();
        Self { version: 1, skills }
    }
}

fn part_uri(skill_name: &str, path: &str, index: usize) -> String {
    format!("{}part-{index:05}", part_uri_prefix(skill_name, path))
}

fn part_uri_prefix(skill_name: &str, path: &str) -> String {
    format!(
        "skill://{}/{}/",
        percent_encode(skill_name),
        percent_encode(path)
    )
}

fn percent_encode(value: &str) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

fn is_ignored_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "__pycache__" || name == ".git" || name == "target")
}

fn is_ignored_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("pyc") || extension.eq_ignore_ascii_case("pyo")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn create() -> Self {
            loop {
                let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "memvid-mcp-skills-test-{}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => panic!("creating test directory {}: {error}", path.display()),
                }
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn skill_document(name: &str, heading: &str) -> String {
        format!("---\nname: {name}\ndescription: Test Skill\n---\n\n# {heading}\n")
    }

    #[test]
    fn splits_utf8_without_losing_content() {
        let content = "中".repeat(MAX_PART_CHARS + 3);
        let parts = split_text(&content);

        assert_eq!(parts.len(), 2, "long UTF-8 text should be split");
        assert_eq!(parts.concat(), content, "split parts must round-trip");
    }

    #[test]
    fn parses_skill_frontmatter() {
        let metadata = parse_frontmatter(
            "---\nname: demo\ndescription: Demo skill\ndisable-model-invocation: true\n---\n# Demo",
        );

        assert_eq!(metadata.get("name").map(String::as_str), Some("demo"));
        assert_eq!(
            metadata.get("disable-model-invocation").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn encodes_resource_paths_as_stable_uris() {
        assert_eq!(
            part_uri("C skill", "references/a b.md", 7),
            "skill://C%20skill/references%2Fa%20b.md/part-00007"
        );
    }

    #[test]
    fn creates_updates_and_deletes_skill_files() {
        let root = TestDirectory::create();
        let initial = skill_document("demo-skill", "Initial");
        let directory = create_skill(root.path(), "demo-skill", &initial)
            .expect("valid Skill should be created");
        let resource_directory = directory.join("references");
        fs::create_dir(&resource_directory).expect("resource directory should be created");
        fs::write(resource_directory.join("notes.md"), "notes")
            .expect("resource should be written");

        let updated = skill_document("demo-skill", "Updated");
        update_skill(root.path(), "demo-skill", &updated)
            .expect("existing Skill should be updated");
        assert_eq!(
            fs::read_to_string(directory.join("SKILL.md"))
                .expect("updated entry should be readable"),
            updated,
            "update should replace the complete entry"
        );
        assert!(
            resource_directory.join("notes.md").exists(),
            "update should preserve supporting resources"
        );

        delete_skill(root.path(), "demo-skill").expect("existing Skill should be deleted");
        assert!(!directory.exists(), "delete should remove the entire Skill");
    }

    #[test]
    fn rejects_unsafe_skill_names() {
        let root = TestDirectory::create();
        let error = create_skill(
            root.path(),
            "../escape",
            &skill_document("../escape", "Unsafe"),
        )
        .expect_err("path traversal name should be rejected");

        assert!(error.is_request_error(), "unexpected error: {error}");
        assert!(
            error.to_string().contains("single hyphens"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_mismatched_frontmatter_name() {
        let root = TestDirectory::create();
        let error = create_skill(
            root.path(),
            "requested-name",
            &skill_document("different-name", "Mismatch"),
        )
        .expect_err("mismatched frontmatter name should be rejected");

        assert!(error.is_request_error(), "unexpected error: {error}");
        assert!(
            error.to_string().contains("must match"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn scans_an_empty_skill_directory() {
        let root = TestDirectory::create();
        let catalog = SkillCatalog::scan(root.path()).expect("empty directory should be valid");

        assert_eq!(catalog.iter().count(), 0, "catalog should be empty");
    }
}
