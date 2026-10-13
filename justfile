# FocusMode task runner — https://github.com/casey/just
# Install: winget install Casey.Just  (or: scripts\setup-windows.ps1)
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Run the app in dev mode (hot reload)
dev:
    npm run tauri dev

# Run the app in demo mode (40 s focus, 20 s break)
[windows]
demo:
    $env:FOCUSMODE_DEMO = "1"; npm run tauri dev

[unix]
demo:
    FOCUSMODE_DEMO=1 npm run tauri dev

# Run the app in presentation mode (demo timings + showcase data, developer mode for this run only)
[windows]
presentation:
    $env:FOCUSMODE_PRESENTATION = "1"; npm run tauri dev

[unix]
presentation:
    FOCUSMODE_PRESENTATION=1 npm run tauri dev

# Format Rust code
fmt:
    cargo fmt --all

# Full check: same steps as /check and CI
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked
    npm run check

# Rust tests (also regenerates TypeScript bindings)
test:
    cargo test --workspace --locked

# Regenerate TypeScript bindings of the IPC contract (src/lib/bindings)
bindings:
    cargo test -p focus-core --locked

# Release build + installer
build:
    npm run tauri build
