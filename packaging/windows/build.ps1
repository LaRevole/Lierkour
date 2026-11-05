# Build MyFilmLibrary for Windows (x64).
#
# Usage (from the repository root):
#   powershell -ExecutionPolicy Bypass -File packaging/windows/build.ps1
#
# The resulting portable folder is written to dist/MyFilmLibrary-windows-x64.

$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$frontend = Join-Path $root "frontend"
$manifest = Join-Path $root "src-tauri/Cargo.toml"
$outDir = Join-Path $root "dist/MyFilmLibrary-windows-x64"

Write-Host "=== Building MyFilmLibrary for Windows ===" -ForegroundColor Cyan

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm was not found. Install Node.js from https://nodejs.org/"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo was not found. Install Rust from https://rustup.rs/"
}

Write-Host "Step 1/3: Building frontend..." -ForegroundColor Yellow
Push-Location $frontend
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw "npm ci failed" }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "npm run build failed" }
} finally {
    Pop-Location
}

Write-Host "Step 2/3: Compiling Rust backend..." -ForegroundColor Yellow
# custom-protocol embeds frontend/dist into the binary. Without it the
# release binary tries to connect to the dev server and fails at startup.
cargo build --release --features custom-protocol --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

Write-Host "Step 3/3: Collecting portable build..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
Copy-Item (Join-Path $root "target/release/myfilmlibrary-app.exe") $outDir -Force
Copy-Item (Join-Path $PSScriptRoot "run-portable.bat") $outDir -Force

Write-Host "Done: $outDir" -ForegroundColor Green
Write-Host "  myfilmlibrary-app.exe - normal launch (data in %APPDATA%)"
Write-Host "  run-portable.bat      - portable launch (data in .\data)"
