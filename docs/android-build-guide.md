# Amemory (xcat) Android 端编译与安装指南

本指南记录了如何将 **Amemory**（基于 Native Shell / NSH 跨平台 UI 引擎）构建并打包为 **Android APK** 安装包的完整流程与环境配置。

---

## 一、架构设计与工作原理

NSH（Native Shell）在 Android 平台采用**“Rust 逻辑与渲染驱动 + 原生 Android Java/Kotlin 视图宿主”**的分层架构：

```
+-------------------------------------------------------------------+
|                     Android 设备 (Android 8.0+)                   |
+-------------------------------------------------------------------+
|  Java 宿主层 (android/app)                                         |
|    - AppActivity.java (主入口 Activity)                           |
|    - NshView.java (自定义 Native 视图容器)                         |
|    - Native.java (JNI 桥接层，调用 System.loadLibrary("nsh"))     |
+---------------------------------+---------------------------------+
                                  | JNI (runRust / commit / Choreographer)
+---------------------------------v---------------------------------+
|  Rust 原生动态库 (libnsh.so, 来自 crates/amemory-android)          |
|    - nsh-platform-android (JNI 胶水、StaticLayout 排版、事件循环)  |
|    - xcat::ui::window_manager::AmemoryGuiComponent (主 UI 组件)    |
|    - xcat 核心引擎、XERJ 本地知识库与检索适配器                    |
+-------------------------------------------------------------------+
```

1. **Rust 共享库**：子 crate `crates/amemory-android` 被配置为 `crate-type = ["cdylib"]`，编译为 `libnsh.so`。
2. **Java 宿主**：位于工程根目录的 `android/` 下，负责应用生命周期、窗口输入和屏幕 Surface 承载。
3. **架构隔离**：桌面端专用的 Windows API（全局热键、托盘通知、DWM 置顶等）已通过 `#[cfg(target_os = "windows")]` 条件编译完全隔离，在 Android 端编译时自动降级或禁用，保证全平台编译零冲突。

---

## 二、编译所需前置环境

编译 Android APK 需要配置以下 4 项开发工具链：

| 组件 | 推荐版本 | 说明 |
| :--- | :--- | :--- |
| **Rust Toolchain** | Stable (1.88+) | 包含 `rustup` 与 `cargo` |
| **Rust Android Target** | `aarch64-linux-android` | 对应绝大多数 Android 手机架构（arm64-v8a） |
| **Java JDK** | JDK 17 (OpenJDK / Temurin) | Android Gradle 插件构建必须 |
| **Android SDK & NDK** | SDK 34 / NDK r26+ | 编译 C/汇编依赖（如 ring、zlib）及链接 JNI |

---

## 三、环境安装与环境变量配置

### 1. 安装 Rust 编译目标
在终端执行：
```powershell
rustup target add aarch64-linux-android
# 如需兼容 32 位老机型或模拟器，可同时添加：
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
```

### 2. 安装 cargo-ndk 工具
`cargo-ndk` 可以自动为 Rust 编译器配置 Android NDK 中的 Clang 交叉编译器与 sysroot：
```powershell
cargo install cargo-ndk
```

