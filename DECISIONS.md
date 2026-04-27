# Decisions Log

Architecture Decision Records (ADRs) and business logic decisions. **Newest first.**

---

## 2026-04-27: Default-OFF for Route-Based Time Inference

### BIZ-014: Opt-In Auto-Fill of Trip Start/End Times

**Context:** Version 0.33.0 introduced silent auto-fill of new-row start/end datetimes from the most recent matching route (with ±15 min / ±15% jitter; see [calculations/time_inference.rs](./src-tauri/core/src/calculations/time_inference.rs)). The feature is technically correct but UX-hostile: the user types `startDatetime` and `endDatetime`, then picks origin and destination, and their typed values are silently overwritten. There was no indication that this was intentional and no escape hatch — even users who knew about the feature could not opt out short of code changes.

**Decision:** Make `infer_trip_times: Option<bool>` an opt-in setting on [LocalSettings](./src-tauri/core/src/settings.rs) that defaults to OFF (`None` and `Some(false)` both mean disabled). When enabled, surface every inference with a 6-second toast that includes a `Vrátiť` ("Undo") button restoring the pre-inference values for that single row and clearing the row's `inferredKey` so the user can deliberately re-trigger inference if they change their mind.

**Alternatives considered:**
- *Default ON with a discovery toast.* Preserves prior behavior for existing users while adding an in-app way to learn about the feature. Rejected because the very first inference still surprises the user — the no-surprise principle wins over discoverability for an action that overwrites typed input.
- *Default ON without any toast.* The 0.33.0 status quo. Rejected as user-hostile.
- *Remove the feature entirely.* Rejected — users who legitimately repeat the same routes find auto-fill valuable; an opt-in toggle keeps the value while removing the surprise.

**Trade-offs accepted:**
- Existing users who relied on the auto-fill lose it silently after upgrade. Mitigation: prominent [CHANGELOG](./CHANGELOG.md) entry and the in-app discovery path via the toast (visible the first time they enable the toggle).

**Implementation note:** The gate lives at the public command boundary (`get_inferred_trip_time_for_route_internal` in [commands_internal/trips.rs](./src-tauri/core/src/commands_internal/trips.rs)), not inside the pure helpers `compute_inferred_times` / `inferred_trip_time_for_route`. ADR-008 (frontend calculation duplication) and ADR-014 (jitter stays in Rust) are preserved: the calculation core stays a pure function (testable with deterministic jitter); the user setting is read at the orchestration layer.

**Related:** [Task 59](./_tasks/59-time-inference-toggle/), original feature in [v0.33.0 changelog entry](./CHANGELOG.md).

---

## 2026-04-26: Cargo Workspace Split for Tauri/Web Boundary

### ADR-018: Workspace Members Over Feature Flags

**Context:** The headless [`web` binary](./src-tauri/web/src/main.rs) lived in the same crate as the Tauri desktop app, so Cargo linked the entire transitive Tauri/GTK/WebKit dependency graph into the binary even though it never called any Tauri API. The Docker runtime image therefore had to ship ~150 MB of GUI runtime libraries that were never used. Two solutions were on the table: (a) feature-gate `tauri` behind a `desktop` Cargo feature (`#[cfg(feature = "desktop")]` on every wrapper), or (b) split [`src-tauri/`](./src-tauri/) into a workspace with separate crates ([`core/`](./src-tauri/core/), [`desktop/`](./src-tauri/desktop/), [`web/`](./src-tauri/web/)).

**Decision:** Workspace split (option b). [`kniha-jazd-core`](./src-tauri/core/Cargo.toml) is a pure library with no Tauri deps; [`kniha-jazd-desktop`](./src-tauri/desktop/Cargo.toml) holds the Tauri shell + thin `#[tauri::command]` wrappers; [`kniha-jazd-web`](./src-tauri/web/Cargo.toml) depends only on core. Boundary enforced by Cargo's per-crate dep graph, not by `#[cfg]` discipline.

