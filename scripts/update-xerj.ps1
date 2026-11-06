param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [string]$Repo = "https://github.com/xerj-org/xerj.git"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("xerj-" + [guid]::NewGuid().ToString("N"))

try {
    git clone --depth 1 --branch $Tag $Repo $tmp
    $commit = (git -C $tmp rev-parse HEAD).Trim()

    $target = Join-Path $root "crates/xerj"
    if (Test-Path $target) {
        Remove-Item -Recurse -Force $target
    }
    New-Item -ItemType Directory -Force -Path $target | Out-Null

    Copy-Item (Join-Path $tmp "LICENSE") (Join-Path $target "LICENSE")
    Copy-Item (Join-Path $tmp "NOTICE") (Join-Path $target "NOTICE")
    Copy-Item (Join-Path $tmp "README.md") (Join-Path $target "README.md")
    Copy-Item -Recurse (Join-Path $tmp "engine") (Join-Path $target "engine")
    Copy-Item -Recurse (Join-Path $tmp "xerj-ux") (Join-Path $target "xerj-ux")

    $patchDir = Join-Path $root "patches/xerj"
    New-Item -ItemType Directory -Force -Path $patchDir | Out-Null
    Get-ChildItem $patchDir -Filter *.patch | Sort-Object Name | ForEach-Object {
        git apply --directory="crates/xerj" $_.FullName
    }

    $upstream = @"
repository = "$Repo"
tag = "$Tag"
commit = "$commit"
fetched_at = "$(Get-Date -Format o)"
license = "Apache-2.0"
patch_series = "patches/xerj"
"@
    Set-Content -Path (Join-Path $patchDir "upstream.toml") -Value $upstream -Encoding UTF8
}
finally {
    if (Test-Path $tmp) {
        Remove-Item -Recurse -Force $tmp
    }
}
