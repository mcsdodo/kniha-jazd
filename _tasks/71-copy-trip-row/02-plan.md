**Date:** 2026-09-03
**Subject:** Implementation plan — copy button on trip rows
**Status:** Planning

# Copy Trip Row Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Add a copy button to every trip row that opens a new, editable row prefilled with that trip's route, distance, purpose and time-of-day, dated today.

**Architecture:** A new Rust command `get_copied_trip_defaults(trip_id, year)` owns the two business rules — resolving the target date against the viewed year, and transferring the source trip's time-of-day (including its day offset for overnight trips). The rule itself is a pure function in [calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs) with no DB and no clock; a thin `_internal` wrapper supplies both. The frontend seeds a new `TripRow` from the response and renders it in the existing top new-row slot. Per ADR-008 the frontend performs no date arithmetic.

**Tech Stack:** Rust (chrono, diesel), Tauri IPC + server-mode dispatcher, SvelteKit + TypeScript, typesafe-i18n, WebdriverIO.

**Design doc:** [01-task.md](./01-task.md)

---

## Background for the implementer

Read these before starting — they explain constraints that are not obvious from the code:

- **ADR-008** (in [DECISIONS.md](../../DECISIONS.md)): all business logic lives in Rust. The frontend is display-only. Do not compute the target date in TypeScript.
- **[Task 56](../_done/56-smart-trip-defaults/)** added time inference: on a new row, once origin and destination are both filled, the backend returns a *jittered* start/end pair which overwrites the fields. A copied row must suppress this or the copied times get randomised away. The suppression mechanism already exists — `inferredKey` in [TripRow.svelte:171](../../src/lib/components/TripRow.svelte) is a guard that makes `tryInferTimes()` skip a route pair it has already handled. Pre-setting it is the whole fix.
- **[Task 65](../_done/65-datetime-is-order/)** made `start_datetime` the sole source of trip order. There is no `sort_order` column and no insertion-position parameter. A copied row simply saves and lands chronologically.
- **Adding a command touches 4 files**, because the app ships in two modes (desktop Tauri and server). Missing the dispatcher arm means the feature silently 404s in server mode without a compile error. Task 3 covers all of them.

**Repo command note:** always use `--manifest-path`, never `cd &&`:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core
```

---

## Task 1: Pure date/time rule + `CopiedTripDefaults` model

**Files:**
- Create: [src-tauri/core/src/calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs)
- Modify: [src-tauri/core/src/calculations/mod.rs:158](../../src-tauri/core/src/calculations/mod.rs) (add `pub mod trip_copy;`)
- Modify: [src-tauri/core/src/models.rs:323](../../src-tauri/core/src/models.rs) (add struct after `InferredTripTime`)

This mirrors [calculations/time_inference.rs](../../src-tauri/core/src/calculations/time_inference.rs) exactly: pure function plus an inline `#[cfg(test)] mod tests`. Tests live in the same file, not in `commands_tests.rs`.

**Step 1: Write the failing tests**

Create [src-tauri/core/src/calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs) with the tests only (no implementation yet):