**Reasoning:**
- The `wrapper → _internal` pattern from [Task 55 Server Mode](./_tasks/_done/55-server-mode/) was already screaming for a crate boundary — every `_internal` function was framework-free, every wrapper was Tauri-only.
- Workspace split is **self-enforcing**: a future contributor cannot accidentally couple core code to Tauri because the dep does not exist in [`core/Cargo.toml`](./src-tauri/core/Cargo.toml). With feature flags, that discipline lives in `#[cfg(feature = "desktop")]` annotations on ~74 wrapper functions — easy to forget, easy to break.
- Calendar cost was roughly equal (~3 days for either option).
- Side benefit: two binaries that need separate version metadata, separate publishing cadence, and separate CI build steps line up naturally with two crate manifests.

**Trade-offs accepted:**
- Three Cargo manifests instead of one — slightly more boilerplate when adding new deps (decide which crate gets it).
- Desktop wrappers became thin delegators — extra layer of indirection for any `#[tauri::command]`.
- Migration was mechanical but touched ~30 files in 27 commits.

**Result:** Web binary's dep graph (`cargo tree -p kniha-jazd-web`) contains zero Tauri packages. [Dockerfile.web](./Dockerfile.web) drops GTK/WebKit runtime libs (~150 MB savings, image goes from ~300 MB to ~80 MB target). All 280 backend tests preserved across the move.

