# Web Deployment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Convert Tauri desktop app to Docker-hosted web app with Axum backend

**Architecture:** Thin Axum HTTP wrapper around existing Rust logic, SvelteKit static frontend

**Estimated Duration:** 2-3 weeks

---

## Task 0: Async Database Adapter

**Problem:** Current `Database` uses `Mutex<SqliteConnection>`. Holding mutex across async `.await` points causes deadlock with Axum's async handlers.

**Files:**
- Modify: `src-tauri/src/db.rs`
- Create: `src-tauri/src/db_async.rs` (optional wrapper)

**Steps:**
1. Option A (Simplest): Use `spawn_blocking` for all DB operations
   ```rust
   use tokio::task::spawn_blocking;

   pub async fn get_vehicles_async(db: Database) -> Vec<Vehicle> {
       spawn_blocking(move || db.get_vehicles().unwrap_or_default())
           .await
           .unwrap()
   }
   ```

2. Option B: Replace `std::sync::Mutex` with `tokio::sync::Mutex`
   ```rust
   use tokio::sync::Mutex;

   pub struct Database {
       conn: Mutex<SqliteConnection>,  // Now tokio's async Mutex
   }
   ```

3. Option C: Use connection pool with `deadpool-diesel` (overkill for single-user)

**Recommendation:** Option A (`spawn_blocking`) - minimal changes to existing code.

**Verification:** Create simple test that calls DB method from async context without deadlock.

---

## Task 0.5: WebConfig Environment Abstraction

**Problem:** Desktop uses `get_app_data_dir(&app)` and `LocalSettings::load()` which rely on Tauri app context. Web needs environment-based configuration.

**Files:**
- Create: `src-tauri/src/web/config.rs`
- Modify: `src-tauri/src/settings.rs` (if needed)

**Steps:**
1. Create `WebConfig` struct:
   ```rust
   pub struct WebConfig {
       pub database_path: PathBuf,      // DATABASE_PATH env
       pub receipts_path: PathBuf,      // RECEIPTS_PATH env
       pub backups_path: PathBuf,       // BACKUPS_PATH env
       pub gemini_api_key: Option<String>, // GEMINI_API_KEY env
       pub static_dir: PathBuf,         // STATIC_DIR env (default: /var/www/html)
       pub port: u16,                   // PORT env (default: 8080)
   }

   impl WebConfig {
       pub fn from_env() -> Result<Self, String> {
           Ok(Self {
               database_path: env::var("DATABASE_PATH")
                   .map(PathBuf::from)
                   .unwrap_or_else(|_| PathBuf::from("/data/kniha-jazd.db")),
               receipts_path: env::var("RECEIPTS_PATH")
                   .map(PathBuf::from)
                   .unwrap_or_else(|_| PathBuf::from("/data/receipts")),
               backups_path: env::var("BACKUPS_PATH")
                   .map(PathBuf::from)
                   .unwrap_or_else(|_| PathBuf::from("/data/backups")),
               gemini_api_key: env::var("GEMINI_API_KEY").ok(),
               static_dir: env::var("STATIC_DIR")
                   .map(PathBuf::from)
                   .unwrap_or_else(|_| PathBuf::from("/var/www/html")),
               port: env::var("PORT")
                   .ok()
                   .and_then(|p| p.parse().ok())
                   .unwrap_or(8080),
           })
       }
   }
   ```

2. Use `WebConfig` in web binary entry point instead of Tauri's app context.

**Verification:** `WebConfig::from_env()` loads correctly with and without env vars set.

---

## Task 1: Create Axum Web Module Structure

**Files:**
- Create: `src-tauri/src/web/mod.rs`
- Create: `src-tauri/src/web/handlers.rs`
- Create: `src-tauri/src/web/router.rs`
- Create: `src-tauri/src/web/config.rs` (from Task 0.5)
- Create: `src-tauri/src/bin/web.rs`
- Modify: `src-tauri/Cargo.toml` (add axum dependencies)

