# C 代码与接口约定

## 基本原则

- 清晰优先于炫技；只有确认性能瓶颈后才牺牲可读性优化。
- 一个函数只完成一项功能。新增控制块嵌套不超过 4 层，优先使用早返回和职责拆分。
- 重复代码出现两次时考虑提炼，超过三次应重构。
- 删除废弃函数、变量和注释掉的旧实现。
- 头文件保持职责单一，不定义变量；确需公开全局对象时只做必要的 `extern` 声明，但新代码优先改为函数接口。

## 格式基准

以仓库根目录 `.clang-format` 为准：

| 项目 | 约定 |
| --- | --- |
| 语言标准 | C11 |
| 基础风格 | Google |
| 缩进 | 4 个空格，禁止 Tab |
| 行宽 | 最多 160 列 |
| 指针格式 | `const char* text` |
| 控制语句 | `if (condition)` |
| Include | 自动排序，保留合理的现有分组 |

只格式化本次修改的文件，不手工维护另一套排版规则。

## 命名

### 文件与函数

- 一方 C 源文件和头文件使用小写 `snake_case`。
- 公共头通常与主实现同名：`audio_input.h`、`audio_input.c`。
- 公共函数使用小写 `snake_case`，格式优先为 `<layer-or-package>_<module>_<operation>()`。

| 所属模块 | 前缀 | 示例 |
| --- | --- | --- |
| app/UI | `ui_` | `ui_app_init()` |
| business | `<package>_` | `<package>_player_open_video()` |
| business 通知 | `notify_` | `notify_register_device_event_handler()` |
| business/DAL | `dal_` | `dal_config_get()` |
| HAL | `hal_` | `hal_media_player_open()` |
| drivers | `drivers_` | `drivers_audio_input_read()` |
| pkg | 包名 | `logger_init()`、`config_init()` |

文件内部函数使用描述性的 `snake_case` 并声明为 `static`。

### 类型、枚举与变量

- 新增普通类型使用小写 `snake_case_t`。
- 常用后缀：`_config_t`、`_state_t`、`_result_t`、`_event_t`、`_kind_t`、`_callback_t`。
- 枚举值使用带层或模块前缀的全大写下划线，例如 `DRIVERS_AUDIO_RESULT_TIMEOUT`。
- 局部变量和结构体字段使用小写 `snake_case`，名称直接表达状态和单位，例如 `timeout_ms`、`frame_length`。
- 新增文件作用域状态优先声明为 `static`，并沿用当前一方代码的 `g_` 前缀，例如 `static bool g_capture_started;`。
- 不新增可被其他模块直接修改的全局变量；跨模块状态通过函数接口访问。

### 头文件保护

使用基于项目相对路径生成的全大写保护宏：

```c
#ifndef HAL_AUDIO_INPUT_H
#define HAL_AUDIO_INPUT_H

/* declarations */

#endif /* HAL_AUDIO_INPUT_H */
```

## 公共头文件与 API

### 可见性

```text
src/hal/audio/audio_input.h       # 对外公共头
src/hal/audio/audio_input_priv.h  # 模块内部共享头
src/hal/audio/audio_input.c       # 实现
```

- 公共头只声明调用方需要的类型、接口常量和函数。
- `_priv.h` 仅供同一模块的多个实现文件共享，禁止模块外包含。
- 只在单个 `.c` 使用的声明和定义直接放在该文件，不为此新增私有头。
- API 提供模块拥有请求、结果、状态、事件和回调类型，不向上泄漏下层结果或私有句柄。

### Doxygen 契约

公共 API 优先使用 Doxygen，至少说明：

- `@brief`：行为而非函数名复述；
- `@param`：输入/输出方向、单位、可否为 `NULL`、所有权和有效期；
- `@return`：成功语义或每类结果由谁处理；
- 必要时补充线程、重入、阻塞和调用顺序约束。

