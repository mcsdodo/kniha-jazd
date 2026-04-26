**Date:** 2026-04-26
**Subject:** Implementation plan — Cargo workspace split for src-tauri
**Status:** Planning

## Context

See [01-task.md](./01-task.md) for the full problem statement and goals. In short: convert the single `kniha-jazd` crate at [src-tauri/](../../src-tauri/) into three sibling crates so the headless web binary stops linking Tauri (and the GTK runtime libs vanish from the Docker image).

This plan is a file-by-file execution map. Every claim about Tauri-coupling has been verified against the current source.

## Why a workspace, not a feature flag

The competing approach in [06-tauri-feature-gating.md](../_TECH_DEBT/06-tauri-feature-gating.md) sprinkles `#[cfg(feature = "desktop")]` across ~74 wrapper functions. Same Docker savings, but every future contributor must remember to gate new code or the web build silently couples to Tauri. A workspace makes the boundary structural — a member crate physically cannot reference what it didn't declare. Same calendar cost (~3 days), better long-term hygiene.

## Target structure

```
src-tauri/
├── Cargo.toml                  # workspace root: [workspace] members = ["core","desktop","web"]
├── Cargo.lock                  # shared
├── target/                     # shared
├── core/                       # NEW — pure library, NO tauri
│   ├── Cargo.toml
│   ├── migrations/             # MOVED from src-tauri/migrations/
│   └── src/
│       ├── lib.rs              # pub mod calculations; pub mod db; pub mod server; pub mod commands_internal; ...
│       ├── calculations/       # MOVED
│       ├── db.rs               # MOVED
│       ├── db_location.rs      # MOVED
│       ├── models.rs           # MOVED
│       ├── schema.rs           # MOVED
│       ├── app_state.rs        # MOVED
│       ├── settings.rs         # MOVED
│       ├── export.rs           # MOVED
│       ├── receipts.rs         # MOVED
│       ├── gemini.rs           # MOVED
│       ├── constants.rs        # MOVED
│       ├── server/
│       │   ├── mod.rs          # MOVED minus resolve_static_dir* (extracted to desktop)
│       │   ├── dispatcher.rs   # MOVED — already Tauri-free
│       │   ├── dispatcher_async.rs
│       │   └── manager.rs
│       └── commands_internal/  # NEW — extracted *_internal fns + pure helpers
│           ├── mod.rs          # check_read_only! macro, helper re-exports
│           ├── helpers.rs      # parse_iso_datetime, get_db_paths_for_dir, calculate_trip_numbers, etc.
│           ├── trips.rs
│           ├── vehicles.rs
│           ├── backup.rs
│           ├── export_cmd.rs
│           ├── receipts_cmd.rs
│           ├── statistics.rs
│           ├── settings_cmd.rs
│           ├── server_cmd.rs
│           └── integrations.rs
├── desktop/                    # NEW — Tauri shell
│   ├── Cargo.toml              # depends on kniha-jazd-core + tauri + tauri-plugin-*
│   ├── build.rs                # MOVED — tauri_build::build()
│   ├── tauri.conf.json         # MOVED
│   ├── tauri.conf.dev.json     # MOVED
│   ├── capabilities/           # MOVED
│   ├── icons/                  # MOVED
│   └── src/
│       ├── main.rs             # fn main() { app::run() }
│       ├── lib.rs              # MOVED from src-tauri/src/lib.rs (Tauri Builder + setup + invoke_handler!)
│       ├── static_dir.rs       # NEW — resolve_static_dir(&tauri::App) + _from_handle (extracted from server/mod.rs)
│       └── commands/           # 9 files — ONLY #[tauri::command] wrappers
│           ├── mod.rs          # get_app_data_dir(&AppHandle), get_db_paths(&AppHandle) — Tauri-flavored helpers
│           ├── trips.rs        # 7 thin wrappers calling kniha_jazd_core::commands_internal::trips::*
│           ├── vehicles.rs     # 6 wrappers
│           ├── backup.rs       # 8 wrappers
│           ├── export_cmd.rs   # 1 wrapper
│           ├── receipts_cmd.rs # 5 wrappers
│           ├── statistics.rs   # 3 wrappers
│           ├── settings_cmd.rs # 3 wrappers
│           ├── server_cmd.rs   # 3 wrappers
│           └── integrations.rs # 4 wrappers
└── web/                        # NEW — replaces src/bin/web.rs
    ├── Cargo.toml              # depends ONLY on kniha-jazd-core + tokio
    └── src/main.rs             # MOVED from src-tauri/src/bin/web.rs (rename app_lib::* → kniha_jazd_core::*)
```

