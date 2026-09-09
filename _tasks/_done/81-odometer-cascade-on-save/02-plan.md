# Odometer Cascade On Save Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Editing a trip's distance moves that row's odometer and shifts every later row of the same year by the same amount, after the user approves the shift in a modal.

**Architecture:** Three pure Rust functions plan the cascade for an edit, an insert and a delete; three commands apply them in a single transaction behind a `dryRun` flag; the grid calls the dry run first, shows the plan in a modal, and applies on OK. The row editor loses all of its odometer arithmetic, because the backend owns the anchor (ADR-008).

**Tech Stack:** Rust (kniha-jazd-core, diesel, SQLite), SvelteKit + TypeScript, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

## Global Constraints

- `anchor` is always the previous row's stored odometer in `trip_order`, or `yearStartOdometer` when there is no previous row in that year.
- **Edit:** km differs -> `odometer := anchor + km`; else odo differs -> `distanceKm := odometer - anchor`; else nothing moves. `delta = new_odometer - stored_odometer`.
- **Insert:** `odometer := anchor + km`, and `delta = km`.
- **Delete:** `delta = -(odometer[X] - anchor[X])`, the removed row's span, not its distance.
- In all three, every row after the affected position gets `odometer += delta`, and rows before it are never touched.
- The cascade never crosses a year boundary.
- A submitted `startDatetime` that differs from the stored one cascades nothing.
- Float comparison tolerance is `0.001`, the same value `recalculate_odometers_internal` uses ([trips.rs:229](../../../src-tauri/core/src/commands_internal/trips.rs)).
- `update_trip`, `create_trip`, `delete_trip` and `recalculate_odometers` keep their current behaviour. Do not change them.
- `yearEndOdometerMoved` says the year's last odometer changed. `nextYearChainBreaks` says that AND a later year has trips. Only the second opens a modal on its own -- appending to the newest year must stay a silent write.
- All business logic in Rust ([ADR-008](../../../DECISIONS.md)). The frontend displays what the backend returns.
- Slovak UI strings go through i18n. Run `npm run i18n` after editing `src/lib/i18n/{sk,en}/index.ts`.
- The task 79 span warnings must still fire. A change that silences them is wrong.
- Do not write to the production database. Test against a copy under `_tmp/`.
- Stage only the files of the task you are on. Never `git add -A`.

---

### Task 1: The pure cascade planner

**Files:**
- Modify: `src-tauri/core/src/models.rs` (add `CascadePlan` next to `OdometerChange` at `:905`)
- Modify: `src-tauri/core/src/commands_internal/trips.rs` (add `plan_odometer_cascade` after `recalculate_odometers_internal`)
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: `trip_order` and `OdometerChange`, both already exported.
- Produces:
  - `pub struct CascadePlan { new_odometer: f64, new_distance_km: f64, delta: f64, delta_from_distance: f64, delta_from_repair: f64, repair_crosses_year: bool, year_end_odometer_moved: bool, next_year_chain_breaks: bool, changes: Vec<OdometerChange> }`
  - `pub fn plan_odometer_cascade(trips: &[Trip], year_start_odometer: f64, trip_id: &str, submitted_distance_km: f64, submitted_odometer: f64) -> Result<CascadePlan, String>`
  - `pub fn plan_insert_cascade(trips: &[Trip], year_start_odometer: f64, new_start_datetime: NaiveDateTime, new_distance_km: f64) -> CascadePlan`
  - `pub fn plan_delete_cascade(trips: &[Trip], year_start_odometer: f64, trip_id: &str) -> Result<CascadePlan, String>`

- [ ] **Step 1: Write the failing tests**

Add to `commands_tests.rs`, after the `recalculate_odometers` tests. `setup_db_with_start_odometer` and `seed_chain_trip` already exist there at `:5588` and `:5596`, but this function needs no database -- build the trips directly with `make_trip_detailed`, which is also already there.

```rust
/// Three consecutive rows of 2026, chain intact from a year start of 50000.
/// Returns them in `trip_order`.
fn make_cascade_chain() -> Vec<Trip> {
    let mut trips = Vec::new();
    let mut odo = 50000.0;
    for (day, km) in [(1u32, 50.0), (2, 70.0), (3, 30.0)] {
        let date = NaiveDate::from_ymd_opt(2026, 3, day).unwrap();
        let mut trip = make_trip_detailed(date, km, None, false);
        odo += km;
        trip.odometer = odo;
        trips.push(trip);
    }
    trips
}

#[test]
fn test_cascade_km_edit_shifts_every_later_row() {
    // The whole point of task 81: change the middle row's km by +10 and the
    // rows after it move +10. The row before it does not move.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 80.0, trips[1].odometer)
        .unwrap();

    assert_eq!(plan.new_distance_km, 80.0);
    assert_eq!(plan.new_odometer, 50130.0, "anchor 50050 + 80 km");
    assert_eq!(plan.delta, 10.0);
    assert_eq!(plan.changes.len(), 1, "only the row after the target moves");
    assert_eq!(plan.changes[0].trip_id, trips[2].id.to_string());
    assert_eq!(plan.changes[0].old_odometer, 50150.0);
    assert_eq!(plan.changes[0].new_odometer, 50160.0);
}

#[test]
fn test_cascade_odo_edit_derives_the_km() {
    // The mirrored direction: type an odometer, and the km becomes the gap to
    // the anchor. The shift is the same.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 70.0, 50130.0).unwrap();

    assert_eq!(plan.new_odometer, 50130.0);
    assert_eq!(plan.new_distance_km, 80.0, "50130 - anchor 50050");
    assert_eq!(plan.delta, 10.0);
    assert_eq!(plan.changes.len(), 1);
}

#[test]
fn test_cascade_km_wins_when_both_fields_change() {
    // Save can beat the preview, so both fields can arrive changed. The km is
    // what the user types; the odometer is derived. The km wins.
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 80.0, 99999.0).unwrap();

    assert_eq!(plan.new_odometer, 50130.0, "anchor + km, not the stale 99999");
    assert_eq!(plan.new_distance_km, 80.0);
}

#[test]
fn test_cascade_does_nothing_when_neither_field_changed() {
    // Editing a purpose or a time must not move a single odometer. A row that
    // sits below its anchor is a record the book keeps on purpose (task 79).
    let mut trips = make_cascade_chain();
    trips[1].odometer = 50999.0; // a broken row, left alone on an unrelated save
    let target = trips[1].id.to_string();

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 70.0, 50999.0).unwrap();

    assert_eq!(plan.delta, 0.0);
    assert_eq!(plan.new_odometer, 50999.0, "unchanged");
    assert!(plan.changes.is_empty());
    assert_eq!(plan.delta_from_repair, 0.0, "no repair on an untouched row");
}

#[test]
fn test_cascade_decomposes_the_repair_from_the_edit() {
    // Row 1 of 2025 in the production book: anchor 38056.5 carried from
    // 2024-12-31, stored odometer 38145, recorded 88 km, so the span is 88.5.
    // Change the km 88 -> 90 and the delta is +1.5, not +2. The modal must be
    // able to say why, so the plan reports both parts.
    let date = NaiveDate::from_ymd_opt(2025, 1, 12).unwrap();
    let mut first = make_trip_detailed(date, 88.0, None, false);
    first.odometer = 38145.0;
    let second_date = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
    let mut second = make_trip_detailed(second_date, 10.0, None, false);
    second.odometer = 38155.0;
    let trips = vec![first.clone(), second.clone()];

    let plan =
        plan_odometer_cascade(&trips, 38056.5, &first.id.to_string(), 90.0, 38145.0).unwrap();

    assert_eq!(plan.new_odometer, 38146.5);
    assert_eq!(plan.delta, 1.5);
    assert_eq!(plan.delta_from_distance, 2.0, "what the user typed");
    assert_eq!(plan.delta_from_repair, -0.5, "the half kilometre from 2024");
    assert!(plan.repair_crosses_year, "the anchor is the previous year's");
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].new_odometer, 38156.5);
}

#[test]
fn test_cascade_reports_that_the_year_end_moved() {
    // A shift confined to one year opens the boundary to the next. The modal
    // must warn, so the plan says whether the year's last odometer moved.
    let trips = make_cascade_chain();
    let target = trips[2].id.to_string(); // the last row of the year

    let plan = plan_odometer_cascade(&trips, 50000.0, &target, 40.0, trips[2].odometer)
        .unwrap();

    assert!(plan.changes.is_empty(), "no row follows the last one");
    assert_eq!(plan.delta, 10.0);
    assert!(plan.year_end_odometer_moved);
}

#[test]
fn test_cascade_keeps_the_order_of_a_tied_group() {
    // The imported years tie on start_datetime AND created_at, so trip_order
    // falls through to the odometer. Measured on the production copy: 2025 has
    // 6 such groups, 14 rows. A shift moves every member by the same delta, so
    // the group must keep its order and its trip numbers. This is the one place
    // a cascade could reorder a legal book.
    let date = NaiveDate::from_ymd_opt(2025, 5, 5).unwrap();
    let stamp = chrono::Utc::now();
    let mut edited = make_trip_detailed(
        NaiveDate::from_ymd_opt(2025, 5, 1).unwrap(), 20.0, None, false,
    );
    edited.created_at = stamp;
    edited.odometer = 43000.0;
    let mut low = make_trip_detailed(date, 91.0, None, false);
    low.created_at = stamp;
    low.odometer = 43091.0;
    let mut high = make_trip_detailed(date, 120.0, None, false);
    high.created_at = stamp;
    high.odometer = 43211.0;
    let trips = vec![edited.clone(), low.clone(), high.clone()];

    let plan =
        plan_odometer_cascade(&trips, 42980.0, &edited.id.to_string(), 30.0, 43000.0)
            .unwrap();

    assert_eq!(plan.delta, 10.0);
    assert_eq!(plan.changes.len(), 2);
    assert_eq!(plan.changes[0].trip_id, low.id.to_string(), "the lower odometer stays first");
    assert_eq!(plan.changes[0].new_odometer, 43101.0);
    assert_eq!(plan.changes[1].trip_id, high.id.to_string());
    assert_eq!(plan.changes[1].new_odometer, 43221.0);
    assert!(
        plan.changes[0].trip_number < plan.changes[1].trip_number,
        "the shift must not renumber the group"
    );
}

#[test]
fn test_cascade_rejects_a_trip_that_is_not_in_the_year() {
    let trips = make_cascade_chain();

    let result = plan_odometer_cascade(&trips, 50000.0, "not-a-trip-id", 10.0, 1.0);

    assert!(result.is_err());
}

#[test]
fn test_cascade_insert_in_the_middle_shifts_every_row_after_it() {
    // The chain is 1 March 50 km, 2 March 70 km, 3 March 30 km. Put a 25 km
    // row on 2 March at 06:00 and everything from the 70 km row onward moves
    // +25.
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap().and_hms_opt(6, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 25.0);

    assert_eq!(plan.new_odometer, 50075.0, "anchor 50050 + 25 km");
    assert_eq!(plan.delta, 25.0);
    assert_eq!(plan.delta_from_repair, 0.0, "a new row carries no span error");
    assert_eq!(plan.changes.len(), 2);
    assert_eq!(plan.changes[0].new_odometer, 50145.0);
    assert_eq!(plan.changes[1].new_odometer, 50175.0);
}

#[test]
fn test_cascade_insert_at_the_end_moves_no_other_row() {
    // Appending is the daily action. It must stay a silent write: no row moves,
    // so the grid opens no modal.
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap().and_hms_opt(8, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 25.0);

    assert_eq!(plan.new_odometer, 50175.0, "anchor is the last row, 50150");
    assert!(plan.changes.is_empty());
    assert!(plan.year_end_odometer_moved);
    assert!(!plan.next_year_chain_breaks, "the planner never sets this");
}

#[test]
fn test_cascade_insert_before_every_row_anchors_on_the_year_start() {
    let trips = make_cascade_chain();
    let when = NaiveDate::from_ymd_opt(2026, 1, 4).unwrap().and_hms_opt(8, 0, 0).unwrap();

    let plan = plan_insert_cascade(&trips, 50000.0, when, 12.0);

    assert_eq!(plan.new_odometer, 50012.0);
    assert_eq!(plan.changes.len(), 3, "the whole year moves");
    assert_eq!(plan.changes[0].new_odometer, 50062.0);
}

#[test]
fn test_cascade_delete_shifts_by_the_removed_span_not_its_distance() {
    // The row records 70 km but spans 80. Deleting it takes 80 out of the
    // chain, because 80 is what the chain actually loses. Shifting by 70 would
    // leave the next row 10 km adrift.
    let mut trips = make_cascade_chain();
    trips[1].odometer = 50130.0; // 70 km recorded, 80 km span from 50050
    trips[2].odometer = 50160.0;
    let target = trips[1].id.to_string();

    let plan = plan_delete_cascade(&trips, 50000.0, &target).unwrap();

    assert_eq!(plan.delta, -80.0);
    assert_eq!(plan.delta_from_distance, -70.0, "what the row records");
    assert_eq!(plan.delta_from_repair, -10.0, "the span error it also removes");
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].old_odometer, 50160.0);
    assert_eq!(plan.changes[0].new_odometer, 50080.0, "50050 + 30 km");
}

#[test]
fn test_cascade_delete_of_a_consistent_row_shifts_by_its_distance() {
    let trips = make_cascade_chain();
    let target = trips[1].id.to_string();

    let plan = plan_delete_cascade(&trips, 50000.0, &target).unwrap();

    assert_eq!(plan.delta, -70.0);
    assert_eq!(plan.delta_from_repair, 0.0);
    assert_eq!(plan.changes[0].new_odometer, 50080.0);
}

#[test]
fn test_cascade_delete_of_the_last_row_moves_nothing_but_the_year_end() {
    let trips = make_cascade_chain();
    let target = trips[2].id.to_string();

    let plan = plan_delete_cascade(&trips, 50000.0, &target).unwrap();

    assert!(plan.changes.is_empty());
    assert!(plan.year_end_odometer_moved);
}
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core cascade
```