**Related:** [Task 58](./_tasks/58-tauri-workspace-split/) (implementation), [Tech Debt #06](./_tasks/_TECH_DEBT/06-tauri-feature-gating.md) (origin).

---

## 2026-04-23: Server Mode Architecture

### ADR-017: LAN-Only CORS Without Authentication

**Context:** The embedded HTTP server exposes the full app API on the local network. Should it require authentication (password, token, etc.)?

**Decision:** No authentication. CORS allowlist restricts origins to RFC 1918 private IP ranges (`10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`) and `localhost`. Any request from a non-LAN origin is blocked by the browser's CORS preflight.

**Reasoning:**
- Target environment is a home or small office LAN — all devices on the network are trusted
- Adding authentication would require password management UI, token storage, and login flow — significant complexity for minimal benefit
- CORS enforcement happens in the browser, which is the only client (no curl/API use case)
- If the user's LAN is compromised, authentication wouldn't help much anyway (attacker could sniff traffic on unencrypted HTTP)
- Same trust model as other LAN devices (printers, NAS, smart home)

**Trade-offs:**
- Anyone on the same LAN can access the app without a password
- No protection against malicious devices on the network (accepted risk for simplicity)

---

### ADR-016: _internal Extraction Pattern for Command Reuse

**Context:** Tauri commands take `tauri::State<Database>` wrappers injected by the framework. The Axum RPC dispatcher has `Arc<Database>` directly. How should both call paths share the same business logic?

**Decision:** Extract pure `_internal` functions from each Tauri command. These take `&Database` and/or `&AppState` as plain references. The Tauri `#[command]` wrapper extracts from `State<>`, the RPC dispatcher passes `&state.db` directly. Both call the same `_internal` function.

**Pattern:**
```
Tauri command (thin wrapper) ──→ _internal(db, args) ←── RPC dispatcher
```

**Reasoning:**
- Zero behavior change — existing tests verify the `_internal` functions work correctly
- No new traits or abstractions needed — just function extraction
- Tauri wrappers become trivially thin (extract state, call internal, return)
- Clean separation: framework concerns (State extraction) vs business logic (pure functions)
- 68 out of 72 commands extracted; 4 remain Tauri-only (file dialogs, DB replacement)

**Rejected alternatives:**
- *Trait-based abstraction* — over-engineered for what is a simple call delegation
- *Separate REST routes* — would require maintaining a parallel API surface (see ADR-015)

---

### ADR-015: RPC Over REST for Server Mode API

**Context:** The embedded HTTP server needs to expose the same 68 commands that Tauri IPC provides. Should we create individual REST endpoints (`GET /api/vehicles`, `POST /api/trips`, etc.) or use a single RPC endpoint?

**Decision:** Single `POST /api/rpc` endpoint accepting `{ "command": "get_vehicles", "args": { ... } }` JSON. The dispatcher maps command names to `_internal` functions.

**Reasoning:**
- Mirrors the Tauri IPC model exactly — `invoke("command", args)` maps 1:1 to `POST /api/rpc` with `{ command, args }`
- No need to design, document, or version 68 separate REST routes
- Frontend adapter is trivial: swap `invoke()` for `fetch('/api/rpc')` based on runtime detection
- Adding new commands requires zero HTTP routing changes — just register in the dispatcher
- Not a public API — only consumed by the same frontend code, so REST conventions (proper HTTP methods, status codes per resource) add no value

**Trade-offs:**
- Not RESTful — all operations are POST, no resource-based URLs
- No HTTP caching (all POST) — acceptable for a LAN app with local-speed responses
- Error responses are always 400 with a string message — no structured error codes

---

## 2026-04-15: Time Inference for New Trip Rows

### ADR-014: Jitter Stays in Rust; Testability via `Jitter` Trait

**Context:** Task 56 introduces auto-fill of start/end datetimes on new trip rows from the most recent matching `(vehicle_id, origin, destination)` trip. To prevent machine-identical timestamps across days, the inferred start is jittered by ±15 minutes and duration by ±15 %. The question was where the jitter should live: Rust backend (consistent with ADR-008) or Svelte frontend (where non-determinism is "easier to test" by injecting a mock random fn).

**Decision:** All inference logic — DB lookup, base-time extraction, **and** the random jitter — lives in the Rust backend. The Tauri command `get_inferred_trip_time_for_route` returns the *final* ISO start/end strings; the frontend writes them directly without any computation.

**Testability pattern:** A `Jitter` trait abstracts the source of randomness:

```rust
pub trait Jitter {
    fn minutes(&mut self) -> i64;        // [-15, 15]
    fn duration_factor(&mut self) -> f64; // [0.85, 1.15]
}
pub struct ThreadRngJitter;     // production: rand::thread_rng
struct StubJitter { /* test */ } // tests: deterministic returns
```

Unit tests (4 in `time_inference.rs`, 4 in `commands_tests.rs`) supply a `StubJitter` so assertions are exact. Production code constructs `ThreadRngJitter` inside the thin `#[tauri::command]` wrapper and calls the same pure helper.

**Reasoning:**
- ADR-008 protects against having calculation logic in two places. Jitter that produces values written into trip records *is* business logic — same category as consumption rates, not the same category as `toFixed()` formatting.
- The trait split keeps tests pure (no `rand::thread_rng()` calls in test code) without requiring randomness to cross the Tauri boundary.
- Future requirement changes (e.g., "use ±10 min instead of ±15") become a one-line change in one place.

**Rejected alternatives:**
- *Frontend jitter (initially proposed)* — would have meant a value-producing computation in Svelte, breaking ADR-008. Rejected during design review.
- *Eager seeding inside `compute_inferred_times`* — would have hard-coded `rand::thread_rng()` and made tests non-deterministic.

---

## 2026-02-12: HA Sensor Display Conversion

### ADR-013: HA Sensor Percentage-to-Liters Conversion Lives in Frontend

**Context:** The new HA real fuel level feature fetches a percentage (0-100%) from a Home Assistant sensor and needs to convert it to liters (`value × tankSize / 100`) for display on the zostatok line. ADR-008 requires all business logic calculations in the Rust backend only.

**Decision:** This conversion stays in the Svelte frontend as display formatting.

**Reasoning:**
- ADR-008 protects against **duplicating calculation logic** (consumption rates, margins, zostatok from trip data). This conversion transforms an external HA sensor reading for display only.
- The backend never uses this value for any calculation — it calculates zostatok independently from trip/fillup data.
- Same category as `toFixed()` or `toLocaleString()` — formatting an external value for display.
- No duplication risk: the HA fuel level and the computed zostatok are independent data sources shown side by side.

---

## 2026-01-29: No Backward Compatibility for Older App Versions

### ADR-012: Forward-Only Database Migration Strategy

**Context:** When adding new database columns or changing schemas, we previously considered maintaining backward compatibility so older app versions could still read databases modified by newer versions.

**Decision:** We are **NOT** enforcing backward compatibility for older app versions reading newer databases.

**What this means:**
- Older app versions may fail to read databases migrated by newer versions
- We don't need to keep legacy columns populated (e.g., `end_time` alongside `end_datetime`)
- Migration strategy is forward-only: users must upgrade the app to use migrated databases
- Code should not include "backward compat" workarounds for legacy fields

**What we DO maintain:**
- Data integrity during migrations (no data loss)
- Clean upgrade path (migrations run automatically on app start)
- Backup creation before migrations (existing behavior)

**Reasoning:**
- Simplifies code by removing legacy field sync logic
- Single-user desktop app - no need for multi-version DB access
- Auto-update ensures users get latest version quickly
- Reduces maintenance burden of dual-column strategies

**Impact on CLAUDE.md:** The database migration guidelines about "older app versions must be able to READ data" should be removed or updated to reflect this decision.

---

## 2026-01-29: Commands Module Split

### ADR-011: Split commands.rs into Feature Modules

**Context:** `commands.rs` has grown to 3,908 lines with 68 Tauri commands. While internally organized with section comments, the file size makes navigation and maintenance difficult.

**Decision:** Split into 9 feature-based modules under `src-tauri/src/commands/`:

| Module | Lines | Commands | Purpose |
|--------|-------|----------|---------|
| `common.rs` | ~180 | 0 | Shared helpers, macros (`check_read_only!`), types |
| `vehicles.rs` | ~130 | 5 | Vehicle CRUD |
| `trips.rs` | ~220 | 8 | Trip CRUD, routes, year-start helpers |
| `statistics.rs` | ~1,170 | 3 | Grid data, calculations, magic fill |
| `backup.rs` | ~400 | 11 | Backup/restore operations |
| `export.rs` | ~280 | 2 | HTML export |
| `receipts.rs` | ~710 | 8 | Receipt scanning, assignment |
| `settings.rs` | ~310 | 15 | Theme, columns, DB location |
| `integrations.rs` | ~180 | 8 | Home Assistant, Gemini API |

**Key decisions:**
- `statistics.rs` exports 3 public helpers for use by `export.rs`: `calculate_period_rates()`, `calculate_fuel_remaining()`, `calculate_fuel_consumed()`
- Year-start helpers (`get_year_start_*`) live in `trips.rs` but are `pub(crate)` for statistics/export
- Tests remain in `commands_tests.rs` initially (can split later)
- `lib.rs` invoke_handler imports from submodules

**Phased approach:**
1. Extract low-risk: `common`, `vehicles`, `backup`
2. Extract complex: `statistics`, `export`, `trips`
3. Extract integrations: `receipts`, `settings`, `integrations`

**Reasoning:**
- Reduces cognitive load when editing a specific feature
- Clearer module boundaries and dependencies
- Enables parallel development on different features
- No functional changes - pure refactoring

---

## 2026-01-12: Additional Costs Recognition

### BIZ-013: Other Cost Invoice Recognition and Assignment

**Context:** Users want to scan and assign non-fuel receipts (car wash, parking, service, etc.) to trips, similar to existing fuel receipt workflow.

**Options considered:**
1. New `ReceiptType` enum with categories (Fuel, CarWash, Parking, Toll, Service, Other)
2. Separate `CostInvoice` table parallel to `Receipt`
3. Binary classification using existing `liters` field (null = other cost)

**Decision:** Use multi-stage matching for classification.

- **Fuel receipt**: `liters != null` AND trip exists where `date + liters + price` match
- **Other cost receipt**: `liters == null` OR no matching trip found

**Why multi-stage:** A receipt for windshield washer fluid (2L / 5€) has liters but isn't fuel. Since no trip has "2L fuel for 5€", it won't match and becomes "other cost" automatically.

**Additional decisions:**
- **Single cost per trip:** One "other cost" invoice per trip. Assignment blocked if `other_costs_eur` already populated.
- **No type categories:** User writes description manually in `other_costs_note` field.
- **Same folder:** All receipts (fuel + other) in same folder, AI auto-classifies.
- **Minimal schema change:** Only 2 new columns: `vendor_name`, `cost_description`.

**Reasoning:**
- Simplest implementation (~6h vs ~13h for enum approach)
- No new enums or types to maintain
- Existing `liters` field already indicates receipt type
- Backward compatible - existing fuel receipts unchanged
- User already has freedom to write any description in note field

**Trade-offs:**
- Cannot filter by specific cost type (parking vs car wash) - only fuel vs other
- User accepted this limitation in favor of simplicity

---

## 2026-01-05: Fuel Carryover

### BIZ-012: Year-End Fuel Carryover Between Years

**Context:** ADR-009 originally specified "zostatok starts fresh (full tank assumption)" for each new year. However, this didn't reflect reality - fuel doesn't magically reset on January 1st.

**Previous behavior:** Each year started with full tank assumption, ignoring actual fuel state from December 31st.

**Decision:** Fuel (zostatok) now carries over from the previous year's ending state.

**Implementation:**
- `get_year_start_zostatok()` calculates carryover from previous year's last trip
- If no previous year data exists, falls back to full tank assumption
- This also prepares for EV support where battery SoC carries over between years

**Reasoning:**
- Matches real-world behavior (fuel doesn't reset on Jan 1)
- Provides accurate consumption tracking across year boundaries
- Enables proper EV battery state tracking (future feature)

**Note:** This supersedes the "zostatok starts fresh" part of ADR-009. The ODO carryover behavior from ADR-009 remains unchanged.

---

## 2025-12-30: Receipt Organization

### ADR-010: Receipt Year Filtering

**Context:** Users may organize receipts in different folder structures - either flat (all files in one folder) or year-based (files in YYYY subfolders like `2024/`, `2025/`). The app needs to handle both cases and filter receipts by year while maintaining clear behavior.

**Decision:**
- **Flat mode:** Files directly in receipts folder → shown in all years (no year filtering)
- **Year-based mode:** Files in YYYY subfolders (e.g., `2024/`) → filtered by selected year
- **Invalid structure:** Mixed content (files + folders) or non-year folders → warning shown, files not loaded
- **Year determination priority:**
  1. Primary: Use `receipt_date.year()` from OCR recognition
  2. Fallback: Use `source_year` from folder name (for unprocessed receipts)
- **Mismatch warning:** When folder year differs from OCR-detected receipt date year, show indicator to user

**Reasoning:**
- Users have different organizational preferences; supporting both flat and year-based is flexible
- OCR date is more accurate than folder placement (user may misfile receipts)
- Folder year serves as fallback for new/unprocessed receipts before OCR runs
- Warning on mismatch helps users identify misfiled receipts without blocking workflow

---

## 2025-12-25: Year Picker

### ADR-009: Year-Scoped Vehicle Logbook

**Context:** Each year is a standalone "kniha jázd" for legal purposes.

**Decision:**
- Year picker in header next to vehicle dropdown
- Stats and trips scoped to selected year
- App starts on current calendar year
- Export only shows years with actual data
- ODO carries over from previous year, zostatok starts fresh (full tank assumption)

**Reasoning:** Slovak legal requirements treat each year as independent logbook. Fresh zostatok per year simplifies accounting.

---

## 2025-12-25: Architecture Refactor

### ADR-008: Remove Frontend Calculation Duplication

**Context:** Frontend (`src/lib/calculations.ts`) duplicated Rust backend calculations (`src-tauri/src/calculations.rs`) "for instant UI responsiveness."

**Problem:**
- ~500 lines of duplicate code
- 21 frontend tests duplicating 41 backend tests
- Risk of logic divergence between frontend and backend
- Double maintenance burden

**Options considered:**
1. Keep duplication - test both implementations
2. Move all to Rust - frontend calls Tauri commands
3. Move all to frontend - backend becomes thin data layer

**Decision:** Move all calculations to Rust backend only.

**Reasoning:**
- Tauri IPC is local and fast (microseconds, not network)
- No other clients will ever exist - single desktop app
- Rust backend already has 41 well-tested calculation functions
- Single source of truth eliminates divergence risk
- Frontend becomes simpler display-only logic

**Implementation:** Add `get_trip_grid_data` Tauri command returning pre-calculated values.

---

## 2025-12-23: UI/UX Decisions

### ADR-007: Database Backup/Restore

**Context:** User needs ability to backup and restore database for data safety.

**Decision:**
- Backups stored in `{app_data_dir}/backups/`
- Manual trigger only (no auto-backup)
- Filename: `kniha-jazd-backup-YYYY-MM-DD-HHmmss.db`
- Restore: Full DB replacement with confirmation showing date, counts, warning
- Keep all backups (no auto-deletion)

**Reasoning:** Simple, transparent backup system. User controls when to backup/restore.

---

### ADR-006: Navigation Header

**Context:** Settings button was buried at bottom of page, requiring scroll.

**Decision:** Top header bar with "Kniha jázd | Nastavenia" navigation links.

**Reasoning:** Always visible, no scrolling needed, clear app structure.

---

### ADR-005: Totals Section Redesign

**Context:** Original single-row totals were cramped and unclear.

**Decision:**
- Two-row layout for totals
- Rename "Km" to "Celkovo najazdené" for clarity
- Show fuel totals and cost summary on separate row

**Reasoning:** Better readability, clearer labels for legal documentation.

---

## 2025-12-23: Calculation Logic Fixes

### BIZ-011: Legal Limit Based on Average Consumption

**Context:** Should the 20% over-limit warning use the last fill-up rate or overall average?

**Decision:** Use **average consumption** (total_fuel / total_km × 100) for legal compliance check.

**Reasoning:** Legal compliance is about the overall picture, not a single fill-up. If average is 6.00 and limit is 6.12 (5.1 × 1.2), we're compliant even if one fill-up was higher.

---

### BIZ-010: Retroactive Consumption Rate Application

**Context:** When a fill-up occurs, which trips should use that rate?

**Decision:** Apply the rate **retroactively** to ALL trips since the previous fill-up.

**Example:** If trips A, B, C happen, then fill-up on C gives rate 6.0 → A, B, and C all show 6.0 l/100km.

**Reasoning:** Matches Excel behavior. The rate represents the consumption for that entire period.

---

### BIZ-009: Same-Day Trip Ordering

**Context:** Multiple trips on the same date need deterministic ordering for correct calculations.

**Decision:** Sort by date, then by **odometer** as tiebreaker.

**Reasoning:** Odometer is sequential and represents actual trip order. Using created_at would fail for imported data.

---

### BIZ-008: ODO Auto-Calculation

**Context:** Manual ODO entry is error-prone and redundant since ODO = previous ODO + km driven.

**Decision:** Auto-calculate ODO when km is entered: `ODO = previousODO + km`. User can still manually override.

**Reasoning:** Reduces data entry errors, matches Excel workflow where this was a formula.

---

## 2024-12-23: Business Logic Decisions

### BIZ-007: Fill-up Detection

**Context:** How to distinguish regular trips from fill-ups?

**Decision:** Auto-detect. If liters field is filled → it's a fill-up. No separate entry types.

**Reasoning:** Simpler UX, matches Excel behavior.

---

### BIZ-006: UI Display Order vs Export Order

**Context:** How to show trips in UI vs PDF export?

**Decision:**
- UI: Newest trips on top (reverse chronological) - easier access
- Export: Oldest first (chronological) - matches Excel/legal format

---

### BIZ-005: Route Distance Memory

**Context:** User often drives same routes.

**Decision:** Store origin→destination pairs with their distances. When user selects a known route, auto-fill the km field.

**Reasoning:** Reduces data entry, fewer errors.

---

### BIZ-004: Compensation Trip Suggestions

**Context:** How to help user plan trips to stay within legal margin?

**Decision:**
1. Calculate km needed to bring margin under limit
2. First, try to find existing route from current location matching needed km (±10%)
3. Fallback: Suggest buffer trip with configurable purpose (e.g., "služobná cesta")
4. Target margin: 16-19% (provides safety buffer below 20% limit)

**Reasoning:** Maintaining a buffer below the 20% limit helps ensure compliance even with measurement variations.

---

### BIZ-003: Legal Margin Limit

**Context:** What's the allowed over-consumption?

**Decision:** Max 20% over the vehicle's TP (technical passport) consumption rate.

**Example:** TP = 5.1 l/100km → Max allowed = 6.12 l/100km

---

### BIZ-002: Pouzita Spotreba (Used Consumption Rate)

**Context:** What rate is used to calculate fuel consumption between fill-ups?

**Decision:**
- Initial value: TP rate from vehicle (e.g., 5.1 l/100km)
- After first fill-up: Use the calculated l/100km from that fill-up
- Rate carries forward until next fill-up recalculates it

**Validation:** Matches Excel pattern - each fill-up sets the rate for subsequent trips.

---

### BIZ-001: Consumption Rate Calculation

**Context:** How is l/100km calculated?

**Decision:** On each fill-up: `l/100km = liters_filled / km_since_last_fillup × 100`

**Validation:** Verified against Excel data - formula matches exactly.

---

## 2024-12-23: Architecture Decisions

### ADR-004: Code in English, UI in Slovak

**Context:** User is Slovak, app is for Slovak legal requirements.

**Decision:**
- All code, variables, comments: English
- UI text: Slovak with i18n support for future translation

**Reasoning:**
- English code is industry standard, easier to maintain
- Slovak UI serves the primary user
- i18n-ready for potential future users

---

### ADR-003: Test-Driven Development

**Context:** Need reliable calculations for legal compliance (20% margin rule).

**Decision:** TDD with focus on business logic tests only

**Reasoning:**
- Calculation errors = legal compliance issues
- Tests must be meaningful, not filler
- Focus: consumption calculations, margin checks, compensation suggestions
- Skip: trivial CRUD, UI rendering, getters/setters

---

### ADR-002: SQLite for Local Storage

**Context:** Need to store trips, vehicles, and calculated data.

**Decision:** SQLite (single local file)

**Reasoning:**
- Simple, portable, robust
- Single file easy to backup/move
- Can still export to Excel/CSV for accountants
- No server needed for personal logbook

---

### ADR-001: Desktop App with Tauri + SvelteKit

**Context:** Need to build a vehicle logbook app to replace Excel spreadsheet.

**Options considered:**
1. Electron + React/Vue - Cross-platform, larger bundle (~150MB+)
2. Tauri + SvelteKit - Cross-platform, Rust backend, small bundle (~10-20MB)
3. Python + PyQt - Good for data apps, simpler
4. C# WPF - Windows-only, excellent Excel interop
5. .NET MAUI + Blazor - Cross-platform, C# everywhere

**Decision:** Tauri + SvelteKit

**Reasoning:**
- User said "don't limit ourselves" - open to learning Rust
- Best end-user experience (small, fast, native)
- Svelte is the simplest modern frontend framework
- No need for Excel interop - reimplementing functionality, not integrating
