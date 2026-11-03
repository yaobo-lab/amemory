# Zed 中的 Memvid Skill 工作流

本项目会把 `skills/` 下每个 Skill 的主说明、references、scripts、agents 配置及其他 UTF-8 文本资源同步到 `memvid.mv2`，再通过 stdio 或 Stateless Streamable HTTP MCP 提供给 Zed Agent。

## 数据流

```text
skills 目录
    |
    v
启动时扫描 / sync_skills
    |
    v
按 1800 字符稳定分块
    |
    v
+-------------+
| memvid.mv2 |
+-------------+
    ^       ^
    |       |
find_skills load_skill
    ^       ^
    |       |
    +---+---+
        |
    Zed Agent
        |
        v
主说明或引用资源
```

每个资源分块使用稳定 URI。同步会比较现有载荷：未变化的分块不会重写；源文件缩短、删除或整个 Skill 删除时，旧分块会被标记删除。全局 Skill 目录也保存在 Memvid 中。

## 启动与 Zed 配置

所有运行配置统一存放在 `etc/config.toml`，程序通过 `toolkit-rs` 读取。仓库默认配置使用 Stateless Streamable HTTP：

```toml
[server]
transport = "http"
listen = "127.0.0.1:3000"
sync_skills_only = false

[storage]
memory = "memvid.mv2"
skills = "skills"
books = "books"
```

先构建并启动 release 程序：

```powershell
cargo build --release
.\target\release\memvid-mcp.exe
```

HTTP 配置只负责让 Zed 连接现有 URL，**不会自动启动**程序。请保持该终端或后台服务持续运行。启动成功时会看到：

```text
skills loaded: memory=memvid.mv2, skills=skills, loaded_skills=5, written_parts=0, unchanged_parts=54, deleted_parts=0
books loaded: memory=memvid.mv2, books=books, loaded_books=0, written_parts=0, unchanged_parts=0, deleted_parts=0
memory statistics: memory=memvid.mv2, size_bytes=..., total_frames=..., active_frames=..., inactive_frames=..., payload_bytes=..., logical_bytes=..., saved_bytes=..., savings_percent=...%, vectors=..., indexes=lex:true,vec:false,time:false,clip:false
MCP server ready: transport=http, bind_address=127.0.0.1:3000, http_port=3000, endpoint=http://127.0.0.1:3000/mcp, loaded_skills=5, loaded_books=0
```

日志等级、控制台输出、文件目录和轮转策略在 `etc/config.toml` 的 `[logger]` 中配置。日志文件默认写入 `logs/`。HTTP Server 当前没有认证，并且只允许监听回环地址；不要配置 `0.0.0.0` 或直接暴露到公网。

### Zed HTTP 配置

仓库内的 `.zed/settings.json` 已配置：

```json
{
    "context_servers": {
        "memvid-skills": {
            "url": "http://127.0.0.1:3000/mcp"
        }
    }
}
```

也可以通过设置页面添加：

1. 打开 **Settings → AI → MCP Servers**。
2. 点击 **Add Server**，选择 **Add Remote Server**。
3. 服务器名称填写 `memvid-skills`。
4. 传输类型选择 `HTTP`。
5. URL 填写 `http://127.0.0.1:3000/mcp`。
6. 自定义 Headers 留空。
7. 保存后确认状态指示点变为绿色，再新建 Agent Thread。

URL 必须包含 `/mcp`。如果修改 `server.listen` 的端口，也必须同步修改 `.zed/settings.json` 中的 URL。

远程部署必须由反向代理提供 HTTPS、认证和访问控制。通过反向代理增加 Bearer Token 后，可在 Zed 条目中增加：

```json
"headers": {
    "Authorization": "Bearer <token>"
}
```

## 当前 Zed 使用步骤

### 1. 连接 MCP Server

1. 使用 Zed 打开本仓库，并确认信任当前项目。
2. 在仓库根目录启动 `target/release/memvid-mcp.exe`。
3. 确认日志打印了 `MCP server ready` 和完整 `/mcp` endpoint。
4. 打开 **Settings → AI → MCP Servers**。
5. 确认 `memvid-skills` 状态为绿色。
6. 新建一个 Agent Thread，使新线程重新获取 MCP 工具列表。

```text
memvid-mcp 读取 etc/config.toml
      |
      v
监听 http://127.0.0.1:3000/mcp
      |
      v
Zed 读取 .zed/settings.json 中的 url
      |
      v
Agent 获取并调用 MCP 工具
```

