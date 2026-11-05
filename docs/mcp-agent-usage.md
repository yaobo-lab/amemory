# MCP 与本项目的 Agent 使用指南

本文说明什么是 MCP、当前 `memvid-mcp` 项目启动后如何工作，以及一个 Agent 如何通过 MCP 使用 Memvid 长期记忆。

## 1. 什么是 MCP

MCP（Model Context Protocol，模型上下文协议）是一套让 AI 应用以统一方式连接外部工具和数据源的开放协议。

它解决的问题是：不同 Agent 不需要分别适配每一种数据库、文件系统或业务服务。外部能力只要实现 MCP Server，支持 MCP 的 Agent 宿主就可以发现并调用这些能力。

MCP 本身不是大模型，也不负责决定何时调用工具。它主要负责：

- 定义客户端和服务器之间的消息格式；
- 协商双方支持的协议版本与能力；
- 发现服务器提供的工具、资源和提示词；
- 调用工具并返回结果或错误；
- 在不同传输方式上承载 JSON-RPC 消息。

MCP 常见的三种能力如下：

| 能力      | 作用                                         | 当前项目是否提供 |
| --------- | -------------------------------------------- | ---------------- |
| Tools     | 由模型选择并调用的操作，例如搜索或写入记忆   | 是               |
| Resources | 由客户端读取的数据资源，例如文件或数据库记录 | 否               |
| Prompts   | 服务器提供的可复用提示词模板                 | 否               |

当前项目是一个只提供 **Tools** 能力的 MCP Server。

## 2. MCP 中的角色

一次完整的 MCP 使用过程通常包含以下角色：

- **用户**：用自然语言向 Agent 提出任务。
- **Agent/LLM**：理解任务，判断是否需要调用外部工具。
- **MCP Host**：承载 Agent 的应用，例如 Claude Desktop、IDE 或自研 Agent 程序。
- **MCP Client**：Host 内部负责连接某个 MCP Server 的协议客户端。
- **MCP Server**：暴露工具或数据。本项目就是 MCP Server。
- **Memvid**：本项目背后的实际存储和搜索引擎。

```text
+------+    +-------------+    +----------+    +------------+
| 用户 | <-> | Agent / LLM | <-> | MCP Host | <-> | MCP Client |
+------+    +-------------+    +----------+    +------+-----+
                                                      |
                                    Streamable HTTP / stdio
                                                      |
                                                      v
                                              +---------------+
                                              |  memvid-mcp   |
                                              +-------+-------+
                                                      |
                                                      v
                                              +---------------+
                                              |  memvid-core  |
                                              +-------+-------+
                                                      |
                                                      v
                                              +---------------+
                                              |  memvid.mv2   |
                                              +---------------+
```

一般情况下，LLM 不会亲自拼接 JSON-RPC 消息。MCP Host 会把服务器提供的工具定义转换成模型能够理解的工具描述；模型选择工具后，Host 再替模型发送 MCP 请求。

## 3. 当前项目提供了什么

当前实现的主要信息如下：

| 项目           | 当前实现                                                    |
| -------------- | ----------------------------------------------------------- |
| MCP Rust SDK   | `rmcp 3.8.0`                                                |
| 传输方式       | Stateless Streamable HTTP（默认）或 stdio                   |
| 初始化协议版本 | `LATEST_WITH_INITIALIZE`，在 `rmcp 3.8.0` 中为 `2025-11-25` |
| Server 名称    | `Memvid MCP`                                                |
| Server 版本    | 当前 Cargo 包版本                                           |
| Server 能力    | Tools                                                       |
| 配置文件       | `etc/config.toml`                                           |
| 存储与 Skill   | 由 `[storage]` 配置                                         |
| 日志与轮转     | 由 `toolkit-rs` 和 `[logger]` 配置                          |

代码入口和职责如下：

