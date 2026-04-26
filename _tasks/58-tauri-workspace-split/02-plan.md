# Cargo Workspace Split Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split [src-tauri/](../../src-tauri/) from one crate (`kniha-jazd`) into a Cargo workspace with three members (`kniha-jazd-core`, `kniha-jazd-desktop`, `kniha-jazd-web`) so the headless web binary stops linking Tauri and the Docker image drops ~150 MB of GTK/WebKit runtime libraries.

**Architecture:** `core` is a pure library (DB, calculations, models, server, `_internal` command bodies — no Tauri deps). `desktop` is the Tauri shell + thin `#[tauri::command]` wrappers that delegate into `core::commands_internal::*`. `web` is the headless HTTP server binary that depends only on `core`. Boundary enforced by Cargo's per-crate dep graph instead of `#[cfg(feature = "desktop")]` annotations. During migration each pure module is moved to `core` and re-exported from `desktop`'s `lib.rs` via `pub use kniha_jazd_core::<module>;` to keep the build green; the re-exports are removed in the final cleanup task.

**Tech Stack:** Rust 1.77.2 · Cargo workspaces (resolver = "2") · Tauri 2.9.5 + tauri-plugin-{updater,process,dialog,opener,log} · Diesel 2.2 (sqlite-bundled) · axum 0.8 · tokio 1.x · diesel_migrations 2.2 · WebdriverIO + tauri-driver for integration tests

**Source documents:** [01-task.md](./01-task.md) (problem, goals, success criteria) · [_plan-review.md](./_plan-review.md) (Critical and Important findings this plan addresses) · [_TECH_DEBT/06-tauri-feature-gating.md](../_TECH_DEBT/06-tauri-feature-gating.md) (origin tech debt)

**Platform note:** Primary dev platform is Windows. Bash commands use Unix shell (Git Bash) syntax — `git mv`, `cargo`, `npm`. Linker verification (`ldd`) requires Linux/WSL/Docker; a PowerShell `dumpbin` equivalent is provided.

---

## Pre-flight

Before starting Task 1:

1. Confirm `[package] version = "0.33.0"` in [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) still matches [package.json](../../package.json) — if a release happened between plan-write and execution, update the workspace version field in Task 1 to match.
2. Confirm working tree is clean: `git status --short` returns nothing under `src-tauri/`.
3. Confirm baseline build/tests pass on `main`: `cd src-tauri && cargo test` (expected: 195 tests pass).
4. Confirm Docker baseline: `docker build -f Dockerfile.web -t kj-web:before . && docker images kj-web:before --format "{{.Size}}"` (expected: ~300 MB). Save this number — Task 24 verifies the reduction.
5. Create a feature branch: `git checkout -b feat/tauri-workspace-split`.

---

## Task 1: Relocate existing crate into `desktop/` subdirectory

**Why this first:** The cleanest way to introduce a workspace without breaking the build is to first push the existing single crate into a future-member subdirectory, then add the workspace root above it. After this task the build still works exactly as before, only the path moved.

**Files:**
- Move: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) → `src-tauri/desktop/Cargo.toml`
- Move: [src-tauri/Cargo.lock](../../src-tauri/Cargo.lock) → `src-tauri/Cargo.lock` (stays at workspace root)
- Move: [src-tauri/build.rs](../../src-tauri/build.rs) → `src-tauri/desktop/build.rs`
- Move: [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json), [tauri.conf.dev.json](../../src-tauri/tauri.conf.dev.json) → `src-tauri/desktop/`
- Move: [src-tauri/capabilities/](../../src-tauri/capabilities/), [src-tauri/icons/](../../src-tauri/icons/) → `src-tauri/desktop/`
- Move: [src-tauri/src/](../../src-tauri/src/) → `src-tauri/desktop/src/`
- Move: [src-tauri/migrations/](../../src-tauri/migrations/) → `src-tauri/desktop/migrations/` (will move again to `core/` in Task 5)
- Create: `src-tauri/Cargo.toml` (new workspace root)

