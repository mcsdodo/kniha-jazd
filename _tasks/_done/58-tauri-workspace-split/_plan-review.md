**Date:** 2026-04-26
**Subject:** Plan review — `02-plan.md` Cargo workspace split for src-tauri
**Reviewer:** plan-review-skill (Haiku, isolated context)
**Plan under review:** [`02-plan.md`](./02-plan.md)

## Summary

| Category | Count |
|----------|-------|
| Critical | 4 |
| Important | 6 |
| Minor | 3 |

**Recommendation:** **Needs revisions.** The plan's overall direction is sound (workspace split is the right call, the `wrapper → _internal` boundary is real), and most of the file-by-file map is accurate. But several files actually present in the source are missing from the move tables, two structural issues will block the build mid-step, and a few verification commands are wrong. None of the gaps are fatal — they are all addressable with edits before implementation begins.

---

## Critical Findings

### [ ] C1. `suggestions.rs` is not mentioned anywhere in the plan

`src-tauri/src/suggestions.rs` exists today (compensation trip suggestion logic, listed in `.claude/rules/rust-backend.md` as a key file), and `lib.rs` line 14 declares `pub mod suggestions;`. The plan's target structure (lines 28-43) and Step 2 move table (lines 117-130) do not reference it at all.

It is pure business logic with no Tauri dep, so it must move to `core/src/suggestions.rs` and `core/src/lib.rs` must declare `pub mod suggestions;`.

**Fix:** Add a row to the Step 2 move table for `suggestions.rs`. Add `pub mod suggestions;` to the listed `core/src/lib.rs` snippet (line 134-148).

### [ ] C2. `calculations` is a directory with submodules, not a single file — move table is wrong

The plan's Step 2 table (line 119) writes:
```
src-tauri/src/calculations/  →  src-tauri/core/src/calculations/
```

That's correct as a `git mv`, but the actual contents are not just a flat directory — `src-tauri/src/calculations/` contains `mod.rs`, `energy.rs`, `energy_tests.rs`, `phev.rs`, `phev_tests.rs`, `tests.rs`, and `time_inference.rs`. The plan never inventories these files and never confirms the test files (`energy_tests.rs`, `phev_tests.rs`, `tests.rs`) move with them. Same shape as the other test relocations the plan does call out (`db_tests.rs`, `export_tests.rs`, etc.) — easy to miss, easy to break the test count.

**Fix:** Either replace the single line with an explicit list of files in `calculations/`, or add a one-liner: "the entire directory moves, including all `*_tests.rs` and submodules: `energy.rs`, `phev.rs`, `time_inference.rs`."

### [ ] C3. `src-tauri/src/main.rs` has no migration target

`src-tauri/src/main.rs` exists (six lines: `app_lib::run()` with `windows_subsystem` attribute) and is the actual entry point for the desktop binary. The current `Cargo.toml` defines this implicitly (no explicit `[[bin]]` for it; relies on `default-run = "kniha-jazd"` and the conventional `src/main.rs` location).

Step 5 (lines 210-216) proposes a new `desktop/src/main.rs` with body `fn main() { app::run(); }` — but:
1. Doesn't explain where `app::` comes from (the plan moves `lib.rs` to `desktop/src/lib.rs`, so the local module is unnamed/`crate::`, not `app::`).
2. Drops the `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` attribute — without it, release builds open a console window on Windows. That's a user-visible regression.
3. The parenthetical "Or keep `lib.rs::run()` as the binary entry via Tauri's standard pattern — match what existed before" punts on the decision.

**Fix:** Specify exactly the contents of `desktop/src/main.rs`: preserve the `windows_subsystem` attribute and call `kniha_jazd_desktop::run()` (or whatever the lib name resolves to). Decide explicitly between `main.rs` shim vs. binary-from-lib pattern; don't leave it to the implementer.

### [ ] C4. The dispatcher rewrite is not in the step list

`src-tauri/src/server/dispatcher.rs` has 61 occurrences of `crate::commands::*_internal` and `dispatcher_async.rs` has 6. After Step 4 splits commands into `core/src/commands_internal/<name>.rs`, every one of those 67 call sites breaks because:
- `crate::commands::create_trip_internal` → must become `crate::commands_internal::trips::create_trip_internal`
- The dispatcher itself lives in `core` (Step 3), so it's `crate::` not `kniha_jazd_core::`, but the path inside the crate changes from `commands::` to `commands_internal::<file>::`.

Step 4 mentions "Imports: `use crate::{db::Database, app_state::AppState, models::*, ...}`" for the moved `_internal` files but never tells the implementer to update the dispatchers' 67 call sites. This is the largest mechanical change in the entire migration and it is invisible in the plan.

**Fix:** Add an explicit substep to Step 4: "Update `core/src/server/dispatcher.rs` and `dispatcher_async.rs` — replace every `crate::commands::<fn>_internal` with `crate::commands_internal::<group>::<fn>_internal`. Group = the source file the function came from (e.g., `backup`, `trips`, `vehicles`). Verify with `cargo build -p kniha-jazd-core`." Optionally use `pub use` re-exports in `commands_internal/mod.rs` to keep the old paths working as a compatibility shim during migration.

