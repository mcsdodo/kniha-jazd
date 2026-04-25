# Web/Headless Deployment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run Kniha Jázd without a desktop window — standalone server binary in Docker + headless desktop mode.

**Architecture:** The `server/` module, all `_internal` command functions, and the static frontend are already framework-independent (from [Task 55](../_done/55-server-mode/01-task.md)). A new `web` binary wires them together without Tauri. The existing Tauri binary gets a `--headless` flag.

**Tech Stack:** Existing Axum server, Docker multi-stage build (Rust + Node), `debian:bookworm-slim` runtime image.

**Definition of Done:** All integration tests (all tiers) pass against the Docker container, against the Tauri server mode, and against Tauri desktop mode.

---

## Task 1: Make Library Modules Public

**Goal:** The `web` binary needs to import `db::Database`, `app_state::AppState`, `server::HttpServer`, etc. Currently all modules in [lib.rs](../../src-tauri/src/lib.rs) are private (`mod X`). Make them public.

**Files:**
- Modify: [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs) lines 1-14

**Step 1:** Change all module declarations from `mod` to `pub mod`:

```rust
pub mod app_state;
pub mod calculations;
pub mod commands;
pub mod constants;
pub mod db;
pub mod db_location;
pub mod export;
pub mod gemini;
pub mod models;
pub mod receipts;
pub mod schema;
pub mod server;
pub mod settings;
pub mod suggestions;
```

**Step 2:** The command submodules in [commands/mod.rs](../../src-tauri/src/commands/mod.rs) lines 5-13 are also private. Make them public so the web binary can import `_internal` functions:

```rust
pub mod backup;
pub mod export_cmd;
pub mod integrations;
pub mod receipts_cmd;
pub mod server_cmd;
pub mod settings_cmd;
pub mod statistics;
pub mod trips;
pub mod vehicles;
```

**Step 3:** Also make the helper functions public. In [commands/mod.rs](../../src-tauri/src/commands/mod.rs):
- `pub(crate) fn parse_iso_datetime` → `pub fn parse_iso_datetime` (line 44)
- `pub(crate) fn get_app_data_dir` → keep `pub(crate)` (Tauri-only, web binary won't use it)
- `pub(crate) fn get_db_paths_for_dir` → `pub fn get_db_paths_for_dir` (line 87)
- `pub(crate) fn get_db_paths` → keep `pub(crate)` (Tauri-only)
- `pub(crate) fn calculate_trip_numbers` → `pub fn calculate_trip_numbers` (line 102)
- `pub(crate) fn calculate_odometer_start` → `pub fn calculate_odometer_start` (line 122)
- `pub(crate) fn generate_month_end_rows` → `pub fn generate_month_end_rows` (line 170)

**Step 4:** Verify it compiles:

```bash
cd src-tauri && cargo check
```

Expected: compiles with no errors (warnings OK).

**Step 5:** Run tests to ensure nothing broke:

```bash
cd src-tauri && cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 280 passed; 0 failed`

**Step 6:** Commit:

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands/mod.rs
git commit -m "refactor: make library modules public for web binary"
```

---

## Task 2: Standalone Web Binary

**Goal:** Create a `web` binary that starts the HTTP server without Tauri.

**Files:**
- Modify: [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml)
- Create: `src-tauri/src/bin/web.rs`

**Step 1:** Add binary target to [Cargo.toml](../../src-tauri/Cargo.toml). After the existing `[lib]` section (line ~15):

```toml
[[bin]]
name = "web"
path = "src/bin/web.rs"
```

**Step 2:** Create `src-tauri/src/bin/web.rs`:

```rust
use app_lib::app_state::AppState;
use app_lib::db::Database;
use app_lib::server::HttpServer;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
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

    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    let db = Arc::new(Database::new(db_path).expect("Failed to open database"));
    let app_state = Arc::new(AppState::new());

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let addr = HttpServer::start(
            db,
            app_state,
            data_dir,
            static_dir,
            port,
            true,
            shutdown_rx,
        )
        .await
        .expect("Failed to start server");

        println!("Kniha Jázd server running at http://{addr}");

        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("Shutting down...");
        let _ = shutdown_tx.send(());
    });
}
```

**Step 3:** Verify it compiles:

```bash
cd src-tauri && cargo check --bin web
```

Expected: compiles with no errors. If there are errors from Tauri types leaking into `server/mod.rs` (the `resolve_static_dir` functions), wrap them in `#[cfg]` — see Step 3a.