### 3. 安装 JDK 17
- 前往 [Eclipse Temurin](https://adoptium.net/temurin/releases/?version=17) 或 Oracle 官网下载并安装 **JDK 17**。
- 设置环境变量：
  - `JAVA_HOME` 指向 JDK 安装目录（例如 `C:\Program Files\Eclipse Adoptium\jdk-17.x.x`）
  - 将 `%JAVA_HOME%\bin` 添加到系统的 `PATH` 中。

### 4. 安装 Android SDK 与 NDK
#### 方式 A：通过 Android Studio（推荐）
1. 安装 [Android Studio](https://developer.android.com/studio)。
2. 打开 Android Studio -> **Settings / Preferences** -> **Languages & Frameworks** -> **Android SDK**。
3. 在 **SDK Platforms** 勾选：
   - `Android 14.0 ("UpsideDownCake")` (API 34)
4. 在 **SDK Tools** 勾选：
   - `NDK (Side by side)`（推荐 26.x 或更高版本）
   - `Android SDK Command-line Tools (latest)`
   - `Android SDK Build-Tools`
5. 配置环境变量：
   - `ANDROID_HOME`: 通常为 `C:\Users\<你的用户名>\AppData\Local\Android\Sdk`
   - `ANDROID_NDK_HOME`: 通常为 `%ANDROID_HOME%\ndk\<版本号>`（例如 `C:\Users\<你的用户名>\AppData\Local\Android\Sdk\ndk\26.1.10909125`）
   - 将 `%ANDROID_HOME%\platform-tools` 添加到 `PATH` 中（方便使用 `adb` 调试）。

---

## 四、编译与打包 APK

### 方式一：一键自动编译（推荐）
项目根目录下提供了全自动脚本 `build-android.ps1`，它会自动校验环境、编译 Rust 原生库并调用 Gradle 打包：

```powershell
# 在项目根目录下运行
.\build-android.ps1
```

如需编译正式 Release 版本：
```powershell
.\build-android.ps1 -BuildType release
```

---

### 方式二：手动分步编译

#### 步骤 1：编译 Rust 动态库 (`libnsh.so`)
使用 `cargo-ndk` 将 `crates/amemory-android` 编译为对应架构的共享库，并自动输出到 Android 工程的 `jniLibs` 目录：

```powershell
# 编译 64 位 ARM (arm64-v8a)
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p amemory-android --release

# 如需同时支持 x86_64 电脑模拟器：
cargo ndk -t x86_64 -o android/app/src/main/jniLibs build -p amemory-android --release
```

编译完成后，确认以下路径存在 `libnsh.so`：
- `android/app/src/main/jniLibs/arm64-v8a/libnsh.so`

#### 步骤 2：打包 Android APK
进入 `android` 目录执行 Gradle 构建：

```powershell
cd android

# Windows 下执行 Gradle Wrapper
.\gradlew.bat assembleDebug

# 或打包 Release 版本
.\gradlew.bat assembleRelease
```

构建成功后，APK 生成于：
- **Debug 版路径**：`android/app/build/outputs/apk/debug/app-debug.apk`
- **Release 版路径**：`android/app/build/outputs/apk/release/app-release.apk`

---

## 五、安装到 Android 手机

### 1. 通过 USB 线直连安装 (ADB)
1. 在手机上打开 **设置** -> **关于手机** -> 连续点击 **版本号** 5~7 次开启 **开发者选项**。
2. 进入 **开发者选项**，开启 **USB 调试** 以及 **允许通过 USB 安装应用**。
3. 用数据线将手机连接到电脑，手机上弹出“允许 USB 调试吗？”选择“始终允许”。
4. 在电脑终端运行安装命令：
```powershell
adb install android\app\build\outputs\apk\debug\app-debug.apk
```
若提示已安装同名应用，可使用 `-r` 覆盖安装：
```powershell
adb install -r android\app\build\outputs\apk\debug\app-debug.apk
```

### 2. 通过文件传输手动安装
直接将 `android/app/build/outputs/apk/debug/app-debug.apk` 复制或通过微信/网盘发送至手机，在手机文件管理器中点击即可完成安装。

---

## 六、常见问题排查与解决

### 1. `failed to find tool "aarch64-linux-android-clang"`
- **原因**：Rust 编译某些依赖（如 `ring`、`zlib`）时需要 Android NDK 中的 C 编译器。
- **解决**：确保已安装 Android NDK，并已设置 `ANDROID_NDK_HOME` 环境变量；推荐使用 `cargo-ndk` 工具进行编译。

### 2. 运行时应用启动白屏或闪退 (`UnsatisfiedLinkError: dlopen failed: library "libnsh.so" not found`)
- **原因**：编译出的 `.so` 未被正确打包进 APK，或者架构与手机 CPU 不匹配。
- **解决**：
  1. 检查 `android/app/src/main/jniLibs/arm64-v8a/` 目录下是否存在 `libnsh.so`。
  2. 确认 `android/app/build.gradle` 中配置了 `jniLibs.srcDirs = ['src/main/jniLibs']`。

### 3. Gradle 构建失败：`Unsupported class file major version`
- **原因**：安装的 JDK 版本与 Gradle 插件不兼容（建议使用 **JDK 17**）。
- **解决**：检查 `javac -version`，确保使用 JDK 17。
