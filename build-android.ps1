<#
.SYNOPSIS
    Amemory Android APK 一键编译脚本
.DESCRIPTION
    检查 Android NDK、JDK 和 Rust Android 目标，编译 libnsh.so 并构建最终的 Android APK 安装包。
#>

param(
    [string]$TargetAbi = "arm64-v8a",
    [string]$RustTarget = "aarch64-linux-android",
    [string]$BuildType = "debug"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Amemory Android 一键编译与打包脚本" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. 检查 Rust Android Target
Write-Host "`n[1/5] 检查 Rust 交叉编译目标: $RustTarget..." -ForegroundColor Yellow
$installedTargets = rustup target list --installed
if ($installedTargets -notcontains $RustTarget) {
    Write-Host "正在安装 Rust Target: $RustTarget..." -ForegroundColor Cyan
    rustup target add $RustTarget
} else {
    Write-Host "  -> Rust Target 已就绪: $RustTarget" -ForegroundColor Green
}

# 2. 检查 Java JDK
Write-Host "`n[2/5] 检查 Java 开发环境 (JDK 17+)..." -ForegroundColor Yellow
$javacPath = Get-Command javac -ErrorAction SilentlyContinue
if (-not $javacPath) {
    Write-Host "  [错误] 未检测到 Java JDK (javac)！" -ForegroundColor Red
    Write-Host "  构建 Android APK 需要 JDK 17 或更高版本。" -ForegroundColor Yellow
    Write-Host "  请安装 OpenJDK 17 (例如 Eclipse Temurin 或 Microsoft JDK)，并配置 JAVA_HOME 及 PATH。" -ForegroundColor Yellow
    exit 1
}
Write-Host "  -> JDK 已就绪: $($javacPath.Source)" -ForegroundColor Green

# 3. 检查 Android SDK 与 NDK
Write-Host "`n[3/5] 检查 Android SDK / NDK..." -ForegroundColor Yellow
$androidHome = [System.Environment]::GetEnvironmentVariable("ANDROID_HOME")
$androidNdkHome = [System.Environment]::GetEnvironmentVariable("ANDROID_NDK_HOME")
if (-not $androidNdkHome) {
    $androidNdkHome = [System.Environment]::GetEnvironmentVariable("NDK_HOME")
}

if (-not $androidNdkHome) {
    # 尝试在 ANDROID_HOME/ndk 中自动探查
    if ($androidHome -and (Test-Path "$androidHome\ndk")) {
        $latestNdk = Get-ChildItem "$androidHome\ndk" | Sort-Object Name -Descending | Select-Object -First 1
        if ($latestNdk) {
            $androidNdkHome = $latestNdk.FullName
            Write-Host "  -> 自动发现 NDK 目录: $androidNdkHome" -ForegroundColor Green
        }
    }
}

if (-not $androidNdkHome -or -not (Test-Path $androidNdkHome)) {
    Write-Host "  [错误] 未检测到 Android NDK！" -ForegroundColor Red
    Write-Host "  Rust 编译原生 C/汇编依赖（如 ring、zlib 等）需要 NDK 提供的 Clang 编译器。" -ForegroundColor Yellow
    Write-Host "  请通过 Android Studio SDK Manager 安装 NDK (版本 >= 26.0)，并设置环境变量 ANDROID_NDK_HOME。" -ForegroundColor Yellow
    exit 1
}
Write-Host "  -> NDK 已就绪: $androidNdkHome" -ForegroundColor Green

# 4. 检查/安装 cargo-ndk
Write-Host "`n[4/5] 检查 cargo-ndk 工具..." -ForegroundColor Yellow
$cargoNdk = Get-Command cargo-ndk -ErrorAction SilentlyContinue
if (-not $cargoNdk) {
    Write-Host "  正在安装 cargo-ndk 辅助编译工具 (只需一次)..." -ForegroundColor Cyan
    cargo install cargo-ndk
} else {
    Write-Host "  -> cargo-ndk 工具已就绪" -ForegroundColor Green
}

# 5. 编译 Rust cdylib (libnsh.so)
Write-Host "`n[5/5] 编译 Rust 动态库 (libnsh.so) 并打包 APK..." -ForegroundColor Yellow
$jniLibsDir = Join-Path $PSScriptRoot "android\app\src\main\jniLibs\$TargetAbi"
if (-not (Test-Path $jniLibsDir)) {
    New-Item -ItemType Directory -Force -Path $jniLibsDir | Out-Null
}

$releaseFlag = if ($BuildType -eq "release") { "--release" } else { "" }
Write-Host "  -> 正在执行 cargo-ndk 编译..." -ForegroundColor Cyan
$ndkCmd = "cargo ndk -t $TargetAbi -o `"$PSScriptRoot\android\app\src\main\jniLibs`" build -p amemory-android $releaseFlag"
Invoke-Expression $ndkCmd

# 6. 执行 Gradle 打包 APK
$androidDir = Join-Path $PSScriptRoot "android"
Push-Location $androidDir
try {
    $gradleTask = if ($BuildType -eq "release") { "assembleRelease" } else { "assembleDebug" }
    Write-Host "  -> 正在执行 Gradle 打包: $gradleTask..." -ForegroundColor Cyan
    if (Test-Path ".\gradlew.bat") {
        .\gradlew.bat $gradleTask
    } else {
        gradle $gradleTask
    }
} finally {
    Pop-Location
}

$apkPath = Join-Path $androidDir "app\build\outputs\apk\$BuildType\app-$BuildType.apk"
if (Test-Path $apkPath) {
    Write-Host "`n==========================================" -ForegroundColor Green
    Write-Host "  APK 构建成功！" -ForegroundColor Green
    Write-Host "  输出文件: $apkPath" -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Green
} else {
    Write-Host "`n[提示] 编译流程已完成，请检查输出目录: $androidDir\app\build\outputs\apk" -ForegroundColor Yellow
}