## Step-by-step execution

Each step ends in a verifiable state — run the verification commands before moving on. **Do steps in order**; later steps depend on earlier ones.

### Step 1 — Create the workspace skeleton (no code moved yet)

1. Rename current [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) → `src-tauri/Cargo.toml.old` (kept for reference during migration; deleted at end).
2. Write new minimal `src-tauri/Cargo.toml`:
   ```toml
   [workspace]
   members = ["core", "desktop", "web"]
   resolver = "2"

   [workspace.package]
   version = "0.33.0"
   edition = "2021"
   rust-version = "1.77.2"
   license = "GPL-3.0"
   repository = "https://github.com/mcsdodo/kniha-jazd"

   [workspace.dependencies]
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   chrono = { version = "0.4", features = ["serde"] }
   tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros", "net", "signal", "sync"] }
   log = "0.4"
   thiserror = "1"
   ```
3. Create empty `src-tauri/core/`, `src-tauri/desktop/`, `src-tauri/web/` with stub `Cargo.toml` files (each with `[package] name = "..."`, version, edition, an empty `src/lib.rs` or `src/main.rs`).
4. Verify: `cd src-tauri && cargo metadata --no-deps` lists 3 members. No source moved yet.

### Step 2 — Move pure modules into core (no API surface changes)

Files to relocate (`git mv` to preserve history):

| From | To |
|------|-----|
| [src-tauri/src/calculations/](../../src-tauri/src/calculations/) | `src-tauri/core/src/calculations/` |
| [src-tauri/src/db.rs](../../src-tauri/src/db.rs), [db_tests.rs](../../src-tauri/src/db_tests.rs) | `src-tauri/core/src/` |
| [src-tauri/src/db_location.rs](../../src-tauri/src/db_location.rs) | `src-tauri/core/src/` |
| [src-tauri/src/models.rs](../../src-tauri/src/models.rs) | `src-tauri/core/src/` |
| [src-tauri/src/schema.rs](../../src-tauri/src/schema.rs) | `src-tauri/core/src/` |
| [src-tauri/src/app_state.rs](../../src-tauri/src/app_state.rs) | `src-tauri/core/src/` |
| [src-tauri/src/settings.rs](../../src-tauri/src/settings.rs) | `src-tauri/core/src/` |
| [src-tauri/src/export.rs](../../src-tauri/src/export.rs) and `export_tests.rs` | `src-tauri/core/src/` |
| [src-tauri/src/receipts.rs](../../src-tauri/src/receipts.rs) and `receipts_tests.rs` | `src-tauri/core/src/` |
| [src-tauri/src/gemini.rs](../../src-tauri/src/gemini.rs) and `gemini_tests.rs` | `src-tauri/core/src/` |
| [src-tauri/src/constants.rs](../../src-tauri/src/constants.rs) | `src-tauri/core/src/` |
| [src-tauri/migrations/](../../src-tauri/migrations/) | `src-tauri/core/migrations/` |

Populate `core/Cargo.toml` with deps actually used by these modules: `diesel`, `libsqlite3-sys` (bundled feature), `diesel_migrations`, `chrono`, `uuid`, `rand`, `thiserror`, `reqwest`, `base64`, `tokio`, `local-ip-address`, `hostname`, `url`, `axum`, `tower-http`, `log`, `serde`, `serde_json`. **No tauri.**

Write `core/src/lib.rs`:
```rust
pub mod app_state;
pub mod calculations;
pub mod constants;
pub mod db;
pub mod db_location;
pub mod export;
pub mod gemini;
pub mod models;
pub mod receipts;
pub mod schema;
pub mod settings;
// server and commands_internal added in later steps
```

Verify: `cargo build -p kniha-jazd-core` succeeds. `cargo test -p kniha-jazd-core` passes (subset of 195 tests covering moved modules).

### Step 3 — Move server/ into core (extract Tauri-flavored helpers)