| 文件             | 职责                                         |
| ---------------- | -------------------------------------------- |
| `src/main.rs`    | 初始化 toolkit logger、panic hook 并启动应用 |
| `src/config.rs`  | 通过 toolkit 读取并验证 `etc/config.toml`    |
| `src/app.rs`     | 打开或创建 `.mv2`、同步 Skill、启动 MCP      |
| `src/models.rs`  | 定义工具参数及其 JSON Schema                 |
| `src/service.rs` | 注册工具、处理工具调用并声明 Server 能力     |
| `src/skills.rs`  | 扫描、同步、匹配和完整读取 Skill 资源        |

### 3.1 `ping`

检查 MCP Server 是否能够正常响应。

参数：无。

返回示例：

```text
pong
```

### 3.2 `add_chunks`

把一组文本写入 Memvid。

参数：

```json
{
    "chunks": ["项目使用 Rust 开发", "MCP Server 通过 HTTP 与 Agent 通信"]
}
```

约束：

- `chunks` 不能为空数组；
- 每一个文本块去除首尾空白后都不能为空。

返回示例：

```text
Inserted 2 chunks. Sequences: [1, 2]
```

### 3.3 `search`

使用自然语言查询已存储的文本。

参数：

```json
{
    "query": "项目如何与 Agent 通信？",
    "top_k": 5,
    "snippet_chars": 200
}
```

字段含义：

- `query`：自然语言查询，不能为空；
- `top_k`：最多返回多少条结果，必须大于等于 1；
- `snippet_chars`：每条结果摘要最多包含的字符数。

返回示例：

```text
total_hits=1, elapsed_ms=2
- frame_id=2, title=Untitled, score=0.923
  snippet=MCP Server 通过 HTTP 与 Agent 通信
```

### 3.4 Skill 工具

- `list_skills`：列出全部 Skill、入口、资源和自动/手动调用策略。
- `find_skills`：根据开发任务匹配允许自动调用的 Skill，并从 Memvid 返回主说明片段。
- `load_skill`：按名称加载完整主说明，或通过 `resource` 加载 references/scripts 等资源。
- `add_skill`：校验并创建新的 `SKILL.md`，然后自动同步到 Memvid。
- `update_skill`：完整替换现有 `SKILL.md`，保留其他资源并自动同步。
- `delete_skill`：永久删除整个 Skill 目录及其 Memvid 帧。
- `sync_skills`：重新扫描 Skill 目录并增量同步到 Memvid。

详细的 Zed 配置和推荐调用顺序见 [Zed 中的 Memvid Skill 工作流](zed-skill-workflow.md)。

### 3.5 Markdown 电子书工具

- `list_books`：列出 `storage.books` 下所有电子书的元数据和 Markdown 资源。
- `find_books`：按名称、标题、简介和作者匹配电子书，并读取入口摘要。
- `load_book`：加载一本书的 `BOOK.md` 或指定章节资源。
- `add_book`：创建 `books/<name>/BOOK.md` 并自动同步到独立 Book track。
- `update_book`：替换现有 `BOOK.md`，保留章节并自动同步。
- `delete_book`：永久删除整本书目录、源文档和对应 Memvid 父/子帧。
- `import_book_document`：将 PDF、DOCX、PPTX、XLS 或 XLSX 归档到用户明确选择的 Book，并执行 reader 提取、富化、分块、Smart Frame 写入和索引更新。
- `sync_books`：重新扫描手工添加、修改或删除的 Markdown 与 `sources/` 文档。

一本书使用如下结构：

```text
books/rust-book/
├── BOOK.md
├── chapters/
│   ├── 01-introduction.md
│   └── 02-ownership.md
└── sources/
    ├── rust-reference.pdf
    └── examples.xlsx
```

Book 使用独立的 `books://catalog`、`book://` URI 和 `books` track，不会覆盖 Skill 数据。

调用 `import_book_document` 前，Agent 必须确认用户指定了 `book_name`；如果没有指定，先调用 `list_books` 并询问用户，不能自行选择。工具支持两种互斥来源：

