# Web/Headless Deployment Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run Kniha Jázd without a desktop window — headless mode for always-on PCs.

**Prerequisite:** [Task 55 (Server Mode)](../_done/55-server-mode/01-task.md) — all HTTP infrastructure is already built.

**Estimated Duration:** 1 day

---

## Task 1: Headless Mode Flag

**Goal:** Add a `--headless` CLI flag and `KNIHA_JAZD_HEADLESS` env var that prevents the Tauri window from appearing.

**Files:**
- Modify: [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs)
- Modify: [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json)

**Steps:**

1. Write a test (manual — Tauri window behavior can't be unit-tested):
   - Run app with `KNIHA_JAZD_HEADLESS=1 KNIHA_JAZD_SERVER_AUTOSTART=1` → no window appears, server starts, URL printed to console.

2. Detect headless mode early in `setup()`:
   ```rust
   let headless = std::env::var("KNIHA_JAZD_HEADLESS").is_ok()
       || std::env::args().any(|a| a == "--headless");
   ```

3. If headless, hide the default window after setup:
   ```rust
   if headless {
       if let Some(window) = app.get_webview_window("main") {
           window.hide().ok();
       }
   }
   ```

4. If headless and server not configured, force-enable it:
   ```rust
   if headless {
       // Auto-start server even if not previously enabled in settings
       let auto_start_env = true; // Override
   }
   ```

5. Print server URL to stdout (visible in console/Docker logs):
   ```rust
   if headless {
       println!("Kniha Jázd server running at {}", url);
       println!("Press Ctrl+C to stop.");
   }
   ```

**Verify:**
```bash
# Windows
set KNIHA_JAZD_HEADLESS=1 && cargo run
# Or
cargo run -- --headless
```

Server starts, URL printed, no window appears.

**Commit:** `feat: add headless mode for running without desktop UI`

---

## Task 2: Documentation

**Goal:** Document the headless deployment option.

**Files:**
- Modify: [docs/features/server-mode.md](../../docs/features/server-mode.md) (add headless section)
- Modify: [README.md](../../README.md) (mention headless mode)
- Modify: [README.en.md](../../README.en.md) (same)

**Steps:**

1. Add "Headless Mode" section to [server-mode.md](../../docs/features/server-mode.md):
   - How to start (`--headless` flag or `KNIHA_JAZD_HEADLESS=1`)
   - Environment variables (`KNIHA_JAZD_SERVER_PORT`, `KNIHA_JAZD_SERVER_AUTOSTART`)
   - Example: always-on home server, Windows Task Scheduler, systemd service
   - Limitations (no file dialogs, no updater — same as browser mode)

2. Add one-liner to both READMEs under features.

**Commit:** `docs: add headless mode documentation`

---

## Task 3: Verify and Close

**Goal:** Run the full test suite, verify headless mode works.

**Steps:**

1. Backend tests:
   ```bash
   cd src-tauri && cargo test
   ```

2. Manual headless test:
   ```bash
   KNIHA_JAZD_HEADLESS=1 cargo run
   ```
   - Verify: no window, server URL in console, browser can access UI

3. Normal desktop test:
   ```bash
   cargo run
   ```
   - Verify: window appears normally, server mode toggle still works in Settings

4. Update [_tasks/index.md](../index.md): mark task 33 complete.

**Commit:** `chore: mark task 33 web-deployment complete`

---

## Future: Docker Deployment (Target B)

Not in scope for this plan. When needed, the work is:

1. **Standalone binary** — `src-tauri/src/bin/web.rs` (~100 lines) using [server/mod.rs](../../src-tauri/src/server/mod.rs) directly, no Tauri dependency
2. **`WebConfig`** — env-based config (database path, port, Gemini API key)
3. **Receipt path normalization** — desktop absolute paths → Docker volume relative paths
4. **Dockerfile** — multi-stage build (Rust + Node), `debian:bookworm-slim` final image
5. **docker-compose.yml** — volume mount for `/data` (DB + receipts + backups)

The `server/` module, all `_internal` functions, and the static frontend are already framework-independent — the standalone binary just wires them together without Tauri.

---

## Post-Implementation Checklist

- [ ] `--headless` flag works (no window, server auto-starts)
- [ ] `KNIHA_JAZD_HEADLESS=1` env var works
- [ ] Server URL printed to stdout
- [ ] Normal desktop mode unaffected
- [ ] All 280 backend tests pass
- [ ] Documentation updated
