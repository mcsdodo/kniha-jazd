# Invoice-to-Trip Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Each task should be independently testable and committable.

**Goal:** Let the user create a trip from an uploaded fuel invoice in one click, pre-filling everything derivable from OCR + app history, so mid-trip fillup splits stop being tedious.

**Architecture:** All business logic in Rust (ADR-008). Frontend calls a single `prepare_trip_from_receipt(receipt_id)` command that returns a trip draft; user reviews in a dialog; save calls existing `create_trip` plus new `link_receipt_to_trip` and `save_station_alias`.

---

## Task 1: Auto-fill origin from previous trip's destination (standalone UX win)

**Rationale:** Ship independently first — it benefits every trip row, and it's a prerequisite for the invoice-to-trip flow using the same data source.

**Files:**
- Modify: `src-tauri/src/db.rs` — add `find_previous_trip_destination(vehicle_id) -> Option<String>` (most recent trip by `start_datetime DESC`, return `destination` or `None`).
- Modify: `src-tauri/src/commands/trips.rs` — add `get_previous_trip_destination` Tauri command wrapping the db function.
- Modify: `src/lib/components/TripRow.svelte` (or wherever new rows are instantiated — likely `TripGrid.svelte`) — when a new unsaved row is added, invoke the command and pre-fill `origin` **only if** origin is empty and the row is new.

**Steps:**
1. Write Rust unit test: `find_previous_trip_destination` returns the destination of the most recent trip for the given vehicle; returns `None` if no trips exist; ignores other vehicles' trips.
2. Implement the db function to pass the test.
3. Write Tauri command wrapper + smoke test.
4. Write integration test (WebdriverIO): add a new trip row → verify `origin` field is pre-filled with the last trip's destination.
5. Wire the frontend: on new-row creation, call the command and set `origin` if empty.

**Verification:**
- `npm run test:backend` passes including new test.
- Integration test passes.
- Manual check: add a new trip row in the grid → origin appears pre-filled with previous trip's destination.

---

## Task 2: `fuel_station_aliases` table + Rust CRUD

**Files:**
- Create: `src-tauri/migrations/{next_number}_create_fuel_station_aliases.sql`
- Create: `src-tauri/src/fuel_station_aliases.rs` with CRUD functions.
- Modify: `src-tauri/src/lib.rs` — register module.

**Schema:**
```sql
CREATE TABLE fuel_station_aliases (
    id TEXT PRIMARY KEY,
    vehicle_id TEXT NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
    station_address TEXT NOT NULL,
    destination TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(vehicle_id, station_address)
);
CREATE INDEX idx_fuel_station_aliases_vehicle_address ON fuel_station_aliases(vehicle_id, station_address);
```

**Functions:**
- `find_alias(db, vehicle_id, station_address) -> Option<String>` — normalized lookup (same `normalize_location` util as routes).
- `upsert_alias(db, vehicle_id, station_address, destination) -> Result<()>` — insert or update `destination + updated_at`.

**Steps:**
1. Write migration SQL.
2. Write Rust unit tests: insert → find returns destination; upsert of existing pair updates destination and `updated_at`; different vehicle is isolated; normalization handles whitespace/case on `station_address` input.
3. Implement functions to pass tests.

**Verification:** `cd src-tauri && cargo test fuel_station_aliases` passes.

---

## Task 3: Backend `prepare_trip_from_receipt` command

**Files:**
- Create: `src-tauri/src/trip_drafts.rs` — pure logic (alias lookup, distance lookup, speed heuristic, jitter).
- Modify: `src-tauri/src/commands/receipts_cmd.rs` — add `prepare_trip_from_receipt(receipt_id) -> TripDraft` thin wrapper.
- Modify: `src-tauri/src/models.rs` — add `TripDraft` struct with all pre-fillable fields + status flags (`destination_needs_pick: bool`, `distance_needs_manual: bool`, `end_time_approximate: bool`).

**Pure function signature:**
```rust
pub fn build_trip_draft(
    receipt: &Receipt,
    previous_trip_destination: Option<&str>,
    previous_trip_purpose: Option<&str>,
    alias_lookup: Option<String>,              // destination already known for this station
    route_distance_lookup: Option<f64>,         // distance_km for (vehicle, origin, alias_destination)
    jitter: &mut dyn Jitter,                    // same trait as task 56
    clock: &dyn Clock,                          // for determinism in tests
) -> TripDraft;
```

**Logic:**
1. End datetime:
   - If receipt has time → `receipt_datetime + jitter.positive_minutes(0, 5)`.
   - If date-only → `{date}T12:00:00`, set `end_time_approximate = true`.
2. Destination:
   - If `alias_lookup.is_some()` → use it.
   - Else → `destination_needs_pick = true`, suggest `"{station_name}, {station_address}"` as placeholder.
3. Distance:
   - If `route_distance_lookup.is_some()` → use it.
   - Else → `distance_needs_manual = true`, distance_km = None.
4. Start datetime:
   - If distance known → compute via speed heuristic (100 km/h over 50 km, else 40 km/h).
   - Else → None (user will fill after entering distance).
5. Origin = `previous_trip_destination` (or None).
6. Purpose = `previous_trip_purpose` (or None).
7. Fuel fields from receipt (`liters`, `total_price_eur`).

