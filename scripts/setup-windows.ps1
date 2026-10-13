<#
.SYNOPSIS
  Prepares a Windows 10/11 machine for FocusMode development.

.DESCRIPTION
  Idempotent: checks every component first and installs only what is missing.
    - MSVC C++ build tools + Windows SDK (adds them to an existing Visual Studio 2022,
      or installs VS 2022 Build Tools)
    - Git LFS (binary assets: music, mascot art, icons) + git lfs install / lfs pull
    - Rust (rustup, stable toolchain, clippy, rustfmt)
    - Node.js LTS
    - GitHub CLI (gh) - optional, for pull requests from the terminal
    - just - optional task runner (justfile in the repo root)
    - WebView2 runtime (check only; it ships with Windows 10/11)

  Run from PowerShell:
    powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1
    powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1 -CheckOnly
    powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1 -SkipGh

  Installers may show a UAC prompt. Restart VS Code / the terminal afterwards so PATH is reloaded.
#>
param(
    [switch]$CheckOnly,
    [switch]$SkipGh
)

# Not 'Stop': Windows PowerShell 5.1 turns harmless stderr output of native tools (rustup, winget)
# into terminating errors. Failures are detected via $LASTEXITCODE and explicit throws instead.
$ErrorActionPreference = 'Continue'

$VcComponent  = 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
$SdkComponent = 'Microsoft.VisualStudio.Component.Windows11SDK.22621'
$VsInstaller  = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\setup.exe"

