# Poster

A desktop API testing tool built with [Tauri v2](https://tauri.app), React 19, TypeScript and Vite. It lets you organize requests into projects, send HTTP requests, inspect responses, manage variables/extractions, and run load tests — all stored locally in SQLite.

## Prerequisites

- **Rust** (stable toolchain) — https://rustup.rs
  - Verify it's on your PATH in the terminal you'll use: `rustc --version` and `cargo --version`. If they're not recognized, install Rust via rustup and **restart your terminal**.
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

### Verifying your toolchain before building

The Tauri CLI shells out to `cargo` during builds. If you see an error like
`failed to run cargo metadata command`, the CLI couldn't find or run `cargo`. Check:

1. **Rust is on PATH in this terminal** — open a new terminal and run:
   ```sh
   rustc --version
   cargo --version
   ```
2. **Cargo actually works** (from anywhere, not just the project):
   ```sh
   cargo metadata --format-version 1 > /dev/null   # Linux/macOS
   cargo metadata --format-version 1 > $null       # Windows PowerShell
   ```
3. **VS Code integrated terminals inherit your PATH** — if `cargo` works in a normal terminal but not inside VS Code, restart VS Code (or select the default shell profile).
4. **Windows-specific gotchas**:
   - Install the C++ Build Tools with the **"Desktop development with C++"** workload.
   - Make sure the repo is **not inside OneDrive** — move it to a plain path like `D:\code\poster`.
   - Exclude the project folder and `%USERPROFILE%\.cargo` from antivirus scanning if you get odd failures.
5. **Isolate Rust vs Tauri** — run cargo directly in the backend:
   ```sh
   cd src-tauri
   cargo check
   ```
   If this fails, fix your Rust toolchain first; if it passes but `npm run tauri build` still fails, re-run the build from the same terminal where `cargo --version` works. For more detail: `npm run tauri build -- --verbose`.

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
