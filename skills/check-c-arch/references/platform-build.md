# 平台、文件与构建

## 驱动目录

驱动按硬件能力组织，不按页面或业务场景组织：

```text
src/drivers/<capability>/
├── <public_api>.h
├── <common_impl>.c
├── ssd20x_<implementation>.c
└── x86_<implementation>.c
```

每个驱动单元应能只依赖自身公共头、`pkg` 和平台依赖独立构建。跨设备编排属于 `hal`，驱动单元不能包含其他同级驱动单元。

私有定义只被一个 `.c` 使用时直接放入实现文件；只有多个实现文件确实共享时才新增 `_priv.h`。

## 平台文件命名

`src/drivers` 的平台专属实现统一使用前缀：

- SSD20x：`ssd20x_*.c`；
- x86：`x86_*.c`；
- 多平台共享：无平台前缀。

```text
src/drivers/display/ssd20x_backlight.c
src/drivers/display/x86_backlight.c
src/drivers/display/fbdev.c
```

app 的 LVGL 平台端口沿用后缀约定：

```text
src/app/basic/lvgl_port_ssd20x.c
src/app/basic/lvgl_port_x86.c
```

不要把 app 端口后缀规则与 drivers 平台前缀规则混用。

## 公共头与平台实现

- 公共头只暴露中立 C 类型，不包含不必要的平台 SDK 类型。
- 平台实现文件可以包含对应的 Linux、PulseAudio、FFmpeg 或厂商 SDK 头。
- drivers 可以直接调用平台 SDK，但不得包含项目 hal 头；方向只能是 hal 调用 drivers。
- 同一公共接口的不同平台实现必须由构建系统互斥选择，避免重复定义。

## CMake 目标

新增或拆分的分层目标应尽可能独立，只公开必要 include 目录和链接依赖：

```text
app -> business -> hal -> drivers
```

参考媒体链：

```text
app_media_lvgl
    -> business_media
        -> hal_media
            -> driver_media_<platform>
```

检查 CMake 时同时确认：

1. `target_link_libraries` 没有反向或越层链接；
2. `target_include_directories` 没有向无关层公开私有目录；
3. 平台条件只选择一个实现；
4. driver 目标能脱离 app/business 独立配置或构建；
5. 新文件已加入正确目标，而不是为了省事聚合到顶层 app。

历史主目标可能仍直接聚合部分源码或平台库，不把这种历史关系视为新增代码越层的许可。

## 平台配置

主要平台配置：

- `CHIP_LINUX_AMD64_DEBUG`：Linux x86_64/WSL 开发环境；
- `CHIP_SSD20X_LINUX_ARM64`：SSD202D 设备交叉编译环境。

构建系统必须只选择当前平台实现。

## 验证命令

Linux Debug：

```bash
cmake --preset linux-amd64-debug
cmake --build --preset linux-amd64-debug
```

SSD202 Debug：

```bash
cmake --preset ssd202-debug
cmake --build --preset ssd202-debug
```

新增或删除普通 `.c` 文件后必须重新执行 CMake 配置，因为源码列表可能只在配置阶段收集。

执行架构审查时默认检查全部受控层；用户明确限定范围时传入对应路径：

```bash
python3 .agents/skills/check-c-arch/scripts/check_c_arch.py [src/<path> ...]
```

架构脚本和 CMake 构建只提供证据，不能替代对公开符号、回调签名、传递包含、条件编译和链接关系的人工审查。