function Write-Step($msg) { Write-Host "`n== $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "   OK   $msg" -ForegroundColor Green }
function Write-Miss($msg) { Write-Host "   MISS $msg" -ForegroundColor Yellow }

function Update-SessionPath {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user    = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin"
}

function Test-Command($name) { $null -ne (Get-Command $name -ErrorAction SilentlyContinue) }

function Get-Vs2022Paths {
    Get-ChildItem "$env:ProgramFiles\Microsoft Visual Studio\2022" -Directory -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty FullName
}

function Test-Msvc {
    foreach ($vs in Get-Vs2022Paths) {
        $linker = Get-ChildItem "$vs\VC\Tools\MSVC\*\bin\Hostx64\x64\link.exe" -ErrorAction SilentlyContinue
        if ($linker) { return $true }
    }
    return $false
}

function Test-WinSdk {
    $null -ne (Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\Lib\*\um\x64\kernel32.lib" -ErrorAction SilentlyContinue)
}

function Invoke-VsModify($installPath) {
    $vsArgs = @('modify', '--installPath', "`"$installPath`"",
        '--add', $VcComponent, '--add', $SdkComponent, '--passive', '--norestart')
    # The VS installer sometimes exits with 1 while it is still checking for updates; retry once.
    foreach ($attempt in 1..2) {
        $p = Start-Process -FilePath $VsInstaller -ArgumentList $vsArgs -Verb RunAs -Wait -PassThru
        if ($p.ExitCode -eq 0 -or $p.ExitCode -eq 3010) { return }
        Write-Host "   VS installer exit code $($p.ExitCode), retrying in 60 s..." -ForegroundColor Yellow
        Start-Sleep -Seconds 60
    }
    throw "Visual Studio installer failed. Open 'Visual Studio Installer' and add 'MSVC v143 x64/x86 build tools' and 'Windows 11 SDK' manually."
}

function Install-Winget($id) {
    winget install --id $id -e --source winget --accept-source-agreements --accept-package-agreements --silent
    if ($LASTEXITCODE -ne 0) { throw "winget install $id failed (exit $LASTEXITCODE)" }
    Update-SessionPath
}

$missing = @()
Update-SessionPath

# --- winget ---------------------------------------------------------------
Write-Step 'winget'
if (Test-Command winget) { Write-Ok 'winget' }
else { throw "winget not found. Install 'App Installer' from the Microsoft Store and re-run." }

# --- MSVC + Windows SDK ---------------------------------------------------
Write-Step 'MSVC C++ build tools + Windows SDK'
$hasMsvc = Test-Msvc
$hasSdk  = Test-WinSdk
if ($hasMsvc) { Write-Ok 'MSVC linker' } else { Write-Miss 'MSVC linker (link.exe)' }
if ($hasSdk)  { Write-Ok 'Windows SDK' } else { Write-Miss 'Windows SDK (kernel32.lib)' }
if (-not ($hasMsvc -and $hasSdk)) {
    $missing += 'MSVC/SDK'
    if (-not $CheckOnly) {
        $vs = Get-Vs2022Paths | Select-Object -First 1
        if ($vs -and (Test-Path $VsInstaller)) {
            Write-Host "   Adding components to existing $vs"
            Invoke-VsModify $vs
        } else {
            Write-Host '   Installing Visual Studio 2022 Build Tools (several GB)...'
            winget install --id Microsoft.VisualStudio.2022.BuildTools -e --source winget `
                --accept-source-agreements --accept-package-agreements `
                --override "--add $VcComponent --add $SdkComponent --passive --wait --norestart"
            if ($LASTEXITCODE -ne 0) { throw "Build Tools install failed (exit $LASTEXITCODE)" }
        }
        if (-not (Test-Msvc) -or -not (Test-WinSdk)) { throw 'MSVC or Windows SDK still missing after install.' }
        Write-Ok 'MSVC + Windows SDK installed'
    }
}

# --- Git LFS (music, mascot art, icons) -------------------------------------
Write-Step 'Git LFS'
$hasLfs = $false
if (Test-Command git) {
    git lfs version 2>$null | Out-Null
    $hasLfs = ($LASTEXITCODE -eq 0)
}
if ($hasLfs) { Write-Ok "$(git lfs version 2>$null)" }
else {
    Write-Miss 'git-lfs'
    $missing += 'git-lfs'
    if (-not $CheckOnly) { Install-Winget 'GitHub.GitLFS' }
}
if (-not $CheckOnly -and (Test-Command git)) {
    # Hooks + smudge filter for this user, then replace pointer files in this checkout with the real binaries.
    git lfs install 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "git lfs install failed (exit $LASTEXITCODE)" }
    git -C (Split-Path $PSScriptRoot -Parent) lfs pull 2>$null | Out-Null
    Write-Ok 'git lfs install + lfs pull'
}

# --- Rust -----------------------------------------------------------------
Write-Step 'Rust (rustup)'
if (Test-Command rustup) {
    Write-Ok "$(rustup --version 2>$null)"
} else {
    Write-Miss 'rustup'
    $missing += 'rustup'
    if (-not $CheckOnly) { Install-Winget 'Rustlang.Rustup' }
}
if (-not $CheckOnly -and (Test-Command rustup)) {
    # rust-toolchain.toml in the repo pins the exact version; this just makes sure a default exists.
    rustup default stable 2>$null | Out-Null
    rustup component add clippy rustfmt 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "rustup component add failed (exit $LASTEXITCODE)" }
    Write-Ok "$(rustc --version), $(cargo clippy --version 2>$null)"
}

# --- Node.js --------------------------------------------------------------
Write-Step 'Node.js LTS'
if (Test-Command node) { Write-Ok "node $(node --version), npm $(npm --version)" }
else {
    Write-Miss 'node'
    $missing += 'node'
    if (-not $CheckOnly) { Install-Winget 'OpenJS.NodeJS.LTS'; Write-Ok "node $(node --version)" }
}

# --- GitHub CLI -----------------------------------------------------------
if (-not $SkipGh) {
    Write-Step 'GitHub CLI (optional)'
    if (Test-Command gh) { Write-Ok "$((gh --version)[0])" }
    else {
        Write-Miss 'gh'
        $missing += 'gh'
        if (-not $CheckOnly) { Install-Winget 'GitHub.cli'; Write-Ok 'gh installed - run: gh auth login' }
    }
}

# --- just (task runner) ---------------------------------------------------
Write-Step 'just (task runner, optional)'
if (Test-Command just) { Write-Ok "$(just --version)" }
else {
    Write-Miss 'just'
    $missing += 'just'
    if (-not $CheckOnly) { Install-Winget 'Casey.Just'; Write-Ok 'just installed' }
}

# --- WebView2 -------------------------------------------------------------
Write-Step 'WebView2 runtime'
$wv = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
if ($wv.pv) { Write-Ok "WebView2 $($wv.pv)" }
else { Write-Miss 'WebView2 - Tauri dev builds install it on demand; or get it from https://developer.microsoft.com/microsoft-edge/webview2/' }

# --- Summary --------------------------------------------------------------
Write-Step 'Summary'
if ($CheckOnly) {
    if ($missing.Count -eq 0) { Write-Ok 'Everything is installed.' }
    else { Write-Host "   Missing: $($missing -join ', '). Run without -CheckOnly to install." -ForegroundColor Yellow; exit 1 }
} else {
    Write-Ok 'Done. Restart VS Code / the terminal so the new PATH is picked up.'
    Write-Host '   Recommended VS Code extensions: rust-analyzer, Tauri, Svelte for VS Code, Even Better TOML, CodeLLDB.'
}
