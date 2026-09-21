# Poster

A desktop API testing tool built with [Tauri v2](https://tauri.app), React 19, TypeScript and Vite. It lets you organize requests into projects, send HTTP requests, inspect responses, manage variables/extractions, and run load tests — all stored locally in SQLite.

## Prerequisites

- **Rust** (stable toolchain) — https://rustup.rs
- **Node.js** (v20 or newer recommended) — https://nodejs.org
- Platform-specific build dependencies for Tauri:
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` (see [Tauri Linux prerequisites](https://tauri.app/start/prerequisites/))
  - **Windows**: Microsoft C++ Build Tools and WebView2 runtime (usually preinstalled on Windows 10/11)

## Getting started

```sh
git clone https://github.com/ocakkemalettin/poster.git
cd poster
npm install
```

## Running in development

Start the Tauri dev server (compiles the Rust backend and opens a window with hot-reloading frontend):

```sh
npm run tauri dev
```

### Linux

Install the system dependencies first if you haven't already:

```sh
# Debian/Ubuntu
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Then run:

```sh
npm run tauri dev
```

### Windows

Make sure the **Microsoft C++ Build Tools** (with the "Desktop development with C++" workload) and the **WebView2 runtime** are installed. Then, in PowerShell or CMD:

```powershell
npm install
npm run tauri dev
```

## Building a release binary

```sh
npm run tauri build
```

- Linux: installer is written to `src-tauri/target/release/bundle/` (`.deb`, `.rpm`, AppImage depending on your distro)
- Windows: installers are written to `src-tauri/target/release/bundle/msi/` and `.../nsis/`

## Project structure

```
├── src/                  # React frontend
│   ├── components/       # UI panels (project tree, request editor, response viewer, ...)
│   ├── api.ts            # Tauri invoke wrappers
│   └── types.ts          # Shared TypeScript types
├── src-tauri/            # Rust backend
│   ├── src/commands.rs   # Tauri commands (CRUD, send request, load tests)
│   ├── src/db.rs         # SQLite persistence
│   └── src/load_test.rs  # Load testing engine
└── vite.config.ts        # Vite config with Tauri integration
```

## Recommended IDE setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://rust-lang.github.io/rust-analyzer/)
