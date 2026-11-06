# Amemory (xcat) GUI 实施计划（基于 Longbridge NSH 1:1 复刻 Clippi UI）

## 1. 总体目标与架构约束

### 1.1 核心目标
1. **UI 风格与尺寸 1:1 像素级复刻**：完全参照 [`examples/clippi`](file:///D:/my_github/amemory/examples/clippi/src/main.rs) 的视觉设计、窗口尺寸、圆角、配色主题（Dark / Light）、字体排版、顶部标题栏、搜索框与展开式新建按钮、双层过滤栏、卡片列表、悬浮工具栏（HoverToolbar）、右键菜单、左侧附着标签栏（Sidebar）、编辑面板（EditPanel）、设置面板（SettingsPanel）、底部状态栏（BottomBar）、模态弹窗与 Toast 提示。
2. **GUI 框架替换（禁用 GPUI，采用 NSH）**：
   - **严禁引入** `gpui = "0.2.2"`、`gpui-component = "0.5"`、`gpui_transitions = "0.1.5"`。
   - **选型框架**：采用 [`https://github.com/longbridge/nsh`](https://github.com/longbridge/nsh)（Longbridge Native Shell，`nsh-core` / `nsh-ui` 纯 Rust 原生桌面 UI 框架），使用其响应式状态（Signal / Effect）、Flexbox 布局与样式系统构建全部视图。
3. **前后端与职责严格分离**：
   - `src/ui/`：纯前端视图与通用 UI 组件（不直接操作底层数据库/文件系统，仅通过 `state` 与 `core::frontend` 交互）。
   - `src/state/`：响应式 UI 状态树、窗口状态、服务运行状态桥接。
   - `src/core/`：前端抽象契约（`Frontend` trait / `AppEvent`）、GUI 配置模型、过滤与检索条件、国际化（i18n）、文本查找替换与类型定义。
   - `src/services/`：对接本项目 `amemory` 核心业务（`SkillCatalog`、`BookCatalog`、`XerjAdapter`、`UnifiedMcpServer`、文档导入、目录同步、音效播放、后台轮询）。
   - `src/platform/`：系统窗口无边框圆角、多显示器定位（居中/跟随鼠标/记忆位置）、全局热键、系统托盘、剪贴板、系统强调色与系统字体探测。
   - `assets/`：存放所有字体（`iconfont.ttf`）、SVG 图标（`picker-icons/`）、音频（`copy_*.wav`）与应用图标（`LOGO*.png/ico/icns`）。
4. **本项目业务数据与配置全面接入**：
   - 将 `amemory` 的 **Skills（技能）**、**Books（书籍/章节/笔记）**、**Documents（导入文档）**、**Memory（记忆分片）**、**XERJ 混合检索结果（Lexical / Semantic / Hybrid）** 与 **索引统计/MCP 服务状态** 接入主界面卡片列表、过滤栏、编辑面板与底部状态栏。
   - 将 [`etc/config.toml`](file:///D:/my_github/amemory/etc/config.toml)（[`AppConfig`](file:///D:/my_github/amemory/src/config.rs#L14-L32)）的所有参数（`[server]`、`[storage]`、`[xerj]`、`[document_import]`、`[logger]`、`[panic]`）及 GUI 外观/热键设置接入 6 页签设置面板，支持实时修改与持久化保存。

---

## 2. 目录结构与模块划分

```text
D:\my_github\amemory
├── assets/                                  # 静态资源目录（从 examples/clippi/assets 迁移并扩展）
│   ├── LOGO.png / LOGO-1024.png / LOGO.ico / LOGO.icns
│   ├── copy_default.wav / copy_pop.wav / copy_ding.wav / copy_drop.wav / copy_klick.wav
│   ├── fonts/
│   │   └── iconfont.ttf                     # Clippi 图标字体（1:1 复刻所有图标字形）
│   └── picker-icons/                        # 32 个 SVG 标签选择器图标
│       └── bookmark.svg, code.svg, terminal.svg, ...
├── src/
│   ├── main.rs                              # 入口：支持 --config 与 GUI / CLI / MCP 模式启动
│   ├── lib.rs                               # 导出 config, core, platform, services, state, ui
│   ├── config.rs                            # AppConfig + GuiConfig 读写与校验
│   ├── core/                                # 核心抽象与纯逻辑层（无 UI 框架耦合）
│   │   ├── mod.rs
│   │   ├── frontend.rs                      # AppEvent 枚举与 Frontend 事件总线契约
│   │   ├── types.rs                         # KnowledgeItem, ItemKind, ScopeFilter, CardHeightMode 等模型
│   │   ├── settings.rs                      # GuiSettings（主题、字体、热键、窗口定位、音效等）与持久化
│   │   ├── filters.rs                       # 类型过滤栏配置（Skill/Book/Doc/Memory/Script/Ref）与排序
│   │   ├── search.rs                        # 搜索防抖、关键字分词、高亮片段计算
│   │   ├── find_replace.rs                  # 编辑面板查找/替换（大小写敏感、正则）纯逻辑
│   │   ├── i18n.rs                          # 多语言翻译引擎（zh-CN / en-US）
│   │   ├── i18n_keys.rs                     # 翻译键常量定义
│   │   └── paths.rs                         # 资源路径、配置路径、日志与数据目录解析
│   ├── platform/                            # 平台原生能力适配层
│   │   ├── mod.rs
│   │   ├── window_frame.rs                  # 无边框圆角窗口、阴影、置顶（AlwaysOnTop）、尺寸约束
│   │   ├── monitor.rs                       # 显示器工作区探测与窗口定位（Center / FollowMouse / Remember）
│   │   ├── tag_window.rs                    # 左侧附着标签副窗口（56px 宽）坐标同步与吸附计算
│   │   ├── hotkey.rs                        # 全局呼出热键与快捷检索热键注册/监听
│   │   ├── tray.rs                          # 系统托盘菜单（显示主窗口、同步索引、切换 MCP、退出）
│   │   ├── clipboard.rs                     # 系统剪贴板读写与一键粘贴上屏
│   │   ├── accent.rs                        # 操作系统强调色读取与主题跟随
│   │   └── system_font.rs                   # 系统已安装字体枚举
│   ├── services/                            # 业务服务层（桥接 crates/domain, adapter, mcp 与版本更新）
│   │   ├── mod.rs
│   │   ├── amemory_runtime.rs               # 封装 XerjAdapter、SkillCatalog、BookCatalog 核心操作
│   │   ├── catalog_sync.rs                  # 技能与书籍全量/增量同步到 XERJ 索引服务
│   │   ├── mcp_controller.rs                # 后台 MCP Server (mcp-http / server) 启停控制与状态监控
│   │   ├── document_importer.rs             # PDF/Word/Excel/PPT/MD/TXT 文档导入到 Book 服务
│   │   ├── update.rs                        # 版本检查器（复刻 Clippi：UpdateChecker/UpdateChannel/UpdatePhase/24h定时检查）
│   │   ├── updater.rs                       # 更新编排器（下载 -> SHA256 校验 -> 准备安装包 -> 重启应用）
│   │   ├── downloader.rs                    # 安装包流式下载（带进度回调）与 SHA256 摘要校验
│   │   ├── install.rs                       # 平台安装程序启动（Windows NSIS 静默升级 / macOS .app 替换）
│   │   ├── copy_sound.rs                    # 复制/操作完成音效播放服务
│   │   └── poll_loop.rs                     # 索引统计刷新、服务健康检查、24h 自动检查更新与防抖调度
│   ├── state/                               # 响应式状态管理层（基于 NSH Signals）
│   │   ├── mod.rs
│   │   ├── app.rs                           # AppState：过滤条件、卡片列表、激活视图、弹窗、Toast、UpdatePhase/UpdateInfo
│   │   └── server_state.rs                  # ServerRuntimeState：索引文档计数、MCP 端口状态、同步进度
│   └── ui/                                  # NSH 视图与组件层（1:1 复刻 Clippi UI）
│       ├── mod.rs
│       ├── root.rs                          # ClippiRoot 主窗口容器（Titlebar + Search + Filter + Stack + BottomBar）
│       ├── theme.rs                         # ClippiTheme 颜色 Token（Dark/Light 1:1 色值）与圆角/动画常量
│       ├── font.rs                          # 字体缩放（font_scale）与字号计算辅助函数
│       ├── assets.rs                        # 内嵌与本地静态资源加载器（iconfont + SVG + WAV）
│       ├── window_manager.rs                # 主窗口、左侧标签窗口、更新检查/下载/重启调度生命周期管理
│       ├── titlebar.rs                      # 30px 顶部标题栏（Logo、拖拽区、同步/收藏/置顶/设置按钮+新版本圆点角标）
│       ├── search_box.rs                    # 28px 搜索框 + 240ms 悬浮展开式 "+" 新建按钮
│       ├── filter_bar.rs                    # 双排 22px 过滤栏（Row1: 知识类型+实时数量角标；Row2: 检索模式/命名空间）
│       ├── stats_overview.rs                # 首页核心数据统计面板（书本数、Skills数、Memory数与XERJ核心指标卡片）
│       ├── clipboard_list.rs                # 虚拟滚动卡片列表（支持键盘上下导航、Enter 复制/打开、空状态）
│       ├── clipboard_card.rs                # 知识/记忆卡片（68/96/128px 动态高度、左侧类型图标、高亮摘要、底部胶囊）
│       ├── hover_toolbar.rs                 # 20px 右上角悬浮工具栏（复制、编辑、导入、标签、打开目录、收藏、删除）
│       ├── context_menu.rs                  # 184px 右键上下文菜单（含快捷键提示与分割线）
│       ├── edit_panel.rs                    # 新建/编辑面板（支持 Skill/Book/Memory 编辑、查找替换、分栏实时预览）
│       ├── bottom_bar.rs                    # 26px 底部状态栏（MCP/XERJ 状态、Skills/Books/Memory 计数、快捷按钮）
│       ├── sidebar.rs                       # 56px 左侧标签栏视图（22px 胶囊、悬浮跑马灯滚动、多选过滤）
│       ├── sidebar_window.rs                # 左侧标签独立附着窗口封装
│       ├── tag_filter.rs                    # 标签筛选浮层面板
│       ├── tag_picker.rs                    # 卡片标签分配与新建标签（含 32 图标选择器）浮层
│       ├── type_filter_config.rs            # 类型过滤栏显示/隐藏与拖拽排序配置面板
│       ├── rich_preview.rs                  # Markdown / JSON / 代码块语法着色与结构化预览组件
│       ├── search_highlight.rs              # 搜索命中关键字背景高亮渲染器
│       ├── quick_paste.rs                   # 紧凑型快捷检索/粘贴浮窗
│       ├── components/                      # 通用原子 UI 组件
│       │   ├── mod.rs
│       │   ├── toggle.rs                    # 32x18px 胶囊开关（14px 滑块 + 140ms 过渡动画）
│       │   ├── slider.rs                    # 步进滑块组件（用于字体缩放、日志大小、分片参数等）
│       │   ├── confirm_dialog.rs            # 240px 模态确认对话框（180ms 缩放渐显）
│       │   ├── modal_overlay.rs             # 半透明遮罩层（0x00000055）
│       │   ├── toast.rs                     # 底部居中浮动 Toast 提示（220ms 动画）
│       │   ├── shortcut_hint.rs             # Alt 键长按 200ms 呼出的快捷键角标气泡
│       │   └── spinner.rs                   # 同步/加载旋转指示器
│       └── settings/                        # 36px 顶栏 + 36px 6-Tab 设置面板
│           ├── mod.rs                       # SettingsPanel 容器、Tab 切换动画（含版本 Tab 更新绿点）与分组卡片构建器
│           ├── general.rs                   # Tab 1: 通用（主题、强调色、字体、缩放、语言、窗口定位）
│           ├── storage.rs                   # Tab 2: 知识库与存储（skills/books 路径、上传白名单、最大字节数）
│           ├── xerj_engine.rs               # Tab 3: 检索引擎（data_dir、embed_mode、索引名、ES/REST/gRPC 端口）
│           ├── mcp_hotkey.rs                # Tab 4: 服务与热键（server.mode、host/port、endpoint、全局热键）
│           ├── data_logger.rs               # Tab 5: 数据与日志（日志级别/目录/大小、音效、全量重建索引）
│           └── version.rs                   # Tab 6: 版本与更新（1:1复刻Clippi检查更新、下载进度条、SHA256校验、Release Notes与立即重启）
```

---

## 3. Clippi UI 1:1 像素级复刻规范与 NSH 映射

### 3.1 窗口与组件几何尺寸规范（严格 1:1 对照 [`examples/clippi/src/ui`](file:///D:/my_github/amemory/examples/clippi/src/ui/root.rs)）

| 区域 / 组件 | 参照 Clippi 源文件 | 精确尺寸与布局参数 |
| :--- | :--- | :--- |
| **主窗口默认尺寸** | [`root.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/root.rs#L35-L45) | 默认 `324.0 × 480.0` px；最小 `324.0 × 480.0` px；最大 `1164.0 × 1200.0` px |
| **窗口外框圆角** | [`theme.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/theme.rs#L6-L15) | `PANEL_RADIUS = 12.0px`，外层 `1px` `divider` 边框 |
| **视图切换与弹窗动画** | [`theme.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/theme.rs#L11-L15) | `VIEW_ANIM_DURATION = 180ms`（滑动偏移 `28px`），`TOAST_ANIM_DURATION = 220ms` |
| **顶部标题栏 (`Titlebar`)** | [`titlebar.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/titlebar.rs#L37-L145) | 高度 `30px`；左侧 `pl 10px`、`20×20` Logo + `12px` 粗体标题；右侧 `28×28` 图标按钮（`rounded 6px`, `gap 2px`, `pr 4px`, 字号 `14px`） |
| **搜索栏 (`SearchBox`)** | [`search_box.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/search_box.rs#L65-L212) | 高度 `28px`，`px 8px`，`mb 6px`，`gap 6px`；输入框 `rounded 10px`、`1px` 边框；右侧 `+` 按钮默认 `28×28px`，悬浮 `240ms` 展开 `+62px` 至 `90px` 宽（实心 `accent` 背景） |
| **过滤栏 (`FilterBar`)** | [`filter_bar.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/filter_bar.rs#L89-L360) | 每排高度 `22px`，`px 8px`，`mb 6px`；过滤胶囊 `h 22px`、`rounded 5px`（`12px` 图标 + `11px` 文字 + 实时数量角标，窄窗口自动折叠）；右侧 `1px × 14px` 竖线 + `24×22px` 标签按钮 `\u{e886}` |
| **首页数据统计看板 (`StatsOverview`)** | 新增首页概览组件（遵循 [`theme.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/theme.rs) 卡片规范） | 位于过滤栏下方（`px 8px`, `mb 6px`，可一键折叠）；上层为 `44px` 高三宫格统计卡（`Skills` / `Books` / `Memory` 数量与分片数，`gap 6px`, `rounded 8px`），下层为 `24px` 高 XERJ 核心指标条（健康度、总文档、Segments、磁盘占用、检索耗时） |
| **卡片 (`ClipboardCard`)** | [`clipboard_card.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/clipboard_card.rs#L40-L120) | 动态高度 `68px` / `96px` / `128px`；圆角 `10px`；左侧 `3px` 收藏/置顶指示条；左列 `36px` 宽（`16px` 类型图标 + `9px` 类型文字）；底部信息胶囊 `h 18px`、`9px` 字号 |
| **悬浮工具栏 (`HoverToolbar`)** | [`hover_toolbar.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/hover_toolbar.rs#L17-L95) | 高度 `20px`，绝对定位于卡片右上角 `right 8px, top 0px`；每个操作签 `20×20px`（底部圆角 `5px`，悬浮向下伸展 `+6px` 至 `26px` 高，显示实心功能色与白色图标） |
| **右键菜单 (`ContextMenu`)** | [`context_menu.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/context_menu.rs#L20-L110) | 宽度 `184px`，内边距 `4px`，圆角 `8px`；菜单项高度 `30px`，圆角 `5px`，`13px` 图标 + `12px` 标题 + `10px` 快捷键提示 |
| **左侧标签栏 (`Sidebar`)** | [`sidebar.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/sidebar.rs#L23-L115) | 附着在主窗口左侧，宽度 `56px`，顶部偏移 `65.0px`，底部内边距 `8.0px`；每行 `24px`（`22px` 高胶囊 + `2px` 间距），左圆角 `6px`，长标签悬浮跑马灯滚动（`28px/s`） |
| **编辑面板 (`EditPanel`)** | [`edit_panel.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/edit_panel.rs#L65-L210) | `36px` 顶栏（返回 `\u{e62b}` + 标题）；`24px` 工具栏（类型切换胶囊 + 查找替换 `\u{e64c}` + JSON 格式化 + Trim）；`26px` 查找/替换折叠栏；上下分栏编辑器 + 预览（`4px` 拖拽分割条） |
| **设置面板 (`SettingsPanel`)** | [`settings/mod.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/settings/mod.rs#L27-L160) | `36px` 顶栏 + `36px` 6-Tab 导航栏（激活项 `2px` `accent` 下划线）；分组卡片 `rounded 10px`、`surface` 背景、`px 14px, py 10px` 行高 |
| **底部状态栏 (`BottomBar`)** | [`bottom_bar.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/bottom_bar.rs#L17-L110) | 高度 `26px`，顶部 `1px` `divider` 边框，`px 10px`；左侧 `15×15` 状态图标 + `12px` 状态文案 + `11px` 统计数字；右侧 `22×22px` 快捷图标按钮 |

### 3.2 主题色板 Token（1:1 复刻 [`ClippiTheme`](file:///D:/my_github/amemory/examples/clippi/src/ui/theme.rs#L66-L169)）

| Token 字段 | Dark 模式色值 | Light 模式色值 | 用途说明 |
| :--- | :--- | :--- | :--- |
| `bg` / `titlebar_bg` | `#191a1b` | `#f2f3f8` | 主窗口与标题栏背景 |
| `surface` | `#232425` | `#ffffff` | 卡片、输入框、设置分组背景 |
| `surface_press` | `#3a3b3c` | `#ebedf5` | 按钮/卡片按下态背景 |
| `text_1` | `#eaebec` | `#1a1c2e` | 主文本颜色 |
| `text_2` | `#919496` | `#7c809a` | 次要文本、标题栏文字、未激活图标 |
| `text_3` | `#5f6264` | `#b4b9ca` | 占位符、快捷键角标、辅助计数文本 |
| `accent` | `#7ecba3`（支持自定义） | `#6ab890`（支持自定义） | 主题强调色（选中边框、激活开关、新建按钮） |
| `accent_soft` | `#1a2e24`（或动态混色） | `#e8f5ee`（或动态混色） | 选中卡片背景、激活过滤胶囊背景 |
| `fav_color` | `#d8a155` | `#d8a155` | 收藏星标与左侧收藏指示条颜色 |
| `transfer_pin_color` | `#3B82F6` | `#3B82F6` | 置顶/Hub 同步标记颜色 |
| `danger` | `#ff5f57` | `#ff5f57` | 删除操作、错误状态颜色 |
| `divider` | `#2b2c2d` | `#e6e8f0` | 1px 边框与分割线 |
| `btn_hover` | `#2b2c2d` | `#f0f1f8` | 图标按钮悬浮态背景 |
| `tag_bg` / `tag_text` | `#2c2e2f` / `#e4e9e6` | `#e6e8f0` / `#767e7a` | 标签胶囊背景与文字色 |
| `panel_surface` / `panel_input_bg` | `#2c2d2e` / `#1e1f20` | `#ffffff` / `#f5f6fc` | 浮层面板背景与浮层输入框背景 |
| `toast_bg` | `rgba(0x3a,0x3b,0x3d,0xe8)` | `rgba(0xff,0xff,0xff,0xf0)` | Toast 提示半透明背景 |

### 3.3 GPUI 到 Longbridge NSH 的技术映射

| Clippi (GPUI) 机制 | 本项目 (Longbridge NSH) 实现方式 |
| :--- | :--- |
| `gpui::Entity<AppState>` + `cx.observe` / `cx.notify` | NSH 响应式 `Signal<T>` / `RwSignal<T>` / `Memo<T>`，状态变更自动驱动细粒度组件重绘，无需手动 `notify` |
| `div().flex().items_center().w(px(324.))` | NSH View 构建器与 Flexbox/Tailwind 风格布局修饰符（精确设置宽高、内边距、圆角、边框与背景色） |
| `gpui_component::input::InputState` | NSH 原生文本输入组件，绑定 `RwSignal<String>` 并通过 Effect 实现 `150ms` 搜索防抖 |
| `gpui::uniform_list` / 虚拟列表 | NSH 滚动容器与虚拟列表视图，支持键盘上下键选中滚动定位与动态卡片高度（`68/96/128px`） |
| `gpui_transitions::use_keyed_transition` | NSH 动画/过渡状态钩子与定时插值，实现 `180ms` 面板切换、`240ms` 新建按钮展开、`140ms` Toggle 滑块动画 |
| `gpui::Window` / 双窗口（主窗口 + Sidebar 窗口） | NSH 多窗口管理 API + `src/platform/tag_window.rs` 实现主窗口与左侧 `56px` 标签栏窗口的联动与吸附 |

---

## 4. 本项目（Amemory / XERJ）业务数据、首页统计看板与参数设置接入设计

### 4.1 首页核心数据统计与 XERJ 引擎监控看板 (`stats_overview.rs`)

在首页（主窗口默认视图）集成实时数据统计体系，通过 **顶部数据统计看板 (`StatsOverview`)**、**过滤栏数量角标 (`FilterBar`)** 与 **底部状态栏 (`BottomBar`)** 三处协同展示 **书本 (Books)**、**技能 (Skills)**、**记忆 (Memory)** 以及 **XERJ 检索引擎** 的核心运行数据：

#### 1) 统计指标数据源与维度划分

| 统计分类 | 核心指标字段 | 数据来源（Domain / Adapter / XERJ Engine） | 首页展示内容说明 |
| :--- | :--- | :--- | :--- |
| **技能统计 (Skills)** | `skills_count`<br>`skill_files_count`<br>`skills_docs` | [`SkillCatalog::list_skills()`](file:///D:/my_github/amemory/crates/domain/src/skill.rs#L46-L56)<br>[`StoreStats.skills_docs`](file:///D:/my_github/amemory/crates/domain/src/memory.rs#L116-L125) | **技能总数**（`SKILL.md` 个数）、关联资源文件总数（`scripts/`、`references/`、`assets/`、`agents/`）以及在 `amemory-skills-v1` 索引中的 **已索引分片数** |
| **书本与文档统计 (Books)** | `books_count`<br>`book_files_count`<br>`imported_docs_count`<br>`books_docs` | [`BookCatalog::list_books()`](file:///D:/my_github/amemory/crates/domain/src/book.rs#L59-L69)<br>[`StoreStats.books_docs`](file:///D:/my_github/amemory/crates/domain/src/memory.rs#L116-L125) | **书本总数**（`BOOK.md` 个数）、章节/笔记文件总数（`chapters/`、`notes/`、`summaries/`）、已导入外部文档数（PDF/Word/Excel/PPT/MD）以及在 `amemory-books-v1` 中的 **已索引分片数** |
| **记忆统计 (Memory)** | `memory_docs`<br>`namespace_count` | [`StoreStats.memory_docs`](file:///D:/my_github/amemory/crates/domain/src/memory.rs#L116-L125)<br>XERJ 聚合统计 | `amemory-memory-v1` 中的 **记忆分片总数** 及活跃命名空间（`default` 等）个数 |
| **XERJ 核心引擎数据 (XERJ Core)** | `health_status` (`GREEN`/`YELLOW`/`RED`)<br>`engine_version`<br>`index_count` / `total_docs`<br>`total_segments`（各索引段数汇总）<br>`embed_mode` (`lexical`/`semantic`/`hybrid`)<br>`disk_usage_bytes`<br>`last_search_latency_ms`<br>`last_sync_report` | [`Engine::health()`](file:///D:/my_github/amemory/crates/xerj/engine/crates/xerj-engine/src/engine.rs#L433-L438)<br>[`Engine::list_indices()`](file:///D:/my_github/amemory/crates/xerj/engine/crates/xerj-engine/src/engine.rs#L402-L407) (`IndexInfo`)<br>[`XerjConfig`](file:///D:/my_github/amemory/crates/adapter/src/engine.rs#L33-L54)<br>[`SyncReport`](file:///D:/my_github/amemory/crates/domain/src/memory.rs#L100-L112) | 1. **引擎健康度与版本**：实时健康状态灯（绿/黄/红）与 XERJ 版本<br>2. **索引与存储段 (Segments)**：活跃索引数（3 个）、全库总文档分片数 (`total_docs`)、底层倒排/列存总段数 (`segment_count`) 与 Schema 版本<br>3. **性能与存储指标**：当前向量/词法模式 (`embed_mode`)、`data/xerj` 磁盘占用体积（MB/KB）、最近一次检索耗时（`ms`）<br>4. **同步账本摘要**：最近一次目录同步扫描数、新增/更新分片数、未变更跳过数与清理过期分片数 |

#### 2) 首页统计看板 (`StatsOverview`) 视觉与交互布局（契合 `324px` 紧凑宽度）
- **位置与折叠控制**：位于首页 `FilterBar` 正下方、`ClipboardList` 列表上方（`px 8px, mb 6px`）；标题栏或看板右上角提供折叠切换开关，支持展开完整看板或收起为单行紧凑胶囊条。
- **第一层：三宫格核心资产卡片（高度 `44px`，`gap 6px`）**：
  - **Skills 卡**：左侧技能图标 `\u{e7b0}` + 主数字 **`{skills_count} 技能`**（`13px` 粗体 `text_1`），副标题 **`{skills_docs} 分片 · {skill_files_count} 文件`**（`9px` `text_3`）。点击卡片直接将列表切换至 `Skill` 过滤视图。
  - **Books 卡**：左侧书本图标 `\u{e658}` + 主数字 **`{books_count} 书本`**（`13px` 粗体 `text_1`），副标题 **`{books_docs} 分片 · {imported_docs_count} 导入`**（`9px` `text_3`）。点击卡片直接将列表切换至 `Book` 过滤视图。
  - **Memory 卡**：左侧记忆图标 `\u{e665}` + 主数字 **`{memory_docs} 记忆`**（`13px` 粗体 `text_1`），副标题 **`{namespace_count} 空间 · 实时写入`**（`9px` `text_3`）。点击卡片直接将列表切换至 `Memory` 过滤视图。
- **第二层：XERJ 引擎核心状态胶囊条（高度 `24px`，`surface` 背景，`rounded 6px`，`1px divider` 边框）**：
  - 左侧：XERJ 健康度圆点徽章（`GREEN` `#7ecba3` / `YELLOW` `#d8a155` / `RED` `#ff5f57`）+ `XERJ` 引擎标识 + 当前 `embed_mode` 胶囊（`Lexical` / `Hybrid`）。
  - 中部：核心存储指标 **`Docs: {total_docs} · Segs: {total_segments} · 磁盘: {disk_mb} MB`**（`10px` `text_2`）。
  - 右侧：最近一次查询耗时徽章 **`{last_search_latency_ms} ms`**（`9px` `accent` 色）与一键 Flush/刷新统计按钮。

---

### 4.2 主界面其余组件数据映射（Clippi 组件 -> Amemory 业务实体）

1. **顶部标题栏 (`Titlebar`)**：
   - 左侧：应用 Logo + `Amemory` 标题（显示当前模式如 `GUI + MCP`）。
   - 右侧 5 个 `28×28` 图标按钮：
     - **统计看板显隐 (`\u{e64a}`)**：一键展开/折叠首页 `StatsOverview` 数据统计看板。
     - **一键同步目录 (`\u{e782}`)**：触发 `catalog_sync` 将 `resource/skills` 与 `resource/books` 同步至 XERJ 索引，完成后刷新首页统计数据并弹出 `SyncReport` Toast 提示。
     - **仅看收藏/核心知识 (`\u{e630}`)**：过滤带 `favorite` / `pinned` 标记的技能、书籍或记忆条目。
     - **窗口置顶 (`\u{e633}`)**：切换主窗口 `AlwaysOnTop` 状态（激活态使用 `accent` 色）。
     - **打开设置 (`\u{e6b6}`)**：平滑切换至 `SettingsPanel`（`Alt` 快捷键提示）。

2. **搜索框与新建按钮 (`SearchBox`)**：
   - **搜索框**：输入关键字后经 `150ms` 防抖，调用 `XerjAdapter::search(SearchRequest)` 并记录检索耗时 `last_search_latency_ms`（当输入为空时展示按更新时间倒序的 Skills / Books / Memory 列表；当有查询词时执行全文/向量/混合检索，并在卡片上高亮命中词）。
   - **展开式 `+` 按钮**：点击进入 `EditPanel`，可快速新建 **Memory 记忆分片**、**Skill 技能 (`SKILL.md`)**、**Book 书籍 (`BOOK.md`)** 或调用文件对话框 **导入文档 (`import_book_document`)**。

3. **双排过滤栏 (`FilterBar`)**：
   - **第一排（知识类型过滤 Chips + 实时数量角标 + 标签筛选按钮）**：
     - `全部 ({total})`
     - `技能 ({skills_count})` — 对应 `SkillCatalog` 中的 `SKILL.md`
     - `书籍 ({books_count})` — 对应 `BookCatalog` 中的 `BOOK.md` 与章节
     - `文档 ({imported_docs_count})` — 对应导入的 PDF / Word / Excel / PPT / Markdown 文档 (`files/imports/...`)
     - `记忆 ({memory_docs})` — 对应 `IndexTarget::Memory` 中的记忆分片 (`memory://...`)
     - `脚本/代码` — 对应技能/书籍下的 `scripts/` 与代码片段
     - `参考/笔记` — 对应 `references/` 与 `notes/`
     - 右侧 `\u{e886}` 按钮：切换左侧 `56px` 标签栏窗口或弹出 `TagFilterPanel`。
   - **第二排（检索模式与命名空间胶囊条，复刻 Clippi 的来源应用过滤栏样式）**：
     - 左侧固定检索模式切换胶囊：`Hybrid（混合）` / `Lexical（词法）` / `Semantic（语义）`
     - 右侧横向滚动 Scope / Namespace 胶囊：如 `skill://`、`book://`、`memory://default` 等。

4. **卡片列表与卡片 (`ClipboardList` + `ClipboardCard`)**：
   - **左侧类型区 (`36px`)**：根据 `Skill` / `Book` / `Document` / `Memory` / `Script` / `Note` 渲染对应 `iconfont` 图标与 `9px` 类型标签。
   - **右侧内容区**：
     - 第一行：条目标题（`SkillEntry.name` / `BookEntry.name` / `SearchHit.title`）+ 检索得分（如有）。
     - 正文预览：Markdown 摘要或检索命中的 `snippet`（带 `accent_highlight` 关键字高亮）。
   - **底部胶囊栏 (`18px`)**：
     - URI / Scope 胶囊（如 `skill://wrust`、`book://rust-guide#part-1`、`memory://session`）。
     - 标签胶囊（显示 `tags` 列表，如 `#rust`、`#architecture`）。
     - 分片/资源统计胶囊（如 `3 scripts · 2 refs` 或 `#part 1/4`）与相对时间。

5. **悬浮工具栏 (`HoverToolbar`) 与右键菜单 (`ContextMenu`)**：
   - **复制内容 (`\u{e65c}`) / 复制 URI**：复制条目 Markdown 正文或 `skill://` / `book://` URI 到剪贴板（播放 `assets/copy_*.wav` 音效）。
   - **编辑 (`\u{e603}`)**：在 `EditPanel` 中打开该 Skill、Book 章节或 Memory 分片进行源码编辑与实时预览。
   - **导入文档 (`\u{e649}`)**：针对 `Book` 条目，直接弹出文件选择框调用 `import_book_document` 导入外部文档并自动重索引。
   - **标签管理 (`\u{e886}`)**：弹出 `TagPickerPanel` 编辑条目的 `tags`。
   - **打开所在目录 (`\u{e664}`)**：在资源管理器中定位 `resource/skills/<slug>` 或 `resource/books/<slug>`。
   - **收藏 (`\u{e630}`) / 删除 (`\u{e60e}`)**：切换收藏状态，或弹出 `ConfirmDialog` 确认后调用 `delete_skill` / `delete_book` / 删除索引文档并同步更新首页统计数字。

6. **编辑面板 (`EditPanel`)**：
   - 支持 **新建 / 编辑 Skill**（自动生成/解析 `SKILL.md` YAML Frontmatter + 正文）、**新建 / 编辑 Book**（`BOOK.md` / `chapters/*.md` / `notes/*.md`）、**新建 / 追加 Memory**（指定 `namespace`、`title`、`tags`、`content` 写入 `XerjAdapter`）。
   - 内置查找替换栏（支持大小写敏感与正则表达式）与上下分栏 Markdown 实时预览。

7. **底部状态栏 (`BottomBar`)**：
   - 左侧：XERJ 健康状态灯 + 当前服务状态（如 `MCP :8000 · ES :9200`）+ 紧凑统计摘要（`Skills: {skills_count} · Books: {books_count} · Docs: {total_docs}`）。
   - 右侧快捷按钮：一键启停后台 `mcp-http` 服务、一键复制 MCP 客户端接入地址（`http://127.0.0.1:8000/mcp`）。

---

### 4.3 设置面板 (`SettingsPanel`) 与 `etc/config.toml` 全参数对接

设置面板严格复刻 Clippi 的 `36px` 顶栏 + `36px` 标签栏 + 圆角分组卡片样式，包含以下 6 个页签：

| 页签 | 绑定配置项（`etc/config.toml` + `GuiSettings`） | 交互控件类型 |
| :--- | :--- | :--- |
| **1. 通用 (`general.rs`)** | 主题模式 (`system`/`dark`/`light`)、自定义强调色、UI 字体与代码字体选择、字体缩放 (`0.85x ~ 1.30x`)、界面语言 (`zh_CN`/`en_US`)、卡片高度模式 (`紧凑 68px`/`标准 96px`/`宽松 128px`/`自适应`)、窗口弹出位置 (`居中`/`跟随鼠标`/`记忆位置`)、失焦自动隐藏 | 分段胶囊选择器、颜色圆点选择器、下拉菜单、`SteppedSlider`、`Toggle` 开关 |
| **2. 存储 (`storage.rs`)** | `storage.skills` 路径、`storage.books` 路径、`document_import.allowed_roots` 上传白名单目录列表、`document_import.max_file_bytes` 最大单文件字节限制（MB 滑块/输入框） | 路径显示 + 文件夹浏览按钮、白名单列表增删、数值输入框、一键打开目录按钮 |
| **3. 检索引擎 (`xerj_engine.rs`)** | `xerj.data_dir` 数据目录、`xerj.embed_mode` (`lexical` / `semantic` / `hybrid`)、索引名 (`skills_index`, `books_index`, `memory_index`)、网络监听 (`bind_address`, `es_compat_port: 9200`, `rest_port: 8080`, `grpc_port: 8081`)、`external_es_url`、`hub_api_key` | 分段选择器、端口数字输入框、密码掩码输入框、实时索引文档统计卡片 |
| **4. 服务与热键 (`mcp_hotkey.rs`)** | `server.mode` (`gui`, `mcp-http`, `server`, `mcp-stdio`, `sync`)、`server.host`、`server.port` (`8000`)、`server.endpoint_path` (`/mcp`)、`server.allow_remote`、`server.max_request_body_bytes`、全局呼出热键、快捷检索粘贴热键 | 模式切换器、`Toggle` 开关、端口/路径输入框、热键录制胶囊、后台 MCP 服务一键重启按钮 |
| **5. 数据与日志 (`data_logger.rs`)** | `logger.level` (`trace`/`debug`/`info`/`warn`/`error`)、`logger.size_mb`、`logger.console`、`logger.dir`、`panic.exit_on_panic`、操作音效开关与音效选择 (`copy_default`/`pop`/`ding`/`drop`/`klick`)、全量同步知识库、清空 Memory 索引 | 下拉/分段选择器、`Toggle` 开关、音效试听按钮、带 `ConfirmDialog` 的危险操作按钮 |
| **6. 版本与更新 (`version.rs`)** | 当前版本 (`CARGO_PKG_VERSION`)、`auto_check_updates`（24h 定时检查开关）、`update_channel` (`Auto`/`Oss`/`GitHub`)、在线检查更新/下载进度/SHA256 校验/Release Notes 展示/立即重启安装 | 自动检查复选框 (`\u{e61f}`/`\u{e831}`)、状态机动作按钮 (`检查更新`/`下载更新`/`{progress}%`/`立即重启`)、`4px` 进度条、右上角 GitHub Releases 图标按钮 (`\u{ea0a}`)、内置滚动 Markdown 更新日志 |

> 所有针对 [`etc/config.toml`](file:///D:/my_github/amemory/etc/config.toml) 的修改均通过 `src/config.rs` 进行合法性校验（调用 `AppConfig::validate()`），验证通过后原子写回配置文件，并通过 Toast 提示生效状态。

---

### 4.4 版本检查与自动更新全流程设计（1:1 复刻 [`examples/clippi/src/ui/settings/version.rs`](file:///D:/my_github/amemory/examples/clippi/src/ui/settings/version.rs) 与 [`services/update.rs`](file:///D:/my_github/amemory/examples/clippi/src/services/update.rs)）

1. **后端更新服务链路（`src/services/`）**：
   - **`update.rs`（[`UpdateChecker`](file:///D:/my_github/amemory/examples/clippi/src/services/update.rs#L222-L250)）**：
     - 支持 `UpdateChannel::Auto` / `Oss` / `GitHub` 多通道解析最新 Release Manifest 或 GitHub Releases API，通过 SemVer 对比当前 `CARGO_PKG_VERSION`。
     - 支持 24 小时后台定时自动检查（`scheduled_update_check_due`），当 `settings.auto_check_updates` 开启时由 `poll_loop.rs` 自动触发。
     - 定义完整的 `UpdatePhase` 状态机：
       - `Idle` / `UpToDate` -> `Checking` -> `UpdateAvailable` -> `Downloading { progress: u8 }` -> `Verifying` -> `Installing` -> `ReadyToRestart` / `Error(String)`。
   - **`downloader.rs` & `updater.rs`（[`download_latest_and_prepare`](file:///D:/my_github/amemory/examples/clippi/src/services/updater.rs#L12-L34)）**：
     - 启动前自动清理临时更新目录 `cleanup_temp()`（在 Windows 下自动检测并安全终止残留旧安装包进程）。
     - 流式下载平台对应的安装包（校验文件名安全性 `is_safe_asset_name`），实时回调 `UpdatePhase::Downloading { progress }` 更新 UI 进度条。
     - 下载完成后进入 `UpdatePhase::Verifying`，拉取或读取 Manifest 内置的 SHA256 哈希值执行完整性校验（`verify_sha256`）。
   - **`install.rs`（[`launch_prepared_update`](file:///D:/my_github/amemory/examples/clippi/src/services/updater.rs#L121-L137)）**：
     - 进入 `UpdatePhase::ReadyToRestart` 后，用户点击“立即重启”按钮时启动平台安装器（Windows 下启动 NSIS 安装程序，macOS 下挂载替换 `.app` Bundle）并优雅退出当前进程。

2. **前端 UI 与通知角标联动（`src/ui/settings/version.rs` + `titlebar.rs` + `settings/mod.rs`）**：
   - **新版本角标提醒**：当检测到 `update_available.is_some()` 时，主窗口 `Titlebar` 右上角设置图标（`\u{e6b6}`）与 `SettingsPanel` 顶栏第 6 个“版本”页签右上角同步显示 `accent` 绿色提醒圆点，并弹出 Toast 通知。
   - **版本卡片（1:1 复刻 Clippi `render_version_tab`）**：
     - 顶部显示 `Amemory v{version}`（`13px` 粗体 `text_1`），右上角悬浮 GitHub 图标按钮 `\u{ea0a}`（`26px`，默认 `text_3` 灰色，悬浮渐变为 `accent` 绿色，点击打开项目 Releases 页面）。
     - 第二行显示当前 `UpdatePhase` 状态文字（正常为 `accent` 色，异常 `UpdatePhase::Error` 为 `danger` 红色）。
     - 第三行左侧为紧凑型 **自动检查更新** 复选框（`\u{e61f}` 选中 / `\u{e831}` 未选中，`11px` 文字），右侧为 `28px` 高、`rounded 7px` 的状态机主按钮（根据 `UpdatePhase` 自动切换文案与启用状态：`检查更新` -> `检查中...` -> `下载更新` -> `{progress}%` -> `校验中...` -> `立即重启`）。
     - 下载阶段在卡片底部渲染 `4px` 高、`rounded 2px` 的动态进度条（`render_progress_bar`）。
     - 当存在新版本说明（`release_notes` 非空）时，在同一分组卡片下方自动展开带独立滚动条的 Markdown **更新日志（Release Notes）** 预览区。

---

## 5. 分阶段实施步骤与验证标准

### 阶段一：静态资源迁移与 Cargo 依赖配置
1. 将 `examples/clippi/assets/` 下的 `fonts/iconfont.ttf`、`picker-icons/*.svg`、`copy_*.wav`、`LOGO*.png/ico/icns` 完整复制到根目录 `assets/`。
2. 在根 [`Cargo.toml`](file:///D:/my_github/amemory/Cargo.toml) 中引入 `https://github.com/longbridge/nsh` 相关依赖及桌面端辅助库（托盘、全局热键、剪贴板、音频播放等），确保**不包含**任何 `gpui` 依赖。
3. 在 [`src/config.rs`](file:///D:/my_github/amemory/src/config.rs) 中扩展 `ServerMode::Gui`（默认支持直接启动 GUI 或通过 `--mode gui` 启动），并增加 `AppConfig::save()` 方法用于将界面修改持久化回 `etc/config.toml`。

### 阶段二：核心抽象层 (`src/core`)、平台层 (`src/platform`) 与服务层 (`src/services`) 实现
1. 实现 `src/core/` 下的 `frontend.rs`（`AppEvent` 与 `Frontend` 契约）、`types.rs`、`settings.rs`、`filters.rs`、`search.rs`、`find_replace.rs`、`i18n.rs`、`i18n_keys.rs`、`paths.rs`。
2. 实现 `src/platform/` 下的窗口框架控制、多显示器定位、左侧标签附着窗口坐标计算、全局热键、系统托盘与剪贴板服务。
3. 实现 `src/services/` 下的业务与更新服务：
   - 知识与检索服务：`amemory_runtime.rs`、`catalog_sync.rs`、`mcp_controller.rs`、`document_importer.rs`。
   - 版本更新服务（复刻 Clippi）：`update.rs`、`updater.rs`、`downloader.rs`、`install.rs`。
   - 音效与后台调度：`copy_sound.rs`、`poll_loop.rs`。

### 阶段三：响应式状态层 (`src/state`) 与基础 UI 组件库 (`src/ui/components`) 实现
1. 实现 `src/state/app.rs` 与 `src/state/server_state.rs`，将卡片列表、搜索过滤状态、激活面板、弹窗队列、索引统计与服务状态封装为响应式状态。
2. 实现 `src/ui/theme.rs`（1:1 色板与尺寸常量）、`src/ui/font.rs`、`src/ui/assets.rs`。
3. 实现 `src/ui/components/` 下的 `toggle.rs`、`slider.rs`、`confirm_dialog.rs`、`modal_overlay.rs`、`toast.rs`、`shortcut_hint.rs`、`spinner.rs`。

### 阶段四：主窗口核心视图与首页统计看板 1:1 复刻 (`src/ui/*`)
1. 实现 `titlebar.rs`（`30px`）、`search_box.rs`（`28px` + `240ms` 动态展开新建按钮）、`filter_bar.rs`（双排 `22px` 过滤栏 + 实时数量角标）。
2. 实现 `stats_overview.rs`（首页三宫格 `Skills / Books / Memory` 统计卡 + `XERJ` 引擎核心指标条：健康状态、总文档数、Segment 段数、磁盘占用、检索耗时与一键折叠/点击过滤交互）。
3. 实现 `clipboard_card.rs`（`68/96/128px` 知识/记忆卡片）、`hover_toolbar.rs`（`20px` 顶部悬浮伸展工具条）、`context_menu.rs`（`184px` 右键菜单）、`clipboard_list.rs`（虚拟滚动列表与键盘导航）。
4. 实现 `bottom_bar.rs`（`26px` 状态与统计栏）、`sidebar.rs` & `sidebar_window.rs`（左侧 `56px` 附着标签栏）。

### 阶段五：编辑面板、浮层面板与 6 页签设置面板 (`src/ui/edit_panel.rs` & `src/ui/settings/*`)
1. 实现 `edit_panel.rs`（支持 Skill / Book / Memory 创建与编辑、查找替换、分栏 Markdown 预览）。
2. 实现 `tag_filter.rs`、`tag_picker.rs`、`type_filter_config.rs`、`quick_paste.rs`。
3. 实现 `src/ui/settings/` 下的全部 6 个设置页签，打通 [`etc/config.toml`](file:///D:/my_github/amemory/etc/config.toml) 全字段双向绑定与实时校验。

### 阶段六：联调、代码规范检查与测试验证
1. 在 `src/main.rs` 与 `src/lib.rs` 中完成模块挂载与启动流程串联。
2. 遵循仓库 Rust 2024 与 Clippy 规范（`missing_docs = "warn"`、`unsafe_op_in_unsafe_fn = "deny"`、`clippy::pedantic`、`clippy::nursery`），运行：
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets`
   - `cargo test --workspace`
3. 验证 GUI 窗口启动、明暗主题切换、Skills/Books/Memory 检索展示、新建/编辑/导入文档、标签栏联动、MCP 服务启停以及设置保存全链路正常工作。
