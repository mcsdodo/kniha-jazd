**Date:** 2026-04-23
**Subject:** Plan review for 55-server-mode/03-plan.md
**Reviewer:** Claude (Opus 4.6)
**Status:** Review complete

## Recommendation

**Needs Revisions.** The plan is solid overall -- well-structured, correctly ordered (LAN exposure last), and accurately maps the codebase. However, there are 2 Critical issues (will cause compile errors or incorrect behavior), 4 Important issues (will waste time or produce bugs), and 4 Minor issues (polish/clarity).

## Critical

### C1. `Database::in_memory()` returns `Result`, but tests treat it as infallible

**Tasks 2, 6, 8, 9** -- All test scaffolds use `Arc::new(crate::db::Database::in_memory())`, but `in_memory()` returns `Result<Self, diesel::ConnectionError>` (see `db.rs:68`). This will not compile.

**Fix:** Use `Database::in_memory().expect("test db")` or `.unwrap()` in all test helper functions.

```rust
// Wrong (won't compile)
let db = Arc::new(crate::db::Database::in_memory());

// Correct
let db = Arc::new(crate::db::Database::in_memory().unwrap());
```

- [ ] Fix all test scaffolds in Tasks 2, 6, 8, 9

### C2. Receipt image handler treats `file_path` as `Option<String>`, but it's `String`

**Task 8** -- The receipt image handler code checks `receipt.file_path` with a `match` on `Some(p)` / `None`, but `Receipt.file_path` is `String`, not `Option<String>` (see `models.rs:497`). Also, the handler calls `db.get_receipt(&id)` which doesn't exist -- the actual method is `db.get_receipt_by_id(&id)`.