Expected: FAIL, `cannot find function 'plan_odometer_cascade' in this scope`.

- [ ] **Step 3: Add `CascadePlan` to `models.rs`**

Put it directly after `OdometerChange`, which ends at `:920`.

```rust
/// What one cascading save would do to the odometer chain of a year.
///
/// `delta` is the number every row after the edited one moves by. It is
/// reported in two parts because they have different causes and the user
/// needs to tell them apart before approving the write (task 81):
///
/// - `delta_from_distance` is what the user typed: `new_km - stored_km`.
/// - `delta_from_repair` is the pre-existing span error of the edited row,
///   negated. Setting `odometer = anchor + km` repairs that error, and the
///   repair travels down the chain with the edit.
///
/// `delta == delta_from_distance + delta_from_repair` always.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CascadePlan {
    /// The odometer the edited row ends at.
    pub new_odometer: f64,
    /// The distance the edited row records.
    pub new_distance_km: f64,
    pub delta: f64,
    pub delta_from_distance: f64,
    pub delta_from_repair: f64,
    /// True when the edited row is the first of its year, so the repair above
    /// is measured against the previous year's last odometer.
    pub repair_crosses_year: bool,
    /// True when the year's last stored odometer moves, which opens the
    /// boundary to the next year by `delta`.
    pub year_end_odometer_moved: bool,
    /// True when `year_end_odometer_moved` is true AND a later year has trips,
    /// so a chain that used to line up will not any more. The planner cannot
    /// know this -- it sees one year -- so it always writes `false` here and
    /// the command fills it in. Only this flag is worth a warning: appending
    /// to the newest year moves its year end every time and breaks nothing.
    pub next_year_chain_breaks: bool,
    /// Every row AFTER the edited one, in `trip_order`. The edited row is not
    /// here -- its values are `new_odometer` and `new_distance_km` above.
    pub changes: Vec<OdometerChange>,
}
```

- [ ] **Step 4: Add `plan_odometer_cascade` to `trips.rs`**

Put it after `recalculate_odometers_internal`, which ends at `:240`.

```rust
/// Largest float difference the cascade treats as "the same number". Matches
/// the tolerance `recalculate_odometers_internal` uses.
const CASCADE_EPSILON: f64 = 0.001;

/// Plan the odometer cascade for one edited row, without touching the database.
///
/// `trips` is every trip of the vehicle in that year, in any order; this
/// function sorts them by `trip_order` itself. `year_start_odometer` is what
/// `get_year_start_odometer` returns, so the first row of a year is anchored
/// on the previous year and not on nothing.
///
/// Which number gives way depends on which one the user changed. A km edit
/// makes the odometer follow (`anchor + km`); an odometer edit makes the km
/// follow (`odometer - anchor`); an edit to neither moves nothing at all, so
/// a save that only fixed a typo in the purpose leaves a deliberately broken
/// row exactly as it was (task 79).
///
/// The walk stops at the end of the year. That is on purpose and it is
/// visible: `year_end_odometer_moved` tells the caller the boundary to the
/// next year has opened by `delta`, and the span warning marks the first row
/// of that year.
///
/// The reported trip numbers come from the book BEFORE the shift, and they
/// stay correct after it. `trip_order` falls through to the odometer only when
/// `start_datetime` and `created_at` both tie, and every member of such a group
/// moves by the same `delta`, so the group keeps its internal order. A shift
/// can therefore never renumber the book.
pub fn plan_odometer_cascade(
    trips: &[Trip],
    year_start_odometer: f64,
    trip_id: &str,
    submitted_distance_km: f64,
    submitted_odometer: f64,
) -> Result<CascadePlan, String> {
    let mut sorted: Vec<&Trip> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    let idx = sorted
        .iter()
        .position(|t| t.id.to_string() == trip_id)
        .ok_or_else(|| format!("Trip not found in this year: {}", trip_id))?;

    let stored = sorted[idx];
    let anchor = if idx == 0 {
        year_start_odometer
    } else {
        sorted[idx - 1].odometer
    };

    let km_changed = (submitted_distance_km - stored.distance_km).abs() > CASCADE_EPSILON;
    let odo_changed = (submitted_odometer - stored.odometer).abs() > CASCADE_EPSILON;

    let (new_distance_km, new_odometer) = if km_changed {
        (submitted_distance_km, anchor + submitted_distance_km)
    } else if odo_changed {
        (submitted_odometer - anchor, submitted_odometer)
    } else {
        // Nothing the user did moves the chain. Report a no-op rather than a
        // repair: the row keeps whatever the book records for it.
        return Ok(CascadePlan {
            new_odometer: stored.odometer,
            new_distance_km: stored.distance_km,
            delta: 0.0,
            delta_from_distance: 0.0,
            delta_from_repair: 0.0,
            repair_crosses_year: false,
            year_end_odometer_moved: false,
            next_year_chain_breaks: false,
            changes: Vec::new(),
        });
    };

    let delta = new_odometer - stored.odometer;
    let delta_from_distance = new_distance_km - stored.distance_km;
    let delta_from_repair = delta - delta_from_distance;

    let trip_numbers = calculate_trip_numbers(trips);
    let changes = sorted[idx + 1..]
        .iter()
        .map(|t| OdometerChange {
            trip_id: t.id.to_string(),
            trip_number: *trip_numbers.get(&t.id.to_string()).unwrap_or(&0),
            old_odometer: t.odometer,
            new_odometer: t.odometer + delta,
        })
        .collect::<Vec<_>>();

    Ok(CascadePlan {
        new_odometer,
        new_distance_km,
        delta,
        delta_from_distance,
        delta_from_repair,
        repair_crosses_year: idx == 0 && delta_from_repair.abs() > CASCADE_EPSILON,
        year_end_odometer_moved: delta.abs() > CASCADE_EPSILON,
        next_year_chain_breaks: false, // the command knows, this function does not
        changes,
    })
}

/// Plan the cascade for a row that does not exist yet.
///
/// The new row lands where `trip_order` puts it, and its odometer is
/// `anchor + km`. Everything after it then starts `km` later, so the whole
/// tail shifts by exactly the distance of the new row (task 81, R8).
///
/// The position is decided on `start_datetime` and then `created_at` alone. A
/// new row's `created_at` is the moment it is written, so it sorts last inside
/// any group it ties with, and the odometer key of `trip_order` never decides
/// an insert. That is what stops the position and the odometer -- which is
/// derived from the position -- from depending on each other.
///
/// A new row carries no span error, so `delta_from_repair` is always 0.
pub fn plan_insert_cascade(
    trips: &[Trip],
    year_start_odometer: f64,
    new_start_datetime: NaiveDateTime,
    new_distance_km: f64,
) -> CascadePlan {
    let mut sorted: Vec<&Trip> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    // The new row goes after every row whose start_datetime is not later. Its
    // created_at is now, so it also goes after every row it ties with.
    let idx = sorted
        .iter()
        .position(|t| t.start_datetime > new_start_datetime)
        .unwrap_or(sorted.len());

    let anchor = if idx == 0 {
        year_start_odometer
    } else {
        sorted[idx - 1].odometer
    };

    let trip_numbers = calculate_trip_numbers(trips);
    let changes = sorted[idx..]
        .iter()
        .map(|t| OdometerChange {
            trip_id: t.id.to_string(),
            trip_number: *trip_numbers.get(&t.id.to_string()).unwrap_or(&0),
            old_odometer: t.odometer,
            new_odometer: t.odometer + new_distance_km,
        })
        .collect::<Vec<_>>();

    CascadePlan {
        new_odometer: anchor + new_distance_km,
        new_distance_km,
        delta: new_distance_km,
        delta_from_distance: new_distance_km,
        delta_from_repair: 0.0,
        repair_crosses_year: false,
        year_end_odometer_moved: new_distance_km.abs() > CASCADE_EPSILON,
        next_year_chain_breaks: false,
        changes,
    }
}

/// Plan the cascade for removing a row.
///
/// The row after the deleted one inherits the deleted row's start, so the
/// chain loses exactly the deleted row's SPAN -- `odometer - anchor` -- and
/// not its recorded distance (task 81, R9). The two are the same number when
/// the row was consistent. When it was not, only the span leaves the chain
/// continuous.
pub fn plan_delete_cascade(
    trips: &[Trip],
    year_start_odometer: f64,
    trip_id: &str,
) -> Result<CascadePlan, String> {
    let mut sorted: Vec<&Trip> = trips.iter().collect();
    sorted.sort_by(|a, b| trip_order(a, b));

    let idx = sorted
        .iter()
        .position(|t| t.id.to_string() == trip_id)
        .ok_or_else(|| format!("Trip not found in this year: {}", trip_id))?;

    let removed = sorted[idx];
    let anchor = if idx == 0 {
        year_start_odometer
    } else {
        sorted[idx - 1].odometer
    };

    let span = removed.odometer - anchor;
    let delta = -span;

    let trip_numbers = calculate_trip_numbers(trips);
    let changes = sorted[idx + 1..]
        .iter()
        .map(|t| OdometerChange {
            trip_id: t.id.to_string(),
            trip_number: *trip_numbers.get(&t.id.to_string()).unwrap_or(&0),
            old_odometer: t.odometer,
            new_odometer: t.odometer + delta,
        })
        .collect::<Vec<_>>();

    Ok(CascadePlan {
        new_odometer: removed.odometer,
        new_distance_km: removed.distance_km,
        delta,
        delta_from_distance: -removed.distance_km,
        delta_from_repair: delta + removed.distance_km,
        repair_crosses_year: idx == 0 && (span - removed.distance_km).abs() > CASCADE_EPSILON,
        year_end_odometer_moved: delta.abs() > CASCADE_EPSILON,
        next_year_chain_breaks: false,
        changes,
    })
}
```