### 2. 验证工具是否可用

在新的 Agent Thread 中输入：

```text
调用 memvid-skills 的 list_skills，列出当前所有 Skill。
```

如果 Agent 返回 Skill 名称、描述和资源列表，说明 Zed 已成功接入 MCP。

### 3. 在开发任务中调用 Skill

建议在任务提示中明确要求先查找和加载相关 Skill：

```text
先使用 memvid-skills 的 find_skills 查找与当前任务相关的 Skill，
加载并遵守所有相关 Skill 后，再开始修改代码。

任务：根据设计图新增一个 LVGL 设备页面。
```

Agent 的预期执行过程如下：

```text
开发任务
   |
   v
find_skills
   |
   v
load_skill 加载主说明
   |
   v
按主说明加载 references / scripts
   |
   v
遵守 Skill 完成修改与验证
```

MCP Server 会在初始化 instructions 中建议该流程，但 MCP 协议不能绝对强制模型调用工具。提示词中明确写出“先使用 `memvid-skills`”通常更可靠。

### 4. 显式加载手动型 Skill

标记了 `disable-model-invocation: true` 的 Skill 不会由 `find_skills` 自动选中，需要在提示中明确指定。例如：

```text
调用 memvid-skills 的 load_skill 加载 ccomments，
严格遵守该 Skill 的说明，整理指定目录中的 C 函数注释。
```

### 5. 修改 Skill 后重新同步

修改 `skills/` 中的文件后，可在 Agent Thread 中输入：

```text
调用 memvid-skills 的 sync_skills。
```

同步是增量执行的：未变化的分块不会重写，新增、修改和删除的资源会更新到 `memvid.mv2`。如需启动时同步后立即退出，把 `etc/config.toml` 中的 `server.sync_skills_only` 临时改为 `true`，运行程序后再恢复为 `false`。

### 6. MCP 未连接时检查

如果 `memvid-skills` 未显示绿色，依次检查：

1. 当前项目是否已被 Zed 信任。
2. `target/release/memvid-mcp.exe` 是否存在；不存在时重新执行 `cargo build --release`。
3. 是否从仓库根目录启动，使程序能够找到 `etc/config.toml`。
4. `server.transport` 是否为 `"http"`，且 `server.sync_skills_only` 是否为 `false`。
5. 程序是否仍在运行；关闭启动终端会同时停止 Server。
6. 启动日志是否包含 `MCP server ready`、`http_port=3000` 和完整 endpoint。
7. `.zed/settings.json` 的 URL 是否与 `server.listen` 端口一致，并包含末尾 `/mcp`。
8. 端口是否被其他程序占用。可执行 `Test-NetConnection 127.0.0.1 -Port 3000`。
9. 本地无认证连接的 Headers 是否留空。
10. 是否有另一个进程正在打开同一个 `storage.memory` 文件。

如果 3000 端口不可用，把 `server.listen` 改为 `"127.0.0.1:3001"`，并把 Zed URL 同步改为 `http://127.0.0.1:3001/mcp`。

如果 Server 已连接但 Agent 没有主动调用 Skill，可明确要求先调用 `find_skills`，或为 Zed 创建只启用 Skill MCP 工具的 Agent profile。

## MCP 工具

### `list_skills`

列出全部 Skill、描述、入口文件、资源和调用策略。

### `find_skills`

输入当前开发任务和最大结果数。该工具根据名称、英文关键词和中文二元词组匹配 Skill，并从 Memvid 读取主说明片段作为结果依据。

标记了 `disable-model-invocation: true` 的 Skill 不参与自动匹配。

### `load_skill`

按名称加载 Skill：

```json
{
    "name": "lvgl-maker",
    "resource": null
}
```

`resource` 省略或为 `null` 时加载主说明。主说明要求读取引用文件时，再按相对路径加载：

```json
{
    "name": "embedded-layer-architecture",
    "resource": "references/architecture.md"
}
```

返回内容来自 `memvid.mv2`，不是在调用时直接读取源文件。

### `add_skill`

把完整的 `SKILL.md` 写入新的 Skill 目录，并立即同步到 Memvid：

```json
{
    "name": "rust-review",
    "content": "---\nname: rust-review\ndescription: 审查 Rust 代码的正确性与可维护性。\n---\n\n# Rust 代码审查\n\n检查用户指定的 Rust 改动并报告可操作的问题。\n"
}
```