**Step 3a (if needed):** The `resolve_static_dir` and `resolve_static_dir_from_handle` functions in [server/mod.rs](../../src-tauri/src/server/mod.rs) take `&tauri::App` / `&tauri::AppHandle`. These compile because `tauri` is in `[dependencies]`. If this causes issues (e.g., tauri requires system libs), gate them:

```rust
#[cfg(feature = "tauri")]
pub fn resolve_static_dir(app: &tauri::App) -> PathBuf { ... }
```

But since Tauri is already a dependency and we're not removing it, this likely isn't needed. The web binary just doesn't call these functions.

**Step 4:** Build the binary:

```bash
cd src-tauri && cargo build --bin web
```

Expected: binary at `target/debug/web.exe` (Windows) or `target/debug/web` (Linux).

**Step 5:** Test it runs:

```bash
# Create temp data dir
mkdir -p /tmp/web-test

# Start server
KNIHA_JAZD_DATA_DIR=/tmp/web-test ./target/debug/web &
WEB_PID=$!

# Wait for startup
sleep 2

# Hit health endpoint
curl -s http://localhost:3456/health
# Expected: "ok"

# Hit RPC endpoint
curl -s -X POST http://localhost:3456/api/rpc \
  -H "Content-Type: application/json" \
  -d '{"command":"get_vehicles","args":{}}' 
# Expected: []

# Stop server
kill $WEB_PID
rm -rf /tmp/web-test
```

**Step 6:** Run all existing tests to ensure nothing broke:

```bash
cd src-tauri && cargo test 2>&1 | grep "test result"
```

Expected: `test result: ok. 280 passed; 0 failed`

**Step 7:** Commit:

```bash
git add src-tauri/Cargo.toml src-tauri/src/bin/web.rs
git commit -m "feat: add standalone web server binary (no Tauri UI dependency)"
```

---

## Task 3: Docker Configuration

**Goal:** Multi-stage Dockerfile that builds the `web` binary and SvelteKit frontend, runs in a minimal image.

**Files:**
- Create: `Dockerfile.web`
- Create: `docker-compose.web.yml`
- Modify: `.dockerignore` (create if missing)

**Step 1:** Create or update `.dockerignore`:

```
node_modules/
src-tauri/target/
.git/
_tasks/
.claude/
.worktrees/
.github/
data/
*.md
```

**Step 2:** Create `Dockerfile.web`. Key decisions:
- Stage 1 (Rust): Use `rust:1.85-bookworm` (match your toolchain). Install diesel CLI for schema setup. Build `--bin web`.
- Stage 2 (Node): `node:20-alpine`. `npm ci && npm run build` produces `build/` directory.
- Stage 3 (Runtime): `debian:bookworm-slim`. Copy binary + static files. Minimal deps: `ca-certificates`, `libsqlite3-0`.