```json
{
    "book_name": "voip",
    "source": {
        "type": "local_path",
        "path": "uploads/voip.pdf"
    }
}
```

`local_path` 必须位于 `document_import.allowed_roots` 白名单内。能访问附件原始字节的 MCP Client 也可以发送：

```json
{
    "book_name": "voip",
    "source": {
        "type": "base64",
        "file_name": "voip.pdf",
        "content": "JVBERi0xLjQK..."
    }
}
```

Base64 可带 `data:*;base64,` 前缀。解码后的文件不能超过 `document_import.max_file_bytes`。配置 `embedding.provider = "none"` 时仍会完成提取、富化、分块和词法索引；`openai` 从 `embedding.api_key_env` 指定的环境变量读取密钥；`local` 需要用 `--features local-embedding` 构建并预先准备 ONNX 模型和 tokenizer。

## 4. 项目启动后发生了什么

### 4.1 构建项目

项目要求 Rust 1.88 或更高版本。

```bash
cargo build --release
```

Windows 可执行文件位于：

```text
target/release/memvid-mcp.exe
```

Linux 和 macOS 可执行文件位于：

```text
target/release/memvid-mcp
```

### 4.2 直接启动

开发时可以运行：

```bash
cargo run
```

程序从 `etc/config.toml` 读取传输、存储、日志和 panic 配置。仓库默认使用 HTTP，并监听 `http://127.0.0.1:3000/mcp`。进程会持续运行并等待 HTTP MCP 请求，这是正常行为。

启动时，程序会打开 `storage.memory` 指定的 `.mv2`；仅在文件不存在时创建新存储。相对路径以进程工作目录为基准，因此应从仓库根目录启动，以便同时找到 `etc/config.toml`。

日志由 `toolkit-rs` 管理，默认同时输出到控制台和 `logs/` 下的轮转文件。日志等级、文件大小和保留数量都在 `[logger]` 中配置。

## 5. Agent 如何连接当前项目

先常驻启动 Server，再让支持 Streamable HTTP 的 MCP Host 连接配置的 endpoint：

```json
{
    "mcpServers": {
        "memvid": {
            "url": "http://127.0.0.1:3000/mcp"
        }
    }
}
```

Host 的字段名可能不同；Zed 使用 `context_servers.<name>.url`。如果修改 `server.listen`，必须同步修改 Host URL。当前 HTTP Server 仅监听回环地址且不提供认证；远程访问需要 HTTPS 反向代理和认证。

如需 stdio，把 `server.transport` 改为 `"stdio"`，并把 `logger.console` 改为 `false`，再让 Host 通过 command 启动程序。stdio 模式禁止控制台日志，因为 stdout 用于 MCP 协议。

## 6. 一次完整的协议交互

当前项目使用传统 `initialize` 生命周期。典型交互顺序如下：

```text
[MCP Host]  -- 连接 Streamable HTTP endpoint ----------> [memvid-mcp]
[MCP Host]  -- initialize ----------------------------> [memvid-mcp]
[MCP Host]  <- 协议版本、Server 信息和 Tools 能力 ------ [memvid-mcp]
[MCP Host]  -- notifications/initialized -------------> [memvid-mcp]
[MCP Host]  -- tools/list ----------------------------> [memvid-mcp]
[MCP Host]  <- 记忆与 Skill 工具及参数 Schema ---------- [memvid-mcp]

[用户]      -- 记住“项目使用 Rust 开发” --------------> [Agent / LLM]
[Agent/LLM] -- 选择 add_chunks 工具 ------------------> [MCP Host]
[MCP Host]  -- tools/call add_chunks -----------------> [memvid-mcp]
[memvid-mcp] -- 写入并提交文本 ------------------------> [memvid.mv2]
[memvid-mcp] <- 返回序列号 ---------------------------- [memvid.mv2]
[MCP Host]  <- 返回工具调用结果 ------------------------ [memvid-mcp]
[Agent/LLM] <- 返回工具结果 ---------------------------- [MCP Host]
[用户]      <- 已保存这条信息 -------------------------- [Agent / LLM]
```