`plan_insert_cascade` and `plan_delete_cascade` need `chrono::NaiveDateTime` in scope.

Add `CascadePlan` to the `use crate::models::{...}` list at the top of `trips.rs`.

- [ ] **Step 5: Run the tests and watch them pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core cascade
```

Expected: PASS, 14 tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/models.rs src-tauri/core/src/commands_internal/trips.rs src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "feat(trips): plan the odometer cascade for an edit, an insert and a delete"
```

---

### Task 2: Three transactional writes, each with its shift

**Files:**
- Modify: `src-tauri/core/src/db.rs` (add after `update_trip`, which ends at `:465`)
- Test: `src-tauri/core/src/db_tests.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces, all on `Database`:
  - `pub fn update_trip_with_odometer_shift(&self, trip: &Trip, shifts: &[(String, f64)]) -> QueryResult<()>`
  - `pub fn create_trip_with_odometer_shift(&self, trip: &Trip, shifts: &[(String, f64)]) -> QueryResult<()>`
  - `pub fn delete_trip_with_odometer_shift(&self, id: &str, shifts: &[(String, f64)]) -> QueryResult<()>`

- [ ] **Step 1: Write the failing test**

Add to `db_tests.rs`. Use the file's existing setup helper for an in-memory database and a vehicle.

```rust
#[test]
fn test_update_trip_with_odometer_shift_writes_all_or_nothing() {
    // The shifted rows and the edited row are one legal correction. A failure
    // on any of them must leave the book exactly as it was.
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Shift Car".to_string(), "BA111AA".to_string(), 66.0, 5.1, 0.0);
    db.create_vehicle(&vehicle).unwrap();

    let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let mut edited = Trip::new(vehicle.id, "A".into(), "B".into(), 50.0, 50.0, "work".into(), date);
    let mut later = Trip::new(vehicle.id, "B".into(), "C".into(), 20.0, 70.0, "work".into(), date);
    db.create_trip(&edited).unwrap();
    db.create_trip(&later).unwrap();

    edited.distance_km = 60.0;
    edited.odometer = 60.0;
    db.update_trip_with_odometer_shift(&edited, &[(later.id.to_string(), 80.0)])
        .unwrap();

    assert_eq!(db.get_trip(&edited.id.to_string()).unwrap().unwrap().odometer, 60.0);
    assert_eq!(db.get_trip(&edited.id.to_string()).unwrap().unwrap().distance_km, 60.0);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 80.0);
    // A shift touches the odometer and nothing else.
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().distance_km, 20.0);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().origin, "B");
}

#[test]
fn test_update_trip_with_odometer_shift_rejects_an_unknown_row() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Shift Car".to_string(), "BA111AA".to_string(), 66.0, 5.1, 0.0);
    db.create_vehicle(&vehicle).unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let edited = Trip::new(vehicle.id, "A".into(), "B".into(), 50.0, 50.0, "work".into(), date);
    db.create_trip(&edited).unwrap();

    let result = db.update_trip_with_odometer_shift(
        &edited,
        &[("00000000-0000-0000-0000-000000000000".to_string(), 80.0)],
    );

    assert!(result.is_err(), "a shift naming a row that is not there must fail");
    assert_eq!(
        db.get_trip(&edited.id.to_string()).unwrap().unwrap().odometer,
        50.0,
        "and it must roll the edited row back"
    );
}
```

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core odometer_shift
```

Expected: FAIL, `no method named 'update_trip_with_odometer_shift'`.

- [ ] **Step 3: Implement it**

Add after `update_trip` in `db.rs`. Follow the transaction pattern already used by `save_route_map` at `:1154`.

```rust
    /// Write one full row and move the odometer of others, in one transaction.
    ///
    /// A cascading save is one correction to a legal record, so it commits
    /// whole or not at all (task 81). The shifted rows change their odometer
    /// and their `updated_at` and nothing else -- never their distance, which
    /// is what the book records as driven.
    ///
    /// A shift naming a row that is not in the table is an error, not a
    /// silent no-op: it means the caller planned against a book that has
    /// since moved.
    pub fn update_trip_with_odometer_shift(
        &self,
        trip: &Trip,
        shifts: &[(String, f64)],
    ) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        let id_str = trip.id.to_string();
        let vehicle_id_str = trip.vehicle_id.to_string();
        let start_datetime_str = trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        let end_datetime_str = trip
            .end_datetime
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        let updated_at_str = trip.updated_at.to_rfc3339();

        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            diesel::update(trips::table.filter(trips::id.eq(&id_str)))
                .set((
                    trips::vehicle_id.eq(&vehicle_id_str),
                    trips::origin.eq(&trip.origin),
                    trips::destination.eq(&trip.destination),
                    trips::distance_km.eq(trip.distance_km),
                    trips::odometer.eq(trip.odometer),
                    trips::purpose.eq(&trip.purpose),
                    trips::fuel_liters.eq(trip.fuel_liters),
                    trips::fuel_cost_eur.eq(trip.fuel_cost_eur),
                    trips::other_costs_eur.eq(trip.other_costs_eur),
                    trips::other_costs_note.eq(&trip.other_costs_note),
                    trips::full_tank.eq(if trip.full_tank { 1 } else { 0 }),
                    trips::energy_kwh.eq(trip.energy_kwh),
                    trips::energy_cost_eur.eq(trip.energy_cost_eur),
                    trips::full_charge.eq(Some(if trip.full_charge { 1 } else { 0 })),
                    trips::soc_override_percent.eq(trip.soc_override_percent),
                    trips::updated_at.eq(&updated_at_str),
                    trips::start_datetime.eq(&start_datetime_str),
                    trips::end_datetime.eq(end_datetime_str.as_deref()),
                ))
                .execute(tx)?;

            for (shift_id, new_odometer) in shifts {
                let rows = diesel::update(trips::table.filter(trips::id.eq(shift_id)))
                    .set((
                        trips::odometer.eq(new_odometer),
                        trips::updated_at.eq(&updated_at_str),
                    ))
                    .execute(tx)?;
                if rows != 1 {
                    return Err(diesel::result::Error::NotFound);
                }
            }

            Ok(())
        })
    }
```

- [ ] **Step 4: Write the failing tests for create and delete**

```rust
#[test]
fn test_create_trip_with_odometer_shift_inserts_and_shifts() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Shift Car".to_string(), "BA111AA".to_string(), 66.0, 5.1, 0.0);
    db.create_vehicle(&vehicle).unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let later = Trip::new(vehicle.id, "B".into(), "C".into(), 20.0, 70.0, "work".into(), date);
    db.create_trip(&later).unwrap();
    let inserted =
        Trip::new(vehicle.id, "A".into(), "B".into(), 50.0, 50.0, "work".into(), date);

    db.create_trip_with_odometer_shift(&inserted, &[(later.id.to_string(), 120.0)])
        .unwrap();

    assert_eq!(db.get_trip(&inserted.id.to_string()).unwrap().unwrap().odometer, 50.0);
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 120.0);
}

#[test]
fn test_delete_trip_with_odometer_shift_removes_and_shifts() {
    let db = Database::in_memory().unwrap();
    let vehicle = Vehicle::new("Shift Car".to_string(), "BA111AA".to_string(), 66.0, 5.1, 0.0);
    db.create_vehicle(&vehicle).unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    let removed =
        Trip::new(vehicle.id, "A".into(), "B".into(), 50.0, 50.0, "work".into(), date);
    let later = Trip::new(vehicle.id, "B".into(), "C".into(), 20.0, 70.0, "work".into(), date);
    db.create_trip(&removed).unwrap();
    db.create_trip(&later).unwrap();

    db.delete_trip_with_odometer_shift(&removed.id.to_string(), &[(later.id.to_string(), 20.0)])
        .unwrap();

    assert!(db.get_trip(&removed.id.to_string()).unwrap().is_none());
    assert_eq!(db.get_trip(&later.id.to_string()).unwrap().unwrap().odometer, 20.0);
}
```

- [ ] **Step 5: Implement them**

Both follow the same shape as Step 3. Factor the shift loop out first so the three
methods share it:

```rust
    /// Apply the odometer shifts of one cascade inside an open transaction.
    /// A shift naming a row that is not there is an error, not a silent no-op:
    /// it means the caller planned against a book that has since moved.
    fn apply_odometer_shifts(
        tx: &mut SqliteConnection,
        shifts: &[(String, f64)],
        updated_at: &str,
    ) -> QueryResult<()> {
        for (shift_id, new_odometer) in shifts {
            let rows = diesel::update(trips::table.filter(trips::id.eq(shift_id)))
                .set((
                    trips::odometer.eq(new_odometer),
                    trips::updated_at.eq(updated_at),
                ))
                .execute(tx)?;
            if rows != 1 {
                return Err(diesel::result::Error::NotFound);
            }
        }
        Ok(())
    }
```

`create_trip_with_odometer_shift` inserts the row exactly as `create_trip` does, then
calls `apply_odometer_shifts`, both inside one `conn.transaction`.

`delete_trip_with_odometer_shift` deletes from `paperless_trip_links` and then from
`trips`, exactly as `delete_trip` does at [db.rs:466-473](../../../src-tauri/core/src/db.rs),
then calls `apply_odometer_shifts`, all inside one `conn.transaction`. Use the current
UTC time for `updated_at`.

- [ ] **Step 6: Run the tests and watch them pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core odometer_shift
```

Expected: PASS, 4 tests.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "feat(db): write a trip and its odometer shift in one transaction"
```

---

### Task 3: The three cascade commands

**Files:**
- Modify: `src-tauri/core/src/models.rs` (add `CascadeResult` after `CascadePlan`)
- Modify: `src-tauri/core/src/commands_internal/trips.rs` (add after `plan_odometer_cascade`)
- Test: `src-tauri/core/src/commands_internal/commands_tests.rs`

**Interfaces:**
- Consumes: the three planners (Task 1), the three `*_with_odometer_shift` methods (Task 2), `get_year_start_odometer` (already in `statistics.rs`), `db.get_years_with_trips` ([db.rs:411](../../../src-tauri/core/src/db.rs)).
- Produces:
  - `pub struct CascadeResult { trip: Option<Trip>, plan: CascadePlan }`
  - `pub fn update_trip_cascade_internal(db, app_state, id, start_datetime, end_datetime, origin, destination, distance_km, odometer, purpose, fuel_liters, fuel_cost_eur, full_tank, energy_kwh, energy_cost_eur, full_charge, soc_override_percent, other_costs_eur, other_costs_note, dry_run) -> Result<CascadeResult, String>`
  - `pub fn create_trip_cascade_internal(db, app_state, vehicle_id, start_datetime, end_datetime, origin, destination, distance_km, purpose, fuel_liters, fuel_cost, full_tank, energy_kwh, energy_cost_eur, full_charge, soc_override_percent, other_costs, other_costs_note, dry_run) -> Result<CascadeResult, String>` -- note there is **no** `odometer` argument: the backend derives it.
  - `pub fn delete_trip_cascade_internal(db, app_state, id, dry_run) -> Result<CascadePlan, String>`