```dockerfile
# Stage 1: Build Rust backend
FROM rust:1.85-bookworm AS rust-builder
WORKDIR /app/src-tauri

# Install diesel CLI for schema setup
RUN cargo install diesel_cli --no-default-features --features sqlite

# Copy Cargo files for dependency caching
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./

# Create dummy main and lib for dependency caching
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/web.rs && \
    echo "pub fn run() {}" > src/lib.rs && \
    cargo build --release --bin web 2>/dev/null || true

# Copy actual source
COPY src-tauri/src ./src
COPY src-tauri/migrations ./migrations

# Setup schema for diesel (needed at compile time)
ENV DATABASE_URL=sqlite:///tmp/build.db
RUN diesel setup && diesel migration run

# Build release binary
RUN cargo build --release --bin web

# Stage 2: Build SvelteKit frontend
FROM node:20-alpine AS node-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Stage 3: Final runtime image
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/src-tauri/target/release/web /usr/local/bin/kniha-jazd-web
COPY --from=node-builder /app/build /var/www/html

RUN mkdir -p /data

ENV DATABASE_PATH=/data/kniha-jazd.db
ENV KNIHA_JAZD_DATA_DIR=/data
ENV STATIC_DIR=/var/www/html
ENV PORT=3456

VOLUME /data
EXPOSE 3456

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
    CMD curl -f http://localhost:3456/health || exit 1

CMD ["kniha-jazd-web"]
```

**Step 3:** Create `docker-compose.web.yml`:

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
      - KNIHA_JAZD_DATA_DIR=/data
      - PORT=3456
      # Uncomment for receipt OCR:
      # - GEMINI_API_KEY=your-key
    restart: unless-stopped
```

**Step 4:** Build the Docker image:

```bash
docker compose -f docker-compose.web.yml build
```

Expected: builds all 3 stages successfully. This will take several minutes on first run (Rust compilation).

**Step 5:** Start the container:

```bash
mkdir -p data
docker compose -f docker-compose.web.yml up -d
```

**Step 6:** Verify it's running:

```bash
# Health check
curl -s http://localhost:3456/health
# Expected: "ok"

# RPC works
curl -s -X POST http://localhost:3456/api/rpc \
  -H "Content-Type: application/json" \
  -d '{"command":"get_vehicles","args":{}}' 
# Expected: []