```rust
//! Defaults for a copied trip row: resolves the target date against the year
//! the grid is showing, and transfers the source trip's time-of-day onto it.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Trip;
    use chrono::{NaiveDate, NaiveDateTime, Utc};
    use uuid::Uuid;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    /// A source trip carrying values in every field the copy must NOT take,
    /// so the exclusion assertions have something to catch.
    fn make_source(start: NaiveDateTime, end: Option<NaiveDateTime>) -> Trip {
        let now = Utc::now();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            start_datetime: start,
            end_datetime: end,
            origin: "Bratislava".to_string(),
            destination: "Trnava".to_string(),
            distance_km: 47.0,
            odometer: 10_000.0,
            purpose: "služobná cesta".to_string(),
            fuel_liters: Some(40.0),
            fuel_cost_eur: Some(60.0),
            full_tank: true,
            energy_kwh: Some(12.0),
            energy_cost_eur: Some(4.0),
            full_charge: true,
            soc_override_percent: Some(80.0),
            other_costs_eur: Some(9.0),
            other_costs_note: Some("parkovné".to_string()),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn target_date_is_today_when_viewed_year_is_current() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        assert_eq!(resolve_copy_target_date(2026, today), today);
    }

    #[test]
    fn target_date_is_dec_31_when_viewing_a_past_year() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        assert_eq!(
            resolve_copy_target_date(2025, today),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            "a past year's grid must receive its latest day"
        );
    }

    #[test]
    fn target_date_is_jan_1_when_viewing_a_future_year() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        assert_eq!(
            resolve_copy_target_date(2027, today),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
            "a future year's grid must receive its earliest day"
        );
    }

    #[test]
    fn transfers_time_of_day_onto_the_target_date() {
        let source = make_source(dt(2026, 3, 20, 8, 30), Some(dt(2026, 3, 20, 9, 15)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.start_datetime, "2026-09-03T08:30:00");
        assert_eq!(result.end_datetime.unwrap(), "2026-09-03T09:15:00");
    }

    #[test]
    fn overnight_source_keeps_its_day_offset() {
        // 22:00 → 02:00 next day. Without carrying the +1 day offset the copy
        // would end four hours BEFORE it starts.
        let source = make_source(dt(2026, 3, 20, 22, 0), Some(dt(2026, 3, 21, 2, 0)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.start_datetime, "2026-09-03T22:00:00");
        assert_eq!(result.end_datetime.unwrap(), "2026-09-04T02:00:00");
    }

    #[test]
    fn null_source_end_stays_null() {
        let source = make_source(dt(2026, 3, 20, 8, 30), None);
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.end_datetime, None);
    }

    #[test]
    fn copies_the_route_fields_verbatim() {
        let source = make_source(dt(2026, 3, 20, 8, 30), Some(dt(2026, 3, 20, 9, 15)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2026, today);

        assert_eq!(result.origin, "Bratislava");
        assert_eq!(result.destination, "Trnava");
        assert_eq!(result.distance_km, 47.0);
        assert_eq!(result.purpose, "služobná cesta");
    }

    #[test]
    fn clamping_applies_to_the_end_datetime_too() {
        // Overnight trip copied into a past year: the +1 day offset pushes the
        // end into the following year. Documents the accepted behaviour —
        // start stays inside the viewed year, which is what the grid needs.
        let source = make_source(dt(2026, 3, 20, 22, 0), Some(dt(2026, 3, 21, 2, 0)));
        let today = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();

        let result = compute_copied_trip_defaults(&source, 2025, today);

        assert_eq!(result.start_datetime, "2025-12-31T22:00:00");
        assert_eq!(result.end_datetime.unwrap(), "2026-01-01T02:00:00");
    }
}
```

**Step 2: Run the tests to verify they fail**

Register the module first, or the file is never compiled. In [src-tauri/core/src/calculations/mod.rs](../../src-tauri/core/src/calculations/mod.rs), after line 158 (`pub mod time_inference;`) add:

```rust
pub mod trip_copy;
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core trip_copy
```

Expected: **compile error** — `cannot find function 'resolve_copy_target_date' in this scope`, `cannot find function 'compute_copied_trip_defaults' in this scope`, `cannot find type 'CopiedTripDefaults'`. A compile failure is the failing-test state here.

**Step 3: Add the model struct**

In [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs), immediately after the `InferredTripTime` struct (ends line 323), add:

```rust
/// Field set used to seed a copied trip row. Deliberately excludes fuel,
/// energy, cost and note fields — a fill-up is a one-off event, not a
/// property of a route, and copying `fuel_liters` would corrupt the
/// consumption rate and the 20 % margin calculation. The struct shape *is*
/// that guarantee: do not widen it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopiedTripDefaults {
    pub start_datetime: String,         // ISO "YYYY-MM-DDTHH:MM:SS"
    pub end_datetime: Option<String>,   // ISO, or None if the source had none
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
}
```

**Step 4: Write the implementation**

At the top of [src-tauri/core/src/calculations/trip_copy.rs](../../src-tauri/core/src/calculations/trip_copy.rs), above the `#[cfg(test)] mod tests` block, add:

```rust
use crate::models::{CopiedTripDefaults, Trip};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

/// Resolve the date a copied row should carry, given the year the grid is
/// currently showing.
///
/// The row must land inside the visible grid, so "today" is clamped into
/// `year`: a past year gets its last day, a future year its first.
pub fn resolve_copy_target_date(year: i32, today: NaiveDate) -> NaiveDate {
    use std::cmp::Ordering;
    match today.year().cmp(&year) {
        Ordering::Equal => today,
        Ordering::Greater => NaiveDate::from_ymd_opt(year, 12, 31)
            .expect("31 Dec is valid in every supported year"),
        Ordering::Less => NaiveDate::from_ymd_opt(year, 1, 1)
            .expect("1 Jan is valid in every supported year"),
    }
}

/// Build the seed values for a row copied from `source`.
///
/// Only the time-of-day travels from the source, never its date. The end
/// datetime additionally carries the source's day span, so an overnight trip
/// stays overnight instead of collapsing into a negative duration.
pub fn compute_copied_trip_defaults(
    source: &Trip,
    year: i32,
    today: NaiveDate,
) -> CopiedTripDefaults {
    let target_date = resolve_copy_target_date(year, today);
    let start = NaiveDateTime::new(target_date, source.start_datetime.time());

    let end = source.end_datetime.map(|src_end| {
        let day_offset = (src_end.date() - source.start_datetime.date()).num_days();
        NaiveDateTime::new(
            target_date + Duration::days(day_offset),
            src_end.time(),
        )
    });

    CopiedTripDefaults {
        start_datetime: start.format("%Y-%m-%dT%H:%M:%S").to_string(),
        end_datetime: end.map(|e| e.format("%Y-%m-%dT%H:%M:%S").to_string()),
        origin: source.origin.clone(),
        destination: source.destination.clone(),
        distance_km: source.distance_km,
        purpose: source.purpose.clone(),
    }
}
```

**Step 5: Run the tests to verify they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core trip_copy
```

Expected: **8 passed**.

**Step 6: Commit**

```bash
git add src-tauri/core/src/calculations/trip_copy.rs src-tauri/core/src/calculations/mod.rs src-tauri/core/src/models.rs
git commit -m "feat(trips): add pure copy-defaults rule for trip rows"
```

---

## Task 2: `_internal` wrapper

**Files:**
- Modify: [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs) (append after `get_inferred_trip_time_for_route_internal`, ~line 218)
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) (append inside the same test module as the inference tests, ~line 5323)

The wrapper is where the two impure dependencies live: the DB read and the clock. Everything else was proven in Task 1. The only branch worth a test here is the not-found path, which the frontend surfaces as a toast.

**Step 1: Write the failing test**

In [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs), inside the same `mod` that holds `test_db_with_completed_trip` (immediately before its closing brace at ~line 5323), add:

```rust
    #[test]
    fn copy_defaults_errors_when_trip_is_missing() {
        use crate::commands_internal::trips::get_copied_trip_defaults_internal;
        let (db, _vehicle_id) = test_db_with_completed_trip();

        let result = get_copied_trip_defaults_internal(
            &db,
            Uuid::new_v4().to_string(),
            2026,
        );

        assert!(result.is_err(), "an unknown trip id must not silently succeed");
    }
```

**Step 2: Run it to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core copy_defaults_errors_when_trip_is_missing
```

Expected: **compile error** — `cannot find function 'get_copied_trip_defaults_internal'`.

**Step 3: Write the implementation**

In [src-tauri/core/src/commands_internal/trips.rs](../../src-tauri/core/src/commands_internal/trips.rs), extend the imports at the top of the file:

```rust
use crate::calculations::trip_copy::compute_copied_trip_defaults;
use crate::models::{CopiedTripDefaults, InferredTripTime, Route, Trip};
use chrono::{Local, NaiveDate, Utc};
```

(The existing lines to replace are `use crate::models::{InferredTripTime, Route, Trip};` at line 8 and `use chrono::{NaiveDate, Utc};` at line 10.)

Then append at the end of the file:

```rust
/// Seed values for a row copied from an existing trip.
///
/// Thin wrapper around [`compute_copied_trip_defaults`]: supplies the DB read
/// and the clock so the rule itself stays pure and unit-testable. `Local` (not
/// `Utc`) is deliberate — "today" must mean the user's calendar day, matching
/// how `defaultNewDate` is derived in the grid.
pub fn get_copied_trip_defaults_internal(
    db: &Database,
    trip_id: String,
    year: i32,
) -> Result<CopiedTripDefaults, String> {
    let source = db
        .get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", trip_id))?;

    Ok(compute_copied_trip_defaults(
        &source,
        year,
        Local::now().date_naive(),
    ))
}
```

No change is needed in `commands_internal/mod.rs` — line 16 is already `pub use trips::*;`.