**Fix:** Update handler to use:
- `db.get_receipt_by_id(&id)` instead of `db.get_receipt(&id)`
- `receipt.file_path` directly (it's always a `String`)

- [ ] Fix receipt image handler in Task 8

## Important

### I1. `cleanup_pre_update_backups_internal` takes `&AppHandle`, not `&Path` -- Pattern B is wrong for backup commands

**Task 5** -- The plan claims Pattern B extracts `app_dir: &Path` from `AppHandle`, but `cleanup_pre_update_backups_internal` already exists and takes `&tauri::AppHandle` (see `backup.rs:252`). It internally calls `get_db_paths(app)` and `list_backups(app.clone())`, both of which need `AppHandle` because they resolve the custom DB path from `local.settings.json` via `get_app_data_dir`.

The backup module can't simply swap `AppHandle` for `app_dir: &Path` because `get_db_paths` also resolves `custom_db_path` from settings loaded relative to `app_dir`. The `_internal` signature needs `app_dir` AND the resolved `DbPaths`, OR a single `app_dir` parameter with the understanding that `get_db_paths` is refactored to accept `&Path` instead of `&AppHandle`.

**Fix:** Either:
(a) Refactor `get_db_paths` and `get_app_data_dir` to accept `&Path` (one-time effort, enables clean Pattern B for all modules), or
(b) Accept that backup `_internal` fns take `(db_paths: &DbPaths, app_dir: &Path)` -- two parameters instead of AppHandle.

The existing `cleanup_pre_update_backups_internal` will need its signature changed from `&AppHandle` to whatever new pattern is chosen. This is a behavioral change that should be called out since it's used in `lib.rs:143`.

- [ ] Decide on backup extraction approach, update Task 5

### I2. Settings theme commands use `app_handle.path().app_data_dir()` directly, not `get_app_data_dir`

**Task 4** -- The plan shows Pattern B using `get_app_data_dir(&app)` in the example, which correctly respects the `KNIHA_JAZD_DATA_DIR` env var. But the actual settings commands (`get_theme_preference`, `set_theme_preference`, etc.) call `app_handle.path().app_data_dir()` directly (see `settings_cmd.rs:94-95, 117-119`), which does NOT respect the env var override.

When extracting to `_internal(app_dir: &Path)`, the Tauri wrapper should call `get_app_data_dir(&app)` (not `app.path().app_data_dir()`) to maintain env var compatibility for integration tests.

**Fix:** When writing the Tauri wrapper, use `get_app_data_dir(&app)?` consistently:
```rust
pub fn get_theme_preference(app: AppHandle) -> Result<String, String> {
    let app_dir = get_app_data_dir(&app)?;
    get_theme_preference_internal(&app_dir)
}
```
This also fixes an existing bug where settings commands don't respect the test env var.

- [ ] Note in Task 4 to use `get_app_data_dir` consistently

### I3. Task 13 server control commands are underspecified -- no `_internal` pattern, unclear state management

**Task 13** -- Creates `server_cmd.rs` with `start_server`, `stop_server`, `get_server_status`, but doesn't explain how these interact with the `HttpServer::start` from Task 2. Questions:

- Where is the server handle stored? A new managed state? `AppState` extension?
- `start_server` needs the `Arc<Database>`, `Arc<AppState>`, `app_dir`, and `static_dir` -- how does it get them from within a Tauri command?
- `stop_server` needs to send on the shutdown channel -- where is the `oneshot::Sender` stored?
- Do these commands need `_internal` variants for the RPC dispatcher? (Probably not -- starting/stopping the server from a browser client that's connected TO the server is paradoxical.)

**Fix:** Add a design note in Task 13 explaining:
- Server handle + shutdown sender stored in a new managed `ServerManager` state
- `start_server` extracts `Database` and `AppState` from Tauri managed state
- These commands are Tauri-only (not exposed via RPC)

- [ ] Flesh out Task 13 state management design

### I4. Task 15 dual-mode integration tests have feasibility concerns

**Task 15** -- The `wdio.server.conf.ts` spawns the Tauri binary in the background, but:

1. The Tauri binary opens a window (webview). On CI, this may fail without a display server. The test needs `--headless` or needs to suppress the window.
2. `beforeTest` deletes the DB file and refreshes -- but the Tauri process has an open connection. This will fail on Windows (file locked by the process).
3. ChromeDriver is needed alongside the Tauri binary. The config specifies `port: 9515` for chromedriver but doesn't set up chromedriver service in the WebdriverIO config.
4. The `waitForUrl` helper is referenced but not defined.

**Fix:** Add implementation notes addressing:
- Window suppression for headless CI (env var or Tauri config)
- DB reset strategy (RPC command to reset, or in-memory DB for tests)
- ChromeDriver service configuration in wdio config
- `waitForUrl` implementation (poll with retry)

- [ ] Address feasibility concerns in Task 15

## Minor

### M1. Command count mismatch

Plan says "67 server-safe" and "4 Tauri-only" = 71 total. But the `invoke_handler` in `lib.rs` lists 72 commands (counted 72 entries in lines 159-230). The `export_to_browser` is correctly flagged as Tauri-only, but the count is off by 1. Not blocking but worth reconciling.

- [ ] Recount commands and update classification

### M2. `get_trip_grid_data` already has `build_trip_grid_data` as its internal function

**Task 4** -- The plan says `get_trip_grid_data` needs "Special" treatment with `app_dir: Option<&Path>`. But `build_trip_grid_data` (statistics.rs:363) is already the `_internal` equivalent -- it takes `&Database` directly. The Tauri wrapper (`get_trip_grid_data`) just calls `build_trip_grid_data` then does HA push.

For the RPC dispatcher, just call `build_trip_grid_data` directly (skipping HA push, which is a Tauri-specific side effect). No need for a new `_internal` function -- one already exists.

- [ ] Note in Task 4 that `build_trip_grid_data` is the existing internal fn

### M3. Plan says "56 call sites" in api.ts but `invoke(` appears 56 times including import

The plan's Task 10 says "56 calls". Grep confirms 56 occurrences of `invoke(` in `api.ts`, but one of those is the import statement. The actual call sites are 55, plus 1 in TripRow.svelte = 56 total. Minor but prevents confusion during implementation.

- [ ] Clarify count: 55 in api.ts + 1 in TripRow.svelte

### M4. Task 6 test references `"len na citanie"` (Slovak), but actual error message is different

**Task 6** -- The `write_command_fails_in_read_only_mode` test asserts `result.unwrap_err().contains("len na citanie")`. But the actual `check_read_only!` macro error message is `"Aplikacia je v rezime len na citanie."` (see `commands/mod.rs:69`). The test string should match a substring of the actual message, but "len na citanie" contains diacritics that differ from the assertion -- `"len na čítanie"` (with hacek on c and i). The test uses ASCII `"len na čítanie"` which actually matches because the macro text uses `"čítanie"`. Need to verify the exact assertion string matches.

Actually, looking more carefully at the test: `assert!(result.unwrap_err().contains("len na čítanie"))` -- this looks correct as a substring of `"Aplikácia je v režime len na čítanie."`. But the plan's markdown rendering may strip diacritics. The implementer should copy from the macro source.

- [ ] Verify test assertion string matches actual error message

---

## What's Good

- **Correct task ordering** -- LAN exposure (Task 14) comes last, after all security and functionality is proven. Tasks 3-5 (_internal extraction) before Task 6 (dispatcher) is the right dependency order.
- **Accurate codebase references** -- Line numbers, function signatures, and file paths match the actual code (verified against commit 43ab4ba).
- **Pattern A vs B classification is correct** -- Commands that use `AppHandle` for `app_data_dir` are properly identified. Async commands are correctly flagged.
- **Existing `_internal` functions identified** -- `assign_receipt_to_trip_internal`, `get_trips_for_receipt_assignment_internal`, `verify_receipts_internal`, and `cleanup_pre_update_backups_internal` are all acknowledged.
- **`build_trip_grid_data` reuse** -- Correctly identified as the existing internal function for grid data.
- **Frontend migration strategy** -- Correctly identifies that ALL `invoke()` calls are centralized in `api.ts` (55 calls) + 1 in `TripRow.svelte`. The migration is mechanical.
- **Receipt handling** -- Correctly identifies that receipts use `openPath` (Tauri-only) and need an alternative for browser mode. The HTTP endpoint approach is sound.
- **CORS implementation** -- `is_lan_origin` with RFC1918 range checking is well-designed. The 172.16-31 parsing is correctly handled.
- **Test strategy** -- Dispatcher unit tests + HTTP integration tests is the right approach. Keeping WebdriverIO Tauri-only and adding server tests separately avoids test infrastructure bloat.

---

## Summary

| Severity | Count | Action |
|----------|-------|--------|
| Critical | 2 | Must fix before implementation |
| Important | 4 | Should fix before implementation |
| Minor | 4 | Fix during implementation |

**Recommendation:** Address C1, C2, I1, I2, I3, I4 before starting implementation. C1 and C2 are compile errors that will block progress. I1-I4 are design gaps that will cause wasted time during Tasks 4, 5, 13, and 15.
