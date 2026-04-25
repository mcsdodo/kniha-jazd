# Code Review

**Target:** Server-mode implementation, commits 43ab4bab..HEAD (20 commits, 47 files, ~5800 lines)
**Reference:** `_tasks/55-server-mode/01-task.md`, `02-design.md`, `03-plan.md`
**Started:** 2026-04-25
**Status:** Ready for User Review
**Focus:** Quality, correctness, best practices, plan adherence

**Baseline Test Status:** 280 tests pass (85 new tests added)

---

## Iteration 1

### Findings

#### Critical

1. **Separate Database connections instead of shared `Arc<Database>`** — `server_cmd.rs:34-40`, `lib.rs:190`

   The design doc (`02-design.md` §1) explicitly states: "Shared Database Instance (not a second connection) — The HTTP server uses the **same** `Arc<Database>` that Tauri commands use." However, both `start_server` and the auto-start code create **new** `Database::new()` connections from the DB file path.

   This introduces:
   - Two independent `Mutex<SqliteConnection>` instances — operations are serialized within each connection but **not across** them. Two threads (one Tauri IPC, one HTTP) can attempt concurrent writes, causing `SQLITE_BUSY` errors.
   - Potential data visibility lag between connections (though WAL mode mitigates this).
   - A separate `AppState::new()` instance (line `server_cmd.rs:43`) that won't reflect read-only state changes made through the desktop UI after server start.

   **Root cause:** `lib.rs:133` does `app.manage(db)` which moves a plain `Database`, not `Arc<Database>`. There's no Arc to clone out of Tauri state.

   **Suggested fix:** Change `app.manage(db)` to `app.manage(Arc::new(db))` and update all `State<Database>` to `State<Arc<Database>>` across Tauri commands. Then pass the same `Arc<Database>` to the server. Same approach for `AppState`.

   **Impact:** Under concurrent desktop + browser usage, write operations could fail with SQLITE_BUSY. Also, toggling read-only in desktop won't propagate to server's separate AppState.

#### Important

2. **3 commands missing from RPC dispatcher** — `dispatcher.rs`, `dispatcher_async.rs`

   The following commands have `_internal` functions but are not wired into the RPC dispatcher:
   - `get_receipt_settings` (`receipts_cmd.rs:42`) — needed by Settings page receipt scanning section
   - `set_gemini_api_key` (`receipts_cmd.rs:59`) — needed to save Gemini API key
   - `set_receipts_folder_path` (`receipts_cmd.rs:87`) — needed to save receipts folder

   Browser users who navigate to Settings will see the receipt scanning section but get errors when it tries to load/save these settings.

   **Fix:** Add 3 match arms to `dispatch_sync` for these commands (Pattern B: `&state.app_dir`).

3. **`export_to_browser` not feature-gated in frontend** — `src/routes/+page.svelte:176`

   The Export button calls `openExportPreview()` → `apiCall('export_to_browser')` with no capabilities check. `export_to_browser` is correctly classified as Tauri-only (uses `open::that()`), so browser users get "Unknown command" error.

   The dispatcher _does_ have `export_html` (returns HTML string), but the frontend never calls it.

   **Suggested fix:** Either:
   - (a) Hide the export button in server mode: `{#if $capabilities.features.openExternal}`, or
   - (b) Use `export_html` in server mode to return HTML and open in a new tab via `window.open()`.

   Option (b) is better UX since export is a core feature.

4. **`revealBackup` uses Tauri-only import unconditionally** — `src/lib/api.ts:237-241`

   `revealBackup()` calls `import('@tauri-apps/plugin-opener')` which will fail at runtime in browser mode. The `if (IS_TAURI)` guard prevents execution, but the dynamic import is still attempted in the condition's truthy branch. In browsers, this should be safe since the code path isn't reached, but it's fragile.

   **Note:** This is a minor fragility, not a runtime bug — the `IS_TAURI` check works correctly.

