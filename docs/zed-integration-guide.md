# Amemory (xcat) Zed 编辑器接入与技能/电子书知识库使用指南

本文档介绍如何将本项目（**Amemory + 内嵌 XERJ 检索引擎**）接入 **Zed 编辑器**，以及如何在 Zed Agent 中检索并调用**技能（Skills）**、阅读与导入**电子书/技术文档（Books）**与**长期记忆（Memory）**。

---

## 1. 接入 Zed 编辑器（两种连接方式）

本项目编译生成的二进制文件为 `target/debug/xcat.exe`（或 Release 产物 `target/release/xcat.exe`）。它内置统一的 MCP（Model Context Protocol）服务，同时提供 **Streamable HTTP** 与 **Stdio** 两种通信模式。

### 方式 A：Streamable HTTP 模式（推荐，与 GUI/多窗口无缝共享）

1. **启动服务**（任选其一）：
   - **启动 GUI 程序**（默认模式）：
     双击运行 `xcat.exe`，GUI 启动的同时会在后台自动开启 `http://127.0.0.1:8000/mcp` 的 Streamable HTTP 服务。
   - **纯后台命令行模式**：
     在终端运行：
     ```powershell
     .\target\debug\xcat.exe --mcp-http
     ```
2. **在 Zed 中配置端点**：
   打开 Zed 的配置（全局 `settings.json` 或项目根目录下的 `.zed/settings.json`），添加如下内容：
   ```json
   {
       "context_servers": {
           "amemory": {
               "url": "http://127.0.0.1:8000/mcp"
           }
       }
   }
   ```
3. **优势**：
   - 多个 Zed 窗口、外部终端或不同 Agent 会话可同时共享同一个知识库；
   - 在 GUI 界面中做的新增、编辑、导入与同步，Zed Agent 可即时检索到最新数据。

---

### 方式 B：Stdio 子进程拉起模式（Zed 独立管理生命周期）

如果不需要常驻运行 GUI 或 HTTP 服务，可以让 Zed 直接在后台拉起 `xcat.exe` 进程：

1. **在 Zed 中配置 command**：
   在 Zed 的 `settings.json` 中配置：
   ```json
   {
       "context_servers": {
           "amemory": {
               "command": {
                   "path": "D:/my_github/amemory/target/debug/xcat.exe",
                   "args": ["--mcp-stdio"]
               }
           }
       }
   }
   ```
2. **说明**：
   - 当启动参数为 `--mcp-stdio` 时，程序会自动禁用标准输出（stdout）的控制台日志输出，确保管道 100% 专用于纯净的 JSON-RPC 2.0 数据传输。

---

## 2. 验证连接与可用工具

在 Zed 中打开 **Agent Panel**（快捷键 `Ctrl+?` 或点击右侧 Agent 图标），新建对话并输入以下测试提示词：

```text
请调用 amemory 的 ping 工具和 list_skills 工具，检查当前连接状态与已加载技能。
```

Agent 将自动调用 MCP 工具并返回当前连接的健康状态及全部可用技能。

---

## 3. 编辑器如何使用“技能”（Skills）

技能保存在项目的 `resource/skills/<skill-name>/` 目录下：
- 每个技能必须包含一个 `SKILL.md`（带 YAML Frontmatter，声明 `name`、`description` 等元数据）；
- 可包含附属目录：`references/`（参考手册）、`scripts/`（脚本工具）、`assets/` 等。

### 3.1 技能工作流机制

```text
用户提出任务 
   │
   ▼
Agent 调用 find_skills(query) 检索匹配技能
   │
   ▼
Agent 调用 load_skill(name) 加载 SKILL.md 规范全文
   │
   ▼（若需要进一步参考文档或脚本）
Agent 调用 load_skill(name, resource="references/xxx.md")
   │
   ▼
Agent 遵循规范编写/审查代码
```

### 3.2 技能相关 MCP 工具

| 工具名 | 功能说明 | 典型入参 |
| :--- | :--- | :--- |
| `find_skills` | 基于语义/全文混合检索，查找与任务最相关的可自动调用技能（自动排除 `disable-model-invocation: true` 的条目） | `query: "Rust 错误处理规范"` |
| `load_skill` | 加载指定技能的主说明 `SKILL.md`，或指定读取其附属资源 | `name: "wrust"`, `resource: "references/patterns.md"` |
| `list_skills` | 列出系统中所有技能的摘要列表及调用策略 | 无 |
| `sync_skills` | 增量扫描 `resource/skills` 并更新 XERJ 向量与倒排索引 | 无 |
| `add_skill` | 动态新增技能并写入磁盘与索引 | `name`, `description`, `body`, `files` |
| `update_skill` | 更新已有技能的文档或资源 | `name`, `skill_markdown` |
| `delete_skill` | 删除指定技能及其在索引中的分片 | `name` |

### 3.3 在 Zed Agent 对话中的使用范例

