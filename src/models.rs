/// `add_chunks` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct AddChunkRequest {
    /// 要追加到内存存储中的文本块。
    pub chunks: Vec<String>,
}

/// `search` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchToolRequest {
    /// 用于检索并排序已存储文本块的自然语言查询。
    pub query: String,
    /// 最多返回的匹配文本块数量。
    pub top_k: usize,
    /// 每条结果摘要最多包含的字符数。
    pub snippet_chars: usize,
}

/// `find_skills` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct FindSkillsRequest {
    /// 当前开发任务的自然语言描述。
    pub task: String,
    /// 最多返回的自动调用型 Skill 数量。
    pub top_k: usize,
}

/// `load_skill` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct LoadSkillRequest {
    /// 要加载的 Skill 名称。
    pub name: String,
    /// 可选资源路径；省略时加载 Skill 的主说明文件。
    pub resource: Option<String>,
}

/// `add_skill` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct AddSkillRequest {
    /// Skill 名称；必须使用小写字母、数字和单连字符，并与 frontmatter 中的名称一致。
    pub name: String,
    /// 要写入新目录中 `SKILL.md` 的完整 UTF-8 Markdown 内容。
    pub content: String,
}

/// `update_skill` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateSkillRequest {
    /// 要更新的现有 Skill 名称。
    pub name: String,
    /// 用于完整替换现有 `SKILL.md` 的 UTF-8 Markdown 内容。
    pub content: String,
}

/// `delete_skill` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteSkillRequest {
    /// 要连同全部资源文件一起永久删除的 Skill 名称。
    pub name: String,
}

/// `find_books` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct FindBooksRequest {
    /// 用于匹配书名、简介和作者的自然语言查询。
    pub query: String,
    /// 最多返回的电子书数量。
    pub top_k: usize,
}

/// `load_book` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct LoadBookRequest {
    /// 要加载的电子书名称。
    pub name: String,
    /// 可选 Markdown 资源路径；省略时加载 `BOOK.md`。
    pub resource: Option<String>,
}

/// `add_book` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct AddBookRequest {
    /// 电子书名称；必须使用小写字母、数字和单连字符。
    pub name: String,
    /// 要写入新目录中 `BOOK.md` 的完整 UTF-8 Markdown 内容。
    pub content: String,
}

/// `update_book` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateBookRequest {
    /// 要更新的现有电子书名称。
    pub name: String,
    /// 用于完整替换现有 `BOOK.md` 的 UTF-8 Markdown 内容。
    pub content: String,
}

/// `delete_book` MCP 工具接收的参数。
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteBookRequest {
    /// 要连同全部 Markdown 资源一起永久删除的电子书名称。
    pub name: String,
}