**Step 1: Move all crate files into desktop/**

```bash
cd src-tauri
git mv Cargo.toml desktop/Cargo.toml
git mv build.rs desktop/build.rs
git mv tauri.conf.json desktop/tauri.conf.json
git mv tauri.conf.dev.json desktop/tauri.conf.dev.json
git mv capabilities desktop/capabilities
git mv icons desktop/icons
git mv src desktop/src
git mv migrations desktop/migrations
```

**Step 2: Adjust [tauri.conf.json](../../src-tauri/tauri.conf.json) `frontendDist` for the new depth**

The conf file moved one level deeper, so the relative path to the SvelteKit `build/` output changes.

Edit `src-tauri/desktop/tauri.conf.json` line 7:
```diff
-    "frontendDist": "../build",
+    "frontendDist": "../../build",
```

`beforeDevCommand` and `beforeBuildCommand` invoke npm scripts via the parent process and don't need path changes (they resolve from the project root via `tauri-cli`).

The `$schema` reference on line 2 also moved one level deeper — fix it:
```diff
-  "$schema": "../node_modules/@tauri-apps/cli/config.schema.json",
+  "$schema": "../../node_modules/@tauri-apps/cli/config.schema.json",
```

Repeat the same two edits in `src-tauri/desktop/tauri.conf.dev.json`.

**Step 3: Write new workspace root [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml)**

```toml
[workspace]
members = ["desktop"]
resolver = "2"

[workspace.package]
version = "0.33.0"
edition = "2021"
rust-version = "1.77.2"
license = "GPL-3.0"
repository = "https://github.com/mcsdodo/kniha-jazd"
authors = ["mcsdodo"]
```

(Members `core` and `web` get added in Tasks 2 and 3.)

**Step 4: Update [package.json](../../package.json) `tauri` config path**

The `tauri` CLI needs to know where the Tauri config lives. Check current value:
```bash
grep -n '"tauri"' package.json
```
If `package.json` has a `"tauri": { "tauriDir": "src-tauri" }` block (or similar), update to `"tauriDir": "src-tauri/desktop"`. If no such block exists (Tauri auto-discovers), no edit needed; the CLI will need `--config src-tauri/desktop/tauri.conf.json` passed explicitly.

**Step 5: Verify the desktop crate still builds**

```bash
cd src-tauri && cargo build -p kniha-jazd
```
Expected: builds successfully (the package name is still `kniha-jazd` until Task 14 renames it to `kniha-jazd-desktop`).

**Step 6: Verify existing backend tests still pass**

```bash
cd src-tauri && cargo test -p kniha-jazd
```
Expected: 195 tests pass.

**Step 7: Verify Tauri CLI still finds the config**

```bash
npm run tauri info
```
Expected: prints config without error and reports `frontendDist` resolved to project root's `build/`.

**Step 8: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/desktop/ package.json
git commit -m "refactor(tauri): relocate crate into desktop/ subdirectory

Sets up the workspace skeleton by pushing the entire existing
kniha-jazd crate one level deeper. No code or behavior changes —
only paths moved. The single-member workspace root at src-tauri/
prepares for adding core/ (Task 2) and web/ (Task 3).

Refs Task 58."
```

---

## Task 2: Add empty `core` member crate

**Why now:** Establishes the destination for Tauri-free code. Empty crate = green build = safe checkpoint.

**Files:**
- Create: `src-tauri/core/Cargo.toml`
- Create: `src-tauri/core/src/lib.rs`
- Modify: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) — add `core` to members

**Step 1: Create `src-tauri/core/Cargo.toml`**

```toml
[package]
name = "kniha-jazd-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Vehicle logbook core library — DB, calculations, command internals"

[lib]
name = "kniha_jazd_core"
path = "src/lib.rs"

[dependencies]
# Populated incrementally as modules move (Tasks 4-13).

[dev-dependencies]
tempfile = "3"
```

**Step 2: Create `src-tauri/core/src/lib.rs`**

```rust
//! Kniha Jázd core library — Tauri-free.
//!
//! Houses all business logic, persistence, HTTP server, and command
//! internals (`*_internal` functions). Both kniha-jazd-desktop and
//! kniha-jazd-web depend on this crate.

// Modules added incrementally as files move from desktop/ → core/
```

**Step 3: Add `core` to workspace members in [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml)**

```diff
 [workspace]
-members = ["desktop"]
+members = ["core", "desktop"]
 resolver = "2"
```

**Step 4: Verify workspace metadata lists 2 members**

```bash
cd src-tauri && cargo metadata --no-deps --format-version 1 | python -c "import sys,json; print([p['name'] for p in json.load(sys.stdin)['packages']])"
```
Expected: `['kniha-jazd', 'kniha-jazd-core']`

**Step 5: Verify both build**

```bash
cd src-tauri && cargo build --workspace
```
Expected: success.

**Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/core/
git commit -m "build(workspace): add empty kniha-jazd-core member crate"
```

---

## Task 3: Add empty `web` member crate

**Files:**
- Create: `src-tauri/web/Cargo.toml`
- Create: `src-tauri/web/src/main.rs`
- Modify: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) — add `web` to members

**Step 1: Create `src-tauri/web/Cargo.toml`**

```toml
[package]
name = "kniha-jazd-web"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Vehicle logbook headless HTTP server"

[[bin]]
name = "kniha-jazd-web"
path = "src/main.rs"

[dependencies]
# Populated in Task 23 when web binary moves in.
```

**Step 2: Create stub `src-tauri/web/src/main.rs`**

```rust
fn main() {
    println!("kniha-jazd-web stub — implementation moved in Task 23");
}
```

**Step 3: Add `web` to workspace members**

```diff
-members = ["core", "desktop"]
+members = ["core", "desktop", "web"]
```

**Step 4: Verify all three members build**

```bash
cd src-tauri && cargo build --workspace
```
Expected: 3 crates compile.

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/web/
git commit -m "build(workspace): add empty kniha-jazd-web member crate"
```

---

## Task 4: Move `models.rs` to core (first pure module)

**Why models first:** It's the foundation type registry and almost every other module imports from it. Moving it first means later module moves only need `kniha_jazd_core::models::*` already available.

**Files:**
- Move: [src-tauri/desktop/src/models.rs](../../src-tauri/desktop/src/models.rs) → `src-tauri/core/src/models.rs`
- Modify: `src-tauri/core/src/lib.rs` — add `pub mod models;`
- Modify: `src-tauri/core/Cargo.toml` — add `serde`, `chrono`, `uuid` deps
- Modify: `src-tauri/desktop/src/lib.rs` — add `pub use kniha_jazd_core::models;` compat re-export
- Modify: `src-tauri/desktop/Cargo.toml` — add `kniha-jazd-core = { path = "../core" }`

**Step 1: Move the file**

```bash
cd src-tauri && git mv desktop/src/models.rs core/src/models.rs
```

**Step 2: Add `models` deps to `src-tauri/core/Cargo.toml`**

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

**Step 3: Declare module in `src-tauri/core/src/lib.rs`**

```rust
pub mod models;
```

**Step 4: Add `kniha-jazd-core` dep to `src-tauri/desktop/Cargo.toml`**

```toml
[dependencies]
kniha-jazd-core = { path = "../core" }
# ... existing deps stay
```

**Step 5: Add compat re-export in `src-tauri/desktop/src/lib.rs`**

At the top of the file (replacing the line `pub mod models;`):
```rust
pub use kniha_jazd_core::models;
```

This keeps `crate::models::Trip` working for every existing `use crate::models::*` site in the desktop crate.

**Step 6: Verify both crates build**

```bash
cd src-tauri && cargo build --workspace
```
Expected: success.

**Step 7: Verify all 195 tests still pass**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass (some now run under `-p kniha-jazd-core`, the rest still under `-p kniha-jazd`).

**Step 8: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move models.rs to kniha-jazd-core

First pure module relocated. Desktop crate keeps the path working
via 'pub use kniha_jazd_core::models;' re-export — removed in
final cleanup (Task 26)."
```

---

## Task 5: Move `db.rs`, `db_tests.rs`, and `migrations/` to core

**Why bundled:** `db.rs` embeds [migrations/](../../src-tauri/desktop/migrations/) via `diesel_migrations::embed_migrations!` macro at compile time, which resolves the path relative to the crate's `Cargo.toml`. They must move atomically.

**Files:**
- Move: `src-tauri/desktop/src/db.rs` → `src-tauri/core/src/db.rs`
- Move: `src-tauri/desktop/src/db_tests.rs` → `src-tauri/core/src/db_tests.rs`
- Move: `src-tauri/desktop/src/schema.rs` → `src-tauri/core/src/schema.rs` (db.rs imports from it)
- Move: `src-tauri/desktop/migrations/` → `src-tauri/core/migrations/`
- Modify: `src-tauri/core/Cargo.toml` — add diesel deps
- Modify: `src-tauri/core/src/lib.rs` — declare modules
- Modify: `src-tauri/desktop/src/lib.rs` — add re-exports
- Modify: `src-tauri/desktop/Cargo.toml` — remove diesel deps (now transitive via core)

**Step 1: Move files**

```bash
cd src-tauri
git mv desktop/src/db.rs core/src/db.rs
git mv desktop/src/db_tests.rs core/src/db_tests.rs
git mv desktop/src/schema.rs core/src/schema.rs
git mv desktop/migrations core/migrations
```

**Step 2: Add diesel deps to `src-tauri/core/Cargo.toml`**

```toml
diesel = { version = "2.2", features = ["sqlite"] }
libsqlite3-sys = { version = "0.30", features = ["bundled"] }
diesel_migrations = "2.2"
```

**Step 3: Declare modules in `src-tauri/core/src/lib.rs`**

```rust
pub mod db;
pub mod models;
pub mod schema;
```

**Step 4: Re-export from `src-tauri/desktop/src/lib.rs`**

Replace the existing `pub mod db; pub mod schema;` lines with:
```rust
pub use kniha_jazd_core::{db, schema};
```
(Models re-export from Task 4 already present.)

**Step 5: Strip diesel deps from `src-tauri/desktop/Cargo.toml`**

Remove the three lines (they're now transitive via `kniha-jazd-core`):
```diff
-diesel = { version = "2.2", features = ["sqlite"] }
-libsqlite3-sys = { version = "0.30", features = ["bundled"] }
-diesel_migrations = "2.2"
```

**Step 6: Verify build and tests**

```bash
cd src-tauri && cargo build --workspace && cargo test --workspace
```
Expected: 195 tests pass.

**Step 7: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move db, schema, and migrations to core

Migrations dir moves with db.rs because diesel_migrations::embed_migrations!
resolves the path at compile time relative to the crate root."
```

---

## Task 6: Move `constants.rs` to core

**Files:**
- Move: `src-tauri/desktop/src/constants.rs` → `src-tauri/core/src/constants.rs`
- Modify: `src-tauri/core/src/lib.rs` — add `pub mod constants;`
- Modify: `src-tauri/desktop/src/lib.rs` — add `pub use kniha_jazd_core::constants;`

**Step 1: Move file**

```bash
cd src-tauri && git mv desktop/src/constants.rs core/src/constants.rs
```

**Step 2: Update `src-tauri/core/src/lib.rs`**

```rust
pub mod constants;
```

**Step 3: Update `src-tauri/desktop/src/lib.rs`**

Replace `pub mod constants;` with:
```rust
pub use kniha_jazd_core::constants;
```

**Step 4: Verify**

```bash
cd src-tauri && cargo build --workspace
```

**Step 5: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move constants to core"
```

---

## Task 7: Move `calculations/` directory to core

**Heads up:** This is a directory with submodules. Confirmed contents (per `git ls-files`):
- `calculations/mod.rs`
- `calculations/energy.rs` + `energy_tests.rs`
- `calculations/phev.rs` + `phev_tests.rs`
- `calculations/time_inference.rs`
- `calculations/tests.rs` (top-level calculations tests)

All files are pure math, no Tauri references.

**Files:**
- Move: entire `src-tauri/desktop/src/calculations/` → `src-tauri/core/src/calculations/` (all 7 files)
- Modify: `src-tauri/core/src/lib.rs` — add `pub mod calculations;`
- Modify: `src-tauri/desktop/src/lib.rs` — add `pub use kniha_jazd_core::calculations;`

**Step 1: Move directory**

```bash
cd src-tauri && git mv desktop/src/calculations core/src/calculations
```

**Step 2: Verify all 7 expected files moved**

```bash
ls src-tauri/core/src/calculations/
```
Expected: `energy.rs energy_tests.rs mod.rs phev.rs phev_tests.rs tests.rs time_inference.rs`

**Step 3: Update `src-tauri/core/src/lib.rs`**

```rust
pub mod calculations;
```

**Step 4: Update `src-tauri/desktop/src/lib.rs`**

Replace `pub mod calculations;` with:
```rust
pub use kniha_jazd_core::calculations;
```

**Step 5: Verify build and tests (calculations has 56 tests across submodules)**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass. Verify count specifically: `cargo test -p kniha-jazd-core calculations 2>&1 | grep "test result"` shows ~56 tests.

**Step 6: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move calculations module to core

Whole directory moves intact — submodules energy/phev/time_inference
plus their _tests.rs files (~56 tests total)."
```

---

## Task 8: Move `db_location.rs` to core

**Files:**
- Move: `src-tauri/desktop/src/db_location.rs` → `src-tauri/core/src/db_location.rs`
- Modify: `src-tauri/core/src/lib.rs` — declare module
- Modify: `src-tauri/core/Cargo.toml` — add `hostname`, `chrono` (already added) deps if needed
- Modify: `src-tauri/desktop/src/lib.rs` — re-export

**Step 1: Move file**

```bash
cd src-tauri && git mv desktop/src/db_location.rs core/src/db_location.rs
```

**Step 2: Add `hostname` to `src-tauri/core/Cargo.toml`**

```toml
hostname = "0.4"
```

**Step 3: Update lib.rs files (core: declare; desktop: re-export)**

In `src-tauri/core/src/lib.rs`:
```rust
pub mod db_location;
```
In `src-tauri/desktop/src/lib.rs`, replace `pub mod db_location;` with:
```rust
pub use kniha_jazd_core::db_location;
```

**Step 4: Verify**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass (db_location has 11 tests).

**Step 5: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move db_location to core"
```

---

## Task 9: Move `app_state.rs` and `settings.rs` to core

**Files:**
- Move: `src-tauri/desktop/src/app_state.rs` → `src-tauri/core/src/app_state.rs`
- Move: `src-tauri/desktop/src/settings.rs` → `src-tauri/core/src/settings.rs`
- Modify: `src-tauri/core/src/lib.rs` — declare both
- Modify: `src-tauri/desktop/src/lib.rs` — re-export both

**Step 1: Move files**

```bash
cd src-tauri
git mv desktop/src/app_state.rs core/src/app_state.rs
git mv desktop/src/settings.rs core/src/settings.rs
```

**Step 2: Update lib.rs files**

In core: add `pub mod app_state; pub mod settings;`. In desktop: replace local `pub mod` lines with `pub use kniha_jazd_core::{app_state, settings};`.

**Step 3: Verify**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass.

**Step 4: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move app_state and settings to core"
```

---

## Task 10: Move `suggestions.rs` to core

**Heads up:** This file was missing from the original plan draft (Critical finding C1). It's pure compensation-trip suggestion logic with no Tauri imports.

**Files:**
- Move: `src-tauri/desktop/src/suggestions.rs` → `src-tauri/core/src/suggestions.rs`
- Modify: `src-tauri/core/src/lib.rs`, `src-tauri/desktop/src/lib.rs`

**Step 1: Verify it's Tauri-free first**

```bash
grep -l "tauri" src-tauri/desktop/src/suggestions.rs
```
Expected: no output (file does not contain `tauri`).

**Step 2: Move**

```bash
cd src-tauri && git mv desktop/src/suggestions.rs core/src/suggestions.rs
```

**Step 3: Declare in core/lib.rs, re-export from desktop/lib.rs**

Same pattern as Task 6.

**Step 4: Verify**

```bash
cd src-tauri && cargo test --workspace
```

**Step 5: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move suggestions to core"
```

---

## Task 11: Move `export.rs` (and `export_tests.rs`) to core

**Files:**
- Move: `src-tauri/desktop/src/export.rs` → `src-tauri/core/src/export.rs`
- Move: `src-tauri/desktop/src/export_tests.rs` → `src-tauri/core/src/export_tests.rs`

**Step 1: Move files**

```bash
cd src-tauri
git mv desktop/src/export.rs core/src/export.rs
git mv desktop/src/export_tests.rs core/src/export_tests.rs
```

**Step 2: Update lib.rs files (declare in core, re-export from desktop)**

**Step 3: Verify (export has 6 tests)**

```bash
cd src-tauri && cargo test --workspace
```

**Step 4: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move export module to core"
```

---

## Task 12: Move `receipts.rs`, `receipts_tests.rs`, and `gemini.rs`, `gemini_tests.rs` to core

**Why bundled:** `receipts.rs` calls `gemini.rs` for OCR; moving them together avoids a transient broken state.

**Files:**
- Move: `receipts.rs` + `receipts_tests.rs` + `gemini.rs` + `gemini_tests.rs` → core

**Step 1: Move files**

```bash
cd src-tauri
git mv desktop/src/receipts.rs core/src/receipts.rs
git mv desktop/src/receipts_tests.rs core/src/receipts_tests.rs
git mv desktop/src/gemini.rs core/src/gemini.rs
git mv desktop/src/gemini_tests.rs core/src/gemini_tests.rs
```

**Step 2: Add `reqwest` and `base64` to core/Cargo.toml**

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls", "blocking"], default-features = false }
base64 = "0.22"
```
(And remove the same from desktop/Cargo.toml.)

**Step 3: Update lib.rs files**

**Step 4: Verify**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass (receipts: 17 + gemini: 4 = 21).

**Step 5: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move receipts and gemini to core"
```

---

## Task 13: Move `server/` to core, extracting Tauri-flavored helpers

**The trick:** [src-tauri/desktop/src/server/mod.rs](../../src-tauri/desktop/src/server/mod.rs) lines 174-191 hold two Tauri-coupled helpers (`resolve_static_dir`, `resolve_static_dir_from_handle`). Everything else in `server/` is Tauri-free (verified). Extract those two functions to `desktop/src/static_dir.rs` before moving the rest.

**Files:**
- Create: `src-tauri/desktop/src/static_dir.rs`
- Move: `src-tauri/desktop/src/server/dispatcher.rs` → `src-tauri/core/src/server/dispatcher.rs`
- Move: `src-tauri/desktop/src/server/dispatcher_async.rs` → `src-tauri/core/src/server/dispatcher_async.rs`
- Move: `src-tauri/desktop/src/server/manager.rs` → `src-tauri/core/src/server/manager.rs`
- Move: `src-tauri/desktop/src/server/mod.rs` → `src-tauri/core/src/server/mod.rs` (with edits)
- Modify: `src-tauri/desktop/src/lib.rs` line 181 — point at new helper location

**Step 1: Create `src-tauri/desktop/src/static_dir.rs`**

```rust
//! Tauri-flavored helpers for resolving the static frontend directory.
//!
//! Lives in the desktop crate because it depends on tauri::App / AppHandle.
//! The web binary passes a plain PathBuf and doesn't need these.

use std::path::PathBuf;

/// Resolve the static frontend directory for serving SPA files.
/// Debug builds serve from `../build` (SvelteKit output, relative to desktop crate root),
/// production from Tauri's resource dir.
pub fn resolve_static_dir(app: &tauri::App) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build")
    } else {
        use tauri::Manager;
        app.path().resource_dir().unwrap_or_default().join("_up_")
    }
}

/// Resolve the static frontend directory from an AppHandle (for use after setup).
pub fn resolve_static_dir_from_handle(app: &tauri::AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build")
    } else {
        use tauri::Manager;
        app.path().resource_dir().unwrap_or_default().join("_up_")
    }
}
```

Note the `../../build` path (was `../build`) because `desktop/` is one level deeper than the old `src-tauri/`.

**Step 2: Strip those two functions and any `use tauri::Manager` from the source `server/mod.rs`**

Edit `src-tauri/desktop/src/server/mod.rs`:
- Delete lines 174-191 (the two `resolve_static_dir*` functions).
- Delete any `use tauri::Manager;` import lines (search the file).

Verify Tauri-free:
```bash
grep -n "tauri" src-tauri/desktop/src/server/mod.rs
```
Expected: no output.

**Step 3: Move the entire `server/` directory to core**

```bash
cd src-tauri && git mv desktop/src/server core/src/server
```

**Step 4: Add `axum`, `tokio`, `tower-http`, `local-ip-address`, `url`, `log` deps to `src-tauri/core/Cargo.toml`**

```toml
axum = "0.8"
tokio = { version = "1", features = ["fs", "rt-multi-thread", "macros", "net", "signal", "sync"] }
tower-http = { version = "0.6", features = ["cors", "fs"] }
local-ip-address = "0.6"
url = "2"
log = "0.4"
thiserror = "1"
serde_json = "1.0"
```
(Remove the corresponding lines from `desktop/Cargo.toml`.)

**Step 5: Update `src-tauri/core/src/lib.rs`**

```rust
pub mod server;
```

**Step 6: Update `src-tauri/desktop/src/lib.rs`**

Replace `pub mod server;` with `pub use kniha_jazd_core::server;`. Also add the new helper module:
```rust
pub mod static_dir;
```

**Step 7: Update the call site in `desktop/src/lib.rs` line 181**

```diff
-let auto_static_dir = server::resolve_static_dir(app);
+let auto_static_dir = crate::static_dir::resolve_static_dir(app);
```

**Step 8: Verify core has zero Tauri references**

```bash
grep -rn "tauri" src-tauri/core/src/server/
```
Expected: no output.

```bash
cd src-tauri && cargo tree -p kniha-jazd-core | grep -i "^├── tauri\|^└── tauri\|^   tauri"
```
Expected: no output (tauri appears nowhere in core's dep tree).

**Step 9: Verify build and tests**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass. The 5 inline tests in `core/src/server/mod.rs` now run under `-p kniha-jazd-core`.

**Step 10: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): move server module to core, extract Tauri helpers

resolve_static_dir(&tauri::App) and resolve_static_dir_from_handle
extracted to desktop/src/static_dir.rs (only callers are in
desktop/src/lib.rs::setup). Rest of server/ moves to core unchanged
and is now Tauri-free per cargo tree verification."
```

---

## Task 14: Rename desktop crate to `kniha-jazd-desktop`

**Why now:** Up to this point the desktop member kept the original `kniha-jazd` name to minimize churn. With pure modules done, rename it before splitting commands so the binary name and `[[bin]]` entries are stable for the remaining work.

**Files:**
- Modify: `src-tauri/desktop/Cargo.toml` — `name = "kniha-jazd-desktop"`, drop `default-run`, drop `[[bin]] name = "web"` block
- Modify: `src-tauri/desktop/Cargo.toml` — explicit `[[bin]]` for the desktop binary if needed
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml) — `TAURI_BINARY` filename if it references `kniha-jazd.exe`

**Step 1: Edit `src-tauri/desktop/Cargo.toml`**

Change `name = "kniha-jazd"` → `name = "kniha-jazd-desktop"`. Remove `default-run = "kniha-jazd"`. Remove the `[[bin]] name = "web" path = "src/bin/web.rs"` block (the web binary moves to its own crate in Task 23). Add an explicit `[[bin]]` only if the convention auto-discovery doesn't pick it up:
```toml
[[bin]]
name = "kniha-jazd-desktop"
path = "src/main.rs"
```

**Step 2: Verify the `app_lib::run()` call in `src-tauri/desktop/src/main.rs` still resolves**

[src-tauri/desktop/src/main.rs](../../src-tauri/desktop/src/main.rs) line 5 says `app_lib::run();`. Since [Cargo.toml](../../src-tauri/desktop/Cargo.toml) defines `[lib] name = "app_lib"`, that still works. But the package rename doesn't affect lib name unless `[lib] name` was tied to package — verify it's still `app_lib` after rename. (If you want the lib name to reflect the package, edit `[lib] name` to `kniha_jazd_desktop` and update main.rs to match.)

**Step 3: Update CI binary path in [.github/workflows/test.yml](../../.github/workflows/test.yml)**

```bash
grep -n "TAURI_BINARY\|kniha-jazd.exe" .github/workflows/test.yml
```
Update every occurrence of `target/debug/kniha-jazd.exe` to `target/debug/kniha-jazd-desktop.exe` (and `release/` similarly).

**Step 4: Verify Tauri build still works**

```bash
npm run tauri build -- --debug --config src-tauri/desktop/tauri.conf.dev.json
```
Expected: produces `src-tauri/target/debug/kniha-jazd-desktop.exe`.

**Step 5: Commit**

```bash
git add src-tauri/desktop/Cargo.toml .github/workflows/test.yml
git commit -m "refactor(desktop): rename crate to kniha-jazd-desktop

Drops default-run and the inline [[bin]] for web (web binary moves
to its own crate in Task 23). CI binary path updated to match new
binary filename."
```

---

## Task 15: Scaffold `commands_internal/` in core, move pure helpers

**Files:**
- Create: `src-tauri/core/src/commands_internal/mod.rs`
- Create: `src-tauri/core/src/commands_internal/helpers.rs`
- Modify: `src-tauri/desktop/src/commands/mod.rs` — re-export pure helpers from core
- Modify: `src-tauri/core/src/lib.rs` — declare module

**Step 1: Create `src-tauri/core/src/commands_internal/mod.rs`**

```rust
//! Framework-free command implementations.
//!
//! Each `*_internal` function takes plain types (`&Database`, `&AppState`,
//! plain args). The Tauri-flavored `#[tauri::command]` wrappers in
//! kniha-jazd-desktop's `commands/` module call these. The HTTP RPC
//! dispatcher in `kniha_jazd_core::server::dispatcher` also calls these
//! directly.

pub mod helpers;
pub use helpers::*;

// Per-file modules added incrementally in Tasks 16-22:
// pub mod backup;
// pub mod trips;
// pub mod vehicles;
// ...
```

**Step 2: Create `src-tauri/core/src/commands_internal/helpers.rs`**

Copy from [src-tauri/desktop/src/commands/mod.rs](../../src-tauri/desktop/src/commands/mod.rs) the following items (keep the originals in commands/mod.rs for now as Tauri-side helpers; we'll delete the duplicated pure ones in Step 4):
- `parse_iso_datetime(datetime: &str) -> Result<NaiveDateTime, String>`
- `get_db_paths_for_dir(app_dir: &Path) -> Result<DbPaths, String>` and the `DbPaths` struct
- `calculate_trip_numbers(trips: &[Trip])`
- `calculate_odometer_start(trips: &[Trip], initial_odometer: f64)`
- `generate_month_end_rows(...)` and the `MonthEndRow` struct
- The `check_read_only!` macro definition (and `#[macro_export]` attribute)

Keep these in `desktop/src/commands/mod.rs` (do NOT move them yet):
- `get_app_data_dir(app: &tauri::AppHandle)` — Tauri-flavored
- `get_db_paths(app: &tauri::AppHandle)` — Tauri-flavored, calls `get_app_data_dir`
- The `#[cfg(test)] #[path = "commands_tests.rs"] mod tests;` line at the bottom — moves in Task 22 with the last command split.

**Step 3: Update `src-tauri/core/src/lib.rs`**

```rust
pub mod commands_internal;
```

**Step 4: Replace duplicated helpers in `desktop/src/commands/mod.rs` with re-exports**

Delete the now-duplicated pure functions and instead:
```rust
pub use kniha_jazd_core::commands_internal::{
    parse_iso_datetime,
    get_db_paths_for_dir, DbPaths,
    calculate_trip_numbers,
    calculate_odometer_start,
    generate_month_end_rows, MonthEndRow,
};
// check_read_only! macro is auto-exported via #[macro_export] in core
```

**Step 5: Verify build and tests**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass.

**Step 6: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "refactor(core): scaffold commands_internal module with shared helpers

Pure helpers (parse_iso_datetime, get_db_paths_for_dir, calculate_*,
generate_month_end_rows, check_read_only! macro) move to core. The
two Tauri-flavored helpers (get_app_data_dir, get_db_paths) stay in
desktop until Task 22."
```

---

## Tasks 16-22: Per-command-file split (one file per task)

**Pattern repeated for each of 9 files** ([backup.rs](../../src-tauri/desktop/src/commands/backup.rs), [trips.rs](../../src-tauri/desktop/src/commands/trips.rs), [vehicles.rs](../../src-tauri/desktop/src/commands/vehicles.rs), [export_cmd.rs](../../src-tauri/desktop/src/commands/export_cmd.rs), [receipts_cmd.rs](../../src-tauri/desktop/src/commands/receipts_cmd.rs), [statistics.rs](../../src-tauri/desktop/src/commands/statistics.rs), [settings_cmd.rs](../../src-tauri/desktop/src/commands/settings_cmd.rs), [server_cmd.rs](../../src-tauri/desktop/src/commands/server_cmd.rs), [integrations.rs](../../src-tauri/desktop/src/commands/integrations.rs)).

Each file gets its own task and its own commit. The pattern below uses `<name>` as a placeholder. The Task number runs 16-22 with `backup` at 16, `trips` at 17, etc. Order is **backup first** (it has shared types `BackupInfo`/`CleanupPreview`/`CleanupResult` that exercise the trickiest part of the pattern), then alphabetical for the rest.

### Per-file split pattern

**Files:**
- Create: `src-tauri/core/src/commands_internal/<name>.rs`
- Modify: `src-tauri/desktop/src/commands/<name>.rs` (rewrite as thin wrappers)
- Modify: `src-tauri/core/src/commands_internal/mod.rs` — declare new module
- Modify: `src-tauri/core/src/server/dispatcher.rs` and/or `dispatcher_async.rs` — update call paths for this file's commands

**Step 1: Create `src-tauri/core/src/commands_internal/<name>.rs`**

Copy from `src-tauri/desktop/src/commands/<name>.rs`:
- All `*_internal` functions (full bodies)
- All shared types defined at the top of the source file (e.g., `BackupInfo`, `CleanupPreview`, `CleanupResult` in [backup.rs](../../src-tauri/desktop/src/commands/backup.rs))
- All private helper functions called by `*_internal` (e.g., `parse_backup_filename`, `generate_backup_filename`)

Update imports — replace any `use crate::*` references that now live in core with their `crate::*` equivalent (since this file is now in core, `crate::` resolves to `kniha_jazd_core`):
```rust
use crate::app_state::AppState;
use crate::db::Database;
use crate::models::*;
use crate::commands_internal::check_read_only;  // macro from helpers
// ... etc.
```

Drop any `use tauri::*` lines and any `#[tauri::command]` attributes — internal functions don't have them.

**Step 2: Declare the new module in `src-tauri/core/src/commands_internal/mod.rs`**

```rust
pub mod <name>;
pub use <name>::*;  // re-export so dispatchers can keep flat path during migration
```

**Step 3: Verify core builds**

```bash
cd src-tauri && cargo build -p kniha-jazd-core
```
Expected: success.

**Step 4: Update dispatcher call sites for this file's commands**

Open `src-tauri/core/src/server/dispatcher.rs` and `dispatcher_async.rs`. Replace `crate::commands::<fn>_internal` with `crate::commands_internal::<fn>_internal` for every function from this file (e.g., for `backup.rs`: `create_backup_internal`, `list_backups_internal`, `restore_backup_internal`, `delete_backup_internal`, `get_backup_path_internal`, `get_cleanup_preview_internal`, `cleanup_pre_update_backups_internal`, `get_backup_retention_internal`, `set_backup_retention_internal`, `get_backup_info_internal`, `create_backup_with_type_internal`).

The `pub use <name>::*` re-export from Step 2 means we don't need the per-file module path — `crate::commands_internal::create_backup_internal` resolves directly.

**Step 5: Rewrite `src-tauri/desktop/src/commands/<name>.rs` as thin wrappers**

Each `#[tauri::command]` becomes a delegator. Example for `backup.rs::create_backup`:
```rust
use kniha_jazd_core::app_state::AppState;
use kniha_jazd_core::commands_internal::backup::{self as inner, BackupInfo};
use kniha_jazd_core::db::Database;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn create_backup(
    app: AppHandle,
    db: State<'_, Arc<Database>>,
    app_state: State<'_, Arc<AppState>>,
) -> Result<BackupInfo, String> {
    let app_dir = super::get_app_data_dir(&app)?;
    inner::create_backup_internal(&app_dir, &db, &app_state).await
}
```

For each wrapper in this file, the body becomes one line (or two if it needs to compute `app_dir` from the `AppHandle` first). Drop all helper functions and shared types from this file — they're now in core.

**Step 6: Verify desktop builds**

```bash
cd src-tauri && cargo build -p kniha-jazd-desktop
```
Expected: success.

**Step 7: Run tests for this command's domain**

```bash
cd src-tauri && cargo test --workspace <name>
```
For backup: `cargo test --workspace backup` runs ~10 backup-related tests across `commands_tests.rs` and `db_tests.rs`. Expected: pass.

**Step 8: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/src/commands/<name>.rs src-tauri/core/src/server/
git commit -m "refactor(commands): split <name> into core+desktop

_internal functions and shared types (e.g., BackupInfo for backup.rs)
move to kniha_jazd_core::commands_internal::<name>. Desktop wrappers
become thin delegators. Dispatcher call sites updated."
```

### Task 16: Split `backup.rs` (11 wrappers, 3 shared types)

Apply the per-file pattern. Special notes:
- Shared types `BackupInfo`, `CleanupPreview`, `CleanupResult` defined at lines 24-48 of source — move to core.
- Private helpers `parse_backup_filename`, `generate_backup_filename` — move to core (they're called by `_internal` functions).
- `super::{get_app_data_dir, get_db_paths, get_db_paths_for_dir}` import in source — `get_app_data_dir` and `get_db_paths` stay in desktop's `commands/mod.rs` (Tauri-flavored, called from desktop wrappers); `get_db_paths_for_dir` is already in core via Task 15 (wrappers call core's version directly).

### Task 17: Split `trips.rs` (10 wrappers)

### Task 18: Split `vehicles.rs` (6 wrappers)

### Task 19: Split `export_cmd.rs` (2 wrappers)

### Task 20: Split `receipts_cmd.rs` (17 wrappers)

Special note: this file uses `tauri::Emitter` to emit events. The `_internal` functions return their results; only the wrappers emit Tauri events. Verify no `tauri::Emitter::emit_to` call snuck into an `_internal` function before splitting.

### Task 21: Split `statistics.rs` (4 wrappers)

### Task 22: Split `settings_cmd.rs` (16 wrappers), `server_cmd.rs` (3 wrappers), `integrations.rs` (5 wrappers)

These three are smaller; bundle them in one task. Apply the per-file pattern three times in sequence, with a single combined commit at the end.

After Task 22 completes, [src-tauri/desktop/src/commands/](../../src-tauri/desktop/src/commands/) contains only thin wrappers + `mod.rs` (with the two Tauri-flavored helpers and the test-include line).

---

## Task 22a: Move `commands_tests.rs` and finalize commands/mod.rs

**Files:**
- Move: `src-tauri/desktop/src/commands/commands_tests.rs` → `src-tauri/core/src/commands_internal/commands_tests.rs`
- Modify: `src-tauri/desktop/src/commands/mod.rs` — remove `#[path = "commands_tests.rs"] mod tests;` line
- Modify: `src-tauri/core/src/commands_internal/mod.rs` — add `#[cfg(test)] #[path = "commands_tests.rs"] mod tests;`

**Step 1: Move the file**

```bash
cd src-tauri && git mv desktop/src/commands/commands_tests.rs core/src/commands_internal/commands_tests.rs
```

**Step 2: Inspect the file's imports**

```bash
head -50 src-tauri/core/src/commands_internal/commands_tests.rs
```
The file likely has `use super::*;` plus per-feature imports like `use crate::commands::backup::*;`. Update each `use crate::commands::<file>` to `use crate::commands_internal::<file>` (since `commands_tests.rs` now lives under `commands_internal/`, `super::*` resolves to `commands_internal::*` which has `pub use <file>::*` from Task 15+, so flat paths work).

**Step 3: Update mod attachments**

In `src-tauri/desktop/src/commands/mod.rs`, delete:
```rust
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
```

In `src-tauri/core/src/commands_internal/mod.rs`, add at the bottom:
```rust
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
```

**Step 4: Verify tests still find their dependencies**

```bash
cd src-tauri && cargo test -p kniha-jazd-core --lib
```
Expected: All tests pass (the 61 commands_tests.rs tests now run under core).

**Step 5: Commit**

```bash
git add src-tauri/core/ src-tauri/desktop/
git commit -m "test(core): move commands_tests.rs to core/commands_internal/

61 command tests now run under -p kniha-jazd-core. Imports updated
from 'crate::commands::*' to 'crate::commands_internal::*'."
```

---

## Task 23: Move web binary into `web` crate

**Files:**
- Move: `src-tauri/desktop/src/bin/web.rs` → `src-tauri/web/src/main.rs` (replacing the stub from Task 3)
- Modify: `src-tauri/web/Cargo.toml` — add real deps
- Modify: `src-tauri/desktop/src/bin/` — delete if empty after move

**Step 1: Move the file**

```bash
cd src-tauri
rm web/src/main.rs  # delete the Task 3 stub
git mv desktop/src/bin/web.rs web/src/main.rs
rmdir desktop/src/bin 2>/dev/null  # remove empty dir
```

**Step 2: Update imports in `src-tauri/web/src/main.rs`**

The file currently uses `app_lib::*`. Replace:
```diff
-use app_lib::app_state::AppState;
-use app_lib::db::Database;
-use app_lib::server::HttpServer;
+use kniha_jazd_core::app_state::AppState;
+use kniha_jazd_core::db::Database;
+use kniha_jazd_core::server::HttpServer;
```

**Step 3: Populate `src-tauri/web/Cargo.toml`**

```toml
[package]
name = "kniha-jazd-web"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
description = "Vehicle logbook headless HTTP server"

[[bin]]
name = "kniha-jazd-web"
path = "src/main.rs"

[dependencies]
kniha-jazd-core = { path = "../core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
```

**Step 4: Verify web crate builds**

```bash
cd src-tauri && cargo build -p kniha-jazd-web --release
```
Expected: success.

**Step 5: Verify web binary has zero Tauri symbols (Linux/WSL)**

```bash
ldd src-tauri/target/release/kniha-jazd-web | grep -E 'gdk|webkit|gtk|soup|appindicator|rsvg'
```
Expected: no output.

**Windows alternative** (PowerShell):
```powershell
dumpbin /dependents src-tauri\target\release\kniha-jazd-web.exe | Select-String -Pattern 'webkit|gtk|gdk|soup|appindicator|rsvg'
```
Expected: no matches.

If neither tool is available locally, defer to Task 24's Docker build for definitive verification.

**Step 6: Commit**

```bash
git add src-tauri/web/ src-tauri/desktop/
git commit -m "refactor(web): move web binary into kniha-jazd-web crate

Imports rewired from app_lib::* to kniha_jazd_core::*. The web crate
declares only kniha-jazd-core and tokio — no Tauri in its dep graph."
```

---

## Task 24: Update [Dockerfile.web](../../Dockerfile.web) to drop GTK runtime libs

**Files:**
- Modify: [Dockerfile.web](../../Dockerfile.web)

**Step 1: Update builder stage `apt-get install` (lines 11-18)**

Drop the GTK/WebKit dev packages — the web build no longer needs them:
```diff
 RUN apt-get update \
     && apt-get install -y --no-install-recommends \
-        libwebkit2gtk-4.1-dev \
-        libgtk-3-dev \
-        libsoup-3.0-dev \
-        libayatana-appindicator3-dev \
-        librsvg2-dev \
         libssl-dev \
         pkg-config \
     && rm -rf /var/lib/apt/lists/*
```

**Step 2: Update workspace COPY paths (lines 23-27)**

```diff
 WORKDIR /app/src-tauri

-COPY src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/build.rs ./
-COPY src-tauri/tauri.conf.json src-tauri/tauri.conf.dev.json ./
-COPY src-tauri/capabilities ./capabilities
-COPY src-tauri/icons ./icons
+COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
+COPY src-tauri/core ./core
+COPY src-tauri/web ./web
```

(No `desktop/` copy needed — web build doesn't reference it.)

**Step 3: Update stub-source pre-build block (lines 29-32)**

```diff
-RUN mkdir -p src/bin \
-    && echo 'fn main() {}' > src/bin/web.rs \
-    && echo 'pub fn run() {}' > src/lib.rs \
-    && cargo build --release --bin web || true
+RUN cargo build --release -p kniha-jazd-web || true
```

(The new layout has real source already in `core/` and `web/`; the `|| true` handles the first-pass dep cache build.)

**Step 4: Update real build (lines 38-40)**

```diff
-RUN touch src/lib.rs src/bin/web.rs \
-    && cargo build --release --bin web
+RUN touch core/src/lib.rs web/src/main.rs \
+    && cargo build --release -p kniha-jazd-web
```

**Step 5: Update runtime stage `apt-get install` (lines 65-74)**

Drop the GTK runtime libs — this is the ~150 MB win:
```diff
 RUN apt-get update \
     && apt-get install -y --no-install-recommends \
         ca-certificates \
         curl \
-        libgtk-3-0 \
-        libwebkit2gtk-4.1-0 \
-        libsoup-3.0-0 \
-        libayatana-appindicator3-1 \
-        librsvg2-2 \
     && rm -rf /var/lib/apt/lists/*
```

**Step 6: Update binary copy line (line 76)**

```diff
-COPY --from=rust-builder /app/src-tauri/target/release/web /usr/local/bin/kniha-jazd-web
+COPY --from=rust-builder /app/src-tauri/target/release/kniha-jazd-web /usr/local/bin/kniha-jazd-web
```

**Step 7: Replace the workaround comment (lines 60-64)**

```diff
-# GTK/WebKit runtime libs are needed because the binary is compiled in a
-# crate that has tauri as a non-optional dependency, so its symbols are
-# linked even though the web binary never calls them. Long term, gating
-# tauri behind a "desktop" feature would let the runtime image drop these
-# (~150MB savings). Tracked in _TECH_DEBT.
+# Headless web binary — depends only on kniha-jazd-core, no GUI runtime libs needed.
```

**Step 8: Build and verify image size**

```bash
docker build -f Dockerfile.web -t kj-web:after .
docker images kj-web:after --format "{{.Size}}"
```
Expected: ≤ 120 MB (target ~80 MB; baseline from Pre-flight Step 4 was ~300 MB).

**Step 9: Verify the container runs**

```bash
docker run --rm -d -p 3456:3456 -v kj-data:/data --name kj-test kj-web:after
sleep 2
curl -fs http://localhost:3456/health
docker stop kj-test
```
Expected: `ok` from the health endpoint.

**Step 10: Commit**

```bash
git add Dockerfile.web
git commit -m "build(docker): drop GTK runtime libs from web image

Web binary no longer references GTK/WebKit symbols (workspace split
in Task 23 broke the linker dependency). Image size drops from
~300 MB to ~80 MB. Builder stage simplified — no dev libs needed
for the headless build."
```

---

## Task 25: Update CI workflow

**Files:**
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml)

**Step 1: Inspect current state**

```bash
grep -n "cargo\|tauri.conf\|TAURI_BINARY" .github/workflows/test.yml
```
Note every line that mentions Cargo paths, Tauri config paths, or the binary filename.

**Step 2: Update `cargo test` invocations**

Anywhere the workflow runs `cargo test`, change to `cargo test --workspace`. The workspace root is `src-tauri/` and the working directory of those steps should already be `src-tauri/`.

**Step 3: Add a build-only job for the web crate**

Add a step (or matrix entry) that runs:
```yaml
- name: Verify web binary builds without Tauri
  run: cd src-tauri && cargo build --release -p kniha-jazd-web
```
This catches regressions where someone accidentally couples `core` to Tauri.

**Step 4: Update Tauri config paths**

The integration-build step likely runs `npm run tauri build -- --debug --config src-tauri/tauri.conf.dev.json`. Update to:
```yaml
npm run tauri build -- --debug --config src-tauri/desktop/tauri.conf.dev.json
```
Same for any production-build step using `tauri.conf.json`.

**Step 5: Update `TAURI_BINARY` env var (if present)**

```diff
-TAURI_BINARY: src-tauri/target/debug/kniha-jazd.exe
+TAURI_BINARY: src-tauri/target/debug/kniha-jazd-desktop.exe
```

**Step 6: Verify workflow YAML is still valid**

```bash
npx yaml-lint .github/workflows/test.yml 2>/dev/null || python -c "import yaml; yaml.safe_load(open('.github/workflows/test.yml'))"
```
Expected: no parse errors.

**Step 7: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci: update test workflow for workspace split

- cargo test → cargo test --workspace
- Add web-only build verification step
- tauri.conf paths now under src-tauri/desktop/
- TAURI_BINARY filename now kniha-jazd-desktop.exe"
```

**Step 8: Push and verify CI passes**

```bash
git push -u origin feat/tauri-workspace-split
```
Wait for the PR's CI to go green. If anything fails, fix in a follow-up commit before continuing.

---

## Task 26: Cleanup — remove desktop crate's compat re-exports

**Why now:** With every consumer of `kniha_jazd_core::*` paths now using them directly (the desktop wrappers in Tasks 16-22a, the web binary in Task 23), the temporary `pub use kniha_jazd_core::<module>;` lines in [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs) are no longer needed. Removing them confirms there are no stragglers still using the old paths.

**Files:**
- Modify: `src-tauri/desktop/src/lib.rs` — drop `pub use kniha_jazd_core::*;` lines

**Step 1: Identify the re-exports added in Tasks 4-13**

```bash
grep -n "pub use kniha_jazd_core" src-tauri/desktop/src/lib.rs
```
Expected: ~10 lines (one per module that moved to core).

**Step 2: Delete each re-export line**

Edit `src-tauri/desktop/src/lib.rs` to remove every `pub use kniha_jazd_core::<module>;` line.

**Step 3: Try to build — straggler `use crate::<module>` references will surface as errors**

```bash
cd src-tauri && cargo build -p kniha-jazd-desktop 2>&1 | head -50
```
Expected: either clean build OR a list of `unresolved import` errors pointing to leftover `crate::<module>::*` references in desktop code.

**Step 4: Fix each straggler by switching the import to `kniha_jazd_core::*`**

For every `error[E0432]: unresolved import 'crate::db'` (or similar), find the file and change `use crate::db::Database` → `use kniha_jazd_core::db::Database`. Repeat until clean.

**Step 5: Verify everything still passes**

```bash
cd src-tauri && cargo test --workspace
```
Expected: 195 tests pass.

**Step 6: Commit**

```bash
git add src-tauri/desktop/
git commit -m "refactor(desktop): drop core re-export shim

Compat re-exports added during Phase B (Tasks 4-13) removed. Every
desktop-side import now uses 'kniha_jazd_core::*' explicitly."
```

---

## Task 27: Update CHANGELOG, mark tech debt fixed, record decision

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) — add entry to `[Unreleased]`
- Modify: [_TECH_DEBT/06-tauri-feature-gating.md](../_TECH_DEBT/06-tauri-feature-gating.md) — Status → "Fixed", Decision Log entry
- Modify: [_tasks/index.md](../index.md) — move Task 58 to Completed, mark Tech Debt #06 as Resolved
- Possibly create: [DECISIONS.md](../../DECISIONS.md) entry via `/decision`

**Step 1: Run `/changelog`**

The slash command appends to [CHANGELOG.md](../../CHANGELOG.md)'s `[Unreleased]` section. Suggested entry:
```markdown
### Changed
- Docker image (`kniha-jazd-web`) reduced from ~300 MB to ~80 MB by splitting `src-tauri/` into a Cargo workspace; the web binary no longer links Tauri/GTK runtime libraries.
```

**Step 2: Run `/decision`**

The slash command adds an ADR entry to [DECISIONS.md](../../DECISIONS.md). Topic: "Workspace split over feature flags for Tauri/web boundary". Reasoning: structural enforcement vs maintainer-discipline, same calendar cost.

**Step 3: Update [_TECH_DEBT/06-tauri-feature-gating.md](../_TECH_DEBT/06-tauri-feature-gating.md)**

```diff
-**Status:** Open
+**Status:** Fixed
```
Add Decision Log row:
```markdown
| 2026-MM-DD | Fixed via workspace split (Task 58) | Restructured src-tauri/ into core/desktop/web crates; web binary no longer links Tauri. Docker image dropped from ~300 MB to ~80 MB. PR #NNN merged. |
```
(Replace `2026-MM-DD` with today's date and `#NNN` with the actual PR number.)

**Step 4: Update [_tasks/index.md](../index.md)**

Move Task 58's row from "Active Tasks" to "Completed Tasks". Update Tech Debt #06's Status column from "→ Task 58" to "✅ Resolved (Task 58, PR #NNN)".

**Step 5: Final verification — full sweep**

```bash
cd src-tauri && cargo test --workspace
cd .. && npm run test:integration:tier1
docker build -f Dockerfile.web -t kj-web:final . && docker images kj-web:final --format "{{.Size}}"
```
Expected: 195 tests pass, integration smoke passes, image ≤ 120 MB.

**Step 6: Commit and PR**

```bash
git add CHANGELOG.md DECISIONS.md _tasks/index.md _tasks/_TECH_DEBT/06-tauri-feature-gating.md
git commit -m "docs: mark Task 58 complete, Tech Debt #06 resolved

Workspace split shipped. CHANGELOG, DECISIONS, and tech debt index
updated."
gh pr ready  # if PR is in draft
```

---

## Verification checklist (run after Task 27)

```bash
# 1. Workspace builds clean
cd src-tauri && cargo build --workspace

# 2. All 195 backend tests pass
cargo test --workspace

# 3. Web binary has zero Tauri/GTK symbols (Linux/WSL)
ldd target/release/kniha-jazd-web | grep -E 'gdk|webkit|gtk|soup|appindicator|rsvg'
# expected: empty
# Windows alternative:
# dumpbin /dependents target\release\kniha-jazd-web.exe | Select-String -Pattern 'webkit|gtk|gdk|soup|appindicator|rsvg'

# 4. Core has no Tauri in its dep tree
cargo tree -p kniha-jazd-core | grep -i tauri
# expected: empty

# 5. Desktop still builds and runs
cd .. && npm run tauri build
npm run tauri dev   # smoke check: GUI launches

# 6. Integration tests pass
npm run test:integration:tier1

# 7. Docker image is small
docker build -f Dockerfile.web -t kj-web:final .
docker images kj-web:final --format "{{.Size}}"
# expected: ≤ 120 MB (target: ~80 MB; baseline: ~300 MB)
docker run --rm -p 3456:3456 -v kj-data:/data kj-web:final
curl -fs http://localhost:3456/health
# expected: ok
```

---

## Rollback strategy

Each task is committed independently and reverts cleanly via `git revert`. The riskiest cluster is Tasks 16-22 (per-command-file splits), which is why each file is its own task and its own commit — a single bad split can be reverted without losing the others.

If the workspace skeleton itself proves wrong (Tasks 1-3), reverting all three commits restores the original single-crate layout. Cargo.lock changes will need a `cargo update` after revert to reconcile, but no code is lost.

If the integration test suite breaks anywhere in Tasks 16-22, the most likely cause is a `*_internal` signature drift between the source and the new wrapper — diff `desktop/src/commands/<name>.rs` against the pre-split version (`git show HEAD~1:src-tauri/desktop/src/commands/<name>.rs`) to find the change.

## Out of scope

- Renaming the binary back to `web` (kept as `kniha-jazd-web` to match the crate).
- Splitting the SvelteKit frontend ([src/](../../src/)) into separate packages — Rust only.
- Distroless or scratch-based Docker images — possible after this lands but not required by the success criteria in [01-task.md](./01-task.md).
- Migrating to `[workspace.dependencies]` for shared deps across all three members — currently each member declares its own; consolidation can come later if it proves valuable.

## Execution Handoff

**Plan complete. Two execution options:**

**1. Subagent-Driven (this session)** — Dispatch fresh subagent per task, review between tasks, fast iteration. Required sub-skill: `superpowers:subagent-driven-development`.

**2. Parallel Session (separate)** — Open new session in a worktree with `superpowers:executing-plans`, batch execution with checkpoints.

Which approach?
