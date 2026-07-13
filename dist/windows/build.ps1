param(
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"

$DistDir = Join-Path $PSScriptRoot ".." | Join-Path -ChildPath ".."
$ReleaseDir = Join-Path $DistDir "target" "release"
$OutputDir = Join-Path $DistDir "dist"

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "Building RTML v$Version for Windows..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$ExeName = "rtml-$Version-x86_64-windows.exe"
Copy-Item (Join-Path $ReleaseDir "rtml.exe") (Join-Path $OutputDir $ExeName) -Force

Write-Host "Done: dist/$ExeName" -ForegroundColor Green
