# Memvid MCP Server

Unofficial Model Context Protocol (MCP) server interface to [Memvid](https://github.com/memvid/memvid) - a video-frame memory system for AI agents.

## What is Memvid?

Memvid is a memory layer for AI agents that stores knowledge as compressed "Smart Frames" in a single `.mv2` file (similar to how video stores frames). This approach provides:

- **10x more storage efficiency** than traditional vector databases
- **Sub-5ms memory access times**
- **Append-only, crash-safe storage** with time-travel capabilities
- **Single portable file** containing data, embeddings, search structure, and metadata
- **No external database or infrastructure required**

## Features

- **Persistent long-term memory** for AI agents
- **Semantic search** with natural language queries
- **Instant retrieval** from a single portable file
- **Serverless architecture** - no database infrastructure needed
- **Model-agnostic** - works with any LLM that supports MCP
- **Development Skill catalog** - synchronizes complete Skill packages from `skills/` into Memvid
- **Task-aware Skill loading** - lets an Agent match a task, load instructions, and fetch referenced resources on demand
- **Electronic-book document ingestion** - imports PDF, DOCX, PPTX, XLS, and XLSX into a user-selected Book through Memvid readers, enrichment, chunking, Smart Frames, and indexes

## Installation

### From Cargo (Rust)

```bash
cargo install --git https://github.com/mystique09/memvid-mcp.git
```

### From Source

```bash
git clone https://github.com/mystique09/memvid-mcp.git
cd memvid-mcp
cargo build --release
# The binary will be at target/release/memvid-mcp.exe (Windows) or target/release/memvid-mcp (Linux/Mac)
```

## Configuration

### Application configuration

All runtime settings are stored in [`etc/config.toml`](etc/config.toml) and loaded
through `toolkit-rs`. The checked-in configuration uses stateless Streamable HTTP:

```toml
[server]
transport = "http"
listen = "127.0.0.1:3000"
sync_skills_only = false

[storage]
memory = "memvid.mv2"
skills = "skills"
books = "books"

[document_import]
max_file_bytes = 33554432
allowed_roots = ["uploads"]

[embedding]
provider = "none" # none, local, or openai
local_model = "bge-small-en-v1.5"
openai_model = "text-embedding-3-small"
models_dir = "models/text"
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com/v1"
timeout_secs = 30
max_retries = 3
```

Start the server without command-line arguments:

```bash
memvid-mcp
```

The configured endpoint is `http://127.0.0.1:3000/mcp`. To use stdio, change
`server.transport` to `"stdio"` and set `logger.console = false`, because stdout
carries the MCP protocol in that mode.

Configure an HTTP-capable MCP client with:

```text
Server URL: http://127.0.0.1:3000/mcp
Transport:  HTTP / Streamable HTTP
Headers:    none
```

The built-in HTTP listener intentionally accepts only loopback addresses and
does not provide authentication or TLS. For remote access, place an authenticated
HTTPS reverse proxy on the same machine and forward requests to the local `/mcp`
endpoint.

### Zed

This repository includes `.zed/settings.json`, which connects `memvid-skills` to `http://127.0.0.1:3000/mcp`. Build and start the server first:

```bash
cargo build --release
./target/release/memvid-mcp
```

On Windows, run `target\\release\\memvid-mcp.exe`. Keep the process running, then trust/reopen the project in Zed and check **Settings → AI → MCP Servers**. `memvid-skills` should show a green status indicator. Storage, transport, logging, and panic behavior are configured in `etc/config.toml`.

During development, the intended Agent flow is:

1. Call `find_skills` with the current task.
2. Call `load_skill` for every relevant result before changing code.
3. When the loaded instructions reference another file, call `load_skill` again with its `resource` path.
4. After editing files under `skills/`, call `sync_skills` to refresh Memvid.

Skills containing `disable-model-invocation: true` are listed as `manual-only`. `find_skills` excludes them; they remain available through an explicit `load_skill` call when the user names them.

> MCP exposes tools and guidance, but the selected model ultimately decides whether to invoke a tool. Mentioning `memvid-skills` in the prompt or enabling only its Skill tools in a Zed Agent profile makes invocation more reliable.

### Continue

Add to your `mcpServers/new-mcp-server.yaml`:

```yaml
mcpServers:
    - name: Memvid MCP
      type: stdio
      command: memvid-mcp
```

### Claude Desktop

Add to your Claude Desktop config (`claude_desktop_config.json`):

```json
{
    "mcpServers": {
        "memvid": {
            "command": "memvid-mcp",
            "env": {}
        }
    }
}
```

### Cline (VS Code Extension)

Add to your Cline settings:

```json
{
    "mcpServers": [
        {
            "name": "memvid",
            "command": "memvid-mcp"
        }
    ]
}
```

## Available Tools

| Tool                   | Purpose           | Description                                                                                |
| ---------------------- | ----------------- | ------------------------------------------------------------------------------------------ |
| `ping`                 | Health check      | Returns `pong` to verify the server is running                                             |
| `list_skills`          | Inspect Skills    | Lists names, descriptions, invocation policies, entry files, and resources                 |
| `find_skills`          | Select Skills     | Matches a development task against Skills that permit automatic invocation                 |
| `load_skill`           | Activate a Skill  | Loads complete instructions or one referenced resource from Memvid                         |
| `add_skill`            | Create a Skill    | Writes a validated `SKILL.md` and immediately synchronizes it                              |
| `update_skill`         | Update a Skill    | Replaces `SKILL.md`, preserves resources, and synchronizes the change                      |
| `delete_skill`         | Delete a Skill    | Permanently removes the Skill directory and its Memvid resources                           |
| `sync_skills`          | Refresh Skills    | Incrementally synchronizes added, changed, and deleted UTF-8 Skill resources               |
| `list_books`           | Inspect Books     | Lists book metadata, entry files, and available Markdown resources                         |
| `find_books`           | Find Books        | Matches a query against book names, titles, descriptions, and authors                      |
| `load_book`            | Read a Book       | Loads `BOOK.md` or one chapter resource from Memvid                                        |
| `add_book`             | Create a Book     | Writes a validated `BOOK.md` and immediately synchronizes it                               |
| `update_book`          | Update a Book     | Replaces `BOOK.md`, preserves chapters, and synchronizes the change                        |
| `delete_book`          | Delete a Book     | Permanently removes the entire Book directory and its Memvid resources                     |
| `import_book_document` | Import a document | Saves PDF/DOCX/PPTX/XLS/XLSX under a selected Book and runs the Memvid ingestion pipeline  |
| `sync_books`           | Refresh Books     | Synchronizes Markdown resources and supported files under each Book's `sources/` directory |
| `add_chunks`           | Store memory      | Stores general text chunks as Memvid frames                                                |
| `search`               | Query memory      | Searches general memory using natural language                                             |

## Usage Examples

### Find and Load Skills

Ask the Agent to use `memvid-skills` for a task such as:

```text
Find the relevant Skills for implementing an LVGL device page, then load and follow them before editing code.
```

Equivalent tool arguments are:

```json
{
    "task": "根据设计图实现嵌入式 C 项目的 LVGL 设备页面并检查分层架构",
    "top_k": 3
}
```

Then load a result:

```json
{
    "name": "lvgl-maker",
    "resource": null
}
```

To load a referenced file:

```json
{
    "name": "embedded-layer-architecture",
    "resource": "references/architecture.md"
}
```

### Manage Skills

Create a Skill by sending its complete `SKILL.md`:

```json
{
    "name": "rust-review",
    "content": "---\nname: rust-review\ndescription: Review Rust changes for correctness and maintainability.\n---\n\n# Rust Review\n\nInspect the requested Rust changes and report actionable findings.\n"
}
```

Use the same `name` and complete replacement `content` with `update_skill`.
Supporting files already present under the Skill directory are preserved. Delete the
entire Skill directory and all its resources with:

```json
{
    "name": "rust-review"
}
```

All three mutation tools validate the name and frontmatter, modify the configured
`storage.skills` directory, and synchronize Memvid before returning. Skill names must
contain 1–64 lowercase letters or digits separated by single hyphens.

### Manage Markdown Books

Each electronic book lives under `books/<name>/`, with `BOOK.md` as its entry and
optional Markdown chapters under paths such as `chapters/01-introduction.md`.
Create one with `add_book`:

```json
{
    "name": "rust-book",
    "content": "---\nname: rust-book\ntitle: Rust Book\ndescription: Personal Rust notes\nauthor: Example Author\n---\n\n# Rust Book\n\nReading notes and chapter index.\n"
}
```

Use `update_book` with the same arguments to replace only `BOOK.md`; existing chapter
files are preserved. Use `delete_book` with `{ "name": "rust-book" }` to remove the
entire book directory. `list_books`, `find_books`, and `load_book` inspect and read the
library. Every mutation automatically synchronizes the isolated `book://` resources in
Memvid.

### Import PDF and Office Documents

`import_book_document` requires an existing `book_name`. If the user has not named the
destination Book, the Agent must call `list_books` and ask before importing; it must not
auto-select a Book.

A local file under a configured `document_import.allowed_roots` directory can be imported
without Base64 overhead:

```json
{
    "book_name": "voip",
    "source": {
        "type": "local_path",
        "path": "uploads/voice-over-ip.pdf"
    }
}
```

A client that can access attachment bytes can upload the document through MCP JSON:

```json
{
    "book_name": "voip",
    "source": {
        "type": "base64",
        "file_name": "voice-over-ip.pdf",
        "content": "JVBERi0xLjQK..."
    }
}
```

The server validates the extension and size, saves the original file under
`books/<book_name>/sources/`, invokes Memvid's matching reader, verifies extracted text,
enriches and chunks the content, writes the original and extracted chunks as Smart Frames,
and commits lexical/time indexes. Re-importing unchanged bytes is idempotent. Replacing or
deleting a source removes its previous parent and child frames.

Set `embedding.provider = "openai"` to generate OpenAI-compatible vectors; the API key is
read from the environment variable named by `embedding.api_key_env` and is never stored in
`etc/config.toml`. For local ONNX vectors, build with:

```bash
cargo build --release --features local-embedding
```

Then set `embedding.provider = "local"` and place `<model>.onnx` plus
`<model>_tokenizer.json` in `embedding.models_dir`. On Windows, the current upstream
`ort`/`tokenizers` native dependencies may require a compatible MSVC CRT build; the default
build leaves this optional feature disabled so `cargo run` and tests remain linkable.

### Store Chunks

```
Use the add_chunks tool to store information:
Add chunks ["Meeting discussed Q1 planning", "Action item: Review API integration", "Decision: Use Memvid for memory"]
```

### Search Memory

```
Use the search tool with natural language queries:
Search for "Q1 planning" with top_k=5 and snippet_chars=200
```

### Health Check

```
Ping to verify server is responsive:
Ping
```

## Storage

Storage paths are configured by `storage.memory`, `storage.skills`, and `storage.books` in `etc/config.toml`. Relative paths are resolved from the process working directory.

The `.mv2` file includes:

- Compressed frames
- Full-text index
- Vector index
- Timeline metadata

The `.mv2` file can be shared, versioned, and backed up like any other file.

## Development

### Build

```bash
cargo build
```

### Run (Development)

```bash
cargo run
```

To synchronize Skills and Books and exit without starting the server, set
`server.sync_skills_only = true` in `etc/config.toml`. Paths, transport, logging,
and panic behavior are also changed in that file; the program does not accept
runtime configuration through command-line arguments.

## Requirements

- Rust 1.88.0+
- `memvid-core` crate (included as dependency)

## Learn More

- [MCP 与 Agent 使用指南](docs/mcp-agent-usage.md)
- [Zed 中的 Memvid Skill 工作流](docs/zed-skill-workflow.md)
- [Memvid Documentation](https://docs.memvid.com)
- [Memvid GitHub](https://github.com/memvid/memvid)
- [Model Context Protocol Spec](https://modelcontextprotocol.io)

## License

Apache License 2.0

## Contributing

Contributions welcome! Please feel free to submit a Pull Request.
