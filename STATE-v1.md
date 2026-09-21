# Poster — Project State v1

> **Purpose of this file:** This is the canonical handoff document for LLM inference sessions.
> Feed this file (plus any newer `STATE-vN.md` / feature files stacked on top) to an LLM so it can
> continue development without re-discovering the codebase. Read top-down: overview → architecture →
> features → conventions → known pitfalls. Newer state files override/extend older ones.

## 1. What this project is

**Poster** is a cross-platform desktop **API client / Postman alternative** built with
**Tauri v2 + React 19 + TypeScript** (frontend) and **Rust** (backend), using **SQLite** for storage.

- Working directory: `/home/kemalettin-ocak/Code/poster`
- Original requirements spec: `HANDOFF.md` (Turkish; the approved feature list). This file supersedes its "remaining work" section — everything listed there is now implemented and verified.
- Target platforms: Linux (developed on Ubuntu) + Windows.

## 2. Architecture at a glance

```
src/                        # React frontend (Vite, TypeScript)
  App.tsx                   # Main shell: state, event listeners, layout wiring
  api.ts                    # All invoke() wrappers + file save/pick helpers
  types.ts                  # Shared TS interfaces (mirror Rust serde structs)
  App.css                   # Dark Postman-like theme
  components/
    ProjectTree.tsx         # Left panel: projects → requests tree
    RequestEditor.tsx       # Center: method/URL bar, tabs (Params/Headers/Auth/Body/Extracts), Send, cURL import modal
    ResponseViewer.tsx      # Bottom: status/time/size, JSON tree + raw + headers
    VariablesPanel.tsx      # Right tab 1: project variables CRUD
    LoadTestPanel.tsx       # Right tab 2: load test config, overrides, live stats, report

src-tauri/src/              # Rust backend
  main.rs                   # bin entry → tauri_app_lib::run()
  lib.rs                    # Builder setup, plugins, command registration, AppState init
  db.rs                     # SQLite layer (Db = Mutex<Connection>), schema + all CRUD
  commands.rs               # All #[tauri::command] handlers (~930 lines)
  load_test.rs              # Load test engine: pacer, workers, stats, report (~470 lines)

src-tauri/Cargo.toml        # Rust deps (see §6)
package.json                # Frontend deps: react 19, @tauri-apps/api v2, vite 8
```

**Data flow:** React `invoke()` → Tauri command in `commands.rs` → `db.rs` (SQLite) or
`load_test.rs` (tokio tasks). Async results come back as return values; live updates come back as
Tauri **events** emitted from the backend.

## 3. Implemented features (all verified working)

### 3.1 Projects & requests (CRUD, SQLite)
- Projects: create / rename / delete; left-panel tree with nested requests.
- Requests per project: name, method, URL, headers (JSON array), body type (`none|json|form|raw`),
  body, auth (JSON). `save_request` upserts by id.

### 3.2 Variables & templating
- Project-scoped variables; `{{var}}` placeholders are substituted in **URL, headers, body, and auth**
  before sending (both single requests and load tests).
- UI: right-panel "Variables" tab with add/edit/delete (Enter key adds).

### 3.3 Auth
- Types: `none | bearer | basic | api_key` (api key can go in header or query, `in_header` flag).
- Serialized as JSON string in `requests.auth_json`; TS helper `parseAuth/authToJson` in `api.ts`.

### 3.4 Send request & response viewer
- `send_request(request_id)` resolves variables, applies auth, executes via reqwest (rustls),
  returns `{status, status_text, time_ms, size_bytes, headers, body}`.
- Response viewer: meta bar + JSON tree view / raw text / headers table.

### 3.5 cURL import & export
- **Import:** `parse_curl` parses `-H/--header`, `--data/-d/--data-raw/--json`, `-u/--user`,
  `-X/--request`, URL. Multipart (`-F`) is NOT supported (by design); unknown flags produce warnings
  shown in the import modal.
- **Export:** `export_curl(request_id)` regenerates a curl command including auth headers.

### 3.6 Extracts (JSONPath → variable)
- Per-request extracts: `{jsonpath, target_var_key}`; after sending, matched values are written to
  project variables and a `variables-changed` event is emitted so the UI refreshes.
- Managed in RequestEditor's "Extracts" tab.

### 3.7 Load testing (the most complex subsystem)
- Config: RPS, total requests, concurrency (1–256), timeout_ms; reuses the request's URL/headers/body/auth.
- **Variable overrides** per run: each `{{var}}` can be overridden with mode `fixed`, `range`
  (`value..value_end`, e.g. `1..1000`), or `list` (`a,b,c`).
- Engine (`load_test.rs`): token-bucket pacer dispatching seq numbers round-robin to N tokio worker
  tasks; each worker executes via a shared reqwest client and reports `(seq, status, latency_ms, ok)`.
- Live stats event `load-test-stats` every 500ms (50ms while cancelling): completed/successes/failures/avg/p95.
- Final report event `load-test-finished`: totals, duration, avg, p95, actual RPS, and `per_request`
  detail **capped at 10,000 entries** (first + last half kept) to keep the IPC payload small.
- Report download: CSV (`seq,status,latency_ms,success`) + JSON via frontend `downloadReport`.

### 3.8 Project export / import (`.poster.json`, version 1)
- Export: human-readable JSON `{version, name, exported_at, variables[], requests[]}` (requests include
  headers/body/auth/extracts). Format spec in `HANDOFF.md` §".poster.json formatı".
- Import: schema validation; mode `new` (create project) or `append` (merge into existing project id).