**Steps:**
1. Add dependencies to Cargo.toml:
   ```toml
   [dependencies]
   axum = "0.7"
   tower = "0.4"
   tower-http = { version = "0.5", features = ["cors", "fs"] }
   # tokio already exists, ensure "full" features

   [[bin]]
   name = "web"
   path = "src/bin/web.rs"
   ```

2. Create `src/web/mod.rs`:
   ```rust
   pub mod config;
   pub mod handlers;
   pub mod router;
   ```

3. Create `src/bin/web.rs` - Binary entry point:
   ```rust
   use kniha_jazd::web::config::WebConfig;
   use kniha_jazd::web::router::create_router;
   use kniha_jazd::db::Database;

   #[tokio::main]
   async fn main() {
       let config = WebConfig::from_env().expect("Invalid config");
       let db = Database::new(config.database_path.clone())
           .expect("Failed to open database");

       let app = create_router(db, config);

       let addr = format!("0.0.0.0:{}", config.port);
       println!("Starting server on {}", addr);

       let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
       axum::serve(listener, app).await.unwrap();
   }
   ```

**Verification:** `cargo check --bin web` compiles without errors

---

## Task 2: Implement Vehicle Handlers

**Files:**
- Modify: `src-tauri/src/web/handlers.rs`
- Modify: `src-tauri/src/web/router.rs`

**Commands covered:** 6 of 35
- `get_vehicles`
- `get_active_vehicle`
- `create_vehicle`
- `update_vehicle`
- `delete_vehicle`
- `set_active_vehicle`

**Steps:**
1. Implement handlers using `spawn_blocking` pattern:
   ```rust
   use axum::{Json, extract::{State, Path}, http::StatusCode};
   use tokio::task::spawn_blocking;

   pub async fn get_vehicles(
       State(db): State<Database>,
   ) -> Json<Vec<Vehicle>> {
       let vehicles = spawn_blocking(move || db.get_vehicles().unwrap_or_default())
           .await
           .unwrap();
       Json(vehicles)
   }

   pub async fn create_vehicle(
       State(db): State<Database>,
       Json(req): Json<CreateVehicleRequest>,
   ) -> Result<Json<Vehicle>, StatusCode> {
       let vehicle = spawn_blocking(move || {
           db.create_vehicle(req.name, req.license_plate, /* ... */)
       })
       .await
       .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
       .map_err(|_| StatusCode::BAD_REQUEST)?;

       Ok(Json(vehicle))
   }
   ```

2. Register routes in `router.rs`:
   ```rust
   .route("/api/vehicles", get(handlers::get_vehicles).post(handlers::create_vehicle))
   .route("/api/vehicles/active", get(handlers::get_active_vehicle))
   .route("/api/vehicles/:id", put(handlers::update_vehicle).delete(handlers::delete_vehicle))
   .route("/api/vehicles/:id/activate", post(handlers::set_active_vehicle))
   ```

**Verification:** `cargo test` - existing tests still pass

---

## Task 3: Implement Trip Handlers

**Files:**
- Modify: `src-tauri/src/web/handlers.rs`
- Modify: `src-tauri/src/web/router.rs`

**Commands covered:** 11 of 35
- `get_trips`
- `get_trips_for_year`
- `get_years_with_trips`
- `create_trip`
- `update_trip`
- `delete_trip`
- `reorder_trip`
- `get_trip_grid_data`
- `calculate_trip_stats`
- `preview_trip_calculation`
- `get_compensation_suggestion`

**Steps:**
1. Implement trip CRUD handlers following Task 2 pattern

2. Implement calculation handlers (reuse existing `commands.rs` logic):
   ```rust
   pub async fn get_trip_grid_data(
       State(db): State<Database>,
       Query(params): Query<TripGridParams>,
   ) -> Json<TripGridData> {
       // Reuse logic from commands::get_trip_grid_data
       // Wrap with spawn_blocking
   }
   ```

