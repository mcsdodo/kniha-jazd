# Web/Headless Deployment Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run Kniha Jázd without a desktop window — as a standalone server binary in Docker, and as a headless desktop app.

**Prerequisite:** [Task 55 (Server Mode)](../_done/55-server-mode/01-task.md) — all HTTP infrastructure is already built.

**Estimated Duration:** 2-3 days

**Definition of Done:** Integration tests pass against a locally-running Docker container.

---

## What Already Exists (from Task 55)

All HTTP infrastructure is built and tested:
- Axum server with RPC dispatcher (67 commands), static file serving, SPA fallback, CORS
- All `_internal` functions extracted from Tauri wrappers — framework-independent
- Frontend dual-mode API adapter (`apiCall()` routes to `invoke()` or `fetch()`)
- Server-mode integration test config ([wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts)) — launches Chrome against HTTP, runs same specs
- `npm run test:integration:server` — already works against the Tauri binary with server auto-start

## Two Deliverables

### A. Standalone Server Binary (for Docker)

A separate binary (`web`) that uses the existing [server/](../../src-tauri/src/server/) module without any Tauri dependency. Tauri requires `libwebkit2gtk` + a display server on Linux — can't run in Docker.

### B. Headless Desktop Mode (for always-on PCs)

A `--headless` flag on the existing Tauri binary that hides the window and auto-starts the server. For users who want to run the app on a home PC without the UI.

---

## Task 1: Standalone Server Binary

**Goal:** Create a `web` binary that starts the HTTP server without Tauri.

**Files:**
- Create: `src-tauri/src/bin/web.rs`
- Modify: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml) (add `[[bin]]` section)

**Steps:**

1. Add binary target to [Cargo.toml](../../src-tauri/Cargo.toml):
   ```toml
   [[bin]]
   name = "web"
   path = "src/bin/web.rs"
   ```

2. Create `src/bin/web.rs` — a minimal main() that:
   - Reads config from env vars (`DATABASE_PATH`, `PORT`, `KNIHA_JAZD_DATA_DIR`)
   - Opens the database (same `Database::new()`)
   - Creates `AppState`
   - Resolves static dir from env or default
   - Starts `HttpServer::start()` with `bind_all: true`
   - Waits for Ctrl+C (`tokio::signal::ctrl_c()`) for shutdown
   - Prints server URL to stdout

   ```rust
   use kniha_jazd::db::Database;
   use kniha_jazd::app_state::AppState;
   use kniha_jazd::server::HttpServer;
   use std::sync::Arc;
   use std::path::PathBuf;

   #[tokio::main]
   async fn main() {
       let port: u16 = std::env::var("PORT")
           .ok().and_then(|p| p.parse().ok())
           .unwrap_or(3456);

       let data_dir = std::env::var("KNIHA_JAZD_DATA_DIR")
           .map(PathBuf::from)
           .unwrap_or_else(|_| PathBuf::from("/data"));

       let db_path = std::env::var("DATABASE_PATH")
           .map(PathBuf::from)
           .unwrap_or_else(|_| data_dir.join("kniha-jazd.db"));

       let static_dir = std::env::var("STATIC_DIR")
           .map(PathBuf::from)
           .unwrap_or_else(|_| PathBuf::from("/var/www/html"));

       std::fs::create_dir_all(&data_dir).ok();

       let db = Arc::new(Database::new(db_path).expect("Failed to open database"));
       let app_state = Arc::new(AppState::new());

       let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

       let addr = HttpServer::start(
           db, app_state, data_dir, static_dir, port, true, shutdown_rx,
       ).await.expect("Failed to start server");

       println!("Kniha Jázd server running at http://0.0.0.0:{}", port);
       println!("Press Ctrl+C to stop.");

       tokio::signal::ctrl_c().await.ok();
       let _ = shutdown_tx.send(());
   }
   ```

3. Verify it compiles:
   ```bash
   cd src-tauri && cargo build --bin web
   ```

**Note:** The `web` binary does NOT depend on any Tauri crate — it only uses the library's `db`, `app_state`, and `server` modules. If there are compile errors from Tauri-specific imports in `lib.rs`, the binary target may need conditional compilation or a workspace restructure. Investigate and resolve during implementation.

**Verify:**
```bash
cd src-tauri && cargo build --bin web
KNIHA_JAZD_DATA_DIR=$(mktemp -d) ./target/debug/web
# → Server starts, /health responds, Ctrl+C stops cleanly
```

**Commit:** `feat: add standalone web server binary (no Tauri dependency)`

---

## Task 2: Docker Configuration

**Goal:** Multi-stage Dockerfile that builds the `web` binary and SvelteKit frontend, runs in a minimal image.

**Files:**
- Create: `Dockerfile.web`
- Create: `docker-compose.web.yml`
- Create: `.dockerignore` (if not present)

**Steps:**

1. Create `.dockerignore`:
   ```
   node_modules/
   target/
   .git/
   *.md
   _tasks/
   .claude/
   .worktrees/
   data/
   ```

2. Create multi-stage `Dockerfile.web`:
   - **Stage 1 (Rust):** Build `web` binary from `src-tauri/`
   - **Stage 2 (Node):** Build SvelteKit static frontend (`npm run build`)
   - **Stage 3 (Runtime):** `debian:bookworm-slim` with binary + static files + `/data` volume

   Key details:
   - Diesel needs a `DATABASE_URL` at compile time for schema — use `diesel setup` with a temp DB
   - Copy `migrations/` for the Rust build stage
   - Final image needs only `ca-certificates` and `libsqlite3-0`
   - Health check: `curl -f http://localhost:3456/health`
   - Default env: `PORT=3456`, `DATABASE_PATH=/data/kniha-jazd.db`, `STATIC_DIR=/var/www/html`

