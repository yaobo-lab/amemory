# MCP 与 Amemory + XERJ 的 Agent 使用指南

本文说明什么是 MCP、当前 `amemory` 项目如何将 **Amemory 的 Skill/Book/Memory 领域能力** 与 **内嵌 XERJ 统一搜索与认知记忆引擎** 合二为一，以及 Agent 如何通过单个统一 MCP 端点调用全部工具。

---

## 1. 什么是 MCP

MCP（Model Context Protocol，模型上下文协议）是一套让 AI 应用以统一方式连接外部工具和数据源的开放协议。

MCP 的核心职责包括：
- 定义客户端和服务器之间的 JSON-RPC 2.0 消息格式；
- 协商协议版本与服务端能力（Tools）；
- 通过 `tools/list` 暴露工具名称、描述和输入 JSON Schema；
- 通过 `tools/call` 执行工具并返回文本/结构化结果或错误；
- 在 `stdio` 或 Stateless Streamable HTTP 传输层上承载会话。

---

## 2. 整体架构（4 Crates + Vendored XERJ Capsule）

`amemory` 采用单仓 Workspace + 内嵌 XERJ 胶囊（`crates/xerj`）架构，完全移除了历史上的 `memvid-core` 与单文件 `.mv2` 存储：

```text
+------+    +-------------+    +----------+    +------------+
| 用户 | <-> | Agent / LLM | <-> | MCP Host | <-> | MCP Client |
+------+    +-------------+    +----------+    +------+-----+
                                                      |
                                    Streamable HTTP (/mcp) 或 stdio
                                                      |
                                                      v
                                        +---------------------------+
                                        |   amemory (Unified MCP)   |
                                        |  18 Amemory + 20/23 XERJ  |
                                        +-------------+-------------+
                                                      |
                          +---------------------------+---------------------------+
                          |                                                       |
                          v                                                       v
            +---------------------------+                           +---------------------------+
            |   amemory-domain/adapter  |                           |     xerj-mcp Provider     |
            |  Skills / Books / Memory  |                           | Search/Graph/AutoIndex/.. |
            +-------------+-------------+                           +-------------+-------------+
                          |                                                       |
                          +---------------------------+---------------------------+
                                                      |
                                                      v
                                        +---------------------------+
                                        |   Embedded XERJ Engine    |
                                        |  ./data/xerj (3 Indices)  |
                                        +---------------------------+
```

### Workspace Crate 职责划分

| Crate 路径 | 包名 | 核心职责 |
| :--- | :--- | :--- |
| `.` (`src/`) | `amemory` | 根包二进制入口与装配根（CLI 参数解析、`toolkit-rs` 日志与 panic hook、配置加载校验 `etc/config.toml`、启动目录扫描同步及四种运行模式编排） |
| `crates/domain` | `amemory-domain` | Skill/Book 规范校验、YAML frontmatter 解析、UTF-8 清洗、稳定分块、PDF/DOCX/PPTX/XLSX 提取 |
| `crates/adapter` | `amemory-adapter` | `SearchStore` 端口与 `XerjAdapter` 实现，管理 `amemory-skills-v1`、`amemory-books-v1`、`amemory-memory-v1` 三个索引及增量同步账本 |
| `crates/mcp` | `amemory-mcp` | 统一 MCP 门面，合并 18 个 Amemory 工具与 20/23 个 XERJ 原生工具，启动期零冲突校验 |
| `crates/xerj` | `xerj-*` | Vendored XERJ 引擎源码胶囊（`v1.0.0-rc.101`），通过 `patches/xerj/` 补丁化管理 |

---

## 3. 四种运行模式（`server.mode`）

在 `etc/config.toml` 的 `[server]` 段可配置 `mode`：