3. Routes:
   ```rust
   .route("/api/trips", get(handlers::get_trips).post(handlers::create_trip))
   .route("/api/trips/year/:year", get(handlers::get_trips_for_year))
   .route("/api/trips/years", get(handlers::get_years_with_trips))
   .route("/api/trips/:id", put(handlers::update_trip).delete(handlers::delete_trip))
   .route("/api/trips/:id/reorder", post(handlers::reorder_trip))
   .route("/api/grid-data", get(handlers::get_trip_grid_data))
   .route("/api/stats", get(handlers::calculate_trip_stats))
   .route("/api/preview", post(handlers::preview_trip_calculation))
   .route("/api/compensation", get(handlers::get_compensation_suggestion))
   ```

**Verification:** Manual test with curl/httpie against running server

---

## Task 4: Implement Route, Settings, Backup, Export Handlers

**Files:**
- Modify: `src-tauri/src/web/handlers.rs`
- Modify: `src-tauri/src/web/router.rs`

**Commands covered:** 12 of 35
- `get_routes`
- `get_purposes`
- `get_settings`
- `save_settings`
- `create_backup`
- `list_backups`
- `get_backup_info`
- `restore_backup`
- `delete_backup`
- `export_html`
- `export_to_browser` → becomes `export_html` (returns HTML directly)
- `get_optimal_window_size` → **N/A for web** (skip)

**Steps:**
1. Route/Purpose handlers:
   ```rust
   pub async fn get_routes(...) -> Json<Vec<Route>>
   pub async fn get_purposes(...) -> Json<Vec<String>>
   ```

2. Settings handlers:
   ```rust
   pub async fn get_settings(...) -> Json<Option<Settings>>
   pub async fn save_settings(...) -> StatusCode
   ```

3. Backup handlers (use `WebConfig.backups_path`):
   ```rust
   pub async fn create_backup(
       State(db): State<Database>,
       State(config): State<WebConfig>,
   ) -> Json<BackupInfo> {
       // Use config.backups_path instead of get_app_data_dir
   }
   ```

4. Export handler (returns HTML directly for download):
   ```rust
   use axum::response::Html;

   pub async fn export_html(
       State(db): State<Database>,
       Query(params): Query<ExportParams>,
   ) -> Html<String> {
       // Generate HTML using existing export::generate_html logic
       Html(html_content)
   }
   ```

**Verification:** All handlers compile, basic curl tests work

---

## Task 5: Implement Receipt Handlers with Path Normalization

**Files:**
- Modify: `src-tauri/src/web/handlers.rs`
- Modify: `src-tauri/src/web/router.rs`

**Commands covered:** 10 of 35
- `get_receipt_settings`
- `get_receipts`
- `get_receipts_for_vehicle`
- `get_unassigned_receipts`
- `scan_receipts` → **Modified for web** (scan WebConfig.receipts_path)
- `sync_receipts` → Uses `GEMINI_API_KEY` from WebConfig
- `process_pending_receipts` → Uses `GEMINI_API_KEY` from WebConfig
- `reprocess_receipt` → Uses `GEMINI_API_KEY` from WebConfig
- `update_receipt`
- `delete_receipt`
- `assign_receipt_to_trip`
- `verify_receipts`

**Path Normalization Strategy:**
Desktop stores absolute paths like `C:\Users\...\receipts\2024\file.jpg`. Web needs relative paths.

```rust
/// Convert absolute desktop path to web-relative path
fn normalize_receipt_path(absolute_path: &str, web_receipts_dir: &Path) -> String {
    // Extract filename or relative portion
    // E.g., "C:\Users\...\receipts\2024\img.jpg" → "2024/img.jpg"
    Path::new(absolute_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| absolute_path.to_string())
}

/// When returning receipts, normalize paths for web
pub async fn get_receipts(...) -> Json<Vec<Receipt>> {
    let mut receipts = db.get_receipts(...);
    for r in &mut receipts {
        r.file_path = normalize_receipt_path(&r.file_path, &config.receipts_path);
    }
    Json(receipts)
}
```

**Gemini API Key Handling:**
```rust
pub async fn sync_receipts(
    State(db): State<Database>,
    State(config): State<WebConfig>,
) -> Result<Json<SyncResult>, StatusCode> {
    let api_key = config.gemini_api_key
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Use api_key for Gemini client
}
```