1. `git mv src-tauri/src/server/dispatcher.rs src-tauri/core/src/server/dispatcher.rs` (and `dispatcher_async.rs`, `manager.rs`).
2. Move [src-tauri/src/server/mod.rs](../../src-tauri/src/server/mod.rs) to `src-tauri/core/src/server/mod.rs` but **delete** the two functions `resolve_static_dir(&tauri::App)` and `resolve_static_dir_from_handle(&tauri::AppHandle)` (lines 174-191) and any `use tauri::Manager` lines.
3. Add `pub mod server;` to `core/src/lib.rs`.
4. Verify: `cargo build -p kniha-jazd-core` succeeds, `cargo grep -r 'tauri' core/src/` returns nothing (use [Grep](../..) tool, not shell grep).

### Step 4 — Split commands/ between core (internals) and desktop (wrappers)

This is the bulk of the work. For each file in [src-tauri/src/commands/](../../src-tauri/src/commands/):

1. Open the file. It contains both `#[tauri::command]` wrappers and `*_internal` functions.
2. Create a matching file in `core/src/commands_internal/<name>.rs` containing **only** the `_internal` functions (and any pure helper functions they call locally). Imports: `use crate::{db::Database, app_state::AppState, models::*, ...}`.
3. Create a matching file in `desktop/src/commands/<name>.rs` containing **only** the `#[tauri::command]` wrappers, each rewritten as a thin delegator:
   ```rust
   use kniha_jazd_core::commands_internal::trips as inner;
   use kniha_jazd_core::{db::Database, app_state::AppState};
   use std::sync::Arc;

   #[tauri::command]
   pub async fn create_trip(
       db: tauri::State<'_, Arc<Database>>,
       app_state: tauri::State<'_, Arc<AppState>>,
       vehicle_id: String,
       /* ... */
   ) -> Result<Trip, String> {
       inner::create_trip_internal(&db, &app_state, vehicle_id, /* ... */).await
   }
   ```
4. Delete the original `src-tauri/src/commands/<name>.rs` file.
5. Move the corresponding `*_tests.rs` file (e.g., `commands_tests.rs`) to `core/src/commands_internal/` — tests already call `*_internal` functions directly, so they work unchanged.

Files to split (counts of `#[tauri::command]` per file):

| Source | core/commands_internal/ | desktop/commands/ |
|--------|------------------------|-------------------|
| [trips.rs](../../src-tauri/src/commands/trips.rs) (10) | trips.rs (`*_internal` bodies) | trips.rs (10 wrappers) |
| [vehicles.rs](../../src-tauri/src/commands/vehicles.rs) (6) | vehicles.rs | vehicles.rs |
| [backup.rs](../../src-tauri/src/commands/backup.rs) (11) | backup.rs | backup.rs |
| [export_cmd.rs](../../src-tauri/src/commands/export_cmd.rs) (2) | export_cmd.rs | export_cmd.rs |
| [receipts_cmd.rs](../../src-tauri/src/commands/receipts_cmd.rs) (17) | receipts_cmd.rs | receipts_cmd.rs |
| [statistics.rs](../../src-tauri/src/commands/statistics.rs) (4) | statistics.rs | statistics.rs |
| [settings_cmd.rs](../../src-tauri/src/commands/settings_cmd.rs) (16) | settings_cmd.rs | settings_cmd.rs |
| [server_cmd.rs](../../src-tauri/src/commands/server_cmd.rs) (3) | server_cmd.rs | server_cmd.rs |
| [integrations.rs](../../src-tauri/src/commands/integrations.rs) (5) | integrations.rs | integrations.rs |

Split [src-tauri/src/commands/mod.rs](../../src-tauri/src/commands/mod.rs):
- `parse_iso_datetime`, `get_db_paths_for_dir`, `calculate_trip_numbers`, `calculate_odometer_start`, `generate_month_end_rows`, `check_read_only!` macro → `core/src/commands_internal/mod.rs` (or `helpers.rs`).
- `get_app_data_dir(app: &tauri::AppHandle)`, `get_db_paths(app: &tauri::AppHandle)` → `desktop/src/commands/mod.rs`.

Verify: `cargo build -p kniha-jazd-core` succeeds with `cargo tree -p kniha-jazd-core | grep -i tauri` returning empty.

### Step 5 — Build the desktop crate