### 6.1 初始化

Host 启动进程后发送 `initialize` 请求：

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": {
            "name": "example-agent",
            "version": "1.0.0"
        }
    }
}
```

Server 返回协商后的协议版本、Server 信息和能力。初始化完成后，Host 发送通知：

```json
{
    "jsonrpc": "2.0",
    "method": "notifications/initialized"
}
```

这些消息通常由 MCP SDK 自动处理，Agent 开发者不需要手写。

### 6.2 发现工具

Host 请求工具列表：

```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
}
```

Server 返回 `ping`、通用记忆工具、Skill 工具，以及 `list_books`、`find_books`、`load_book`、`add_book`、`update_book`、`delete_book`、`sync_books` 的名称、描述和输入 JSON Schema。Host 把这些信息交给 Agent，Agent 才知道有哪些工具以及参数如何填写。

### 6.3 写入记忆

当用户说：

```text
请记住：这个项目通过 Streamable HTTP 使用 MCP。
```

Agent 可以选择 `add_chunks`。Host 实际发送的请求类似：

```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
        "name": "add_chunks",
        "arguments": {
            "chunks": ["这个项目通过 Streamable HTTP 使用 MCP。"]
        }
    }
}
```

Server 验证参数，把文本写入 `memvid.mv2`，执行提交，然后返回文本结果。

### 6.4 搜索记忆

当用户随后问：

```text
这个项目通过什么方式使用 MCP？
```

Agent 可以选择 `search`。Host 发送：

```json
{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
        "name": "search",
        "arguments": {
            "query": "项目使用 MCP 的通信方式",
            "top_k": 5,
            "snippet_chars": 200
        }
    }
}
```

Server 搜索 Memvid，并把命中数量、耗时、相关度和摘要返回给 Agent。Agent 再根据工具结果组织自然语言回答。

### 6.5 健康检查

Host 或 Agent 可以调用：

```json
{
    "jsonrpc": "2.0",
    "id": 5,
    "method": "tools/call",
    "params": {
        "name": "ping",
        "arguments": {}
    }
}
```

成功时返回 `pong`。

## 7. Agent 实际看到的内容

Agent 通常不会看到 Rust 类型、`Memvid` 对象或 `memvid.mv2` 的内部格式。它主要看到 MCP Server 提供的：

1. Server 名称和使用说明；
2. 工具名称；
3. 工具描述；
4. 工具参数的 JSON Schema；
5. 每次工具调用的文本结果或错误。

因此，Agent 使用本项目的过程可以概括为：

```text
用户意图
  -> Agent 选择工具
  -> MCP Host 校验并发送 tools/call
  -> memvid-mcp 执行 Memvid 操作
  -> MCP Host 接收结果
  -> Agent 根据结果回答用户