名称必须由 1–64 个小写字母、数字和单连字符组成，并与 `frontmatter.name` 完全一致。目标目录已存在时调用失败，不会覆盖。

### `update_skill`

参数与 `add_skill` 相同。该工具完整替换现有 `SKILL.md`，保留同目录中的 `references/`、`scripts/` 和其他资源，随后立即同步 Memvid。目标 Skill 不存在时调用失败。

### `delete_skill`

永久删除整个 Skill 目录及其中所有资源，并从 Memvid 删除对应帧：

```json
{
    "name": "rust-review"
}
```

这是破坏性操作。调用前应确认名称和用户意图；删除成功后不需要再调用 `sync_skills`。

### `sync_skills`

修改 `skills/` 后，在当前 MCP 进程中重新扫描并增量同步。Server 启动时也会自动同步一次。

### Books 工具

Markdown 电子书存放在 `books/<name>/`，固定入口为 `BOOK.md`，可增加 `chapters/*.md` 等资源。支持：

- `list_books`：列出书名、标题、简介、作者及 Markdown 资源。
- `find_books`：根据名称、标题、简介和作者匹配电子书，并返回入口摘要。
- `load_book`：加载 `BOOK.md` 或 `list_books` 返回的指定章节路径。
- `add_book`：创建 `books/<name>/BOOK.md` 并自动同步。
- `update_book`：替换 `BOOK.md`，保留章节等其他文件并自动同步。
- `delete_book`：永久删除整本书目录和对应 Memvid 帧。
- `sync_books`：重新扫描 `books/` 并增量同步手工修改。

`BOOK.md` 示例：

```markdown
---
name: rust-book
title: Rust 程序设计语言
description: 个人阅读笔记
author: Example Author
---

# Rust 程序设计语言

电子书简介和章节索引。
```

Book 名称必须由 1–64 个小写字母、数字和单连字符组成，并与 frontmatter 的 `name` 一致。`delete_book` 会删除目录内所有文件，调用前必须确认用户意图。

## 建议的 Agent 调用顺序

1. 收到实现、重构、审查、调试或文档任务。
2. 调用 `find_skills`。
3. 对每个相关结果调用 `load_skill`。
4. 根据主说明按需加载 references 或 scripts。
5. 遵守加载的规则完成开发和验证。
6. 如果修改了 Skill 源文件，调用 `sync_skills`。

MCP Server 的初始化 instructions 已声明上述顺序，但 MCP 不能强制模型调用工具。若模型不稳定，可在提示词中明确写“先使用 `memvid-skills` 查找并加载相关 Skill”，或创建只启用这些 MCP 工具的 Zed Agent profile。

## 手动调用型 Skill

`disable-model-invocation: true` 表示不能仅因任务相似而自动加载。例如 `ccomments` 和 `check-c-arch` 需要用户显式点名或使用对应命令。它们仍会完整存入 Memvid，也可以通过 `load_skill` 精确读取。

## `etc/config.toml` 配置

| 配置项                    | 默认值           | 用途                                 |
| ------------------------- | ---------------- | ------------------------------------ |
| `server.transport`        | `http`           | 选择 `stdio` 或 `http`               |
| `server.listen`           | `127.0.0.1:3000` | 指定 HTTP 回环监听地址               |
| `server.sync_skills_only` | `false`          | 同步后退出，不启动 MCP               |
| `storage.memory`          | `memvid.mv2`     | 指定 Memvid 文件                     |
| `storage.skills`          | `skills`         | 指定 Skill 根目录                    |
| `storage.books`           | `books`          | 指定 Markdown 电子书根目录           |
| `logger.level`            | `3`              | 日志等级，1 到 5 对应 error 到 trace |
| `logger.console`          | `true`           | 是否同时输出到控制台                 |
| `logger.dir`              | `logs`           | 轮转日志目录                         |
| `logger.size_mb`          | `2`              | 单个日志文件大小                     |
| `panic.exit`              | `true`           | panic 写入 `panic.log` 后是否退出    |

stdio 模式必须同时把 `logger.console` 设为 `false`，否则控制台日志会污染 stdout 上的 MCP 协议。已有 `.mv2` 使用 `Memvid::open` 打开；仅当文件不存在时才调用 `Memvid::create`。