1. Move [src-tauri/build.rs](../../src-tauri/build.rs) → `desktop/build.rs`.
2. Move [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json), [tauri.conf.dev.json](../../src-tauri/tauri.conf.dev.json), [capabilities/](../../src-tauri/capabilities/), [icons/](../../src-tauri/icons/) → `desktop/`.
3. Move [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs) → `desktop/src/lib.rs`. Update its imports: `use crate::db::Database` → `use kniha_jazd_core::db::Database`, etc. The `commands::*` references in the `invoke_handler!` macro now point to the local `desktop/src/commands/` modules (already created in Step 4).
4. Create `desktop/src/static_dir.rs` containing the two extracted `resolve_static_dir*` functions. Update `lib.rs` line 181 (`server::resolve_static_dir(app)`) → `crate::static_dir::resolve_static_dir(app)`.
5. Create minimal `desktop/src/main.rs`:
   ```rust
   fn main() {
       app::run();
   }
   ```
   (Or keep `lib.rs::run()` as the binary entry via Tauri's standard pattern — match what existed before.)
6. Populate `desktop/Cargo.toml`:
   ```toml
   [package]
   name = "kniha-jazd-desktop"
   version.workspace = true
   edition.workspace = true
   # ...

   [build-dependencies]
   tauri-build = { version = "2.5.3", features = [] }

   [dependencies]
   kniha-jazd-core = { path = "../core" }
   tauri = { version = "2.9.5", features = [] }
   tauri-plugin-log = "2"
   tauri-plugin-opener = "2"
   tauri-plugin-updater = "2"
   tauri-plugin-process = "2"
   tauri-plugin-dialog = "2"
   serde.workspace = true
   serde_json.workspace = true
   log.workspace = true
   tokio.workspace = true
   ```
7. Update [package.json](../../package.json) `tauri` config (or [tauri.conf.json](../../src-tauri/tauri.conf.json)) so `tauri build` resolves the desktop crate. Likely `"build.beforeDevCommand": "npm run dev"` and adjusted `frontendDist` paths (relative to `desktop/`).

Verify: `cargo build -p kniha-jazd-desktop` succeeds. `npm run tauri dev` launches the GUI.

### Step 6 — Build the web crate

1. `git mv src-tauri/src/bin/web.rs src-tauri/web/src/main.rs`.
2. In `web/src/main.rs`, replace `use app_lib::app_state::AppState` → `use kniha_jazd_core::app_state::AppState` (and same for `db`, `server`).
3. Populate `web/Cargo.toml`:
   ```toml
   [package]
   name = "kniha-jazd-web"
   version.workspace = true
   edition.workspace = true

   [[bin]]
   name = "kniha-jazd-web"
   path = "src/main.rs"

   [dependencies]
   kniha-jazd-core = { path = "../core" }
   tokio.workspace = true
   ```
4. Verify: `cargo build -p kniha-jazd-web --release` succeeds. Then on Linux/WSL: `ldd target/release/kniha-jazd-web | grep -E 'gdk|webkit|gtk|soup|appindicator|rsvg'` returns **nothing**.

### Step 7 — Update Dockerfile.web

In [Dockerfile.web](../../Dockerfile.web):

| Line(s) | Change |
|---------|--------|
| 11-18 (builder stage `apt-get install`) | Drop `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`. Keep `libssl-dev`, `pkg-config`. |
| 23-27 (COPY src-tauri/Cargo.toml etc.) | Update to copy workspace structure: `COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./` then `COPY src-tauri/core ./core`, `COPY src-tauri/web ./web`. (No need to copy `desktop/` — web build doesn't reference it.) |
| 29-32 (stub-source pre-build) | Adjust to stub `core/src/lib.rs` and `web/src/main.rs`. |
| 39-40 (real build) | `RUN cargo build --release -p kniha-jazd-web` |
| 65-74 (runtime stage `apt-get install`) | Drop `libgtk-3-0`, `libwebkit2gtk-4.1-0`, `libsoup-3.0-0`, `libayatana-appindicator3-1`, `librsvg2-2`. Keep `ca-certificates`, `curl`. |
| 60-64 (comment block explaining workaround) | Replace with one-liner noting the binary is fully headless. |
| 76 | `COPY --from=rust-builder /app/target/release/kniha-jazd-web /usr/local/bin/kniha-jazd-web` (path now under workspace `target/`, not `src-tauri/target/`). |

Verify: `docker build -f Dockerfile.web -t kj-web:slim .` succeeds. `docker images kj-web:slim` shows ≤120 MB.

### Step 8 — Update CI

In [.github/workflows/test.yml](../../.github/workflows/test.yml):

1. Replace `cargo test` invocations with `cargo test --workspace` (run from `src-tauri/`).
2. Add a build step `cargo build -p kniha-jazd-web --release --no-default-features` to the Linux job to catch desktop/core coupling regressions.
3. Verify Tauri job still uses the desktop crate explicitly (`cargo build -p kniha-jazd-desktop` if needed).

### Step 9 — Cleanup

1. Delete `src-tauri/Cargo.toml.old`.
2. Delete now-empty `src-tauri/src/`, `src-tauri/migrations/`, `src-tauri/build.rs`, etc. (`git mv` already moved them; just rm the empty parent dirs).
3. Remove `default-run = "kniha-jazd"` from any leftover Cargo.toml — workspaces don't need it.
4. Run [CHANGELOG.md](../../CHANGELOG.md) update via `/changelog`: note the Docker image size reduction and headless web binary.
5. Update [_TECH_DEBT/06-tauri-feature-gating.md](../_TECH_DEBT/06-tauri-feature-gating.md): change Status to "Fixed", add Decision Log row pointing at this task and the merge commit.
6. Run `/decision` to record the workspace-split choice as an ADR (architectural decision: "Multi-crate workspace over feature flags for Tauri/web boundary").

## Reused code — no rewrites needed

These already exist and require zero changes (just relocation):

- All `_internal` functions in [src-tauri/src/commands/](../../src-tauri/src/commands/) — already framework-free, take `(db: &Database, app_state: &AppState, plain args)`.
- [server/dispatcher.rs](../../src-tauri/src/server/dispatcher.rs) — sync RPC dispatcher, calls `*_internal` directly.
- [server/dispatcher_async.rs](../../src-tauri/src/server/dispatcher_async.rs) — async RPC dispatcher.
- [server/manager.rs](../../src-tauri/src/server/manager.rs) — server lifecycle.
- The `check_read_only!` macro in [commands/mod.rs](../../src-tauri/src/commands/mod.rs).
- All test files (`*_tests.rs`) — already call `*_internal` functions; no Tauri mocking required.

The only **new** code is:
1. `desktop/src/static_dir.rs` (~20 lines, lifted from server/mod.rs).
2. The two-line wrapper bodies in `desktop/src/commands/*.rs` — each `#[tauri::command]` becomes one delegating call to `kniha_jazd_core::commands_internal::*`.

## Verification checklist

Run after Step 9 to confirm done:

```bash
# From repo root
cd src-tauri

# 1. Workspace builds
cargo build --workspace

# 2. Web binary is GTK-free (run on Linux or WSL)
cargo build --release -p kniha-jazd-web
ldd target/release/kniha-jazd-web | grep -E 'gdk|webkit|gtk|soup|appindicator|rsvg'
# expected: no output

# 3. Core has no Tauri in its dep tree
cargo tree -p kniha-jazd-core | grep -i tauri
# expected: no output

# 4. All tests pass
cargo test --workspace
# expected: 195 backend tests pass

cd ..
npm run test:integration:tier1
# expected: smoke suite passes against debug Tauri build

# 5. Desktop still builds + runs
npm run tauri build
npm run tauri dev   # smoke check: app launches

# 6. Docker image shrinks
docker build -f Dockerfile.web -t kj-web:slim .
docker images kj-web:slim
# expected: SIZE ≤ 120 MB (target: ~80 MB; baseline: ~300 MB)

docker run --rm -d -p 3456:3456 -v kj-data:/data --name kj-test kj-web:slim
sleep 2
curl -fs http://localhost:3456/health
# expected: ok
docker stop kj-test
```

## Rollback plan

Each step's outcome is committable independently:
- Step 1 (workspace skeleton) is reversible by deleting member dirs and restoring `Cargo.toml.old`.
- Steps 2-3 (move pure modules + server) are reversible via `git revert`.
- Step 4 (commands split) is the riskiest — commit per command file split so a single file's bug can be reverted without losing the rest.
- Steps 5-7 (desktop, web, Dockerfile) are reversible via `git revert`.

If integration tests break after Step 4, the most likely cause is a mismatched `*_internal` signature; diff the `desktop/src/commands/<name>.rs` wrapper against the pre-split file to find the drift.

## Out of scope

- Renaming the binary from `kniha-jazd-web` back to `web` — keeping the new name avoids confusion and matches the crate name.
- Splitting frontend ([src/](../../src/)) into separate packages — this task only touches Rust.
- Distroless image experimentation — once the GTK libs are dropped, distroless becomes possible but isn't required by the success criteria.