---

## Important Findings

### [ ] I1. Shared types like `BackupInfo` need explicit ownership

`commands/backup.rs` declares `BackupInfo`, `CleanupPreview`, `CleanupResult` (data structures with `Serialize`/`Deserialize`) at the top of the file, then uses them in both `_internal` functions and `#[tauri::command]` wrappers. After the split:
- `core/commands_internal/backup.rs` needs the types because `_internal` functions return them.
- `desktop/commands/backup.rs` needs the types because the wrappers also return them.

The plan says (line 165) the wrappers become "thin delegators" but doesn't address shared structs. The natural answer is "structs go to core, desktop re-imports via `kniha_jazd_core::commands_internal::backup::BackupInfo`" — but that needs to be stated. Likely affects similar structs in `receipts_cmd.rs`, `statistics.rs`, and others.

**Fix:** Add to Step 4 a line like: "Shared types (request/response structs, enums) defined in the source command file move to `core/commands_internal/<name>.rs` alongside the `_internal` functions; desktop wrappers import them via `use kniha_jazd_core::commands_internal::<name>::*`."

### [ ] I2. `commands_tests.rs` placement claim is suspicious

Step 4 line 182 says "Move the corresponding `*_tests.rs` file (e.g., `commands_tests.rs`) to `core/src/commands_internal/`". But `src-tauri/src/commands/` contains a single `commands_tests.rs` (not one per file), and `commands/mod.rs` line 264-266 attaches it via `#[path = "commands_tests.rs"] mod tests;`. That single test file likely tests across multiple feature areas and may import from multiple sibling modules.

After the split, where does `commands_tests.rs` live? Under `core/commands_internal/mod.rs`? Will it `use super::*` correctly when the underlying modules are now `commands_internal::backup`, `commands_internal::trips`, etc.?

**Fix:** Specify whether `commands_tests.rs` (a) stays as one file under `core/commands_internal/` and updates its imports, (b) gets split into per-module test files, or (c) becomes a workspace-level integration test under `tests/`. Pick one and document.

### [ ] I3. Verification command in Step 3 is invalid

Step 3 line 157 says: "Verify ... `cargo grep -r 'tauri' core/src/` returns nothing (use Grep tool, not shell grep)."

`cargo grep` is not a real cargo subcommand. The intent is clear but the literal command will fail. Also the parenthetical "use Grep tool" suggests the implementer (a future Claude agent) should switch tools mid-bash-block, which isn't actionable as a verification step.

**Fix:** Replace with: "Verify with `cd core && cargo tree | grep -i tauri` (returns empty) and a Grep search for `tauri::` under `core/src/` (returns no matches except in comments)."

### [ ] I4. Step 6 verification of `ldd` only works on Linux/WSL