- [ ] **Step 1: Write the failing tests**

```rust
/// Build the argument list `update_trip_cascade_internal` takes for a row that
/// changes only its distance. Keeps the tests below to the numbers that matter.
fn cascade_args(trip: &Trip, km: f64, odo: f64) -> (String, String, String, f64, f64) {
    (
        trip.start_datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
        trip.origin.clone(),
        trip.destination.clone(),
        km,
        odo,
    )
}

#[test]
fn test_update_trip_cascade_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    )
    .unwrap();

    assert!(result.trip.is_none(), "a dry run returns no saved trip");
    assert_eq!(result.plan.delta, 10.0);
    assert_eq!(result.plan.changes.len(), 1);
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn test_update_trip_cascade_applies_the_shift() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert!(result.trip.is_some());
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50060.0);
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50130.0);
    assert_eq!(db.get_trip(&c.to_string()).unwrap().unwrap().odometer, 50160.0);
    // The shifted rows keep their own distances.
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().distance_km, 70.0);
}

#[test]
fn test_update_trip_cascade_does_not_cross_the_year_boundary() {
    // The user's explicit call: the shift stops at 31 December. Pin it, so the
    // boundary break stays a visible span warning rather than a silent rewrite
    // of the next year.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let mut next_year = make_trip_detailed(
        NaiveDate::from_ymd_opt(2027, 1, 4).unwrap(), 20.0, None, false,
    );
    next_year.vehicle_id = vehicle.id;
    next_year.odometer = 50070.0;
    db.create_trip(&next_year).unwrap();

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert!(result.plan.year_end_odometer_moved, "the caller must be told");
    assert_eq!(
        db.get_trip(&next_year.id.to_string()).unwrap().unwrap().odometer,
        50070.0,
        "2027 is untouched"
    );
}

#[test]
fn test_update_trip_cascade_does_not_cascade_a_re_dated_row() {
    // Moving the datetime moves the row in trip_order, so two positions shift,
    // not one. That is not modelled: write the row, cascade nothing.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);

    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    let moved = "2026-03-05T08:00:00".to_string();
    let result = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), moved.clone(), moved,
        trip_a.origin.clone(), trip_a.destination.clone(), 60.0, 50050.0,
        trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        false,
    )
    .unwrap();

    assert_eq!(result.plan.delta, 0.0);
    assert!(result.plan.changes.is_empty());
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50120.0);
}

#[test]
fn test_update_trip_cascade_is_read_only_guarded() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let trip_a = db.get_trip(&a.to_string()).unwrap().unwrap();
    app_state.enable_read_only("newer migrations");
    let (start, origin, destination, km, odo) = cascade_args(&trip_a, 60.0, 50050.0);

    let applied = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start.clone(), origin.clone(),
        destination.clone(), km, odo, trip_a.purpose.clone(),
        None, None, None, None, None, None, None, None, None, false,
    );
    assert!(applied.is_err(), "a write must respect read-only mode");

    let dry = update_trip_cascade_internal(
        &db, &app_state, a.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_a.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    );
    assert!(dry.is_ok(), "a dry run reads only, so it is always allowed");
}

#[test]
fn test_create_trip_cascade_inserts_and_shifts() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50080.0);

    let result = create_trip_cascade_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-02T08:00:00".to_string(), "2026-03-02T09:00:00".to_string(),
        "A".to_string(), "B".to_string(), 25.0, "work".to_string(),
        None, None, None, None, None, None, None, None, None, false,
    )
    .unwrap();

    let created = result.trip.unwrap();
    assert_eq!(created.odometer, 50075.0, "the backend derived it, 50050 + 25");
    assert_eq!(db.get_trip(&b.to_string()).unwrap().unwrap().odometer, 50105.0);
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0, "untouched");
}

#[test]
fn test_create_trip_cascade_appending_moves_no_row_and_raises_no_warning() {
    // The daily action. It must stay a single silent write: no other row moves,
    // and there is no later year to break.
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);

    let result = create_trip_cascade_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-09T08:00:00".to_string(), "2026-03-09T09:00:00".to_string(),
        "A".to_string(), "B".to_string(), 25.0, "work".to_string(),
        None, None, None, None, None, None, None, None, None, true,
    )
    .unwrap();

    assert!(result.plan.changes.is_empty());
    assert!(result.plan.year_end_odometer_moved);
    assert!(
        !result.plan.next_year_chain_breaks,
        "no later year, so nothing to warn about"
    );
}

#[test]
fn test_create_trip_cascade_warns_when_a_later_year_exists() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let mut next_year = make_trip_detailed(
        NaiveDate::from_ymd_opt(2027, 1, 4).unwrap(), 20.0, None, false,
    );
    next_year.vehicle_id = vehicle.id;
    next_year.odometer = 50070.0;
    db.create_trip(&next_year).unwrap();

    let result = create_trip_cascade_internal(
        &db, &app_state, vehicle.id.to_string(),
        "2026-03-09T08:00:00".to_string(), "2026-03-09T09:00:00".to_string(),
        "A".to_string(), "B".to_string(), 25.0, "work".to_string(),
        None, None, None, None, None, None, None, None, None, true,
    )
    .unwrap();

    assert!(result.plan.next_year_chain_breaks);
}

#[test]
fn test_delete_trip_cascade_closes_the_gap() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let c = seed_chain_trip(&db, vehicle.id, 3, 30.0, 50150.0);

    let plan = delete_trip_cascade_internal(&db, &app_state, b.to_string(), false).unwrap();

    assert_eq!(plan.delta, -70.0);
    assert!(db.get_trip(&b.to_string()).unwrap().is_none());
    assert_eq!(db.get_trip(&a.to_string()).unwrap().unwrap().odometer, 50050.0, "untouched");
    assert_eq!(db.get_trip(&c.to_string()).unwrap().unwrap().odometer, 50080.0);
}

#[test]
fn test_delete_trip_cascade_dry_run_writes_nothing() {
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50070.0);

    let plan = delete_trip_cascade_internal(&db, &app_state, b.to_string(), true).unwrap();

    assert_eq!(plan.delta, -70.0);
    assert!(db.get_trip(&b.to_string()).unwrap().is_some(), "still there");
}

#[test]
fn test_update_trip_cascade_agrees_with_the_preview_command() {
    // preview_trip_calculation answers the open editor and the cascade answers
    // the save. If they ever disagree the ODO jumps on save (task 80).
    let (db, vehicle) = setup_db_with_start_odometer(50000.0);
    let app_state = crate::app_state::AppState::new();
    let a = seed_chain_trip(&db, vehicle.id, 1, 50.0, 50050.0);
    let b = seed_chain_trip(&db, vehicle.id, 2, 70.0, 50120.0);
    let trip_b = db.get_trip(&b.to_string()).unwrap().unwrap();

    let preview = preview_trip_calculation_internal(
        &db, vehicle.id.to_string(), 2026, 80.0, None, true, Some(b.to_string()),
    )
    .unwrap();

    let (start, origin, destination, km, odo) = cascade_args(&trip_b, 80.0, trip_b.odometer);
    let plan = update_trip_cascade_internal(
        &db, &app_state, b.to_string(), start.clone(), start, origin, destination,
        km, odo, trip_b.purpose.clone(), None, None, None, None, None, None, None, None, None,
        true,
    )
    .unwrap()
    .plan;

    assert_eq!(preview.odometer, plan.new_odometer, "one arithmetic, two callers");
    let _ = a;
}
```