5. **Auto-start creates `AppState::new()` without DB path** — `lib.rs:197`

   The auto-start code creates `let app_state = Arc::new(AppState::new())` which is a brand new AppState. It copies `is_read_only` from the main AppState, but `get_db_path()`, `get_db_location_internal()` etc. called through the server would use this empty AppState. The `app_state.get_db_path()` call in the `get_db_location` dispatcher arm would return `None`.

   **Suggested fix:** If continuing with separate instances (not recommended, see Critical #1), at minimum copy the DB path: `server_app_state.set_db_path(...)`.

#### Minor

6. **`_up_` magic string for production static dir** — `server_cmd.rs:31`, `lib.rs:187`

   The production static directory is resolved as `resource_dir.join("_up_")`. This appears to be a Tauri convention for the parent of the webview dist, but the name is opaque. A constant or comment explaining why `_up_` would improve readability. Also, this path may not exist or contain the built frontend files depending on the Tauri build configuration.

7. **Duplicated static dir resolution** — `server_cmd.rs:24-32` vs `lib.rs:183-188`

   The static directory resolution logic (debug vs release path) is duplicated in two places. If the path logic changes, both must be updated.

   **Suggested fix:** Extract to a function in `server/mod.rs` or `commands/mod.rs`.

8. **`serde_json::to_value(...).unwrap()` in dispatcher** — `dispatcher.rs` (all match arms)

   Every dispatch arm does `serde_json::to_value(v).unwrap()`. Since all types implement `Serialize`, this should never fail, but panic-on-failure in a server context is harsh. Consider `map_err` for defense in depth.

   **Impact:** Extremely low risk — all types are already serializable (they round-trip through Tauri IPC). Cosmetic improvement.

9. **No `X-KJ-Client` header enforcement on POST** — `server/mod.rs:136-147`

   The design doc (`02-design.md` §10) specifies requiring `X-KJ-Client: 1` on all POST requests as CSRF mitigation. The CORS layer _allows_ the header but doesn't _require_ it. The RPC endpoint processes POSTs regardless of whether the header is present.

   **Impact:** Reduced CSRF protection. The CORS origin check still provides the primary defense, but the header requirement was designed as a second layer.

   **Suggested fix:** Add Axum middleware that rejects POST requests missing the `X-KJ-Client` header with 403.

10. **Test count in CLAUDE.md outdated** — `CLAUDE.md`

    CLAUDE.md references "195 tests" in multiple places, but the implementation added 85 new tests (now 280 total). The rust-backend.md rule file also says "195 tests".

### Test Gaps

- [ ] No test for unknown/Tauri-only command returning proper error via HTTP (e.g., calling `export_to_browser` or `move_database` through RPC)
- [ ] No test for concurrent writes through separate connections (the Critical #1 scenario)
- [ ] No test verifying `X-KJ-Client` header is required (design says it should be)
- [ ] Missing commands (`get_receipt_settings`, `set_gemini_api_key`, `set_receipts_folder_path`) have no dispatcher test coverage because they're absent from the dispatcher

### Coverage Assessment

**Reviewed:**
- All 4 server module files (`mod.rs`, `dispatcher.rs`, `dispatcher_async.rs`, `manager.rs`)
- Server commands (`server_cmd.rs`)
- Frontend API adapter (`api-adapter.ts`, `api.ts`)
- Capabilities store (`capabilities.ts`)
- Update store integration with capabilities
- Settings page server mode UI
- Layout changes for capabilities loading
- Vehicle/trip `_internal` extraction pattern
- `lib.rs` server wiring and auto-start
- `settings.rs` new fields
- i18n keys (Slovak and English)
- `mod.rs` command re-exports
- CORS and security posture
- Command classification coverage (frontend ↔ dispatcher alignment)

**Not deeply reviewed (spot-checked only):**
- All 8 command module `_internal` extractions (vehicles verified as representative)
- Integration test infrastructure (`wdio.server.conf.ts`, `tests/integration/`)
- CI workflow changes
- Feature documentation (`docs/features/server-mode.md`)

---

## Review Summary

**Status:** Complete
**Iterations:** 1
**Total Findings:** 1 Critical, 4 Important, 5 Minor
**Test Status:** 280 tests pass

### All Findings (Consolidated)

#### Critical
1. [x] Separate Database connections instead of shared `Arc<Database>` — **FIXED**

#### Important
2. [x] 3 receipt commands missing from RPC dispatcher — **FIXED**
3. [x] `export_to_browser` not feature-gated in frontend — **FIXED**
4. [ ] `revealBackup` uses Tauri-only import without full safety — `api.ts:237` (skipped — not a runtime bug)
5. [x] Auto-start creates bare `AppState` without DB path — **FIXED** (resolved by #1)

#### Minor
6. [ ] `_up_` magic string for production static dir — `server_cmd.rs:31` (skipped)
7. [x] Duplicated static dir resolution logic — **FIXED** (extracted to `resolve_static_dir`)
8. [ ] `serde_json::to_value().unwrap()` in dispatcher — `dispatcher.rs` (skipped)
9. [ ] No `X-KJ-Client` header enforcement on POST — `server/mod.rs` (skipped)
10. [ ] Test count in CLAUDE.md outdated — `CLAUDE.md` (skipped)

## Resolution

**Addressed:** 5 findings (1 Critical, 3 Important, 1 Minor)
**Skipped:** 5 findings (1 Important, 4 Minor — user decision)
**Test Status:** All 280 backend tests pass, 0 svelte-check errors

### Applied Fixes

**Finding 1 (Critical — shared DB):** Changed `app.manage(db)` to `app.manage(Arc::new(db))` and `app.manage(app_state)` to `app.manage(Arc::new(app_state))` in `lib.rs`. Updated all Tauri commands from `State<Database>` to `State<Arc<Database>>` (and same for AppState) across 9 command files. Deref coercion chains (`State<Arc<Database>>` → `Arc<Database>` → `Database`) mean zero body changes. `server_cmd.rs` now clones the shared Arcs via `db.inner().clone()` instead of creating new `Database::new()` instances. Auto-start also uses cloned Arcs.

**Finding 2 (Important — missing commands):** Added `get_receipt_settings`, `set_gemini_api_key`, `set_receipts_folder_path` to `dispatcher.rs` sync dispatcher.

**Finding 3 (Important — export feature-gate):** Added `capabilities` check in `+page.svelte`. Desktop uses `openExportPreview` (desktop browser via `open::that()`). Server mode uses `exportHtml` → opens HTML in new tab via `window.open()`. Added `exportHtml()` wrapper to `api.ts`.

**Finding 5 (Important — auto-start AppState):** Resolved by finding #1 — auto-start now clones the same Arc, so DB path and read-only state are shared automatically.

**Finding 7 (Minor — duplicated static dir):** Extracted `resolve_static_dir()` and `resolve_static_dir_from_handle()` to `server/mod.rs`. Both `lib.rs` auto-start and `server_cmd.rs` now call these helpers.