```c
/**
 * @brief 读取并整理一帧音频数据。
 * @param data 调用方提供的输出缓冲区。
 * @param capacity 输出缓冲区容量，单位为字节。
 * @param length 接收实际写入长度，单位为字节。
 * @param timeout_ms 等待超时，单位为毫秒。
 * @return 保留超时、设备失效等具体结果，供 HAL 决定后续行为。
 */
drivers_audio_result_t drivers_audio_input_read(
    uint8_t* data,
    size_t capacity,
    size_t* length,
    int timeout_ms);
```

## 返回值与错误处理

### 返回类型

- 只表示成功或失败时返回 `bool`：`true` 成功，`false` 失败。
- 不使用 `int`、`0/1` 或自定义错误码表达单纯二元结果。
- 调用方需要区分超时、设备失效、未就绪、缓冲区不足等原因时，由提供模块定义自己的 `_result_t`。
- `hal` 和 `business` 将下层结果映射成各自拥有的结果类型，不原样泄漏 driver 结果。

### 处理规则

- 公共入口尽早校验参数和当前状态。
- 会影响控制流、状态或资源生命周期的返回值必须显式处理。
- 有意忽略可选操作结果时使用 `(void)`，并确认失败降级策略明确。
- 获取资源后，所有错误路径都要保持成对释放和状态一致。

### 分层职责

- `drivers`：报告平台或设备错误，不决定产品重启、退出或业务降级。
- `hal`：转换为中立结果，必要时维护硬件能力状态。
- `business`：按业务语义决定重试、降级或终止当前用例。
- `app`：将最终状态映射为界面反馈，不处理驱动细节。

## 回调契约

新增回调类型应携带调用方上下文：

```c
typedef void (*hal_audio_capture_callback_t)(
    const unsigned char* data,
    unsigned int length,
    void* context);
```

注册接口必须说明：

- 保存一个处理器还是多个订阅者，后注册是否覆盖；
- `NULL` 是否表示注销；
- 回调在哪个线程执行，是否同步、是否允许重入；
- 参数是借用还是转移所有权，以及有效期；
- 注销与并发发布之间如何保证生命周期；
- 调用回调时不得持有哪些内部锁。

## 宏、常量与日志

### 放置和命名

宏由提供层或模块维护，消费方不得复制同值定义。宏使用带所有者前缀的全大写下划线。

| 使用范围 | 放置位置 |
| --- | --- |
| 跨本层多个模块 | `src/<layer>/<layer>_def.h` |
| 同一模块多个文件 | `src/<layer>/<module>/<module>_def.h` |
| 仅一个实现文件 | 对应 `.c`；优先 `enum` 或 `static const` |

- `app` 自有宏使用 `APP_` 前缀。
- 模块级 `<module>_def.h` 只能由对应模块包含。
- 头文件保护宏不按业务宏处理。
- 函数式宏优先改为有类型检查的 `static inline`；必须参与预处理时才使用 `#define`。
- 允许沿合法依赖方向引用所有者宏，不要求复制定义或强制改为查询函数。

### 常量选择

- 不需要预处理的整数常量优先匿名 `enum`。
- 具有明确 C 类型的只读数据使用 `static const`。
- 名称表达单位和用途，例如 `APP_NOTIFY_DISPATCH_PERIOD_MS`。

### 日志

- 日志系统初始化后使用 `src/pkg/logger/log.h` 的 `LOGGER_DEV`、`LOGGER_DEBUG`、`LOGGER_MSG`、`LOGGER_WARN`、`LOGGER_ERROR`。
- 模块多文件共享前缀放在 `<module>_def.h`，命名 `<MODULE>_LOG_PREFIX`。
- 日志包含可定位问题的操作、关键上下文和结果。
- 不输出密码、令牌、完整私钥等敏感数据。
- `LOGGER_*` 宏只捕获调用点并转发普通日志函数，不承载业务分支或改变参数求值次数。
