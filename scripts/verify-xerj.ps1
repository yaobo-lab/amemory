# scripts/verify-xerj.ps1
# Verification script for XERJ vendored capsule and custom patches.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Write-Host "==> [1/4] 正在校验 XERJ 上游必要文件完整性..." -ForegroundColor Cyan

$required = @(
    "crates/xerj/LICENSE",
    "crates/xerj/NOTICE",
    "crates/xerj/README.md",
    "crates/xerj/UPSTREAM_VERSION",
    "crates/xerj/engine/Cargo.toml",
    "crates/xerj/engine/Cargo.lock",
    "crates/xerj/engine/.cargo/config.toml",
    "crates/xerj/engine/proto/xerj.proto",
    "crates/xerj/engine/tests/es-compat-yaml/Cargo.toml",
    "crates/xerj/xerj-ux/index.html",
    "patches/upstream.toml",
    "patches/0001-expose-mcp-provider.patch",
    "patches/0002-expose-server-bootstrap.patch"
)

foreach ($item in $required) {
    $path = Join-Path $root $item
    if (-not (Test-Path $path)) {
        throw "缺失关键 XERJ 路径或补丁文件: $item"
    }
}
Write-Host "    必要文件与元数据清单检查通过。" -ForegroundColor Green

Write-Host "==> [2/4] 正在校验定制 Patch 补丁应用状态..." -ForegroundColor Cyan

# 校验 0001-expose-mcp-provider.patch 是否成功应用
$mcpLibPath = Join-Path $root "crates/xerj/engine/crates/xerj-mcp/src/lib.rs"
$mcpLibContent = Get-Content -Path $mcpLibPath -Raw -Encoding UTF8
if ($mcpLibContent -notmatch "pub struct McpProvider") {
    throw "补丁失效: crates/xerj/engine/crates/xerj-mcp/src/lib.rs 中未找到 McpProvider 定义。请运行 git apply patches/0001-expose-mcp-provider.patch"
}

# 校验 0002-expose-server-bootstrap.patch 是否成功应用
$serverLibPath = Join-Path $root "crates/xerj/engine/crates/xerj-server/src/lib.rs"
if (-not (Test-Path $serverLibPath)) {
    throw "补丁失效: crates/xerj/engine/crates/xerj-server/src/lib.rs 不存在。请运行 git apply patches/0002-expose-server-bootstrap.patch"
}
Write-Host "    定制补丁 (McpProvider 与 Server Bootstrap) 生效正常。" -ForegroundColor Green

Write-Host "==> [3/4] 正在校验 XERJ 独立 Workspace 依赖元数据..." -ForegroundColor Cyan
cargo metadata --manifest-path (Join-Path $root "crates/xerj/engine/Cargo.toml") --format-version 1 | Out-Null
Write-Host "    XERJ Cargo Workspace 元数据合法。" -ForegroundColor Green

Write-Host "==> [4/4] 正在对适配层 (crates/adapter) 进行快速编译检查..." -ForegroundColor Cyan
cargo check -p adapter --quiet
Write-Host "    适配防腐层 (adapter) 与底层 XERJ 接口契约一致！" -ForegroundColor Green

Write-Host "`n✅ XERJ 源码胶囊与补丁体系校验全部通过！" -ForegroundColor Green
