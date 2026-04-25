# Tech Debt: Tauri Should Be Behind a `desktop` Cargo Feature

**Date:** 2026-04-25
**Priority:** Medium
**Effort:** High (1-3d)
**Component:** `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/`, `src-tauri/src/server/mod.rs`
**Status:** Open

## Problem

`tauri` and the `tauri-plugin-*` crates are non-optional dependencies of the `kniha-jazd` crate. The `web` binary (Task 33) doesn't call any Tauri APIs, but Cargo compiles the entire transitive dependency graph regardless, so the binary still has dynamic-link references to `libgdk-3.so.0`, `libwebkit2gtk-4.1.so.0`, etc.

That forces the Docker runtime image to install GTK/WebKit/Soup/AppIndicator/RSVG runtime libraries even though no GUI is rendered. The runtime stage grows by ~150 MB compared to a true headless build.

## Impact

- Docker image is ~300 MB instead of an achievable ~80 MB.
- Slower CI pulls, slower deploys, more attack surface in the runtime image.
- Conceptually misleading: a "web binary" image carrying GUI libs.
- Blocks running the `web` binary in distroless or scratch-based images.

## Root Cause

Task 55 (Server Mode) extracted `_internal` command functions and made the HTTP layer framework-independent, but stopped short of feature-gating the Tauri-flavored wrappers. Task 33's plan explicitly noted (Step 3a in `_tasks/_done/33-web-deployment/02-plan.md`) that gating *might* be needed but assumed it wasn't because Tauri was "already a dependency." The plan author didn't account for native shared-library linkage at the binary level.

## Recommended Solution

1. Make Tauri deps optional in `src-tauri/Cargo.toml`:

   ```toml
   [features]
   default = ["desktop"]
   desktop = [
       "dep:tauri",
       "dep:tauri-build",
       "dep:tauri-plugin-updater",
       "dep:tauri-plugin-process",
       "dep:tauri-plugin-opener",
       "dep:tauri-plugin-dialog",
       "dep:tauri-plugin-log",
   ]

   [dependencies]
   tauri = { version = "2.9.5", optional = true }
   tauri-plugin-updater = { version = "2", optional = true }
   # ...
   ```

2. Gate `pub fn run()` in `src-tauri/src/lib.rs` with `#[cfg(feature = "desktop")]`.

3. Gate every `#[tauri::command]` wrapper across `src-tauri/src/commands/` (74 occurrences). The `_internal` helpers stay un-gated.

4. Gate `resolve_static_dir(&tauri::App)` and `resolve_static_dir_from_handle(&tauri::AppHandle)` in `src-tauri/src/server/mod.rs`. Provide a non-Tauri equivalent that takes a plain `PathBuf` (already used by the web binary).

5. Gate the `tauri::AppHandle`-flavored helper functions in `src-tauri/src/commands/mod.rs` (`get_app_data_dir(app: &AppHandle)`, `get_db_paths(app: &AppHandle)`).

6. Add `[[bin]] required-features = ["desktop"]` to bins that need it (none currently — `web` doesn't, the desktop entry-point is via the lib).

7. Build the web binary with `cargo build --no-default-features --bin web`.

8. Strip GTK/WebKit/Soup/AppIndicator/RSVG packages from `Dockerfile.web` stage 3.

## Alternative Options (if any)

- **Workspace split.** Move `web` to a separate crate (`kniha-jazd-web`) that depends on a renamed `kniha-jazd-core` library without Tauri. Cleaner module boundary but a larger restructuring.

- **Status quo.** Keep the GTK runtime libs. Acceptable if image size is not a concern, but loses the conceptual benefit of a headless binary.

## Related

- `_tasks/_done/33-web-deployment/` — Task that surfaced this issue
- `Dockerfile.web` — Currently installs runtime GTK libs as workaround
- `_tasks/_done/55-server-mode/` — Prior task that prepared the framework-independent `_internal` layer

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-04-25 | Created during Task 33 | Discovered when slim Debian runtime image couldn't load `libgdk-3.so.0` |
| 2026-04-25 | Workaround: install runtime GTK libs | Unblocks Task 33 deployment in ~5 min vs days of refactoring |