| `server.mode` | 说明 | 典型场景 |
| :--- | :--- | :--- |
| `"sync"` | 扫描 `./resource/skills` 与 `./resource/books`，将增量变更同步到 `./data/xerj` 并刷盘后立即退出 | CI 预构建索引、全量重建恢复 |
| `"mcp-stdio"` | 启动内嵌 XERJ 引擎 + 本地回环 ES 监听器，在 `stdin/stdout` 上暴露统一 MCP 服务（自动强制关闭控制台日志，防止污染 `stdout`） | Claude Desktop、Cursor、Zed Command Server |
| `"mcp-http"` | 启动内嵌 XERJ 引擎 + 本地回环 ES 监听器，在 `http://127.0.0.1:8000/mcp` 暴露 Streamable HTTP MCP 服务 | Zed Remote Server、多窗口共享常驻服务 |
| `"server"` | 同时启动完整 XERJ Server（ES 兼容端口 `9200`、原生 REST `8080`、gRPC `8081`）与统一 MCP HTTP 服务（`8000/mcp`） | 需要同时使用 XERJ REST/ES API 与 MCP 的高级场景 |

---

## 4. 统一 MCP 工具清单（38 / 41 个工具）

单个 MCP Server 同时暴露 **18 个 Amemory 领域工具** 与 **20 个 XERJ 原生工具**（配置 `xerj.hub_api_key` 或环境变量 `XERJ_HUB_API_KEY` 后额外暴露 **3 个 Hub 工具**，共 **41 个**）。

### 4.1 Amemory 领域工具（18 个）

#### 基础与通用记忆（3 个）
- `ping`：健康检查，返回 `"pong"`。
- `add_chunks`：向 `amemory-memory-v1` 索引批量写入长期记忆文本块。
- `search`：在指定目标（`"memory"`、`"skills"` 或 `"books"`，默认 `"memory"`）中执行全文/混合检索。

#### Skill 管理与检索（7 个）
- `list_skills`：列出 `storage.skills` 下所有 Skill 的元数据、资源文件与自动调用策略。
- `find_skills`：根据任务描述在 `amemory-skills-v1` 索引中检索最相关的可自动调用 Skill。
- `load_skill`：按名称读取 `SKILL.md` 主说明，或通过 `resource` 读取 `references/`、`scripts/` 等附属文件。
- `add_skill`：创建新 Skill（支持直接传入 `skill_markdown` 或结构化 `name` + `description` + `body` + `files`）并立即同步索引。
- `update_skill`：更新已有 Skill 的 `SKILL.md` 或附属文件并重新同步索引。
- `delete_skill`：删除指定 Skill 目录及其在 `amemory-skills-v1` 中的全部分块。
- `sync_skills`：扫描 `./resource/skills` 目录并将新增/修改/删除增量同步至 `amemory-skills-v1`。

#### Book 电子书管理、多格式导入与检索（8 个）
- `list_books`：列出 `storage.books` 下所有电子书及章节资源。
- `find_books`：在 `amemory-books-v1` 中检索匹配的电子书与章节片段。
- `load_book`：读取指定电子书的 `BOOK.md` 或章节文件（如 `chapters/01-intro.md`）。
- `add_book`：创建新电子书目录与 `BOOK.md` 并同步至 `amemory-books-v1`。
- `update_book`：更新现有电子书的 `BOOK.md` 或章节并重新同步。
- `delete_book`：删除整本电子书目录及其在 `amemory-books-v1` 中的全部分块。
- `upload_book_file`：向指定电子书上传或覆盖 Markdown/文本章节文件并立即更新索引。
- `import_book_document`：将本地路径（受 `document_import.allowed_roots` 白名单保护，默认 `./resource/uploads`）或 Base64 编码的 **PDF、DOCX、PPTX、XLS、XLSX** 文档归档到 `resource/books/<name>/sources/`，自动提取正文生成 `chapters/<stem>.md` 并建立索引。
- `sync_books`：扫描 `./resource/books` 目录并增量同步至 `amemory-books-v1`。

### 4.2 XERJ 原生 MCP 工具（20 + 3 个）

由 `crates/mcp/src/xerj.rs` 直接复用 `xerj_mcp::McpServer` 转发至内嵌 XERJ 引擎：