**Receipt Image Serving:**
```rust
// Static file serving for receipt images
.nest_service("/receipts", ServeDir::new(&config.receipts_path))

// Or dynamic endpoint with proper content-type detection
pub async fn get_receipt_image(
    Path(filename): Path<String>,
    State(config): State<WebConfig>,
) -> impl IntoResponse {
    let path = config.receipts_path.join(&filename);
    let bytes = tokio::fs::read(&path).await?;
    let content_type = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    ([(header::CONTENT_TYPE, content_type)], bytes)
}
```

**Verification:** Can view receipt images via browser at `/receipts/{filename}`

---

## Task 6: Migrate Frontend API Layer

**Files:**
- Modify: `src/lib/api.ts`

**Steps:**
1. Remove Tauri import:
   ```typescript
   // DELETE: import { invoke } from '@tauri-apps/api/core';
   ```

2. Add fetch-based implementation:
   ```typescript
   const API_BASE = import.meta.env.VITE_API_URL || '/api';

   async function api<T>(endpoint: string, options?: RequestInit): Promise<T> {
       const res = await fetch(`${API_BASE}${endpoint}`, {
           headers: { 'Content-Type': 'application/json' },
           ...options,
       });
       if (!res.ok) {
           const error = await res.text();
           throw new Error(`API error ${res.status}: ${error}`);
       }
       return res.json();
   }

   // For endpoints that return no content
   async function apiVoid(endpoint: string, options?: RequestInit): Promise<void> {
       const res = await fetch(`${API_BASE}${endpoint}`, {
           headers: { 'Content-Type': 'application/json' },
           ...options,
       });
       if (!res.ok) {
           const error = await res.text();
           throw new Error(`API error ${res.status}: ${error}`);
       }
   }
   ```

3. Convert each function (example conversions):
   ```typescript
   // Vehicles
   export async function getVehicles(): Promise<Vehicle[]> {
       return api('/vehicles');
   }

   export async function createVehicle(...): Promise<Vehicle> {
       return api('/vehicles', {
           method: 'POST',
           body: JSON.stringify({ ... }),
       });
   }

   // Trips
   export async function getTripsForYear(vehicleId: string, year: number): Promise<Trip[]> {
       return api(`/trips/year/${year}?vehicleId=${vehicleId}`);
   }

   // Grid data
   export async function getTripGridData(...): Promise<TripGridData> {
       return api(`/grid-data?vehicleId=${vehicleId}&year=${year}&...`);
   }
   ```

**Verification:** TypeScript compiles without errors

---

## Task 7: Remove Tauri-Specific Frontend Code

**Files to search and modify** (search for `@tauri-apps`):
- `src/routes/+layout.svelte`
- `src/routes/doklady/+page.svelte`
- Any other files with Tauri imports

**Steps:**
1. **Search all Tauri imports:**
   ```bash
   grep -r "@tauri-apps" src/
   ```

2. In `+layout.svelte`, remove:
   ```typescript
   // DELETE these:
   import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

   // DELETE all window management code:
   // - checkWindowSize()
   // - restoreSize()
   // - win.onResized()
   // - win.setSize()
   // - win.center()
   ```

3. In `doklady/+page.svelte`:
   ```typescript
   // DELETE:
   import { openPath } from '@tauri-apps/plugin-opener';
   import { appDataDir } from '@tauri-apps/api/path';
   import { listen, type UnlistenFn } from '@tauri-apps/api/event';

   // REPLACE openPath(filePath) with:
   window.open(`/receipts/${encodeURIComponent(filename)}`, '_blank');

   // REPLACE appDataDir() usage:
   // Not needed in web - receipts served from /receipts/

   // REPLACE listen<T>() for progress:
   // Option A: Remove real-time progress (simplest for MVP)
   // Option B: Implement polling
   // Option C: Add SSE endpoint (future enhancement)
   ```

4. **Real-time progress decision (MVP):**
   Remove real-time progress for MVP. Receipt sync shows loading state, refreshes on completion.
   ```typescript
   // Simple approach - just show loading and refresh
   async function syncReceipts() {
       syncing = true;
       try {
           const result = await api('/receipts/sync', { method: 'POST' });
           // Refresh receipt list
           await loadReceipts();
       } finally {
           syncing = false;
       }
   }
   ```

