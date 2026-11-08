# Amemory (xcat)

> 桌面个人知识库、记忆中心与本地 AI 代理记忆层（基于 Longbridge NSH 原生桌面 GUI、XERJ 向量/全文混合检索核心与标准 MCP 协议）。

---

## 🌟 核心特性

- **1:1 原生桌面 GUI（Longbridge NSH）**：
  - **极致性能**：基于纯 Rust 原生渲染框架 `nsh`，无 Web/Electron 臃肿负担，启动毫秒级，内存占用极低。
  - **全键盘快捷流**：全局呼出热键（默认 `Alt + Shift + V`，可自定义），支持 `↑` / `↓` 导航、`Enter` 复制、`ESC` 隐藏。
  - **双层过滤与混合检索**：支持按类型（Skill、Book、Document、Memory、Script、Note）与按模式（Hybrid 混合 / Lexical 词法 / Semantic 语义）实时毫秒级多重筛选。
  - **标签与子文档管理**：左侧 56px 附着胶囊标签栏、多标签选择器（支持 32 款语义图标）、多子文档实时编辑与同步。
  - **6 大设置页签全持久化**：常规偏好、存储与搜索、MCP 与热键、XERJ 引擎内核参数、数据与日志、版本检查，所有修改双向绑定至 `etc/config.toml` 与 `etc/gui_settings.json`。
  - **无缝系统集成**：单实例端口互斥并自动唤醒现有窗口、Windows 物理置顶（`HWND_TOPMOST`）、失去焦点/复制后自动隐藏。

- **三位一体的知识组织架构**：
  - **Skills（技能）**：Zed / Cursor / Claude 格式的代码技能与自动化指令包。
  - **Books（书籍/知识库）**：树状章节分层文档，支持导入 PDF / Word / Markdown / TXT。
  - **Memory（碎片记忆）**：持久化记忆片断库（`amemory-memories.json`），支持即时增删改查。

- **强大的 XERJ 检索内核与防腐层**：
  - 底层基于 [xerj-org/xerj](https://github.com/xerj-org/xerj) 提供高效的 HNSW 向量检索与词法分词索引。
  - 拥有严格的架构隔离机制：`crates/adapter`（防腐适配层）、独立 Workspace 与一键升级流水线（`scripts/update-xerj.ps1`、`scripts/verify-xerj.ps1`）。

- **标准 Model Context Protocol (MCP) 接口**：
  - 提供 HTTP Server（`http://127.0.0.1:8000/mcp`）与 Stdio 管道两种通信模式。
  - 自动向外部 AI Agent 暴露工具集：知识库搜索、记忆存储、条目列举与技能检索。

---

## 🏗️ 架构分层

```text
amemory (xcat)
├── src/
│   ├── ui/               # NSH 原生组件（Root, Titlebar, SearchBox, FilterBar, EditPanel, Settings...）
│   ├── state/            # 响应式状态树（AppState, ServerRuntimeState, Signals）
│   ├── core/             # 领域核心定义、类型映射、i18n 多语言、查找替换
│   ├── services/         # 业务服务（CatalogSync, AmemoryRuntime, CopySound, Updater, PollLoop）
│   ├── platform/         # 操作系统原生能力（Hotkey, WindowFrame, Tray, Clipboard, Monitor）
│   ├── config.rs         # 双向配置管理（etc/config.toml, etc/gui_settings.json）
│   └── main.rs           # 组合根（单实例守护、命令行分发、生命周期调度）
├── crates/
│   ├── adapter/          # XERJ 引擎防腐适配层
│   ├── domain/           # 知识实体领域模型与分块逻辑
│   ├── mcp/              # MCP 协议服务端与工具注册表
│   └── xerj/             # 上游 XERJ 引擎（源码胶囊模式托管）
├── patches/              # 对上游 XERJ 的极简接口补丁（暴露 McpProvider / Server Bootstrap）
├── docs/                 # 架构方案、GUI 实施计划、XERJ 升级指南与集成文档
└── scripts/              # 自动化升级与四重体检脚本
```

---

## 🚀 快速开始

### 1. 运行环境要求
- **操作系统**：Windows 10 / 11、macOS 或 Linux
- **Rust 工具链**：Rust 1.80+（推荐最新 Stable）

### 2. 构建与运行

#### 启动原生 GUI 桌面客户端（默认）
```powershell
cargo run --release
# 或显式指定
cargo run --release -- --gui
```

#### 启动 MCP 服务供外部 Agent 调用
```powershell
# 启动 HTTP 模式（监听 127.0.0.1:8000/mcp）
cargo run --release -- --mcp-http

# 启动 Stdio 管道模式（适合接入 Zed / Claude Desktop 配置文件）
cargo run --release -- --mcp-stdio
```

#### 一次性全量建立索引
```powershell
cargo run --release -- --sync
```

#### 查看帮助
```powershell
cargo run -- --help
```

---

## 🧪 自动化测试与质量保障

遵循严格的工程质量标准：

```powershell
# 1. 运行全工作区单元测试与集成测试
cargo test --workspace

# 2. 运行严格静态分析检查（零警告）
cargo clippy --all-targets --all-features -- -D warnings

# 3. 运行 XERJ 源码与适配层完整性体检
powershell -ExecutionPolicy Bypass -File scripts\verify-xerj.ps1
```

---

## 🔄 XERJ 引擎升级指南

当 [xerj-org/xerj](https://github.com/xerj-org/xerj) 官方发布新版本时：

1. **一键执行自动化升级**：
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts\update-xerj.ps1 -UpstreamTag "<新版本号或commit>"
   ```
2. **执行四重完整性核验**：
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts\verify-xerj.ps1
   ```
3. 详细排障与手动补丁合并机制请参阅完整文档：[`docs/xerj-upgrade-guide.md`](docs/xerj-upgrade-guide.md)。

---

## 📄 开源许可

本项目遵循 [Apache 2.0 许可证](LICENSE)。
