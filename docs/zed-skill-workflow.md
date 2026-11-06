# Zed 中的 Amemory + XERJ Skill 工作流

本项目将 `resource/skills/` 与 `resource/books/` 下的 Markdown 规范、参考文档、脚本及多格式电子书增量同步到内嵌 **XERJ 引擎**（`./data/xerj`），并通过 `stdio` 或 Stateless Streamable HTTP（`/mcp`）向 Zed Agent 暴露统一的 **38/41 个 MCP 工具**。

---

## 1. 数据流与索引隔离

```text
./resource/skills 目录          ./resource/books 目录
      |                               |
      v                               v
启动时扫描 / sync_skills        启动时扫描 / sync_books
      |                               |
      v                               v
按 1800 字符稳定分块            按 1800 字符稳定分块 + 文档提取
      |                               |
      +---------------+---------------+
                      |
                      v
      +-------------------------------+
      |   内嵌 XERJ 引擎 (./data/xerj) |
      |  - amemory-skills-v1          |
      |  - amemory-books-v1           |
      |  - amemory-memory-v1          |
      +---------------+---------------+
                      ^
                      |
        find_skills / load_skill / xerj_search / xerj_memory_*
                      ^
                      |
                  Zed Agent
```

- **增量同步账本**：`./data/xerj/.amemory-sync-state.json` 记录每个 Skill/Book 的内容哈希（FNV-1a）与已写入文档 ID 列表。内容未变化的条目在启动或 `sync_*` 时直接跳过；缩短或删除的资源会自动清理旧分块。
- **三索引隔离**：Skills（`amemory-skills-v1`）、Books（`amemory-books-v1`）与通用记忆（`amemory-memory-v1`）相互独立，互不污染。

---

## 2. 启动与 Zed 配置

### 方式 A：Streamable HTTP 模式（推荐多窗口共享）

1. 将 `etc/config.toml` 中的 `server.mode` 设为 `"mcp-http"`（或 `"server"`）：
   ```toml
   [server]
   mode = "mcp-http"
   host = "127.0.0.1"
   port = 8000
   endpoint_path = "/mcp"
   ```
2. 构建并常驻运行 `amemory`：
   ```powershell
   cargo build --release
   .\target\release\amemory.exe
   ```
3. 仓库内 `.zed/settings.json` 已预置对应端点：
   ```json
   {
       "context_servers": {
           "amemory": {
               "url": "http://127.0.0.1:8000/mcp"
           }
       }
   }
   ```

### 方式 B：Stdio 模式（由 Zed 自动拉起进程）

1. 保持 `etc/config.toml` 默认的 `mode = "mcp-stdio"`（此时 `amemory` 会自动强制 `logger.console = false`，确保 `stdout` 仅用于 MCP JSON-RPC 通信）。
2. 在 Zed 的 `settings.json` 中配置 `command`：
   ```json
   {
       "context_servers": {
           "amemory": {
               "command": {
                   "path": "D:/my_github/amemory/target/release/amemory.exe",
                   "args": ["-c", "D:/my_github/amemory/etc/config.toml"]
               }
           }
       }
   }
   ```

---

## 3. 在 Zed Agent 中使用 Skill 与 XERJ 工具

### 3.1 验证连接
在 Zed 新建 Agent Thread，输入：
```text
调用 amemory 的 list_skills 和 xerj_status，确认技能库与内嵌 XERJ 引擎状态。
```

### 3.2 开发任务中自动匹配并加载 Skill
```text
先使用 amemory 的 find_skills 查找与当前任务相关的 Skill，
调用 load_skill 加载并遵守相关规范后再修改代码。
```

Agent 的标准调用顺序：
1. `find_skills`：基于 XERJ 检索匹配的自动调用型 Skill（排除 `disable-model-invocation: true` 的手动型 Skill）。
2. `load_skill`：加载目标 Skill 的 `SKILL.md` 全文。
3. `load_skill`（带 `resource` 参数）：按需加载 `references/*.md` 或 `scripts/*`。
4. 修改磁盘上的 `resource/skills/` 后，调用 `sync_skills` 增量更新索引。

### 3.3 显式加载手动型 Skill
标记了 `disable-model-invocation: true` 的 Skill（如 `ccomments`）不会出现在 `find_skills` 的自动推荐中，可在提示词中直接点名：
```text
调用 amemory 的 load_skill 加载 ccomments，按说明整理指定目录的 C 注释。
```

### 3.4 结合 XERJ 认知记忆与电子书检索
除了 Skill 工具外，Zed Agent 还可以在同一连接上直接调用：
- `find_books` / `load_book` / `import_book_document`：检索或导入 PDF/DOCX/PPTX/XLSX 技术文档到电子书库。
- `xerj_memory_store` / `xerj_memory_recall` / `xerj_memory_graph`：跨会话记录架构决策、排查结论与知识图谱关系。

---

## 4. 常见问题与索引恢复

- **修改 `resource/skills/` 或 `resource/books/` 后未生效**：在 Agent 中调用 `sync_skills` 或 `sync_books`。
- **索引目录损坏或需要干净重建**：停止 `amemory` 进程，删除 `./data/xerj` 目录，重新启动 `amemory`（或以 `mode = "sync"` 运行一次），系统会从 `./resource/skills` 与 `./resource/books` 源文件自动全量重建索引。
- **HTTP 端口冲突**：修改 `etc/config.toml` 中的 `server.port`（默认 `8000`），并同步更新 `.zed/settings.json` 中的 URL。