**Verification:** `npm run build` succeeds, no Tauri runtime errors in browser console

---

## Task 8: Create Docker Configuration

**Files:**
- Create: `Dockerfile.web`
- Create: `docker-compose.web.yml`
- Create: `.dockerignore`

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
   ```dockerfile
   # Stage 1: Build Rust backend
   FROM rust:1.75 as rust-builder
   WORKDIR /app

   # Copy Cargo files for dependency caching
   COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./
   RUN mkdir src && echo "fn main() {}" > src/main.rs
   RUN cargo build --release 2>/dev/null || true

   # Copy actual source and build
   COPY src-tauri/src ./src
   COPY src-tauri/migrations ./migrations

   # Diesel offline mode - create schema cache
   ENV DATABASE_URL=sqlite:///tmp/build.db
   RUN cargo install diesel_cli --no-default-features --features sqlite
   RUN diesel setup && diesel migration run

   RUN cargo build --release --bin web

   # Stage 2: Build SvelteKit frontend
   FROM node:20-alpine as node-builder
   WORKDIR /app
   COPY package*.json ./
   RUN npm ci
   COPY . .
   ENV VITE_API_URL=/api
   RUN npm run build

   # Stage 3: Final image
   FROM debian:bookworm-slim
   RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

   # Copy binaries
   COPY --from=rust-builder /app/target/release/web /usr/local/bin/kniha-jazd
   COPY --from=node-builder /app/build /var/www/html

   # Create data directories
   RUN mkdir -p /data/receipts /data/backups

   # Default environment
   ENV DATABASE_PATH=/data/kniha-jazd.db
   ENV RECEIPTS_PATH=/data/receipts
   ENV BACKUPS_PATH=/data/backups
   ENV STATIC_DIR=/var/www/html
   ENV PORT=8080

   VOLUME /data
   EXPOSE 8080

   # Health check
   HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
       CMD curl -f http://localhost:8080/health || exit 1

   CMD ["kniha-jazd"]
   ```

3. Create `docker-compose.web.yml`:
   ```yaml
   version: '3.8'
   services:
     kniha-jazd:
       build:
         context: .
         dockerfile: Dockerfile.web
       ports:
         - "8080:8080"
       volumes:
         - ./data:/data
       environment:
         - DATABASE_PATH=/data/kniha-jazd.db
         - RECEIPTS_PATH=/data/receipts
         - BACKUPS_PATH=/data/backups
         # Uncomment and set for receipt OCR:
         # - GEMINI_API_KEY=your-api-key
       restart: unless-stopped
   ```

**Note:** First container start will auto-run Diesel migrations on the SQLite database.

**Verification:** `docker-compose -f docker-compose.web.yml build` succeeds

---

## Task 9: Static File Serving & Health Check in Axum

**Files:**
- Modify: `src-tauri/src/bin/web.rs`
- Modify: `src-tauri/src/web/router.rs`

**Steps:**
1. Add health check endpoint:
   ```rust
   .route("/health", get(|| async { "ok" }))
   ```

2. Add tower-http ServeDir for frontend and receipts:
   ```rust
   use tower_http::services::ServeDir;
   use tower_http::cors::{CorsLayer, Any};

   pub fn create_router(db: Database, config: WebConfig) -> Router {
       let api_routes = Router::new()
           // ... all /api routes
           .with_state((db, config.clone()));

       // CORS for development (restrict in production)
       let cors = CorsLayer::new()
           .allow_origin(Any)
           .allow_methods(Any)
           .allow_headers(Any);

       Router::new()
           .route("/health", get(|| async { "ok" }))
           .nest("/api", api_routes)
           .nest_service("/receipts", ServeDir::new(&config.receipts_path))
           .fallback_service(ServeDir::new(&config.static_dir))
           .layer(cors)
   }
   ```

**Security Note:** For production behind reverse proxy, restrict CORS or disable entirely.

**Verification:**
- `curl http://localhost:8080/health` returns "ok"
- Browser can load frontend at http://localhost:8080
- API calls work from frontend