# Frontend loads
curl -s http://localhost:3456/ | head -5
# Expected: HTML with <html> tag
```

**Step 7:** Stop the container:

```bash
docker compose -f docker-compose.web.yml down
```

**Step 8:** Commit:

```bash
git add Dockerfile.web docker-compose.web.yml .dockerignore
git commit -m "feat: add Docker configuration for web deployment"
```

---

## Task 4: Integration Tests Against Docker

**Goal:** Run existing server-mode integration tests against the Docker container. The test harness already uses Chrome against an HTTP server — just point it at Docker instead of a local Tauri binary.

**Files:**
- Modify: [tests/integration/wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) (support external server)
- Modify: [package.json](../../package.json) (add scripts)

**Step 1:** Modify `onPrepare` in [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) to skip binary spawn when `WDIO_EXTERNAL_SERVER` is set. At the start of `onPrepare` (line ~135), add:

```typescript
onPrepare: async function () {
    const externalServer = process.env.WDIO_EXTERNAL_SERVER === '1';

    // Create sandboxed temp directory for test data
    testDataDir = mkdtempSync(join(tmpdir(), 'kniha-jazd-server-test-'));
    process.env.KNIHA_JAZD_DATA_DIR = testDataDir;
    process.env.WDIO_SERVER_MODE = '1';
    process.env.WDIO_SERVER_URL = SERVER_URL;

    if (externalServer) {
        // Docker mode: server already running externally
        console.log(`Connecting to external server at ${SERVER_URL}`);
        await waitForUrl(`${SERVER_URL}/health`, 30000);
        console.log('External server is ready');
        return;
    }

    // ... rest of existing Tauri binary spawn logic unchanged ...
```

**Step 2:** Modify `onComplete` to skip process cleanup when external:

```typescript
onComplete: async function () {
    if (process.env.WDIO_EXTERNAL_SERVER === '1') {
        console.log('External server mode — skipping cleanup');
        return;
    }

    // ... rest of existing cleanup logic unchanged ...
```

**Step 3:** The `beforeTest` hook resets data via RPC (deletes all vehicles). This already works against any HTTP server — no changes needed. But verify the `SERVER_URL` is used (not hardcoded):

Check line ~230: `const resp = await fetch(\`${SERVER_URL}/api/rpc\`` — should use the variable, not a literal. If it uses the module-level constant (`const SERVER_URL = \`http://localhost:${SERVER_PORT}\``), that's fine as long as the Docker container is on the same port.

**Step 4:** Add npm scripts to [package.json](../../package.json). After the existing `test:integration:server` scripts:

```json
"test:integration:docker": "cross-env WDIO_EXTERNAL_SERVER=1 WDIO_SERVER_MODE=1 wdio run tests/integration/wdio.server.conf.ts",
"test:integration:docker:tier1": "cross-env TIER=1 npm run test:integration:docker"
```

Note: Use `cross-env` if available, otherwise use platform-specific syntax. Check if `cross-env` is in devDependencies. If not, use the same `set` pattern as existing scripts:

```json
"test:integration:docker": "set WDIO_EXTERNAL_SERVER=1&& set WDIO_SERVER_MODE=1&& wdio run tests/integration/wdio.server.conf.ts",
"test:integration:docker:tier1": "set TIER=1&& npm run test:integration:docker"
```

**Step 5:** Test against Docker. Start the container first:

```bash
docker compose -f docker-compose.web.yml up -d
```

Run tier 1 as a smoke test:

```bash
npm run test:integration:docker:tier1
```

Expected: all tier 1 tests pass.

**Step 6:** Run all tiers:

```bash
npm run test:integration:docker
```

Expected: all tests pass.

**Step 7:** Stop Docker:

```bash
docker compose -f docker-compose.web.yml down
```

**Step 8:** Commit:

```bash
git add tests/integration/wdio.server.conf.ts package.json
git commit -m "test: add Docker integration test mode"
```

---

## Task 5: Headless Desktop Mode

**Goal:** Add `--headless` flag to the Tauri binary that hides the window and auto-starts the server.

**Files:**
- Modify: [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs)

**Step 1:** Detect headless mode at the start of `setup()`, after the `app_dir` setup (around line 52):

```rust
let headless = std::env::var("KNIHA_JAZD_HEADLESS").is_ok()
    || std::env::args().any(|a| a == "--headless");
```

**Step 2:** After the auto-start block (around line 229), if headless, hide the window and force server start if not already started:

```rust
if headless {
    // Hide the desktop window
    if let Some(window) = app.get_webview_window("main") {
        window.hide().ok();
    }

    // If server didn't auto-start, start it now
    if !server_manager.status().running {
        let headless_port = std::env::var("KNIHA_JAZD_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3456);
        let headless_db = app.state::<Arc<db::Database>>().inner().clone();
        let headless_app_state = app.state::<Arc<AppState>>().inner().clone();
        let headless_app_dir = app_dir.clone();
        let headless_static_dir = server::resolve_static_dir(app);
        let headless_manager = server_manager.clone();

        tauri::async_runtime::spawn(async move {
            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
            match crate::server::HttpServer::start(
                headless_db,
                headless_app_state,
                headless_app_dir,
                headless_static_dir,
                headless_port,
                true,
                shutdown_rx,
            )
            .await
            {
                Ok(_addr) => {
                    let url = local_ip_address::local_ip()
                        .map(|ip| format!("http://{}:{}", ip, headless_port))
                        .unwrap_or_else(|_| format!("http://localhost:{}", headless_port));
                    println!("Kniha Jázd server running at {url}");
                    println!("Press Ctrl+C to stop.");
                    headless_manager.set_running(headless_port, url, shutdown_tx);
                }
                Err(e) => {
                    eprintln!("Failed to start server: {e}");
                    std::process::exit(1);
                }
            }
        });
    }
}
```

Note: `app.get_webview_window("main")` requires `use tauri::Manager;` which is already imported.

**Step 3:** Verify it compiles:

```bash
cd src-tauri && cargo check
```

Expected: no errors.

**Step 4:** Test headless mode manually:

```bash
# Windows
set KNIHA_JAZD_HEADLESS=1&& cargo run
# Expected: no window, "Kniha Jázd server running at http://..." printed

# Then in another terminal:
curl http://localhost:3456/health
# Expected: "ok"
```

**Step 5:** Test normal mode still works:

```bash
cargo run
# Expected: window appears, app works normally
```

**Step 6:** Commit:

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add --headless mode for running without desktop UI"
```

---

## Task 6: Documentation

**Goal:** Document both deployment options.

**Files:**
- Modify: [docs/features/server-mode.md](../../docs/features/server-mode.md) (add headless + Docker sections)
- Modify: [README.md](../../README.md) (mention Docker)
- Modify: [README.en.md](../../README.en.md) (same)

**Step 1:** Add "Headless Mode" section to [server-mode.md](../../docs/features/server-mode.md):
- `--headless` flag or `KNIHA_JAZD_HEADLESS=1` env var
- Environment variables: `KNIHA_JAZD_SERVER_PORT`, `KNIHA_JAZD_HEADLESS`
- Use case: always-on home server, Windows Task Scheduler, systemd service

**Step 2:** Add "Docker Deployment" section to [server-mode.md](../../docs/features/server-mode.md):
- Quick start with `docker compose -f docker-compose.web.yml up`
- Migration from desktop: copy DB + receipts to `./data/`
- Environment variable reference table
- Limitations: no file dialogs, no auto-updater

**Step 3:** Add one-liner to both READMEs mentioning Docker deployment.

**Step 4:** Commit:

```bash
git add docs/features/server-mode.md README.md README.en.md
git commit -m "docs: add headless mode and Docker deployment documentation"
```

---

## Task 7: Full Verification

**Goal:** All test suites pass — backend, Tauri desktop integration, server-mode integration, and Docker integration.

**Step 1:** Backend tests:

```bash
cd src-tauri && cargo test
```

Expected: `280 passed; 0 failed` (or more if new tests added).

**Step 2:** Tauri desktop integration tests — all tiers:

```bash
npm run test:integration:build
npm run test:integration
```

Expected: all tests pass.

**Step 3:** Server-mode integration tests — all tiers (Tauri binary with HTTP):

```bash
npm run test:integration:server
```

Expected: all tests pass.

**Step 4:** Docker integration tests — all tiers:

```bash
docker compose -f docker-compose.web.yml up -d
npm run test:integration:docker
docker compose -f docker-compose.web.yml down
```

Expected: all tests pass.

**Step 5:** Update [_tasks/index.md](../index.md) — mark task 33 complete.

**Step 6:** Commit:

```bash
git add _tasks/index.md
git commit -m "chore: mark task 33 web-deployment complete"
```

---

## Post-Implementation Checklist

- [ ] Library modules are public (`pub mod` in [lib.rs](../../src-tauri/src/lib.rs))
- [ ] `web` binary compiles: `cargo build --bin web`
- [ ] `web` binary starts and `/health` responds
- [ ] Docker image builds: `docker compose -f docker-compose.web.yml build`
- [ ] Docker container starts and browser can access `http://localhost:3456`
- [ ] All CRUD operations work in browser against Docker container
- [ ] `--headless` flag works on desktop (no window, server auto-starts)
- [ ] `KNIHA_JAZD_HEADLESS=1` env var works
- [ ] Server URL printed to stdout in headless/web mode
- [ ] All 280+ backend tests pass: `cargo test`
- [ ] Tauri desktop integration tests pass (all tiers): `npm run test:integration`
- [ ] Server-mode integration tests pass (all tiers): `npm run test:integration:server`
- [ ] Docker integration tests pass (all tiers): `npm run test:integration:docker`
- [ ] Documentation updated ([server-mode.md](../../docs/features/server-mode.md), READMEs)
- [ ] Data persists across Docker container restarts
