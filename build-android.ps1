# Amemory Android build & package script

param(
    [string]$TargetAbi = "arm64-v8a",
    [string]$RustTarget = "aarch64-linux-android",
    [string]$BuildType = "debug"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Amemory Android Build & Package" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Check Rust target
Write-Host "`n[1/5] Checking Rust Target: $RustTarget..." -ForegroundColor Yellow
$installedTargets = rustup target list --installed
if ($installedTargets -notcontains $RustTarget) {
    Write-Host "Installing target $RustTarget..." -ForegroundColor Cyan
    rustup target add $RustTarget
} else {
    Write-Host "  -> Rust Target ready: $RustTarget" -ForegroundColor Green
}

# 2. Check Java JDK
Write-Host "`n[2/5] Checking Java JDK..." -ForegroundColor Yellow
$javac = Get-Command javac -ErrorAction SilentlyContinue
if (-not $javac) {
    Write-Host "  [Error] JDK not found (javac is missing)!" -ForegroundColor Red
    Write-Host "  Please install JDK 17+ and add it to PATH." -ForegroundColor Yellow
    exit 1
}
Write-Host "  -> JDK ready: $($javac.Source)" -ForegroundColor Green

# 3. Check Android NDK
Write-Host "`n[3/5] Checking Android NDK..." -ForegroundColor Yellow
$androidHome = [System.Environment]::GetEnvironmentVariable("ANDROID_HOME")
$androidNdkHome = [System.Environment]::GetEnvironmentVariable("ANDROID_NDK_HOME")
if (-not $androidNdkHome) {
    $androidNdkHome = [System.Environment]::GetEnvironmentVariable("NDK_HOME")
}
if (-not $androidNdkHome -and $androidHome -and (Test-Path "$androidHome\ndk")) {
    $latestNdk = Get-ChildItem "$androidHome\ndk" -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
    if ($latestNdk) {
        $androidNdkHome = $latestNdk.FullName
    }
}

if (-not $androidNdkHome -or -not (Test-Path $androidNdkHome)) {
    Write-Host "  [Error] Android NDK not found!" -ForegroundColor Red
    Write-Host "  Please install NDK (>= 26.0) via Android Studio SDK Manager," -ForegroundColor Yellow
    Write-Host "  and set environment variable ANDROID_NDK_HOME." -ForegroundColor Yellow
    exit 1
}
Write-Host "  -> NDK ready: $androidNdkHome" -ForegroundColor Green

# 4. Check cargo-ndk
Write-Host "`n[4/5] Checking cargo-ndk..." -ForegroundColor Yellow
$cargoNdk = Get-Command cargo-ndk -ErrorAction SilentlyContinue
if (-not $cargoNdk) {
    Write-Host "  Installing cargo-ndk..." -ForegroundColor Cyan
    cargo install cargo-ndk
} else {
    Write-Host "  -> cargo-ndk ready" -ForegroundColor Green
}

# 5. Build native library
Write-Host "`n[5/5] Building native library and APK..." -ForegroundColor Yellow
$jniDir = Join-Path $PSScriptRoot "android\app\src\main\jniLibs\$TargetAbi"
if (-not (Test-Path $jniDir)) {
    New-Item -ItemType Directory -Force -Path $jniDir | Out-Null
}

$releaseFlag = if ($BuildType -eq "release") { "--release" } else { "" }
cargo ndk -t $TargetAbi -o "$PSScriptRoot\android\app\src\main\jniLibs" build -p amemory-android $releaseFlag

$androidDir = Join-Path $PSScriptRoot "android"
Push-Location $androidDir
try {
    $task = if ($BuildType -eq "release") { "assembleRelease" } else { "assembleDebug" }
    if (Test-Path ".\gradlew.bat") {
        .\gradlew.bat $task
    } else {
        gradle $task
    }
} finally {
    Pop-Location
}

$apkPath = Join-Path $androidDir "app\build\outputs\apk\$BuildType\app-$BuildType.apk"
if (Test-Path $apkPath) {
    Write-Host "`n==========================================" -ForegroundColor Green
    Write-Host "  APK build success: $apkPath" -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Green
}