**Step 4: Run the test to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core copy_defaults
```

Expected: **1 passed**.

**Step 5: Commit**

```bash
git add src-tauri/core/src/commands_internal/trips.rs src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "feat(trips): add get_copied_trip_defaults_internal command"
```

---

## Task 3: Expose the command in both delivery modes

**Files:**
- Modify: [src-tauri/core/src/server/dispatcher.rs:252-271](../../src-tauri/core/src/server/dispatcher.rs) (add arm after the `get_inferred_trip_time_for_route` arm)
- Modify: [src-tauri/desktop/src/commands/trips.rs:157](../../src-tauri/desktop/src/commands/trips.rs) (append after `get_inferred_trip_time_for_route`)
- Modify: [src-tauri/desktop/src/lib.rs:236](../../src-tauri/desktop/src/lib.rs) (add to `invoke_handler`)

No unit test — this is pure plumbing with no branches. Task 8's integration test exercises it end-to-end. The compiler catches the desktop side; the dispatcher arm is the one that fails *silently* in server mode, so do not skip it.

**Step 1: Add the server-mode dispatch arm**

In [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs), directly after the closing `}` of the `"get_inferred_trip_time_for_route"` arm (line 271), add:

```rust
        "get_copied_trip_defaults" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_copied_trip_defaults_internal(
                &state.db,
                a.trip_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
```

**Step 2: Add the Tauri command**

In [src-tauri/desktop/src/commands/trips.rs](../../src-tauri/desktop/src/commands/trips.rs), append at the end of the file:

```rust
#[tauri::command]
pub fn get_copied_trip_defaults(
    db: State<Arc<Database>>,
    trip_id: String,
    year: i32,
) -> Result<CopiedTripDefaults, String> {
    inner::get_copied_trip_defaults_internal(&db, trip_id, year)
}
```

Extend that file's `use` of the core models to include `CopiedTripDefaults` (it already imports `InferredTripTime` and `Route`).

**Step 3: Register it**

In [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs), after line 236 (`commands::get_inferred_trip_time_for_route,`) add:

```rust
            commands::get_copied_trip_defaults,
```

**Step 4: Verify both crates compile**

```bash
cargo check --manifest-path src-tauri/Cargo.toml --workspace
```

Expected: **Finished** with no errors.

**Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs src-tauri/desktop/src/commands/trips.rs src-tauri/desktop/src/lib.rs
git commit -m "feat(trips): expose get_copied_trip_defaults to desktop and server modes"
```

---

## Task 4: Frontend type + API wrapper

**Files:**
- Modify: [src/lib/types.ts:141](../../src/lib/types.ts) (add after `InferredTripTime`)
- Modify: [src/lib/api.ts](../../src/lib/api.ts) — line 4 (import) and line 481 (append after `getInferredTripTimeForRoute`)

**Step 1: Add the type**

In [src/lib/types.ts](../../src/lib/types.ts), after the `InferredTripTime` interface (closes line 141), add:

```ts
/** Seed values for a row copied from an existing trip. Route fields only —
 *  fuel, energy, costs and notes are deliberately absent (see Task 71). */
export interface CopiedTripDefaults {
	startDatetime: string;        // ISO "YYYY-MM-DDTHH:MM:SS"
	endDatetime: string | null;   // ISO, or null if the source had no end
	origin: string;
	destination: string;
	distanceKm: number;
	purpose: string;
}
```

**Step 2: Add the API wrapper**

In [src/lib/api.ts](../../src/lib/api.ts), add `CopiedTripDefaults` to the `import type { ... }` list on line 4, then after `getInferredTripTimeForRoute` (closes line 481) add:

```ts
export async function getCopiedTripDefaults(
	tripId: string, year: number
): Promise<CopiedTripDefaults> {
	return await apiCall('get_copied_trip_defaults', { tripId, year });
}
```

**Step 3: Verify types**

```bash
npm run check
```

Expected: no new errors. (Pre-existing warnings in unrelated files are fine — compare against `git stash` output if unsure.)

**Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(trips): add getCopiedTripDefaults API wrapper"
```

---

## Task 5: i18n strings

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts) — lines 159 and 554
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) — lines 159 and 554

**Step 1: Add the keys**

In [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), after line 159 (`insertAbove: 'Vložiť záznam nad',`):

```ts
		copyRecord: 'Kopírovať záznam',
```

and after line 554 (`errorCreateTrip: ...`):

```ts
		errorCopyTrip: 'Nepodarilo sa načítať údaje na kopírovanie',
```

In [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts), at the matching lines:

```ts
		copyRecord: 'Copy record',
```

```ts
		errorCopyTrip: 'Failed to load copy data',
```

**Step 2: Regenerate the i18n types**

This is **required** and easy to forget. Nothing else regenerates [i18n-types.ts](../../src/lib/i18n/i18n-types.ts) outside vite watch mode, so `npm run check` reports phantom errors until it runs:

```bash
npm run i18n
```

**Step 3: Verify**

```bash
npm run check
```

Expected: no errors referring to `copyRecord` or `errorCopyTrip`.

**Step 4: Commit**

```bash
git add src/lib/i18n/
git commit -m "feat(i18n): add copy record strings"
```

---

## Task 6: Copy button and prefill in `TripRow`

**Files:**
- Modify: [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) — props (~line 60), init block (after line 171), actions markup (line 728), CSS (after line 903)

**Step 1: Add the props**

After line 62 (`export let onInsertAbove: () => void = () => {};`), add:

```ts
	// Copy (Task 71) - duplicates this row's route into a new today-dated row
	export let onCopy: () => void = () => {};
	export let copyDisabled: boolean = false;
	// Set on a NEW row that was opened via another row's copy button. Seeds
	// formData below; null for an ordinary new row.
	export let copyFrom: CopiedTripDefaults | null = null;
```

Add `CopiedTripDefaults` to the `import type { ... }` on line 2.

**Step 2: Seed `formData` from `copyFrom`**

Place this **immediately after** line 171 (`let inferredKey = '';`) — it must come after that declaration, because it writes to it:

```ts
	// Task 71: seed a copied row. Applied as an override AFTER the base
	// formData init above, so the fuel/energy/cost defaults there still hold —
	// those fields are deliberately not copied.
	if (copyFrom) {
		formData.startDatetime = copyFrom.startDatetime.slice(0, 16);
		formData.endDatetime = (copyFrom.endDatetime ?? copyFrom.startDatetime).slice(0, 16);
		formData.origin = copyFrom.origin;
		formData.destination = copyFrom.destination;
		formData.distanceKm = copyFrom.distanceKm;
		formData.odometer = previousOdometer + copyFrom.distanceKm;
		formData.purpose = copyFrom.purpose;
		// The copied times are explicit user intent. Marking this route pair as
		// already-inferred makes tryInferTimes() short-circuit, so the Task 56
		// jitter never overwrites them. (tryAutoFillDistance needs no guard —
		// it only fires when distanceKm is null, which it no longer is.)
		inferredKey = `${copyFrom.origin}␟${copyFrom.destination}`;
		// Populate the live consumption/zostatok preview, matching what
		// tryAutoFillDistance does when it auto-fills km.
		onPreviewRequest(copyFrom.distanceKm, null, formData.fullTank);
	}
```

**Step 3: Add the button to the actions cell**

In the non-editing actions cell, between the `insert` button (closes line 738) and the `{#if $capabilities.features.routeMaps}` block (line 739), add:

```svelte
				<button
					class="icon-btn copy"
					on:click|stopPropagation={onCopy}
					disabled={copyDisabled}
					title={$LL.trips.copyRecord()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
						<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
					</svg>
				</button>
```

The synthetic "Prvý záznam" row renders its own `<td class="col-actions"></td>` and never mounts `TripRow`, so it is excluded automatically. `.icon-btn:disabled` styling already exists at line 931.

**Step 4: Add the hover colour**

After the `.icon-btn.insert:hover` rule (closes line 901):

```css
	.icon-btn.copy:hover {
		color: var(--accent-primary);
	}
```

**Step 5: Verify it compiles**

```bash
npm run check
```

Expected: no new errors.

**Step 6: Commit**

```bash
git add src/lib/components/TripRow.svelte
git commit -m "feat(trips): add copy button and copy prefill to TripRow"
```

---

## Task 7: Wire the copy flow in `TripGrid`

**Files:**
- Modify: [src/lib/components/TripGrid.svelte](../../src/lib/components/TripGrid.svelte) — import (line 4), state (~line 135), handler (~line 226), clears (lines 253 and 323), top new-row props (~line 650), existing-row props (~line 764)

**Step 1: Import the API + type**

Add `getCopiedTripDefaults` to the `$lib/api` import on line 4, and `CopiedTripDefaults` to the `$lib/types` import on line 3.

**Step 2: Add state and handler**

After line 135 (`let insertDate: string | null = null;`):

```ts
	// Copy source defaults (Task 71) - non-null only while a copied new row is open
	let copyDefaults: CopiedTripDefaults | null = null;
```

After `handleNewRecord` (closes line 226):

```ts
	async function handleCopy(trip: Trip) {
		try {
			// Fetch BEFORE opening the row: TripRow seeds formData at init, so
			// copyDefaults must already be set when the component mounts.
			copyDefaults = await getCopiedTripDefaults(trip.id, year);
			insertAtTripId = null;
			insertDate = null;
			showNewRow = true;
		} catch (error) {
			console.error('Failed to load copy defaults:', error);
			toast.error($LL.toast.errorCopyTrip());
		}
	}
```

**Step 3: Clear it on save and cancel**

In `handleSaveNew`, next to `insertDate = null;` (line 255), and in `handleCancelNew`, next to `insertDate = null;` (line 325), add to both:

```ts
			copyDefaults = null;
```

**Step 4: Pass it to the top new row**

In the `{#if showNewRow && insertAtTripId === null}` block, replace line 650:

```svelte
						defaultDate={defaultNewDate}
```

with:

```svelte
						defaultDate={copyDefaults ? copyDefaults.startDatetime.slice(0, 10) : defaultNewDate}
						copyFrom={copyDefaults}
```

**Step 5: Pass the button props to existing rows**

In the non-first-record `<TripRow>`, after line 764 (`onInsertAbove={() => handleInsertAbove(trip)}`):

```svelte
							onCopy={() => handleCopy(trip)}
							copyDisabled={showNewRow}
```

**Step 6: Verify**

```bash
npm run check
```

Expected: no new errors.

**Step 7: Commit**

```bash
git add src/lib/components/TripGrid.svelte
git commit -m "feat(trips): wire copy button to new-row prefill in TripGrid"
```

---

## Task 8: Integration test

**Files:**
- Create: [tests/integration/specs/tier2/copy-trip.spec.ts](../../tests/integration/specs/tier2/copy-trip.spec.ts)

Covers the UI flow only. Does **not** re-test the date rule or the day-offset rule — Task 1 owns those, and duplicating them here would violate the project's no-duplication testing strategy.

**Step 1: Build the debug binary** (required before any integration run)

```bash
npm run test:integration:build
```

**Step 2: Write the spec**

```ts
/**
 * Tier 2: Copy Trip Row Integration Tests (Task 71)
 *
 * Covers the UI flow only:
 * - Copy opens a new row in edit mode with the route fields prefilled
 * - Fuel fields are NOT prefilled
 * - The row saves with a recalculated ODO
 * - The button is disabled while a new row is open
 *
 * The date-resolution and day-offset rules are backend-owned and exhaustively
 * covered in src-tauri/core/src/calculations/trip_copy.rs — do not retest here.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';

describe('Tier 2: Copy Trip Row', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    const vehicle = await seedVehicle({
      name: 'Copy Test Vehicle',
      licensePlate: 'COPY001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id;
    await setActiveVehicle(vehicleId);

    // Source trip, dated earlier this year so "today" is the newest date.
    const year = new Date().getFullYear();
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-01-15T08:30`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 47,
      odometer: 50047,
      purpose: 'Client visit',
    });

    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);
  });

  it('should prefill the route fields and leave fuel empty', async () => {
    const copyBtn = await $('.icon-btn.copy');
    expect(await copyBtn.isExisting()).toBe(true);
    await copyBtn.click();
    await browser.pause(500);

    const editingRow = await $('tr.editing');
    expect(await editingRow.isExisting()).toBe(true);

    expect(await (await editingRow.$('.col-origin input')).getValue()).toBe('Bratislava');
    expect(await (await editingRow.$('.col-destination input')).getValue()).toBe('Trnava');
    expect(await (await editingRow.$('.col-km input')).getValue()).toBe('47');
    expect(await (await editingRow.$('.col-purpose input')).getValue()).toBe('Client visit');

    // Start time carries over from the source row (08:30).
    const start = await (await editingRow.$('.col-start-datetime input')).getValue();
    expect(start).toContain('08:30');

    // Fuel is a one-off event and must NOT be copied.
    const fuel = await editingRow.$('.col-fuel-liters input');
    expect(await fuel.getValue()).toBe('');
  });

  it('should disable the copy button while a new row is open', async () => {
    await (await $('.icon-btn.copy')).click();
    await browser.pause(500);

    const copyButtons = await $$('.icon-btn.copy');
    for (const btn of copyButtons) {
      expect(await btn.isEnabled()).toBe(false);
    }
  });

  it('should save the copied row with a recalculated ODO', async () => {
    await (await $('.icon-btn.copy')).click();
    await browser.pause(500);

    const editingRow = await $('tr.editing');

    // Fine-tune the distance before applying.
    const kmInput = await editingRow.$('.col-km input');
    await kmInput.setValue('60');
    await browser.pause(300);

    await (await editingRow.$('.icon-btn.save')).click();
    await browser.pause(1000);

    // 50047 (previous ODO) + 60 = 50107
    const rows = await $$('tbody tr');
    const odoTexts = await Promise.all(
      rows.map(async (r) => {
        const cell = await r.$('.col-odo');
        return (await cell.isExisting()) ? cell.getText() : '';
      })
    );
    expect(odoTexts.some((t) => t.includes('50107'))).toBe(true);
  });
});
```

**Step 3: Run this spec only**

Do **not** run the full suite here — it takes ~10 minutes, a single spec under a minute:

```bash
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/copy-trip.spec.ts
```

Expected: **3 passing**.

If a selector does not resolve, check the actual column classes in the editing row markup of [TripRow.svelte](../../src/lib/components/TripRow.svelte) and adjust — the class names above are taken from the non-editing cells and the editing cells may nest the input differently.

**Step 4: Commit**

```bash
git add tests/integration/specs/tier2/copy-trip.spec.ts
git commit -m "test(trips): add integration spec for copy trip row"
```

---

## Task 9: Documentation

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) (`[Unreleased]` section)
- Modify: [DECISIONS.md](../../DECISIONS.md) (new BIZ entry)

**Step 1: Changelog**

Invoke the `/changelog` skill and add under `[Unreleased] → Added`:

```markdown
- **Kopírovanie záznamu** — tlačidlo na každom riadku vytvorí nový záznam s dnešným dátumom, rovnakou trasou, km, účelom a časom. Palivo a náklady sa nekopírujú. Nový riadok sa otvorí v režime úprav.
```

**Step 2: Decision entry**

Invoke the `/decision` skill and record a **BIZ** entry for the year-clamping rule:

> **Rule:** a copied trip row is dated today when the grid shows the current year; when the grid shows a past year it is dated 31 December of that year, and a future year 1 January.
>
> **Why:** the copied row must remain visible in the grid the user is looking at. Using the literal current date while viewing 2025 would save the trip into 2026, where it vanishes from view with no feedback.

**Step 3: Verify the whole change set**

Now run the full suites — this is the final verification, so the ~10-minute integration sweep is warranted:

```bash
npm run test:backend
npm run test:integration
```

Expected: all passing.

**Step 4: Commit**

```bash
git add CHANGELOG.md DECISIONS.md
git commit -m "docs: changelog and decision entry for copy trip row"
```

**Step 5: Mark the task complete**

Update [../index.md](../index.md): change task 71's status from 📋 Planning to ✅ Complete and move the row to Completed Tasks, then move this folder to `_tasks/_done/71-copy-trip-row/` (see [_done/](../_done/)) and fix the index link path. Update this file's and [01-task.md](./01-task.md)'s `**Status:**` headers to `Complete`.

---

## Definition of done

- [ ] `compute_copied_trip_defaults` covered for: current/past/future year, time-of-day transfer, overnight day offset, null end, route fields, end-crossing-year
- [ ] `get_copied_trip_defaults` reachable in **both** desktop and server mode
- [ ] Copy button visible on every real trip row, absent on "Prvý záznam", disabled while a new row is open
- [ ] Copied row opens in edit mode with route fields filled and fuel/energy/cost fields empty
- [ ] Copied times survive — Task 56 inference does not overwrite them
- [ ] ODO shows `previousOdometer + km` and updates when km is edited
- [ ] `npm run i18n` has been run; `npm run check` clean
- [ ] `npm run test:backend` and `npm run test:integration` pass
- [ ] CHANGELOG and DECISIONS updated