---

## Task 10: Testing & Documentation

**Files:**
- Create: `tests/web/` directory for API tests
- Create: `docs/web-deployment.md`
- Modify: Existing integration tests (optional adaptation)

**Steps:**

### A. API Tests (New)
Create simple API tests using `reqwest` or `axum::test`:
```rust
// tests/web/api_tests.rs
#[tokio::test]
async fn test_get_vehicles() {
    let app = create_test_app();
    let response = app.get("/api/vehicles").await;
    assert_eq!(response.status(), 200);
}
```

### B. Integration Tests Decision
Options:
1. **Keep existing WebdriverIO tests** - They test via browser, should work with web version
2. **Create separate API test suite** - Pure HTTP tests without browser
3. **Adapt existing tests** - Change invoke() mocking to HTTP

**Recommendation:** Keep WebdriverIO tests as-is for now. They should work once frontend is migrated.

### C. Documentation
Create `docs/web-deployment.md`:
```markdown
# Web Deployment Guide

## Quick Start

1. Build Docker image:
   ```bash
   docker-compose -f docker-compose.web.yml build
   ```

2. Create data directory:
   ```bash
   mkdir -p ./data/receipts ./data/backups
   ```

3. (Optional) Migrate from desktop:
   ```bash
   cp "%APPDATA%/com.notavailable.kniha-jazd/kniha-jazd.db" ./data/
   cp -r "%APPDATA%/com.notavailable.kniha-jazd/receipts/" ./data/receipts/
   ```

4. Start:
   ```bash
   docker-compose -f docker-compose.web.yml up -d
   ```

5. Access: http://localhost:8080

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| DATABASE_PATH | /data/kniha-jazd.db | SQLite database location |
| RECEIPTS_PATH | /data/receipts | Receipt images directory |
| BACKUPS_PATH | /data/backups | Backup files directory |
| GEMINI_API_KEY | (none) | API key for receipt OCR |
| PORT | 8080 | Server port |

## Security

No authentication is included. For remote access:
- Use VPN, or
- Add nginx reverse proxy with basic auth
```

**Verification:**
- `docker-compose -f docker-compose.web.yml up` runs successfully
- Can access app at http://localhost:8080
- Can create vehicles and trips
- Data persists after container restart

---

## Command Coverage Summary

| Category | Commands | Count |
|----------|----------|-------|
| Vehicles | get_vehicles, get_active_vehicle, create_vehicle, update_vehicle, delete_vehicle, set_active_vehicle | 6 |
| Trips | get_trips, get_trips_for_year, get_years_with_trips, create_trip, update_trip, delete_trip, reorder_trip | 7 |
| Calculations | get_trip_grid_data, calculate_trip_stats, preview_trip_calculation, get_compensation_suggestion | 4 |
| Routes | get_routes, get_purposes | 2 |
| Settings | get_settings, save_settings | 2 |
| Backups | create_backup, list_backups, get_backup_info, restore_backup, delete_backup | 5 |
| Export | export_html, ~~export_to_browser~~ (merged) | 1 |
| Receipts | get_receipt_settings, get_receipts, get_receipts_for_vehicle, get_unassigned_receipts, scan_receipts, sync_receipts, process_pending_receipts, reprocess_receipt, update_receipt, delete_receipt, assign_receipt_to_trip, verify_receipts | 12 |
| **Skip** | get_optimal_window_size (desktop-only) | -1 |
| **Total** | | **39** |

---

## Post-Implementation Checklist

- [ ] All 108 Rust backend tests pass (`cargo test`)
- [ ] Web binary compiles (`cargo build --bin web`)
- [ ] Frontend builds without Tauri dependencies (`npm run build`)
- [ ] Docker image builds successfully
- [ ] Health check endpoint responds
- [ ] Can migrate existing desktop database
- [ ] All CRUD operations work via browser
- [ ] Receipt images display correctly (path normalization works)
- [ ] Export functionality returns HTML for download
- [ ] Backup/restore works with Docker volume
- [ ] Data persists across container restarts
- [ ] Gemini API receipt OCR works (when key configured)