3. Create `docker-compose.web.yml`:
   ```yaml
   services:
     kniha-jazd:
       build:
         context: .
         dockerfile: Dockerfile.web
       ports:
         - "3456:3456"
       volumes:
         - ./data:/data
       environment:
         - DATABASE_PATH=/data/kniha-jazd.db
         - PORT=3456
         # - GEMINI_API_KEY=your-key
       restart: unless-stopped
   ```

**Verify:**
```bash
docker compose -f docker-compose.web.yml build
docker compose -f docker-compose.web.yml up -d
curl http://localhost:3456/health  # → "ok"
# Open browser → http://localhost:3456 → app loads
docker compose -f docker-compose.web.yml down
```

**Commit:** `feat: add Docker configuration for web deployment`

---

## Task 3: Headless Desktop Mode

**Goal:** Add `--headless` flag to the existing Tauri binary for running on desktop PCs without UI.

**Files:**
- Modify: [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs)

**Steps:**

1. Detect headless mode in `setup()`:
   ```rust
   let headless = std::env::var("KNIHA_JAZD_HEADLESS").is_ok()
       || std::env::args().any(|a| a == "--headless");
   ```

2. If headless, hide the window and force server auto-start:
   ```rust
   if headless {
       if let Some(window) = app.get_webview_window("main") {
           window.hide().ok();
       }
       // Force server auto-start regardless of settings
   }
   ```

3. Print URL to stdout when headless.

**Verify:**
```bash
# Windows
set KNIHA_JAZD_HEADLESS=1 && cargo run
# No window, server starts, URL printed
```

**Commit:** `feat: add headless mode for running without desktop UI`

---

## Task 4: Integration Tests Against Docker

**Goal:** Run the existing server-mode integration tests against the Docker container.

**Files:**
- Modify: [tests/integration/wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) (support external server URL)
- Modify: [package.json](../../package.json) (add `test:integration:docker` script)

**Steps:**

1. The existing [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) currently spawns a Tauri binary in `onPrepare`. For Docker mode, it should skip the spawn and connect to an already-running server.

   Modify `onPrepare` to check for `WDIO_SERVER_URL` env var:
   ```typescript
   onPrepare: async function () {
     if (process.env.WDIO_EXTERNAL_SERVER) {
       // Docker mode: server already running, just wait for health
       await waitForUrl(`${SERVER_URL}/health`, 30000);
       console.log('Connected to external server');
       return;
     }
     // ... existing Tauri binary spawn logic ...
   }
   ```

   Similarly, `onComplete` should skip killing the process:
   ```typescript
   onComplete: async function () {
     if (process.env.WDIO_EXTERNAL_SERVER) return;
     // ... existing cleanup logic ...
   }
   ```

2. Add npm scripts to [package.json](../../package.json):
   ```json
   "test:integration:docker": "set WDIO_EXTERNAL_SERVER=1&& set WDIO_SERVER_MODE=1&& wdio run tests/integration/wdio.server.conf.ts",
   "test:integration:docker:tier1": "set TIER=1&& npm run test:integration:docker"
   ```

3. The `beforeTest` hook already resets data via RPC (`delete_vehicle` for each vehicle), which works against Docker too — no file system access needed.

**Verify:**
```bash
# Terminal 1: Start Docker container
docker compose -f docker-compose.web.yml up

# Terminal 2: Run integration tests against it
npm run test:integration:docker:tier1
```

All tier 1 tests should pass.

**Commit:** `test: add Docker integration test mode`

---

## Task 5: Run All Tests and Verify

**Goal:** All test suites pass — backend, server-mode integration (Tauri binary), and Docker integration.

**Steps:**

1. Backend tests (280 tests):
   ```bash
   cd src-tauri && cargo test
   ```

2. Server-mode integration tests — all tiers (against Tauri binary):
   ```bash
   npm run test:integration:server
   ```

3. Docker integration tests — all tiers:
   ```bash
   docker compose -f docker-compose.web.yml up -d
   npm run test:integration:docker
   docker compose -f docker-compose.web.yml down
   ```

4. Normal desktop integration tests — all tiers (Tauri + tauri-driver):
   ```bash
   npm run test:integration
   ```

5. Update documentation:
   - [docs/features/server-mode.md](../../docs/features/server-mode.md) — add headless + Docker sections
   - [README.md](../../README.md) / [README.en.md](../../README.en.md) — mention Docker deployment

**Commit:** `docs: add Docker deployment and headless mode documentation`

---

## Post-Implementation Checklist

- [ ] `web` binary compiles without Tauri dependency (`cargo build --bin web`)
- [ ] Docker image builds (`docker compose -f docker-compose.web.yml build`)
- [ ] Docker container starts and `/health` responds
- [ ] Browser can access app at `http://localhost:3456` from Docker
- [ ] All CRUD operations work via browser against Docker
- [ ] `--headless` flag works on desktop (no window, server starts)
- [ ] `KNIHA_JAZD_HEADLESS=1` env var works
- [ ] All 280 backend tests pass
- [ ] Server-mode integration tests pass (all tiers): `npm run test:integration:server`
- [ ] Docker integration tests pass (all tiers): `npm run test:integration:docker`
- [ ] Normal Tauri integration tests pass (all tiers): `npm run test:integration`
- [ ] Documentation updated (server-mode feature doc, READMEs)
- [ ] Data persists across Docker container restarts