Step 6 line 264 invokes `ldd target/release/kniha-jazd-web` to verify GTK symbols are gone. This is correct for the Docker stage but the development primary platform is Windows (per `package.json` scripts using `set` for env vars and the CI's heavy Windows usage). On Windows, `ldd` doesn't exist; the equivalent is `dumpbin /dependents` or PowerShell's binary inspection.

For a Windows-based developer iterating on this, the verification step is unrunnable.

**Fix:** Add a Windows-equivalent: "On Windows: `dumpbin /dependents target\release\kniha-jazd-web.exe | Select-String -Pattern 'webkit|gtk|gdk|soup|appindicator|rsvg'` should return nothing. On Linux/WSL: use `ldd` as written." Or note that Linux/WSL/Docker is required to fully verify and CI catches the Windows side via the Docker image build.

### [ ] I5. CI workflow changes are under-specified

Step 8 says:
1. "Replace `cargo test` invocations with `cargo test --workspace`"
2. "Add a build step `cargo build -p kniha-jazd-web --release --no-default-features` to the Linux job"

Issues:
- The current workflow's `backend-tests` job at line 65 runs `cargo test` from `src-tauri` working directory. After the split, that's still `src-tauri/` (workspace root) so `cargo test` from there builds the workspace by default. `--workspace` is harmless but unnecessary unless a member opts out — clarify whether this is for safety or required.
- `--no-default-features` makes no sense for `kniha-jazd-web` after the split because `web/Cargo.toml` (per Step 6) only depends on `core` and `tokio` — there are no Tauri features to disable. The `--no-default-features` flag is leftover thinking from the alternative feature-flag approach (option a in 01-task.md). It would build but adds noise.
- The integration-build step (line 98) runs `npm run tauri build -- --debug --config src-tauri/tauri.conf.dev.json`. After the split, `tauri.conf.dev.json` lives in `desktop/`, so the path becomes `src-tauri/desktop/tauri.conf.dev.json`. Same for `tauri.conf.json` in production builds. Plan doesn't list this CI path update.
- `TAURI_BINARY: src-tauri/target/debug/kniha-jazd.exe` (CI line 187) — does the binary name change? `desktop/Cargo.toml` will name the package `kniha-jazd-desktop`, so the binary is `kniha-jazd-desktop.exe` unless renamed. Step 5 doesn't address this and integration tests will fail to find the binary.

**Fix:** Expand Step 8 with explicit list of every line in `.github/workflows/test.yml` that needs touching: (a) `tauri.conf*` paths, (b) `TAURI_BINARY` filename, (c) `cargo test` directory, (d) Docker build context (if changed). Drop the `--no-default-features` flag — it's a vestige of the rejected approach.

### [ ] I6. `tauri.conf.json` paths after move not addressed

`src-tauri/tauri.conf.json` line 7: `"frontendDist": "../build"` (relative to `src-tauri/`). After moving the file to `src-tauri/desktop/`, the relative path becomes wrong — `../build` now points to `src-tauri/build`, not the project root's `build/` directory. The correct path becomes `../../build`.

Same for the icon paths (lines 31-37: `"icons/32x32.png"` etc.) — the `icons/` directory moves to `desktop/icons/` (correct, paths stay relative to the conf file location, so they still work). But `frontendDist` and any build script references break.

`build.rs` (which Step 5 moves to `desktop/build.rs`) calls `tauri_build::build()` which reads `tauri.conf.json` and resolves `frontendDist` relative to itself.

Step 5 line 241 hand-waves this with "Likely `'build.beforeDevCommand': 'npm run dev'` and adjusted `frontendDist` paths (relative to `desktop/`)" — this is the most fragile step in the whole migration and gets one parenthetical sentence.

**Fix:** Specify the exact path changes: `frontendDist: "../build"` → `frontendDist: "../../build"`, and verify `beforeDevCommand` and `beforeBuildCommand` still resolve npm scripts correctly (they invoke from the Tauri working directory, which is now `desktop/`).

---

## Minor Findings

### [ ] M1. `Cargo.toml.old` rename is unnecessary

Step 1 line 88 renames `src-tauri/Cargo.toml` → `Cargo.toml.old`, then Step 9 line 292 deletes it. Cleaner: don't keep the file, since the same content is already in git history and `git show HEAD:src-tauri/Cargo.toml` recovers it instantly. Saves a "what is this `.old` file?" question for any reviewer.

### [ ] M2. `0.33.0` version pinned in workspace, but `Cargo.toml` already has it

Workspace `[workspace.package]` declares `version = "0.33.0"` (line 96 of plan). That matches today's `Cargo.toml` (line 3). Just confirming the plan author noticed — if a release happens between plan-write and execution, this needs updating. Worth a one-line "Pre-execution check: confirm this matches `package.json` version."

### [ ] M3. Step 9 cleanup mentions `default-run = "kniha-jazd"` removal

Step 9 line 294 says "Remove `default-run = "kniha-jazd"` from any leftover Cargo.toml — workspaces don't need it." Correct that workspaces don't need it, but `desktop/Cargo.toml` *does* if it has multiple `[[bin]]` entries (and it doesn't, per Step 5, so this is fine). The note is a bit confusing because it talks about "leftover" Cargo.toml which doesn't exist after Step 9 step 1 deletes `Cargo.toml.old`. Just a wording issue.

---

## Structure & Approach

The overall approach (workspace > feature flag) is well-justified in the plan's "Why a workspace, not a feature flag" section. The step ordering (skeleton → core modules → server → commands split → desktop → web → Docker → CI → cleanup) is logical and each step has a verification gate, which is the right shape.

The "Reused code — no rewrites needed" section (lines 299-312) accurately calls out that `_internal` functions are already framework-free. This claim was verified against `src-tauri/src/commands/backup.rs` (separation pattern is real) and `dispatcher.rs` (already calls `*_internal` directly).

The Rollback plan (lines 358-366) sensibly notes that Step 4 is highest-risk and recommends commit-per-file-split.

What's missing is the bridge between "we have a plan" and "the next agent can execute mechanically": the gaps above (especially C1, C3, C4, I1, I6) require the implementer to make decisions on the fly, which is exactly what a plan should prevent.

---

## Verification of Plan Claims

| Plan claim | Verified | Notes |
|------------|----------|-------|
| "9 command files in src-tauri/src/commands/" | ✅ | Plus `mod.rs` and `commands_tests.rs` |
| "74 `#[tauri::command]` wrappers" | ✅ | Counts match per-file totals |
| "dispatcher.rs is already Tauri-free" | ✅ | No `tauri::` references found |
| "lib.rs invokes 74 commands" | ✅ | `invoke_handler!` macro lists ~70+ entries |
| "resolve_static_dir at lines 174-191 of server/mod.rs" | ✅ | Lines 174-191 match exactly |
| "Tauri config at src-tauri/tauri.conf.json" | ✅ | Plus `tauri.conf.dev.json` |
| "Migrations directory exists" | ✅ | 16 migration files present |

---

## Recommendation

**Ready to implement once Critical findings (C1-C4) and Important findings I1, I5, I6 are addressed.** I2, I3, I4 should also be resolved but are smaller and can be done during implementation if pressed for time. Minor findings are nice-to-have polish.

The plan is structurally sound and the approach is correct. With the gaps filled, this should be a straightforward (if mechanical) refactor.

## Resolution

*To be filled in during Phase 2 after user direction.*