```

Agent 是否调用工具由模型和宿主策略决定。MCP Server 只提供能力，不会主动控制 Agent。

## 8. 当前协议版本说明

`rmcp 3.8.0` 中：

- `ProtocolVersion::LATEST` 是 `2026-07-28`；
- `ProtocolVersion::LATEST_WITH_INITIALIZE` 是 `2025-11-25`。

`2026-07-28` 使用新的发现和每请求 `_meta` 协商机制，不再采用传统 `initialize` 握手。当前 Server 信息仍声明支持传统初始化生命周期，因此配置为：

```rust
ProtocolVersion::LATEST_WITH_INITIALIZE
```

这表示项目使用当前 SDK 支持的最新传统初始化协议，而不是把协议固定在旧的 `2024-11-05`。

不能只把常量改成 `LATEST` 就宣称完成了 `2026-07-28` 升级。真正迁移到该版本还需要 MCP Host 和 Server 一起采用新的发现与请求协商生命周期。

## 9. 错误和安全边界

### 9.1 参数错误

以下调用会被拒绝：

- `add_chunks` 的数组为空；
- `add_chunks` 包含空字符串或纯空白字符串；
- `search.query` 为空或只有空白；
- `search.top_k` 为 0；
- Skill 名称不符合小写字母、数字和单连字符格式；
- `add_skill` 或 `update_skill` 的内容缺少完整 frontmatter、description、正文，或 `frontmatter.name` 与请求名称不一致；
- `add_skill` 的目标已存在，或 `update_skill`/`delete_skill` 的目标不存在。

参数错误会作为 MCP/JSON-RPC 错误返回给 Host，再由 Host 呈现给 Agent。

### 9.2 本地权限

默认 HTTP Server 只监听 `127.0.0.1`，没有单独的用户认证机制。它仍继承启动进程的本地用户权限，因此能够读写该用户有权限访问的路径。不要直接暴露到公网；远程访问必须通过带认证的 HTTPS 反向代理。

Agent Host 最好在执行写入工具前提供用户授权或工具调用确认机制。尤其是 `delete_skill` 和 `delete_book` 会递归删除目标目录及全部资源，必须只在用户明确要求时调用。Skill/Book 变更接口只接受严格名称并拒绝符号链接目录，防止通过 MCP 参数越出配置的 `storage.skills` 或 `storage.books` 根目录。

### 9.3 并发访问

单个 Server 进程内部通过互斥锁保护 `Memvid`，耗时的写入和搜索放在阻塞任务中执行。

该互斥锁不能协调多个独立进程。除非已经确认 Memvid 支持跨进程并发写入，否则不要让多个 `memvid-mcp` 进程同时写同一个 `memvid.mv2`。

## 10. 常见问题排查

### Agent 找不到工具

检查：

1. `memvid-mcp` 是否已启动并打印 HTTP endpoint；
2. MCP Host 是否成功连接并完成 `initialize`；
3. Host 是否调用了 `tools/list`；
4. Host 日志中是否存在协议版本或 JSON Schema 错误。

### 启动后没有端口

检查 `etc/config.toml`：`server.transport` 应为 `"http"`，`server.sync_skills_only` 应为 `false`。成功后日志会打印 `http_port` 和完整 endpoint。

### 手动运行后一直不退出

这是预期行为。进程正在监听 HTTP MCP 请求，按 Ctrl+C 可优雅退出。

### 找不到写入的数据

检查 `etc/config.toml` 的 `storage.memory`。相对路径位于进程工作目录，而不是必然位于二进制文件目录。

### stdio 模式 MCP 消息解析失败

stdio 模式必须设置 `logger.console = false`，避免 toolkit logger 的控制台输出污染 stdout 上的 MCP 协议。

### 协议版本不兼容

当前配置面向支持传统 `initialize` 的 MCP Host，推荐请求 `2025-11-25`。如果 Host 只实现 `2026-07-28` 新生命周期，需要同时升级 Server 的生命周期处理方式。

## 11. 总结

当前项目默认是一个本地 Stateless Streamable HTTP MCP Server，也可通过配置切换为 stdio。它把 Memvid 的写入、搜索和 Skill 能力包装成 MCP 工具。

最重要的运行关系是：

```text
用户或系统服务启动 memvid-mcp
  -> MCP Host 连接 endpoint 并完成 initialize
  -> 发现通用记忆、Skill 与 Books 工具
  -> Agent 先匹配并加载相关 Skill，再按需读取电子书或调用其他工具
  -> 数据持久化到 memvid.mv2
```

对于 Agent 使用者，通常只需要完成三件事：配置 `etc/config.toml`、启动 `memvid-mcp`，并让 Agent Host 连接日志中打印的 `/mcp` endpoint。