Check the real signature of `preview_trip_calculation_internal` before writing the last test and match it exactly; the dispatcher arm is at [dispatcher.rs:365](../../../src-tauri/core/src/server/dispatcher.rs).

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core trip_cascade_
```

Expected: FAIL, `cannot find function 'update_trip_cascade_internal'`.

- [ ] **Step 3: Add `CascadeResult` to `models.rs`**

```rust
/// The answer `update_trip_cascade_internal` gives. `trip` is `None` on a dry
/// run, because a dry run saves nothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CascadeResult {
    pub trip: Option<Trip>,
    pub plan: CascadePlan,
}
```

- [ ] **Step 4: Extract the row builder out of `update_trip_internal`**

Both save paths must build the `Trip` the same way, or they will drift. Cut the body of
`update_trip_internal` between the `check_read_only!` line and the `db.update_trip` call
([trips.rs:127-170](../../../src-tauri/core/src/commands_internal/trips.rs)) into a private
function, and call it from `update_trip_internal`:

```rust
/// Build the `Trip` a save writes, from the submitted fields and the stored row.
/// Shared by `update_trip_internal` and `update_trip_cascade_internal` so the
/// two save paths can never disagree about validation or normalisation.
#[allow(clippy::too_many_arguments)]
fn build_updated_trip(
    db: &Database,
    id: &str,
    start_datetime: &str,
    end_datetime: &str,
    origin: &str,
    destination: &str,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    full_tank: Option<bool>,
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    other_costs_eur: Option<f64>,
    other_costs_note: Option<String>,
) -> Result<Trip, String>
```

It keeps the existing `Uuid::parse_str`, `parse_iso_datetime`, `normalize_location` and
SoC range check exactly as they are, loads `existing` with `db.get_trip`, and returns the
`Trip` with `created_at: existing.created_at` and `updated_at: Utc::now()`.

Run the backend suite before going on. Nothing may change:

```bash
npm run test:backend
```

Expected: PASS.

- [ ] **Step 5: Implement the command in `trips.rs`**

```rust
/// Save one row and move every later row of the same year by the same amount.
///
/// This is the save path the grid uses. `update_trip_internal` stays beside it
/// and writes one row with no cascade: the task 79 correction procedure needs
/// a command that writes exactly what it is given.
///
/// `dry_run == true` writes nothing and is always allowed, even in read-only
/// mode, because reading is always allowed. It is what fills the confirmation
/// modal. The apply call plans again from the stored book rather than
/// replaying the dry run's numbers, so a book that moved in between is
/// corrected against as it is now.
///
/// The row and its shift go to the database in one transaction. They are one
/// correction to a legal record, so a partial write is never acceptable.
#[allow(clippy::too_many_arguments)]
pub fn update_trip_cascade_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
    start_datetime: String,
    end_datetime: String,
    origin: String,
    destination: String,
    distance_km: f64,
    odometer: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost_eur: Option<f64>,
    full_tank: Option<bool>,
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    other_costs_eur: Option<f64>,
    other_costs_note: Option<String>,
    dry_run: bool,
) -> Result<CascadeResult, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let existing = db
        .get_trip(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    let submitted_start = parse_iso_datetime(&start_datetime)?;
    let year = existing.start_datetime.year();
    let re_dated = submitted_start != existing.start_datetime;

    let vehicle_id = existing.vehicle_id.to_string();
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start =
        get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    // A re-dated row moves in trip_order, so its old and its new position both
    // shift. That is not modelled here: write the row and cascade nothing.
    let mut plan = if re_dated {
        CascadePlan {
            new_odometer: odometer,
            new_distance_km: distance_km,
            delta: 0.0,
            delta_from_distance: 0.0,
            delta_from_repair: 0.0,
            repair_crosses_year: false,
            year_end_odometer_moved: false,
            next_year_chain_breaks: false,
            changes: Vec::new(),
        }
    } else {
        plan_odometer_cascade(&trips, year_start, &id, distance_km, odometer)?
    };
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    if dry_run {
        return Ok(CascadeResult { trip: None, plan });
    }

    let trip = build_updated_trip(
        db,
        &id,
        &start_datetime,
        &end_datetime,
        &origin,
        &destination,
        plan.new_distance_km,
        plan.new_odometer,
        purpose,
        fuel_liters,
        fuel_cost_eur,
        full_tank,
        energy_kwh,
        energy_cost_eur,
        full_charge,
        soc_override_percent,
        other_costs_eur,
        other_costs_note,
    )?;

    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.update_trip_with_odometer_shift(&trip, &shifts)
        .map_err(|e| e.to_string())?;

    db.find_or_create_route(
        &trip.vehicle_id.to_string(),
        &trip.origin,
        &trip.destination,
        plan.new_distance_km,
    )
    .map_err(|e| e.to_string())?;

    Ok(CascadeResult { trip: Some(trip), plan })
}
```

Add `Datelike` to the `chrono` import in `trips.rs` if it is not there, and add
`CascadePlan` and `CascadeResult` to the `use crate::models::{...}` list.

The `next_year_chain_breaks` flag is filled in by a shared helper, because all three
commands need it:

```rust
/// A moved year end only matters when a later year has rows to break. Appending
/// to the newest year moves its year end every time and breaks nothing, and a
/// warning on that would fire on the most common action in the app (task 81, R4).
fn mark_next_year_chain_breaks(
    db: &Database,
    vehicle_id: &str,
    year: i32,
    plan: &mut CascadePlan,
) -> Result<(), String> {
    if !plan.year_end_odometer_moved {
        return Ok(());
    }
    let years = db.get_years_with_trips(vehicle_id).map_err(|e| e.to_string())?;
    plan.next_year_chain_breaks = years.iter().any(|y| *y > year);
    Ok(())
}
```

It is called in all three commands, after planning and before the `dry_run` return, so
the dry run and the apply report the same thing. The call is already in the code blocks
of Steps 5, 6 and 7; write the helper itself once, next to them.

- [ ] **Step 6: Implement `create_trip_cascade_internal`**

```rust
/// Insert a trip and move every later row of the same year by its distance.
///
/// It takes no `odometer` argument. The row's odometer is `anchor + km`, and
/// the anchor comes from the book, so the browser has nothing left to guess
/// (ADR-008, task 81 R8).
#[allow(clippy::too_many_arguments)]
pub fn create_trip_cascade_internal(
    db: &Database,
    app_state: &AppState,
    vehicle_id: String,
    start_datetime: String,
    end_datetime: String,
    origin: String,
    destination: String,
    distance_km: f64,
    purpose: String,
    fuel_liters: Option<f64>,
    fuel_cost: Option<f64>,
    full_tank: Option<bool>,
    energy_kwh: Option<f64>,
    energy_cost_eur: Option<f64>,
    full_charge: Option<bool>,
    soc_override_percent: Option<f64>,
    other_costs: Option<f64>,
    other_costs_note: Option<String>,
    dry_run: bool,
) -> Result<CascadeResult, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let trip_start_datetime = parse_iso_datetime(&start_datetime)?;
    let year = trip_start_datetime.year();

    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start =
        get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    let mut plan =
        plan_insert_cascade(&trips, year_start, trip_start_datetime, distance_km);
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    if dry_run {
        return Ok(CascadeResult { trip: None, plan });
    }

    let trip = build_new_trip(
        &vehicle_id,
        &start_datetime,
        &end_datetime,
        &origin,
        &destination,
        distance_km,
        plan.new_odometer,
        purpose,
        fuel_liters,
        fuel_cost,
        full_tank,
        energy_kwh,
        energy_cost_eur,
        full_charge,
        soc_override_percent,
        other_costs,
        other_costs_note,
    )?;

    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.create_trip_with_odometer_shift(&trip, &shifts)
        .map_err(|e| e.to_string())?;

    db.find_or_create_route(&vehicle_id, &trip.origin, &trip.destination, distance_km)
        .map_err(|e| e.to_string())?;

    Ok(CascadeResult { trip: Some(trip), plan })
}
```

Extract `build_new_trip` out of `create_trip_internal`
([trips.rs:64-104](../../../src-tauri/core/src/commands_internal/trips.rs)) the same way
Step 4 extracted `build_updated_trip`: it keeps the `Uuid::parse_str`,
`parse_iso_datetime`, `normalize_location` and SoC range check exactly as they are, and
returns the `Trip`. `create_trip_internal` then calls it too.

**The plan is made before the row exists, so it must be applied before anything else
writes.** `plan_insert_cascade` positions the new row by `start_datetime` alone, which is
known up front, so there is no ordering hazard here.

- [ ] **Step 7: Implement `delete_trip_cascade_internal`**

```rust
/// Remove a trip and close the gap it leaves in the odometer chain.
///
/// The rows after it move by the removed row's SPAN, not by its recorded
/// distance -- the span is what the chain loses (task 81, R9).
pub fn delete_trip_cascade_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
    dry_run: bool,
) -> Result<CascadePlan, String> {
    if !dry_run {
        check_read_only!(app_state);
    }

    let existing = db
        .get_trip(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {}", id))?;

    let vehicle_id = existing.vehicle_id.to_string();
    let year = existing.start_datetime.year();
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;
    let trips = db
        .get_trips_for_vehicle_in_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;
    let year_start =
        get_year_start_odometer(db, &vehicle_id, year, vehicle.initial_odometer)?;

    let mut plan = plan_delete_cascade(&trips, year_start, &id)?;
    mark_next_year_chain_breaks(db, &vehicle_id, year, &mut plan)?;

    if dry_run {
        return Ok(plan);
    }

    let shifts: Vec<(String, f64)> = plan
        .changes
        .iter()
        .map(|c| (c.trip_id.clone(), c.new_odometer))
        .collect();
    db.delete_trip_with_odometer_shift(&id, &shifts)
        .map_err(|e| e.to_string())?;

    Ok(plan)
}
```

- [ ] **Step 8: Run the tests and watch them pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core trip_cascade_
```

Expected: PASS, 11 tests.

- [ ] **Step 9: Run the whole backend suite**

```bash
npm run test:backend
```

Expected: PASS. Nothing else calls the new code yet, so no existing test may change.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/core/src/models.rs src-tauri/core/src/commands_internal/trips.rs src-tauri/core/src/commands_internal/commands_tests.rs
git commit -m "feat(trips): cascade the odometer on save, insert and delete"
```

---

### Task 4: The RPC arms

**Files:**
- Modify: `src-tauri/core/src/server/dispatcher.rs` (add after the `"update_trip"` arm, which ends at `:221`)
- Test: `src-tauri/core/src/server/dispatcher.rs` (the `#[cfg(test)]` module, next to the tests at `:1515`)

**Interfaces:**
- Consumes: `update_trip_cascade_internal` (Task 3).
- Produces three RPC commands, each with the args of the command it wraps plus `dryRun: bool`:
  - `"update_trip_cascade"` -> `CascadeResult`
  - `"create_trip_cascade"` -> `CascadeResult`, with **no** `odometer` arg
  - `"delete_trip_cascade"` -> `CascadePlan`

  All responses are camelCase.

- [ ] **Step 1: Write the failing test**

Model it on `recalculate_odometers_over_rpc_returns_camelcase_changes` at `:1515`.

```rust
#[test]
fn update_trip_cascade_over_rpc_returns_a_camelcase_plan() {
    // The modal reads deltaFromDistance and deltaFromRepair by name. A
    // snake_case response would render an empty explanation.
    let (state, vehicle_id, trip_id) = seed_two_trip_chain();

    let v = dispatch(
        &state,
        "update_trip_cascade",
        serde_json::json!({
            "id": trip_id,
            "startDatetime": "2026-03-01T00:00:00",
            "endDatetime": "2026-03-01T00:00:00",
            "origin": "A",
            "destination": "B",
            "distanceKm": 60.0,
            "odometer": 50050.0,
            "purpose": "work",
            "dryRun": true
        }),
    )
    .unwrap();

    assert_eq!(v["plan"]["delta"], 10.0);
    assert_eq!(v["plan"]["deltaFromDistance"], 10.0);
    assert_eq!(v["plan"]["deltaFromRepair"], 0.0);
    assert_eq!(v["plan"]["yearEndOdometerMoved"], true);
    assert_eq!(v["plan"]["changes"][0]["newOdometer"], 50130.0);
    assert!(v["trip"].is_null(), "a dry run saves nothing");
    let _ = vehicle_id;
}

#[test]
fn create_trip_cascade_over_rpc_takes_no_odometer() {
    // The backend derives the odometer from the anchor. An `odometer` field in
    // the args would let the browser overrule the book (ADR-008).
    let (state, vehicle_id, _trip_id) = seed_two_trip_chain();

    let v = dispatch(
        &state,
        "create_trip_cascade",
        serde_json::json!({
            "vehicleId": vehicle_id,
            "startDatetime": "2026-03-01T12:00:00",
            "endDatetime": "2026-03-01T13:00:00",
            "origin": "B",
            "destination": "C",
            "distanceKm": 25.0,
            "purpose": "work",
            "dryRun": true
        }),
    )
    .unwrap();

    assert_eq!(v["plan"]["newOdometer"], 50075.0);
    assert_eq!(v["plan"]["delta"], 25.0);
    assert_eq!(v["plan"]["deltaFromRepair"], 0.0);
}

#[test]
fn delete_trip_cascade_over_rpc_returns_a_plan() {
    let (state, _vehicle_id, trip_id) = seed_two_trip_chain();

    let v = dispatch(
        &state,
        "delete_trip_cascade",
        serde_json::json!({ "id": trip_id, "dryRun": true }),
    )
    .unwrap();

    assert_eq!(v["delta"], -50.0);
    assert_eq!(v["changes"][0]["newOdometer"], 50070.0);
}

#[test]
fn update_trip_cascade_over_rpc_requires_the_dry_run_field() {
    // Defaulting dryRun to false would make a missing field write the book.
    let (state, _vehicle_id, trip_id) = seed_two_trip_chain();

    let result = dispatch(
        &state,
        "update_trip_cascade",
        serde_json::json!({
            "id": trip_id,
            "startDatetime": "2026-03-01T00:00:00",
            "endDatetime": "2026-03-01T00:00:00",
            "origin": "A",
            "destination": "B",
            "distanceKm": 60.0,
            "odometer": 50050.0,
            "purpose": "work"
        }),
    );

    assert!(result.is_err());
}
```

Write `seed_two_trip_chain` next to the existing dispatcher test helpers, following whatever helper `recalculate_odometers_over_rpc_returns_camelcase_changes` already uses to build a `ServerState`. It seeds a vehicle with `initial_odometer` 50000 and two 2026 trips: 50 km at 50050 and 70 km at 50120. It returns the state, the vehicle id and the **first** trip's id.

- [ ] **Step 2: Run the tests and watch them fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core cascade_over_rpc
```

Expected: FAIL, `Unknown command: update_trip_cascade`.

- [ ] **Step 3: Add the arm**

```rust
        "update_trip_cascade" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
                start_datetime: String,
                end_datetime: String,
                origin: String,
                destination: String,
                distance_km: f64,
                odometer: f64,
                purpose: String,
                fuel_liters: Option<f64>,
                fuel_cost_eur: Option<f64>,
                full_tank: Option<bool>,
                energy_kwh: Option<f64>,
                energy_cost_eur: Option<f64>,
                full_charge: Option<bool>,
                soc_override_percent: Option<f64>,
                other_costs_eur: Option<f64>,
                other_costs_note: Option<String>,
                dry_run: bool,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::update_trip_cascade_internal(
                &state.db,
                &state.app_state,
                a.id,
                a.start_datetime,
                a.end_datetime,
                a.origin,
                a.destination,
                a.distance_km,
                a.odometer,
                a.purpose,
                a.fuel_liters,
                a.fuel_cost_eur,
                a.full_tank,
                a.energy_kwh,
                a.energy_cost_eur,
                a.full_charge,
                a.soc_override_percent,
                a.other_costs_eur,
                a.other_costs_note,
                a.dry_run,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
```

Add the other two arms beside it, copying the arg structs from the `"create_trip"` and
`"delete_trip"` arms and adding `dry_run: bool`. The create arm has no `odometer` field:
the backend derives it, so accepting one would let the browser overrule the book.

Check whether `dispatcher_async.rs` mirrors the command list. If it does, add all three
arms there.

- [ ] **Step 4: Run the tests and watch them pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core cascade_over_rpc
```

Expected: PASS, 4 tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(rpc): expose the three cascade commands with a dry run"
```

---

### Task 5: The frontend API client and types

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts` (add after `updateTrip`, which ends at `:164`)

**Interfaces:**
- Consumes: the `"update_trip_cascade"` RPC command (Task 4).
- Produces:
  - `export interface OdometerChange { tripId: string; tripNumber: number; oldOdometer: number; newOdometer: number }`
  - `export interface CascadePlan { newOdometer: number; newDistanceKm: number; delta: number; deltaFromDistance: number; deltaFromRepair: number; repairCrossesYear: boolean; yearEndOdometerMoved: boolean; changes: OdometerChange[] }`
  - `export interface CascadeResult { trip: Trip | null; plan: CascadePlan }`
  - `export async function updateTripCascade(...same params as updateTrip..., dryRun: boolean): Promise<CascadeResult>`
  - `export async function createTripCascade(...same params as createTrip but WITHOUT odometer..., dryRun: boolean): Promise<CascadeResult>`
  - `export async function deleteTripCascade(id: string, dryRun: boolean): Promise<CascadePlan>`

- [ ] **Step 1: Add the types**

In `src/lib/types.ts`, next to the other trip types:

```ts
/** One row a cascading save moves. Mirrors Rust `OdometerChange`. */
export interface OdometerChange {
	tripId: string;
	tripNumber: number;
	oldOdometer: number;
	newOdometer: number;
}

/**
 * What a cascading save would do (task 81). `delta` is reported in two parts
 * so the modal can say which of them the user asked for:
 * `delta === deltaFromDistance + deltaFromRepair`.
 */
export interface CascadePlan {
	newOdometer: number;
	newDistanceKm: number;
	delta: number;
	/** From the distance the user typed. */
	deltaFromDistance: number;
	/** From repairing the edited row's pre-existing span error. */
	deltaFromRepair: number;
	/** The repair is measured against the previous year's last odometer. */
	repairCrossesYear: boolean;
	/** The year's last odometer moves. */
	yearEndOdometerMoved: boolean;
	/** That, and a later year has trips, so a chain that lined up will not any
	 *  more. Only this one is worth showing: appending to the newest year moves
	 *  the year end every time and breaks nothing. */
	nextYearChainBreaks: boolean;
	/** Rows AFTER the edited one. The edited row is `newOdometer` above. */
	changes: OdometerChange[];
}

export interface CascadeResult {
	trip: Trip | null;
	plan: CascadePlan;
}
```

- [ ] **Step 2: Add the client call**

In `src/lib/api.ts`:

```ts
/**
 * Save a trip and move the odometer of every later row of the same year.
 *
 * With `dryRun: true` nothing is written and the returned plan is what fills
 * the confirmation modal. Call it again with `dryRun: false` to apply. The
 * apply recomputes from the stored book, so the two calls can disagree if the
 * book moved in between -- which is the point.
 */
export async function updateTripCascade(
	id: string,
	startDatetime: string,
	endDatetime: string,
	origin: string,
	destination: string,
	distanceKm: number,
	odometer: number,
	purpose: string,
	fuelLiters: number | null | undefined,
	fuelCostEur: number | null | undefined,
	fullTank: boolean | null | undefined,
	energyKwh: number | null | undefined,
	energyCostEur: number | null | undefined,
	fullCharge: boolean | null | undefined,
	socOverridePercent: number | null | undefined,
	otherCostsEur: number | null | undefined,
	otherCostsNote: string | null | undefined,
	dryRun: boolean
): Promise<CascadeResult> {
	return await apiCall('update_trip_cascade', {
		id,
		startDatetime,
		endDatetime,
		origin,
		destination,
		distanceKm,
		odometer,
		purpose,
		fuelLiters,
		fuelCostEur,
		fullTank,
		energyKwh,
		energyCostEur,
		fullCharge,
		socOverridePercent,
		otherCostsEur,
		otherCostsNote,
		dryRun
	});
}
```

Add the two other calls beside it. `createTripCascade` mirrors `createTrip`
([api.ts:81](../../../src/lib/api.ts)) with the `odometer` parameter **removed** and
`dryRun` appended -- the backend derives the odometer, so passing one would let the
browser overrule the book. `deleteTripCascade` is two lines:

```ts
/**
 * Remove a trip and close the gap it leaves in the odometer chain.
 * With `dryRun: true` nothing is deleted and the returned plan is what fills
 * the confirmation modal.
 */
export async function deleteTripCascade(id: string, dryRun: boolean): Promise<CascadePlan> {
	return await apiCall('delete_trip_cascade', { id, dryRun });
}
```

Add `CascadePlan` and `CascadeResult` to the type import at the top of `api.ts`.

- [ ] **Step 3: Check it compiles**

```bash
npm run check
```

Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts
git commit -m "feat(api): client for the three cascade commands"
```

---

### Task 6: The confirmation modal

**Files:**
- Create: `src/lib/components/OdometerCascadeModal.svelte`
- Modify: `src/lib/i18n/sk/index.ts`
- Modify: `src/lib/i18n/en/index.ts`

**Interfaces:**
- Consumes: `CascadePlan` and `OdometerChange` (Task 5), and a `Trip[]` for the date and route of each changed row.
- Produces: a component with props `plan: CascadePlan`, `trips: Trip[]`, `onConfirm: () => void`, `onCancel: () => void`.

- [ ] **Step 1: Add the Slovak strings**

In `src/lib/i18n/sk/index.ts`, add a `cascade` block inside `trips`:

```ts
		// Odometer cascade on save (Task 81)
		cascade: {
			title: 'Posun tachometra',
			summary: 'Táto zmena posunie {count:number} nasledujúcich jázd o {delta:string} km.',
			summaryInsert: 'Nová jazda posunie {count:number} nasledujúcich jázd o {delta:string} km.',
			summaryDelete: 'Zmazanie posunie {count:number} nasledujúcich jázd o {delta:string} km.',
			fromInsert: '{delta:string} km -- vzdialenosť novej jazdy',
			fromDelete: '{delta:string} km -- vzdialenosť zmazanej jazdy',
			confirmDelete: 'Zmazať a posunúť',
			fromDistance: '{delta:string} km -- vaša zmena vzdialenosti ({oldKm:string} -> {newKm:string} km)',
			fromRepair: '{delta:string} km -- oprava tachometra tejto jazdy: nesedel so zapísanou vzdialenosťou',
			fromRepairCrossYear: '{delta:string} km -- oprava tachometra tejto jazdy oproti koncu predchádzajúceho roka',
			yearEndWarning: 'Posunie sa aj posledná jazda roka, takže prvá jazda nasledujúceho roka bude hlásiť nesúlad tachometra.',
			columnTrip: 'Jazda',
			columnDate: 'Dátum',
			columnRoute: 'Trasa',
			columnOld: 'Tachometer teraz',
			columnNew: 'Tachometer po zmene',
			confirm: 'Uložiť a posunúť',
			cancel: 'Zrušiť',
		},
```

- [ ] **Step 2: Add the English strings**

The same keys in `src/lib/i18n/en/index.ts`:

```ts
		// Odometer cascade on save (Task 81)
		cascade: {
			title: 'Odometer shift',
			summary: 'This change moves {count} following trips by {delta} km.',
			summaryInsert: 'The new trip moves {count} following trips by {delta} km.',
			summaryDelete: 'Deleting moves {count} following trips by {delta} km.',
			fromInsert: '{delta} km -- the distance of the new trip',
			fromDelete: '{delta} km -- the distance of the deleted trip',
			confirmDelete: 'Delete and shift',
			fromDistance: '{delta} km -- your change to the distance ({oldKm} -> {newKm} km)',
			fromRepair: '{delta} km -- correcting the odometer of this trip, which did not match its recorded distance',
			fromRepairCrossYear: '{delta} km -- correcting the odometer of this trip against the end of the previous year',
			yearEndWarning: 'The last trip of the year moves too, so the first trip of the next year will report an odometer mismatch.',
			columnTrip: 'Trip',
			columnDate: 'Date',
			columnRoute: 'Route',
			columnOld: 'Odometer now',
			columnNew: 'Odometer after',
			confirm: 'Save and shift',
			cancel: 'Cancel',
		},
```

- [ ] **Step 3: Regenerate the i18n types**

```bash
npm run i18n
```

Expected: `src/lib/i18n/i18n-types.ts` gains the `cascade` keys. Nothing else regenerates this file.

- [ ] **Step 4: Write the component**

`src/lib/components/OdometerCascadeModal.svelte`. Copy the overlay, modal and button CSS from `ConfirmModal.svelte` verbatim, then add the table style below. `ConfirmModal` itself takes a single `message` string, which cannot hold a table of rows.

```svelte
<script lang="ts">
	import LL from '$lib/i18n/i18n-svelte';
	import type { CascadePlan, Trip } from '$lib/types';

	export let plan: CascadePlan;
	export let trips: Trip[];
	/** The distance the edited row recorded before this change. Unused for
	 *  'insert' and 'delete', where there is no before-and-after distance. */
	export let oldDistanceKm: number = 0;
	/** Which write is being confirmed. It picks the summary and the breakdown
	 *  line, and it is the only thing that differs between the three. */
	export let kind: 'edit' | 'insert' | 'delete' = 'edit';
	export let onConfirm: () => void;
	export let onCancel: () => void;

	// The plan names rows by id only. The date and the route come from the
	// trips the grid already holds, so the backend does not repeat them.
	$: byId = new Map(trips.map((t) => [t.id, t]));

	function signed(value: number): string {
		return `${value >= 0 ? '+' : ''}${value.toFixed(1)}`;
	}

	function shortDate(trip: Trip | undefined): string {
		if (!trip) return '';
		const [year, month, day] = trip.startDatetime.slice(0, 10).split('-');
		return `${day}.${month}.${year}`;
	}

	// The repair line is shown only when the edited row was already broken.
	$: showRepair = Math.abs(plan.deltaFromRepair) >= 0.001;
</script>

<div
	class="modal-overlay"
	on:click={onCancel}
	on:keydown={(e) => e.key === 'Escape' && onCancel()}
	role="button"
	tabindex="0"
>
	<div
		class="modal"
		on:click|stopPropagation
		on:keydown={() => {}}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		data-testid="cascade-modal"
	>
		<h2>{$LL.trips.cascade.title()}</h2>
		<div class="modal-content">
			<p class="summary" data-testid="cascade-summary">
				{#if kind === 'insert'}
					{$LL.trips.cascade.summaryInsert({
						count: plan.changes.length,
						delta: signed(plan.delta)
					})}
				{:else if kind === 'delete'}
					{$LL.trips.cascade.summaryDelete({
						count: plan.changes.length,
						delta: signed(plan.delta)
					})}
				{:else}
					{$LL.trips.cascade.summary({
						count: plan.changes.length,
						delta: signed(plan.delta)
					})}
				{/if}
			</p>
			<ul class="breakdown">
				<li>
					{#if kind === 'insert'}
						{$LL.trips.cascade.fromInsert({ delta: signed(plan.deltaFromDistance) })}
					{:else if kind === 'delete'}
						{$LL.trips.cascade.fromDelete({ delta: signed(plan.deltaFromDistance) })}
					{:else}
						{$LL.trips.cascade.fromDistance({
							delta: signed(plan.deltaFromDistance),
							oldKm: oldDistanceKm.toFixed(0),
							newKm: plan.newDistanceKm.toFixed(0)
						})}
					{/if}
				</li>
				{#if showRepair}
					<li data-testid="cascade-repair">
						{plan.repairCrossesYear
							? $LL.trips.cascade.fromRepairCrossYear({ delta: signed(plan.deltaFromRepair) })
							: $LL.trips.cascade.fromRepair({ delta: signed(plan.deltaFromRepair) })}
					</li>
				{/if}
			</ul>
			{#if plan.nextYearChainBreaks}
				<p class="year-end" data-testid="cascade-year-end">
					{$LL.trips.cascade.yearEndWarning()}
				</p>
			{/if}
			<div class="changes">
				<table>
					<thead>
						<tr>
							<th>{$LL.trips.cascade.columnTrip()}</th>
							<th>{$LL.trips.cascade.columnDate()}</th>
							<th>{$LL.trips.cascade.columnRoute()}</th>
							<th class="number">{$LL.trips.cascade.columnOld()}</th>
							<th class="number">{$LL.trips.cascade.columnNew()}</th>
						</tr>
					</thead>
					<tbody>
						{#each plan.changes as change (change.tripId)}
							<tr>
								<td>{change.tripNumber}</td>
								<td>{shortDate(byId.get(change.tripId))}</td>
								<td class="route">
									{byId.get(change.tripId)?.origin ?? ''} -&gt;
									{byId.get(change.tripId)?.destination ?? ''}
								</td>
								<td class="number">{change.oldOdometer.toFixed(0)}</td>
								<td class="number">{change.newOdometer.toFixed(0)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
		<div class="modal-actions">
			<button class="button-small" on:click={onCancel} data-testid="cascade-cancel">
				{$LL.trips.cascade.cancel()}
			</button>
			<button class="button-small" on:click={onConfirm} data-testid="cascade-confirm">
				{kind === 'delete' ? $LL.trips.cascade.confirmDelete() : $LL.trips.cascade.confirm()}
			</button>
		</div>
	</div>
</div>

<style>
	/* Copy .modal-overlay, .modal, h2, .modal-content, .modal-actions and the
	   button rules from ConfirmModal.svelte verbatim, then: */
	.summary {
		font-weight: 600;
	}
	.breakdown {
		margin: 0 0 0.75rem 0;
		padding-left: 1.25rem;
	}
	.year-end {
		margin: 0 0 0.75rem 0;
	}
	.changes {
		max-height: 40vh;
		overflow-y: auto;
	}
	.changes table {
		width: 100%;
		border-collapse: collapse;
	}
	.changes th,
	.changes td {
		padding: 0.25rem 0.5rem;
		text-align: left;
	}
	.changes .number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.changes .route {
		max-width: 20rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
```

- [ ] **Step 5: Check it compiles**

```bash
npm run check
```

Expected: no new errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/OdometerCascadeModal.svelte src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts
git commit -m "feat(trips): a modal that states the odometer shift before it is written"
```

---

### Task 7: The grid uses the cascade save

**Files:**
- Modify: `src/lib/components/TripGrid.svelte` (`handleSaveNew` at `:262-305`, `handleUpdate` at `:306-340`, `handleDelete` at `:341-357`)
- Modify: `src/lib/components/TripRow.svelte` (the delete confirmation at `:557`)

**Interfaces:**
- Consumes: `updateTripCascade`, `createTripCascade`, `deleteTripCascade` (Task 5), `OdometerCascadeModal` (Task 6).
- Produces: no new exports. All three write paths go through the dry run and the modal.

- [ ] **Step 1: Add the modal state and the new save path**

Replace `handleUpdate` with this. Keep the `import { updateTrip }` line: `handleSaveNew` still uses `createTrip`, and `updateTrip` may still be imported elsewhere in the file -- remove it only if nothing else references it.

```ts
	// The write waiting on the cascade modal (task 81). Null when no modal is up.
	// One shape for all three kinds, so there is one modal and one gate.
	let pendingCascade: {
		kind: 'edit' | 'insert' | 'delete';
		plan: CascadePlan;
		oldDistanceKm: number;
		apply: () => Promise<void>;
	} | null = null;

	/**
	 * A plan needs the user's approval when it moves another row, or when it
	 * breaks the chain of a year that actually has trips. Appending to the
	 * newest year satisfies neither, so it stays a silent write.
	 */
	function needsApproval(plan: CascadePlan): boolean {
		return plan.changes.length > 0 || plan.nextYearChainBreaks;
	}

	async function handleUpdate(trip: Trip, tripData: Partial<Trip>): Promise<boolean> {
		try {
			// Ask what this save would do before doing it. The answer is also
			// the save itself when nothing else moves.
			const preview = await updateTripCascade(
				trip.id,
				tripData.startDatetime!,
				tripData.endDatetime!,
				tripData.origin!,
				tripData.destination!,
				tripData.distanceKm!,
				tripData.odometer!,
				tripData.purpose!,
				tripData.fuelLiters,
				tripData.fuelCostEur,
				tripData.fullTank,
				tripData.energyKwh,
				tripData.energyCostEur,
				tripData.fullCharge,
				trip.socOverridePercent, // Preserve existing SoC override
				tripData.otherCostsEur,
				tripData.otherCostsNote,
				true
			);

			if (!needsApproval(preview.plan)) {
				// Nothing else is affected: an edit to a purpose, a time or the
				// litres. Write it without asking, and let the row close.
				await applyCascade(trip, tripData);
				return true;
			}

			pendingCascade = {
				kind: 'edit',
				plan: preview.plan,
				oldDistanceKm: trip.distanceKm,
				apply: () => applyCascade(trip, tripData)
			};
			// The modal decides. The row stays open until it does.
			return false;
		} catch (error) {
			console.error('Failed to update trip:', error);
			toast.error($LL.toast.errorUpdateTrip());
			return false;
		}
	}

	/**
	 * Write the save for real. Planned again on the backend from the stored
	 * book, so this does not replay the numbers the modal showed.
	 */
	async function applyCascade(trip: Trip, tripData: Partial<Trip>) {
		await updateTripCascade(
			trip.id,
			tripData.startDatetime!,
			tripData.endDatetime!,
			tripData.origin!,
			tripData.destination!,
			tripData.distanceKm!,
			tripData.odometer!,
			tripData.purpose!,
			tripData.fuelLiters,
			tripData.fuelCostEur,
			tripData.fullTank,
			tripData.energyKwh,
			tripData.energyCostEur,
			tripData.fullCharge,
			trip.socOverridePercent,
			tripData.otherCostsEur,
			tripData.otherCostsNote,
			false
		);

		// The existing refresh path. loadTrips(false) leaves TripGrid mounted and
		// the each block is keyed, so the shifted rows repaint in place and the
		// scroll position survives (task 81, R6).
		await onTripsChanged();
		await loadRoutes();
		await loadPurposes();
		await loadPlaces();
		triggerReceiptRefresh();
	}

	/**
	 * `TripRow` closes itself only when the save resolves true. A cancelled
	 * cascade writes nothing, so the row stays open on the values the user
	 * typed (task 81, R5).
	 */
	async function confirmCascade() {
		if (!pendingCascade) return;
		const { apply } = pendingCascade;
		pendingCascade = null;
		try {
			await apply();
		} catch (error) {
			console.error('Failed to update trip:', error);
			toast.error($LL.toast.errorUpdateTrip());
		}
	}

	function cancelCascade() {
		// Nothing was written, including the edited row itself.
		pendingCascade = null;
	}
```

`handleSaveNew` takes the same shape. Replace its `createTrip` call with
`createTripCascade` -- **drop the `odometer` argument**, the backend derives it -- run it
once with `dryRun: true`, and either write straight away or park it in `pendingCascade`
with `kind: 'insert'`. Its `apply` closure runs the same call with `dryRun: false` and
then the existing refresh block. It returns a boolean, like `handleUpdate`.

`handleDelete` becomes:

```ts
	async function handleDelete(id: string) {
		try {
			const plan = await deleteTripCascade(id, true);
			if (!needsApproval(plan)) {
				await applyDelete(id);
				return;
			}
			pendingCascade = {
				kind: 'delete',
				plan,
				oldDistanceKm: 0,
				apply: () => applyDelete(id)
			};
		} catch (error) {
			console.error('Failed to delete trip:', error);
			toast.error($LL.toast.errorDeleteTrip());
		}
	}

	async function applyDelete(id: string) {
		await deleteTripCascade(id, false);
		await onTripsChanged();
		// The book is the autocomplete's source, so a place whose last trip just
		// went is no longer a place.
		await loadPlaces();
		triggerReceiptRefresh();
	}
```

- [ ] **Step 2: Render the modal**

At the end of the markup, beside whatever other modals the file already renders:

```svelte
{#if pendingCascade}
	<OdometerCascadeModal
		plan={pendingCascade.plan}
		kind={pendingCascade.kind}
		{trips}
		oldDistanceKm={pendingCascade.oldDistanceKm}
		onConfirm={confirmCascade}
		onCancel={cancelCascade}
	/>
{/if}
```

Add the imports at the top:

```ts
	import OdometerCascadeModal from './OdometerCascadeModal.svelte';
	import { updateTripCascade, createTripCascade, deleteTripCascade } from '$lib/api';
	import type { CascadePlan } from '$lib/types';
```

- [ ] **Step 3: Drop the second delete dialog**

`TripRow` shows its own confirmation before it calls `onDelete`
([TripRow.svelte:557](../../../src/lib/components/TripRow.svelte)). The cascade modal now
states what the delete does, and two dialogs for one action is worse than one dialog that
says more (task 81, R5). Remove the `confirmStore.show({...})` wrapper so the delete
button calls `onDelete(trip.id)` directly, and drop the `confirmStore` import if nothing
else in the file uses it.

A delete that moves no row then writes with no confirmation at all. That is a deliberate
change. If you would rather keep the old dialog for that case, it is a one-line branch in
`handleDelete`, not a redesign -- decide before implementing.

- [ ] **Step 4: Close the row only when the write happened**

`handleUpdate` now returns `Promise<boolean>`. It is already wired to the `onSave` prop of
`<TripRow>` in this file, so nothing changes at the call site; Task 8 changes the prop's
declared type and makes `TripRow.handleSave` await it.

`handleSaveNew` must return a boolean too: `true` after a successful `createTrip`, `false`
in its `catch`.

After `confirmCascade` resolves, the row is still open, because the save resolved false.
Close it there, using whatever the file already tracks the open row with. Read the file
and use that mechanism; do not add a second one.

- [ ] **Step 5: Check it compiles**

```bash
npm run check
```

Expected: no new errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/TripGrid.svelte src/lib/components/TripRow.svelte
git commit -m "feat(trips): every write goes through the cascade and its modal"
```

---

### Task 8: The row editor stops doing odometer arithmetic

**Files:**
- Modify: `src/lib/components/TripRow.svelte`

**Interfaces:**
- Consumes: nothing new.
- Produces: `TripRow` sends the typed km and the typed odometer unchanged. It no longer clamps, derives or guesses either of them.

- [ ] **Step 1: Delete the clamps**

Remove, in `TripRow.svelte`:

- `handleOdoBlur` (`:398-410`) and the `on:change={handleOdoBlur}` binding on the odometer input at `:645`;
- the clamp block inside `handleSave` (`:483-520`), so it becomes a plain `onSave({ ...formData })`;
- the `manualOdoEdit` and `odoFollowsKm` declarations (`:111`, `:157`), `resetEditSessionFlags` (`:164-167`), and every read of those flags;
- the km derivation inside `handleOdoChange` (`:430-446`), so it sets `formData.odometer` and requests a preview only.

Keep `applyPreviewOdometer`. It fills the ODO field from the backend preview while the row is open, which is the live feedback the user sees before saving. Change its guard from `manualOdoEdit || !odoFollowsKm` to "the preview answers the km the field holds now", which is the tolerance check the function already does at `:185`.

Keep `manualKmEdit`. It has nothing to do with the odometer: it stops a route auto-fill from overwriting a typed distance (`:266`, `:375`).

- [ ] **Step 2: Re-seed `formData` when the row is not being edited**

After the `formData` declaration, add:

```ts
	// A cascade moves the odometer of rows the user never opened, and this
	// component survives that: the grid keys its rows by trip id, so one
	// instance serves display and every edit of that row. formData was seeded
	// once at construction, so without this a shifted row would open with the
	// pre-cascade number and write it back (task 81, R6).
	//
	// Guarded on !isEditing so it can never fight the user mid-edit, and it
	// depends on `trip` alone -- a reactive block that both read and wrote
	// formData would re-trigger itself.
	$: if (trip && !isEditing) {
		formData = {
			startDatetime: toDatetimeLocal(trip.startDatetime),
			endDatetime: toDatetimeLocal(trip.endDatetime),
			origin: trip.origin,
			destination: trip.destination,
			distanceKm: trip.distanceKm,
			odometer: trip.odometer,
			purpose: trip.purpose,
			fuelLiters: trip.fuelLiters,
			fuelCostEur: trip.fuelCostEur,
			fullTank: trip.fullTank,
			energyKwh: trip.energyKwh,
			energyCostEur: trip.energyCostEur,
			fullCharge: trip.fullCharge,
			socOverridePercent: trip.socOverridePercent,
			otherCostsEur: trip.otherCostsEur,
			otherCostsNote: trip.otherCostsNote ?? ''
		};
	}
```

`handleCancel` already rebuilds `formData` from `trip` by hand (`:527-545`). Replace that
body with `isEditing = false`. The block above depends on `isEditing`, so the assignment
re-runs it and restores the fields; `formData` is only written inside the block and never
read there, so it cannot re-trigger itself. Verify this in the browser before you delete
the hand rebuild: open a row, change the km, press Cancel, and confirm the field shows the
stored value again. If it does not, keep the hand rebuild and let the block cover the
cascade case alone.

- [ ] **Step 3: Make `handleSave` wait for the write**

A cancelled cascade writes nothing, so the row must stay open on the values the user
typed (task 81, R5). Change the prop at `:59` and the handler:

```ts
	export let onSave: (tripData: Partial<Trip>) => Promise<boolean>;
```

```ts
	async function handleSave() {
		// The grid answers false when nothing was written -- a cascade the user
		// cancelled. The row then stays open on what the user typed.
		const saved = await onSave({ ...formData });
		if (!saved) return;
		isEditing = false;
		if (!isNew) {
			onEditEnd();
		}
	}
```

- [ ] **Step 4: Check it compiles**

```bash
npm run check
```

Expected: no new errors.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/TripRow.svelte
git commit -m "refactor(trips): the row editor stops guessing the odometer"
```

---

### Task 9: Rewrite the km/odo integration spec

**Files:**
- Modify: `tests/integration/specs/tier1/km-odo-bidirectional.spec.ts`

**Interfaces:**
- Consumes: the behaviour of Tasks 7 and 8.
- Produces: nothing. This task states what the two fields do now.

- [ ] **Step 1: Read the spec and list what it pins**

```bash
grep -n "^\s*it(" tests/integration/specs/tier1/km-odo-bidirectional.spec.ts
```

Three commits built this file in one day (`d101c08`, `9d67262`, `c818fe7`), and all three pin `handleOdoBlur` and the `handleSave` clamp, which Task 8 deletes. Every case must be restated, not deleted and not patched until green.

- [ ] **Step 2: Restate each case**

For each `it(...)` in the list, decide and write the new expectation:

- **km typed -> the ODO follows.** Unchanged behaviour, keep the assertion. The value now comes from the preview alone.
- **ODO typed -> the km follows.** The field no longer changes as you type. The km is derived by the backend on save. Assert the saved row, not the field.
- **ODO typed below the anchor.** There is no clamp any more. Assert that the row saves as typed and that the span warning appears on it -- the warning is the app's answer to a broken chain (ADR-042), not a silent correction.
- **Save beats the preview.** The backend recomputes from the stored row, so the typed km wins. Assert the saved distance equals the typed distance.

- [ ] **Step 3: Build and run the spec**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier1/km-odo-bidirectional.spec.ts
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add tests/integration/specs/tier1/km-odo-bidirectional.spec.ts
git commit -m "test(trips): restate the km/odo spec for the backend-owned odometer"
```

---

### Task 10: Integration test for the cascade flow

**Files:**
- Create: `tests/integration/specs/tier2/odometer-cascade.spec.ts`

**Interfaces:**
- Consumes: Tasks 6, 7 and 8, and the `data-testid` hooks `cascade-modal`, `cascade-summary`, `cascade-repair`, `cascade-year-end`, `cascade-confirm`, `cascade-cancel`.
- Produces: nothing.

- [ ] **Step 1: Write the spec**

Follow the shape of `tests/integration/specs/tier2/odometer-chain-warnings.spec.ts`, which already seeds a chain and reads the odo cells. Do not re-test the arithmetic -- Task 1 owns it. Test the flow.

```ts
describe('Odometer cascade on save', () => {
	it('moves every later row when a distance is edited', async () => {
		// Seed three rows, edit the middle km, confirm the modal, then read the
		// last row's ODO cell. The number is the backend's; this asserts the
		// flow reached it and the grid repainted.
	});

	it('writes nothing when the modal is cancelled', async () => {
		// Cancel must leave the edited row unchanged too, not only the later
		// rows: the two are one write.
	});

	it('keeps the scroll position after a cascade', async () => {
		// The regression the user asked for by name. TripGrid must not unmount.
		const before = await browser.execute(() => window.scrollY);
		// ... edit and confirm ...
		const after = await browser.execute(() => window.scrollY);
		expect(after).toBe(before);
	});

	it('does not open the modal when only the purpose changes', async () => {
		// The common edit must stay a single silent write.
	});

	it('shifts the later rows when a trip is inserted mid-year', async () => {
		// Add a row dated between two existing ones, confirm, then read the last
		// row's ODO cell.
	});

	it('does not open the modal when a trip is appended to the newest year', async () => {
		// The daily action. No row moves and there is no next year, so this must
		// stay a single silent write.
	});

	it('closes the gap when a trip is deleted', async () => {
		// Delete a middle row, confirm, and read the following row's ODO cell.
		// It must equal the deleted row's start plus its own distance.
	});

	it('deletes nothing when the delete modal is cancelled', async () => {
		// The row and every later odometer must be exactly as they were.
	});
});
```

Seed enough rows that the grid scrolls. If the page does not scroll at the default window size, scroll the grid container instead and read its `scrollTop`; check which element actually scrolls before writing the assertion.

- [ ] **Step 2: Run it**

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/odometer-cascade.spec.ts
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/integration/specs/tier2/odometer-cascade.spec.ts
git commit -m "test(trips): cover the cascade modal, cancel and the scroll position"
```

---

### Task 11: The documentation this changes

**Files:**
- Modify: `DECISIONS.md`
- Modify: `CHANGELOG.md`
- Modify: `_tasks/index.md`

**Interfaces:**
- Consumes: everything above.
- Produces: nothing.

- [ ] **Step 1: Supersede ADR-045**

`ADR-045` says "Nothing calls `recalculate_odometers` automatically" and "A save now writes exactly the row the user edited". The second sentence is now false. Add a **Superseded by ADR-046** line to it and correct that sentence. Keep everything else: the ban on the automatic full-year rebase still stands on its 69 and 68 row measurement, and `recalculate_odometers` is still UI-less.

- [ ] **Step 2: Write ADR-046**

At the top of `DECISIONS.md`, newest first. It must record:

- the delta rule and why the km wins over the odometer;
- that the shift touches only rows after the edit, and why rows before it are unreachable (`start[K] = odometer[K-1]`);
- that it repairs the edited row's own span, and that the modal decomposes the delta because of the 2025 row 1 case: anchor 38056.5, stored 38145, 88 km recorded, so a 2 km edit shifts by 1.5;
- that the walk stops at the year end, that this matches the removed `recalculateAllOdo`, and that the boundary break is left visible as a span warning;
- that inserting shifts by the new row's distance, and that appending to the newest year
  therefore moves nothing and asks nothing;
- that deleting shifts by the removed row's **span**, not its distance, because the span
  is what the chain loses;
- that `nextYearChainBreaks` and not `yearEndOdometerMoved` decides whether to warn;
- that a re-dated row cascades nothing;
- what separates ADR-046 from ADR-045: a delta shift never touches an untouched row's *relationship* to its neighbour, so it creates and clears no span warning, while a rebase erases them all.

- [ ] **Step 3: Update the changelog**

Add to `[Unreleased]` under `### Changed`, in user words, in Slovak if that is what the file uses:

> Editing a trip's distance, adding a trip and deleting a trip now move the odometer of
> every later trip in the same year. The app shows the exact rows and the shift, and
> writes nothing until you confirm. Adding a trip at the end of the newest year is
> unchanged: it asks nothing.

- [ ] **Step 4: Update the task index**

The row for task 81 was added to **Active Tasks** when this plan was committed. Move it to **Completed Tasks** with today's date, and update the folder link if the folder moved to `_done/`.

- [ ] **Step 5: Run everything**

```bash
npm run test:all
```

Expected: PASS. This is the full sweep, roughly 10 minutes. Run it only now, after every focused run above has passed.

- [ ] **Step 6: Commit**

```bash
git add DECISIONS.md CHANGELOG.md _tasks/index.md
git commit -m "docs: record the odometer cascade decision and supersede ADR-045"
```

---

## Self-review notes

**Spec coverage.** R1 is Tasks 3 and 4. R2 is Task 1, and its re-dating limit is Task 3.
R3 is Task 1 (`delta_from_repair`, `repair_crosses_year`) and Task 6 (the two breakdown
lines). R4 is Task 1 (`year_end_odometer_moved`), Task 3 (the boundary test) and Task 6
(the warning line). R5 is Tasks 6 and 7, with the all-or-nothing write in Task 2. R6 is
Task 8 (the `formData` re-seed) and Task 10 (the scroll assertion). R7 is Tasks 8 and 9.
The documentation section is Task 11.

R8 (insert) is Task 1 `plan_insert_cascade`, Task 2 `create_trip_with_odometer_shift`,
Task 3 `create_trip_cascade_internal`, Task 4, Task 5 `createTripCascade`, Task 6 the
`insert` strings, Task 7 `handleSaveNew`, and Task 10. R9 (delete) is the matching set
plus the removal of the second dialog in Task 7 Step 3.

**Known gap left on purpose.** A re-dated row cascades nothing, because it moves in
`trip_order` and two positions shift rather than one. It is listed as open in
[01-task.md](./01-task.md) and pinned by a test in Task 3.

**Order.** Tasks 1 to 4 are backend and can land before any frontend work. Task 8 must not
land before Task 7, or the row editor would send a raw odometer to a save path that still
clamps it. Within Task 3, the `build_updated_trip` and `build_new_trip` extractions come
before the commands that call them.