### 3.9 File I/O
- Uses `tauri-plugin-dialog` for save/open dialogs + custom commands `save_file` / `read_file`
  (deliberately NOT `tauri-plugin-fs`, whose Builder API isn't available in v2 runtime scope config).
- Frontend falls back to browser download/file-input when not running inside Tauri (`isTauri()` check).

## 4. Backend command & event reference

Commands registered in `lib.rs` (all in `commands.rs` unless noted):
`list_projects, create_project, rename_project, delete_project, list_requests, create_request,
save_request, delete_request, list_variables, upsert_variable, delete_variable, list_extracts,
create_extract, delete_extract, send_request, export_curl, parse_curl, export_project, import_project,
start_load_test (async), stop_load_test, cleanup_load_test, load_test_status, save_file, read_file`

Events emitted by backend:
- `variables-changed` → payload `project_id: number`
- `load-test-stats` → payload `LoadTestStats {completed, successes, failures, avg_ms, p95_ms, elapsed_ms}`
- `load-test-finished` → payload `LoadTestReport {total, successes, failures, duration_ms, avg_ms, p95_ms, rps_actual, per_request: PerRequest[]}`

Key shared types (Rust in `commands.rs`/`load_test.rs`, mirrored in `src/types.ts`):
`Header{key,value}`, `Auth{type_,token,username,password,key,value,in_header}`,
`VarOverride{key,mode,value,value_end?}`, `LoadTestConfig{url,method,headers,body_type,body,auth,variables:[(k,v)][],overrides,rps,total_requests,concurrency,timeout_ms}`,
`ResponseData`, `CurlImportResult`, `PerRequest{seq,status,latency_ms,success}`.

## 5. SQLite schema (created in `db.rs::new`)

```sql
projects(id PK, name, created_at, updated_at)
requests(id PK, project_id FK→projects ON DELETE CASCADE, name, method DEFAULT 'GET', url, headers_json, body_type, body, auth_json)
variables(id PK, project_id FK, key, value)
extracts(id PK, request_id FK, source_request_id NULL, jsonpath, target_var_key)
```

DB file location: Tauri app-data dir + `poster.db` (created in `lib.rs` setup).

## 6. Dependencies

**Rust (`src-tauri/Cargo.toml`):** tauri 2, tauri-plugin-opener 2, tauri-plugin-dialog 2.7.3,
serde/serde_json, rusqlite 0.40.2 (bundled), tokio 1.53 (full), reqwest 0.13.5 (`json, query, form, multipart, rustls`),
jsonpath_lib 0.3.0, base64 0.23.1, chrono 0.4.45 (serde), fastrand 2.

**Frontend:** react/react-dom 19, @tauri-apps/api v2, @tauri-apps/plugin-dialog, vite 8, typescript.

## 7. Critical conventions & pitfalls (learned the hard way — read before touching load test or IPC)

1. **Async commands only for tokio work.** `start_load_test` MUST be `async fn`. Sync Tauri
   commands run on the GTK main thread which has **no Tokio reactor** → any `tokio::spawn`/`sleep`
   panics with "there is no reactor running". That panic then crosses wry's IPC/URI-scheme FFI
   callback (non-unwindable) and **aborts the whole app**. Never call tokio APIs from sync commands.
2. **Never let a panic reach the main thread via IPC.** Any panic inside an async command or event
   emit path can propagate into webkit2gtk's `register_uri_scheme` callback → `panic_cannot_unwind`
   → abort. Keep payloads small (the 10k `per_request` cap exists for this reason) and handle errors
   as `Result<_, String>` returns instead of panicking.
3. **Load-test handle lifecycle:** the handle map is `Arc<Mutex<HashMap<String, LoadTestHandle>>>`
   in `AppState`. `spawn_load_test` registers the handle in that shared map **before** spawning
   workers (a fast run must not finish before being tracked). `stop_load_test` only sets the cancel
   flag; the frontend calls `cleanup_load_test` after receiving `load-test-finished` to remove it.
4. **reqwest feature name is `rustls`, NOT `tokio-rustls`** (v0.13) — `cargo add --features tokio-rustls` fails.
5. **File I/O:** use dialog plugin + `save_file`/`read_file` commands; do not try to configure
   `tauri-plugin-fs` scopes in v2 runtime config.
6. **Dark theme selects:** native `<select>` needs `color-scheme: dark` (set globally in App.css) or dropdowns render white/broken.
7. **Layout overflow guardrails:** right panel is ~300px wide; inputs need `min-width: 0` inside
   grid/flex tracks, stat boxes use `minmax(0,1fr)` + ellipsis. Override rows are two stacked rows
   (key+mode / value+end) — don't collapse them back into one 4-column row.
8. **First Rust build is slow** (rusqlite bundled + reqwest rustls). Use `cargo check` for iteration.

## 8. Build & verify commands

```bash
# frontend typecheck + build
npx tsc --noEmit && npm run build

# backend check (fast) / full dev run
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri dev        # from project root; first run compiles Rust (slow)
```

Both currently pass cleanly.

## 9. Known limitations / candidates for future features

- No multipart/form-data file upload in cURL import (`-F` unsupported by design).
- Load test report detail capped at 10k rows (no pagination/streaming of full per-request data to disk yet; `build_report_csv`/`build_summary_csv` helpers exist in commands.rs but are currently unused — wire them up if you want on-disk reports for huge runs).
- No request history / saved responses, no environments beyond project variables, no collections sharing between projects, no packaging scripts (`.deb`/`.AppImage`/`.msi`) yet.
- `create-tauri-app` requires a TTY; the scaffold already exists — don't re-run it.

## 10. How to extend this file for future sessions

When adding a feature: create `STATE-v2.md` (or `FEATURE-<name>.md`) at repo root that
(a) references this file, (b) lists what changed/added with file paths and line anchors,
(c) updates §4 command/event tables if the IPC surface changed, and (d) appends new pitfalls to §7.
Keep older files intact so history is auditable; newest file wins on conflicts.
