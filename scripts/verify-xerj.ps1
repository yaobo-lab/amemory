$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$required = @(
    "crates/xerj/LICENSE",
    "crates/xerj/NOTICE",
    "crates/xerj/README.md",
    "crates/xerj/engine/Cargo.toml",
    "crates/xerj/engine/Cargo.lock",
    "crates/xerj/engine/.cargo/config.toml",
    "crates/xerj/engine/proto/xerj.proto",
    "crates/xerj/engine/tests/es-compat-yaml/Cargo.toml",
    "crates/xerj/xerj-ux/index.html",
    "patches/xerj/upstream.toml"
)

foreach ($item in $required) {
    $path = Join-Path $root $item
    if (-not (Test-Path $path)) {
        throw "Missing required XERJ vendored path: $item"
    }
}

cargo metadata --manifest-path (Join-Path $root "crates/xerj/engine/Cargo.toml") --format-version 1 | Out-Null
Write-Host "XERJ vendored capsule verified."