#### 场景 1：自动发现并遵循技能规范编写代码
> **用户提示词**：
> “先调用 amemory 的 `find_skills` 检索符合当前任务的编程规范，再用 `load_skill` 加载规范内容。然后帮我用 Rust 编写一个带重试机制的异步 HTTP 请求客户端。”

#### 场景 2：直接指定调用特定技能
> **用户提示词**：
> “调用 `load_skill` 加载 `wrust` 技能，严格按照其中的工程规范审阅 `src/services/updater.rs` 文件，指出潜在的不符合项并给出修复建议。”

#### 场景 3：调用手动型技能（如 C 语言注释整理）
> **用户提示词**：
> “调用 `load_skill` 加载 `ccomments` 技能，根据其 Doxygen 规范规则为指定 C 头文件补充完整的文档注释。”

---

## 4. 编辑器如何使用“知识库与电子书”（Books / Documents）

电子书与技术文档统一保存在 `resource/books/<book-slug>/` 目录下：
- 每本书以 `BOOK.md` 为主目录/结构定义；
- 包含 `chapters/`（核心章节）、`notes/`（读书笔记）、`summaries/`（提炼总结）；
- 支持直接导入外部多格式技术资料（PDF、Word、Excel、PPT、Markdown、TXT）。

### 4.1 电子书与文档相关 MCP 工具

| 工具名 | 功能说明 | 典型入参 |
| :--- | :--- | :--- |
| `find_books` | 在电子书库中执行**语义 + 词法混合检索**，返回高相关度内容切片及来源章节 | `query: "分布式事务 最终一致性"` |
| `load_book` | 读取指定电子书的 `BOOK.md` 或章节正文 | `name: "rust-book"`, `resource: "chapters/01-ownership.md"` |
| `list_books` | 列出当前已导入的电子书总览、章节数量与文档列表 | 无 |
| `import_book_document` | 将外部 PDF/Word/Excel/PPT/MD 导入到指定书籍中，提取文本、切分块并自动索引 | `book_name: "architecture"`, `source_path: "D:/docs/spec.pdf"` |
| `sync_books` | 增量扫描 `resource/books` 目录更新 XERJ 索引 | 无 |
| `add_book` / `update_book` | 新建或更新电子书元数据与章节内容 | `name`, `description`, `chapters` |
| `delete_book` | 删除电子书及其所有章节与索引 | `name` |

### 4.2 在 Zed Agent 对话中的使用范例

#### 场景 1：知识库提问与精准定位
> **用户提示词**：
> “调用 amemory 的 `find_books` 检索知识库中关于‘XERJ 向量检索与 HNSW 索引重构’的章节，提炼其核心原理并向我汇报。”

#### 场景 2：将外部技术资料导入知识库
> **用户提示词**：
> “调用 `import_book_document`，将位于 `D:/my_docs/api-guide.pdf` 的接口文档导入到电子书 `system-design` 中，完成分块提取与索引建立。”

#### 场景 3：整书/整章阅读与辅助重构
> **用户提示词**：
> “调用 `load_book(name="rust-guide", resource="chapters/concurrency.md")`，阅读并发章节内容，参照该章节推荐的 Actor 模式指导我重构当前的后台任务调度系统。”

---

## 5. 通用认知记忆（Memory）的跨会话能力

除了静态的 Skills 和 Books 外，Zed Agent 还可以将开发过程中的**架构决策、Debug 排查经验、临时约定**沉淀到长期记忆库中：

| 工具名 | 用途 |
| :--- | :--- |
| `add_chunks` | 批量将长期记忆分片存入 `amemory-memory-v1` 索引 |
| `search` | 跨目标检索，可选检索 `memory`、`skills` 或 `books` |
| `xerj_memory_store` | XERJ 原生记忆存储工具，支持命名空间与重要度评分 |
| `xerj_memory_recall` | 按相似度与时间衰减召回长期记忆 |

> **用户提示词示例**：
> “请调用 `add_chunks` 把我们刚才排查‘Windows 进程占用导致可执行文件无法替换’的排查结论与解决方案记录到记忆库中，以便后续遇到类似问题时自动召回。”

---

## 6. 常见问题与日常维护

1. **修改了磁盘上的 Skill 或 Book 后的生效机制**：
   - 修改 `resource/skills` 或 `resource/books` 文件后，无需重启服务，只需在 Zed 对话中让 Agent 执行 `sync_skills` 或 `sync_books`，系统会在后台通过 FNV-1a 增量算法快速完成差量索引更新。
   - 也可以直接在 GUI 主窗口右上角点击“同步”按钮（`\u{e782}`）完成同步。
2. **端口冲突处理**：
   - 默认端口为 `8000`。若需更改，修改 `etc/config.toml` 下的 `[server].port`，并同步更新 Zed 中的 `url`。
3. **索引重建（冷启动恢复）**：
   - 若索引数据损坏或需要重新计算所有分块，清空 `data/xerj` 目录后启动程序，系统会自动依据源文件完成 100% 全量重建。