**Steps:**
1. Define `TripDraft` struct + `Clock` trait (if not already present).
2. Write comprehensive unit tests for `build_trip_draft`:
   - Receipt with full datetime → end gets jittered (stub returns fixed value).
   - Receipt date-only → end at 12:00, `end_time_approximate = true`.
   - Alias hit → destination silent.
   - Alias miss → `destination_needs_pick = true`.
   - Route history hit (65 km) → start = end − 39 min (speed 100 km/h).
   - Route history hit (30 km) → start = end − 45 min (speed 40 km/h).
   - Route miss → start = None, `distance_needs_manual = true`.
   - No previous trip → origin/purpose None.
3. Implement `build_trip_draft` to pass tests.
4. Implement thin wrapper `prepare_trip_from_receipt` command: load receipt, find previous trip (reuse task 1's function), call alias + route lookups, construct real jitter/clock, delegate to pure function.
5. Write wrapper smoke test.

**Verification:** `cd src-tauri && cargo test trip_drafts` passes; all edge cases covered.

---

## Task 4: Frontend — receipt card button + pre-filled dialog

**Files:**
- Modify: receipt card component (likely `src/lib/components/ReceiptCard.svelte` or similar — locate in receipts panel).
- Create: `src/lib/components/TripFromInvoiceDialog.svelte` — new dialog component.
- Modify: `src/lib/i18n/sk.ts` (and `en.ts`) — add Slovak strings: "Vytvoriť jazdu z faktúry", picker labels, warnings.
- Add: new commands `link_receipt_to_trip` and `save_station_alias` in `src-tauri/src/commands/receipts_cmd.rs`.

**Steps:**
1. Add `link_receipt_to_trip(receipt_id, trip_id)` and `save_station_alias(vehicle_id, station_address, destination)` Tauri commands with Rust unit tests.
2. Add button to receipt card — visible only when receipt is FUEL, has assigned vehicle, and is not already linked to a trip.
3. Build `TripFromInvoiceDialog.svelte`:
   - On open: call `prepare_trip_from_receipt` → receive draft.
   - Render form with all fields (disabled where non-editable, warning badge if `end_time_approximate`).
   - If `destination_needs_pick`: show picker combining "use invoice value" + existing destinations from routes.
   - If `distance_needs_manual`: distance field empty and focused.
   - Save flow: call `create_trip` → call `link_receipt_to_trip` → if newly-picked destination, call `save_station_alias` → close dialog → refresh receipts panel and trip grid.

**Verification:** Manual UI test — upload a fuel invoice → click button → dialog opens pre-filled → save → trip appears in grid, receipt linked on receipt card.

---

## Task 5: Integration tests

**Files:**
- Create: `tests/integration/specs/invoice-to-trip.spec.ts` (WebdriverIO).

**Test scenarios (verify UI flow, NOT calculation math — already covered in backend tests):**
1. Upload fuel invoice → OCR completes → "Vytvoriť jazdu z faktúry" button visible.
2. Click button → dialog opens with origin pre-filled (from previous trip).
3. First-time station → destination picker is shown; pick value; save; verify trip is created AND alias persisted (second invoice from same station → destination silent).
4. Receipt already linked → button is hidden.

**Verification:** `npm run test:integration:tier1` includes new spec and passes.

---

## Task 6: Documentation + decisions + changelog

**Files:**
- Create: `docs/features/invoice-to-trip.md` — user flow + technical overview + design rationale (follow `docs/features/receipt-scanning.md` template).
- Modify: `DECISIONS.md` — add 3 BIZ entries:
  - Speed heuristic thresholds (100 km/h > 50 km, 40 km/h ≤ 50 km).
  - End datetime jitter window (0–5 minutes, positive only).
  - Station alias scope (per-vehicle, not global).
- Modify: `CHANGELOG.md` — `[Unreleased]` entry under "Added".

**Steps:**
1. Write feature doc.
2. Run `/decision` three times for the three BIZ decisions.
3. Run `/changelog` for the user-visible feature.
4. Run `/verify` before final commit.

**Verification:** `/verify` passes (tests, git clean, changelog current).

---

## Task ordering and dependencies

```
Task 1 (origin pre-fill)  ← ship first, standalone
        │
        ▼
Task 2 (alias table)  ←──┐
Task 3 (prepare command) ─┤
        │                 │
        ▼                 │
Task 4 (UI + wire-up) ────┘
        │
        ▼
Task 5 (integration tests)
        │
        ▼
Task 6 (docs + changelog + release)
```

Tasks 2 and 3 can be developed in parallel since they're independent modules; Task 4 depends on both.

---

## Out of scope (explicit non-goals)

- **Google Maps / external routing API** — rejected during brainstorming to keep app offline.
- **Editing aliases after creation** — v2; first use picks, subsequent uses are silent. Users can work around by editing the destination on the created trip if wrong.
- **Global (cross-vehicle) aliases** — rejected: different vehicles may have separate histories.
- **Automatic leg 2 creation (F → B)** — user manually creates leg 2. Leg 2 benefits from Task 1's origin pre-fill.
- **Purpose categorization / rule-based inference** — plain inheritance from previous trip is enough.
