# WAKARU Windows packaging: portable zip + single-file setup.exe.
# Usage (from the repository root):
#   powershell -ExecutionPolicy Bypass -File windows/package.ps1 [-Version 1.7.0]
# Prerequisites: release app binary built (`npm run tauri build` or
# `cargo build --release`), Rust stable, PowerShell 5.1+.
param([string]$Version = "1.7.0")

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$payload = Join-Path $root "target\wix-payload"
$dist = Join-Path $root "dist-windows"

function Needs($path, $what) {
    if (-not (Test-Path $path)) { throw "missing ${what}: ${path}" }
}

# 1. Locate the release app binary (Tauri output or plain cargo output).
if ($env:CARGO_TARGET_DIR) {
    $appExe = Join-Path $env:CARGO_TARGET_DIR "release\WAKARU.exe"
} else {
    $appExe = Join-Path $root "src-tauri\target\release\WAKARU.exe"
}
Needs $appExe "release WAKARU.exe (build it first)"
Write-Output "app: $appExe"

# 2. Assemble the payload directory.
$license = Join-Path $root "LICENSE"
$firstLaunch = Join-Path $root "windows\FIRST_LAUNCH_WINDOWS.txt"
Needs $license "LICENSE"
Needs $firstLaunch "FIRST_LAUNCH_WINDOWS.txt"
New-Item -ItemType Directory -Force $payload | Out-Null
Copy-Item $appExe (Join-Path $payload "WAKARU.exe") -Force
Copy-Item $license (Join-Path $payload "LICENSE") -Force
Copy-Item $firstLaunch (Join-Path $payload "FIRST_LAUNCH_WINDOWS.txt") -Force

# 3. Build the installer (payload embedded via WAKARU_PAYLOAD_DIR).
$env:WAKARU_PAYLOAD_DIR = $payload
cargo build --release --manifest-path (Join-Path $root "installer\Cargo.toml")
if ($env:CARGO_TARGET_DIR) {
    $setupSrc = Join-Path $env:CARGO_TARGET_DIR "release\wakaru-setup.exe"
} else {
    $setupSrc = Join-Path $root "installer\target\release\wakaru-setup.exe"
}
Needs $setupSrc "installer binary"

# 4. Emit deliverables.
New-Item -ItemType Directory -Force $dist | Out-Null
$setupExe = Join-Path $dist "WAKARU_${Version}_x64-setup.exe"
Copy-Item $setupSrc $setupExe -Force
$zipPath = Join-Path $dist "WAKARU_${Version}_x64-portable.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $payload "*") -DestinationPath $zipPath

foreach ($f in @($setupExe, $zipPath)) {
    $h = (Get-FileHash $f -Algorithm SHA256).Hash.ToLower()
    $h | Out-File -Encoding ascii -NoNewline ($f + ".sha256")
    $size = (Get-Item $f).Length
    Write-Output ("{0}  {1} bytes  sha256={2}" -f (Split-Path $f -Leaf), $size, $h)
}
Write-Output "dist: $dist"