- **搜索与文档操作（6 个）**：`xerj_search`、`xerj_index_document`、`xerj_get_document`、`xerj_delete_document`、`xerj_list_indices`、`xerj_status`
- **认知记忆系统（6 个）**：`xerj_memory_store`、`xerj_memory_recall`、`xerj_memory_Reflect`、`xerj_memory_contradict`、`xerj_memory_graph`、`xerj_memory_forget`
- **AutoIndex 自动索引（2 个）**：`xerj_autoindex`、`xerj_autoindex_status`
- **会话与生命周期（4 个）**：`xerj_session_start`、`xerj_session_end`、`xerj_session_status`、`xerj_checkpoint`
- **跨索引与代码分析（2 个）**：`xerj_Unified_search`、`xerj_code_impact`
- **XERJ Hub 工具（可选 3 个，配置 `hub_api_key` 后启用）**：`xerj_hub_search`、`xerj_hub_install`、`xerj_hub_publish`

---

## 5. 构建与启动

### 5.1 编译项目

要求 Rust 1.88+（Edition 2024），全仓默认采用纯 Rust TLS（`rustls` + `ring`），无需安装 OpenSSL 或 Perl：

```powershell
cargo build --release
```

编译产物位于 `target/release/amemory.exe`（Windows）或 `target/release/amemory`（Linux/macOS）。

### 5.2 配置文件 `etc/config.toml`

```toml
[server]
mode = "mcp-stdio" # 可选: "sync", "mcp-stdio", "mcp-http", "server"
host = "127.0.0.1"
port = 8000
endpoint_path = "/mcp"
allow_remote = false
max_request_body_bytes = 16777216

[storage]
skills = "./resource/skills"
books = "./resource/books"

[xerj]
data_dir = "./data/xerj"
skills_index = "amemory-skills-v1"
books_index = "amemory-books-v1"
memory_index = "amemory-memory-v1"
embed_mode = "lexical"
bind_address = "127.0.0.1"
es_compat_port = 9200
rest_port = 8080
grpc_port = 8081
# hub_api_key = "" # 可选: 也可通过环境变量 XERJ_HUB_API_KEY 设置

[document_import]
allowed_roots = ["./resource/uploads"]
max_file_bytes = 52428800

[logger]
level = "info"

[panic]
```

### 5.3 命令行启动

```powershell
# 使用默认 etc/config.toml 启动
.\target\release\amemory.exe

# 或指定自定义配置文件路径
.\target\release\amemory.exe -c etc/config.toml
```

---

## 6. 备份、恢复与索引重建 SOP

1. **文件即真相源（Source of Truth）**：
   - `./resource/skills` 与 `./resource/books` 目录下的 Markdown 及 `sources/` 原始文档是唯一持久真相源。
   - `./data/xerj` 是 XERJ 索引与增量同步账本（`.amemory-sync-state.json`）的派生存储目录。
2. **损坏恢复 / 全量重建**：
   - 当 `./data/xerj` 索引损坏、迁移机器或升级索引 Schema 时，直接删除 `./data/xerj` 目录，然后运行 `server.mode = "sync"`（或直接启动服务），`amemory` 会自动从 `./resource/skills` 和 `./resource/books` 全量重建 `amemory-skills-v1` 与 `amemory-books-v1`。
3. **认知记忆快照备份**：
   - 通过 `add_chunks` 或 `xerj_memory_store` 写入的数据存储在 `./data/xerj` 中。如需备份运行时记忆，可在停止服务后备份 `./data/xerj` 目录，或使用 `xerj_checkpoint` 创建检查点。
4. **XERJ 胶囊完整性校验与升级**：
   - 校验本地 `crates/xerj` 与补丁一致性：`powershell -ExecutionPolicy Bypass -File scripts/verify-xerj.ps1`
   - 升级上游版本并重放补丁：`powershell -ExecutionPolicy Bypass -File scripts/update-xerj.ps1 -Tag <new-tag>`
