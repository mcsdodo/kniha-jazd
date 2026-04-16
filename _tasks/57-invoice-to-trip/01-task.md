**Date:** 2026-04-16
**Subject:** Create trip from fuel invoice (mid-trip fillup split)
**Status:** Planning

## Goal

When a fuel invoice is uploaded and OCR'd, offer a one-click way to create the **first leg** of a split trip ending at the fuel station. The primary use case is **mid-trip fillups**: the user is driving A → B, stops for fuel at F, and must log the trip as two legs (A → F, F → B) so consumption calculations stay correct. Manually entering leg 1 today is tedious; the invoice already contains most of the data.

Additionally, as a first deliverable, introduce a small UX improvement that benefits all new trip rows (not only this feature): **auto-fill origin from the previous trip's destination.**

## Requirements

### 1. Auto-fill origin on new trip rows (first deliverable — standalone UX win)

- **Trigger:** new (unsaved) trip row gets added to the grid.
- **Logic:** pre-fill `origin` with the most recent saved trip's `destination` for the same vehicle.
- **Scope:** new rows only. Never overwrite user input or existing rows.
- **Fallback:** no previous trip exists → leave origin empty.
- **Benefits:** reduces typing for every trip the user enters, not only invoice-created ones. Makes leg 2 (manually entered after leg 1 is created from the invoice) essentially one click from the user's perspective.

### 2. "Create trip from invoice" action on receipt card

- **Trigger:** button appears on a receipt card once OCR is complete and the receipt is classified as FUEL with a known vehicle.
- **Label:** "Vytvoriť jazdu z faktúry".
- **Flow:** click opens a pre-filled trip form dialog (reviewable before save).

### 3. Pre-filled fields on the dialog

| Field | Source |
|---|---|
| Vehicle | Receipt's assigned vehicle |
| End datetime | `receipt_datetime` (with optional small positive jitter of 0–5 min — see Technical Notes) |
| Start datetime | Inferred: `end_datetime − distance/speed_heuristic` |
| Origin | Previous trip's destination (from requirement 1) |
| Destination | Station alias lookup (see requirement 4) |
| Distance km | Route history lookup; fallback to empty for manual entry |
| Purpose | Previous trip's purpose, editable |
| Fuel liters | OCR `liters` |
| Fuel cost | OCR `total_price_eur` |
| Full tank | Unchecked by default |
| Odometer | Empty — user fills from dashboard if they use odometer, otherwise they may use distance km |

### 4. Fuel station alias system

New concept that learns OCR station text → user's preferred destination string.

- **Per-vehicle table:** `fuel_station_aliases (id, vehicle_id, station_address, destination, created_at, updated_at)`.
- **Lookup:** on invoice processing, normalize `station_address` from OCR and look up an alias for this vehicle.
  - **Hit:** pre-fill destination silently with the stored string.
  - **Miss:** show destination picker combining (a) "use invoice value: `{station_name}, {station_address}`" and (b) dropdown of existing destinations for this vehicle from `routes`. User picks; selection is saved as a new alias.
- **Rationale:** OCR station strings rarely match the user's historical free-text destinations exactly, which would otherwise break route-history distance lookups. The alias table makes the system self-learning: first use at a station requires a pick; subsequent invoices from the same station are silent.

### 5. Distance resolution strategy

Priority order:

1. **Route history:** `SELECT distance_km FROM routes WHERE vehicle_id = ? AND origin = ? AND destination = ?` (destination after alias resolution). If found, pre-fill and display as read-only with an "edit" affordance.
2. **Manual entry:** field empty, user types distance_km (or fills odometer, same as existing trip form which derives distance from odometer delta).

No Google Maps integration — keeps the app offline-only and removes API key / billing concerns.

### 6. Start time inference (speed heuristic)

- **If distance > 50 km:** average speed = 100 km/h.
- **If distance ≤ 50 km:** average speed = 40 km/h.
- **Compute:** `duration_minutes = round(distance_km / avg_speed × 60)`.
- **Apply:** `start_datetime = end_datetime − duration_minutes`.
- **Fallback:** distance unknown → leave `start_datetime` empty (don't guess).

### 7. End datetime jitter

- End datetime = `receipt_datetime + uniform_int(0, 5) minutes` — acknowledges real-world delay between pump-off and invoice print, and avoids an exact match that looks machine-generated.
- Exact match (0 jitter) is still valid — the heuristic stays within a realistic window.
- Must be testable with a stub jitter source (same pattern as task 56 `Jitter` trait).

### 8. Save behaviour

On dialog save:

1. Create the trip (reuses `create_trip` command).
2. Automatically link the receipt to the created trip (`receipt.trip_id = new_trip.id`).
3. If a new station alias was chosen on this flow, insert it into `fuel_station_aliases`.

### 9. Edge cases

- **Receipt has date-only (no time):** `receipt_datetime` falls back to 12:00; inline warning on the dialog that the end time is approximate.
- **No previous trip exists (first ever trip for this vehicle):** origin is empty, user types.
- **No route history for (origin, destination):** distance field empty, user types.
- **Receipt is already linked to a trip:** button hidden — don't offer creation if it's already done.
- **Invoice currency is not EUR:** `total_price_eur` is the converted value stored on receipt; use it as-is.

## Technical Notes

- **ADR-008 compliance:** alias lookup, distance resolution, speed heuristic, and jitter all live in Rust. The frontend calls a single Tauri command `prepare_trip_from_receipt(receipt_id)` that returns a fully populated trip draft payload. Frontend renders; user reviews; submits via existing `create_trip` + new `link_receipt_to_trip` + optional `save_station_alias` commands.
- **Testability:** speed heuristic and jitter must be injectable. Follow task 56's `Jitter` trait pattern — split `prepare_trip_from_receipt` into a pure function taking a jitter/clock and a thin wrapper constructing real dependencies. Unit tests pass stubs for deterministic output.
- **Existing foundation:**
  - OCR already extracts `station_name`, `station_address`, `receipt_datetime`, `liters`, `total_price_eur` (see `src-tauri/src/gemini.rs` ExtractedReceipt struct).
  - `routes` table already indexes `(vehicle_id, origin, destination) → distance_km, usage_count, last_used` — no schema change needed for distance lookup.
  - `create_trip` command accepts all needed fields already (`commands/trips.rs`).
  - Receipts panel is the natural UI host; button goes on the receipt card (see `docs/features/receipt-scanning.md`).
- **New DB migration:** `fuel_station_aliases` table + index on `(vehicle_id, station_address)`.
- **Relationship to task 56:** task 56 adds ODO clamp + time inference on new rows; this task adds origin pre-fill on new rows and the full invoice-to-trip flow. The three features are complementary — together they make the trip row entry experience mostly automatic for common cases.
- **Decisions to record in DECISIONS.md (BIZ category):**
  - Speed heuristic thresholds (100 km/h over 50 km, 40 km/h otherwise).
  - Jitter window for end datetime (0–5 minutes positive).
  - Station alias scoped per-vehicle (not global).
- **Feature doc:** after completion, add `docs/features/invoice-to-trip.md` following existing format in `docs/features/receipt-scanning.md`.
