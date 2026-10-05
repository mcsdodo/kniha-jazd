**Date:** 2026-10-05
**Subject:** Implementation plan: home mark on places, journey grouping, read-only MCP endpoint
**Status:** Complete

# Home Place, Journeys and Read-Only MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mark one place as home, group trip legs into journeys away from home in core, and serve three read-only tools over MCP at `/mcp`.

**Architecture:** A new column `places.is_home` holds the home mark. A pure function in a new module `journeys` groups the legs of one vehicle. A read-only facade `LogbookReader` in `commands_internal/journeys_cmd.rs` reads the DB and calls the pure function. A new module `mcp` holds only a `LogbookReader`, wraps it in an `rmcp` 3.5 tool router, and the Axum router mounts it at `/mcp` as a stateless streamable HTTP service.

**Tech Stack:** Rust (Diesel, SQLite, Axum 0.8, `rmcp` 3.5.0, `schemars` 1), SvelteKit + TypeScript, WebdriverIO.

**Spec:** [01-task.md](./01-task.md). Read it before each task.

**Depends on:** [Task 88: Places as Entities](../../_done/88-places-as-entities/01-task.md) and its [plan](../../_done/88-places-as-entities/02-plan.md). Phase A (see "Phases") does not depend on 88. Phase B starts only after 88 is merged to `main`; Task B0 checks the 88 interfaces that phase B uses.

## Phases

The work runs in two phases on the branch `feat/89-home-place-mcp`, created from `main`.

| Phase | When | Tasks | Uses task 88? |
|-------|------|-------|---------------|
| **A** | Now, while task 88 runs in a parallel worktree and is not merged | A1, A2 | **No.** The code is the old string model: `Trip.origin` and `Trip.destination` are free text, and `places` is keyed by `normalised_name`. |
| **B** | After 88 is merged to `main` and this branch is rebased on `main` | B0 to B6 | Yes |

Each phase A task ends green on a branch from the current `main`. Phase A must not use any name from task 88: no `Trip.origin_place_id`, no `Trip::from_row`, no `ensure_place_for_test`, no `ensurePlace`, no `/miesta`.

**Integration tests in phase A.** Task 88 runs in a parallel worktree, and both worktrees spawn `kniha-jazd-web` for the integration tests. The spawned port is 3457 by default, but [wdio.server.conf.ts](../../../tests/integration/wdio.server.conf.ts) (lines 145-151) reads `WDIO_SERVER_PORT`: it passes the port to the spawned server as `PORT`, and `onPrepare` exports the matching `WDIO_SERVER_URL`, which [utils/db.ts](../../../tests/integration/utils/db.ts) and `utils/mcp.ts` read. So in phase A, run every wdio command with `WDIO_SERVER_PORT=3467`. Each worktree also needs its own `npm run build` and `cargo build -p kniha-jazd-web`, because the config spawns the binary and serves `build/` from its own checkout.

## Global Constraints

- All business logic stays in Rust ([ADR-008](../../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)). The frontend only displays.
- Write the failing test first, then the code (project [CLAUDE.md](../../../CLAUDE.md), "Core Principle: Test-Driven Development").
- **Read-only by construction.** The MCP module holds a `LogbookReader`, never a `Database`. `LogbookReader` exposes only `vehicles`, `trips_in_range` and `journeys`. A source guard test bans these words in [mcp/mod.rs](../../../src-tauri/core/src/), [journeys_cmd.rs](../../../src-tauri/core/src/commands_internal/) and [journeys/mod.rs](../../../src-tauri/core/src/): `check_read_only`, `connection`, `restore`, `sql_query`, `execute`, `transaction`, `create_`, `update_`, `delete_`, `save_`, `set_`, `upsert`, `insert`.
- Tools, exactly these three: `list_vehicles`, `list_trips`, `list_journeys`. Each returns a JSON object (`{"vehicles": [...]}`, `{"trips": [...]}`, `{"home_place", "journeys"}`), as the spec says.
- `vehicle_id = None` means **all vehicles**. `is_active` marks only the vehicle that is selected in the UI. A merged `list_trips` result is sorted by `start_datetime`, then `odometer`.
- Dates on the MCP surface are `YYYY-MM-DD`. Both ends are inclusive. Datetimes on the MCP surface are `YYYY-MM-DDTHH:MM`.
- Stateless: `with_legacy_session_mode(false)`, `NeverSessionManager`, `with_json_response(true)`, `with_sse_keep_alive(None)`. Host check off: `disable_allowed_hosts()`. No auth.
- Errors: a bad date, an unknown vehicle and "no home place" give `ErrorData::invalid_params` (JSON-RPC code -32602). A DB error or a join error gives `ErrorData::internal_error` (-32603).
- The tools run DB work in `tokio::task::spawn_blocking`, the same as `rpc_handler` ([server/mod.rs:63](../../../src-tauri/core/src/server/mod.rs#L63)).
- No home place is an error, never an empty list. Error text: `Home place is not set. Mark a place as home on the Miesta page.`
- `rmcp` 3.5.0 needs Rust 1.88 (`rust-version = "1.88"` in its `Cargo.toml`). Today [Dockerfile.web](../../../Dockerfile.web) uses `rust:1.86-bookworm`, [src-tauri/Cargo.toml](../../../src-tauri/Cargo.toml) says `rust-version = "1.77.2"`, and [CONTRIBUTING.md](../../../CONTRIBUTING.md) line 10 says `Rust 1.77+`. Task A2 changes all three.
- All UI strings through i18n, Slovak first. Run `npm run i18n` after an edit of `src/lib/i18n/{sk,en}/index.ts`.
- Prose in docs: ASD-STE100 Simplified Technical English. Keyboard-typable characters only (no em-dash, no arrow glyphs, no curly quotes). Slovak diacritics are allowed.
- This repo is public. No homelab host, IP or real trip data in code, tests or docs.
- Commit only the files of the task (`git add <paths>`), never `git add -A`. There is no PR: lists that the spec wants "in the PR" go in the commit message body.

## Review Focus

1. **A chain away from home with no return leg, and then a new departure or a home -> home loop.** The user decided: the first chain is an incomplete journey (`complete = false`). A single-leg chain is still a day trip. Task A1 tests `broken_chain_is_incomplete_and_next_departure_starts_a_new_one`, `loop_after_open_chain_closes_it_as_incomplete` and `single_leg_then_loop_is_a_day_trip`.
2. **A journey that started before `date_from` and ends inside the range.** A query that reads only the trips in the range misses it. Task B2 test `journey_that_starts_before_range_is_found`.
3. **A vehicle ID that does not exist, or a malformed date, on the MCP surface.** The expected result is a clear error, not an empty list. Task B2 test `unknown_vehicle_is_an_error` and Task B3 test `malformed_date_is_invalid_params`.
4. **An MCP client that sends a stale `Mcp-Session-Id` after a restart, or a `Host` header with the public name.** Both must get a normal answer. Task A2 test `stale_session_id_and_public_host_are_accepted`.
5. **Two places marked as home at the same time** (a race or a direct SQL edit). The partial unique index must reject the second mark, and `set_home_place` must move the mark in one transaction. Task B1 tests `second_home_violates_unique_index` and `set_home_place_moves_the_mark`.

---

## File Structure

Phase A touches only: `journeys/`, `mcp/`, `journeys_cmd.rs` (vehicles only), `lib.rs`, `commands_internal/mod.rs`, `server/mod.rs`, the Cargo files, `Dockerfile.web`, `CONTRIBUTING.md`, the image line of `CHANGELOG.md`, `utils/mcp.ts` and `mcp-endpoint.spec.ts`.

| File | Responsibility |
|------|----------------|
| Create [migrations/2026-10-05-110000_add_place_is_home/](../../../src-tauri/core/migrations/) `up.sql`, `down.sql` | `places.is_home` + partial unique index |
| Modify [schema.rs](../../../src-tauri/core/src/schema.rs) | `is_home -> Bool` on `places` (last column) |
| Modify [models.rs](../../../src-tauri/core/src/models.rs) | `PlaceRow.is_home`, `Place.is_home`, set in `Place::from_row` |
| Modify [db.rs](../../../src-tauri/core/src/db.rs) | `get_home_place`, `set_home_place`, `get_trips_for_vehicle_in_range` |
| Modify [places_cmd.rs](../../../src-tauri/core/src/commands_internal/places_cmd.rs) | `set_home_place_internal` |
| Modify [dispatcher.rs](../../../src-tauri/core/src/server/dispatcher.rs) | `set_home_place` arm |
| Create [journeys/mod.rs](../../../src-tauri/core/src/), [journeys/tests.rs](../../../src-tauri/core/src/) | Pure grouping: `Leg`, `Journey`, `group_journeys`, `overlaps`, and its unit tests |
| Create [commands_internal/journeys_cmd.rs](../../../src-tauri/core/src/commands_internal/), `journeys_cmd_tests.rs` | `LogbookReader` (read-only facade), `ReadError`, `JourneyList` |
| Create [mcp/mod.rs](../../../src-tauri/core/src/), [mcp/tests.rs](../../../src-tauri/core/src/) | `rmcp` tool router, DTOs, `mcp_service`, HTTP tests, source guard |
| Modify [server/mod.rs](../../../src-tauri/core/src/server/mod.rs) | `nest_service("/mcp", ...)` in both router branches |
| Modify [core/Cargo.toml](../../../src-tauri/core/Cargo.toml), [src-tauri/Cargo.toml](../../../src-tauri/Cargo.toml), [Dockerfile.web](../../../Dockerfile.web), [CONTRIBUTING.md](../../../CONTRIBUTING.md) | `rmcp`, `schemars`, Rust 1.88 |
| Modify [types.ts](../../../src/lib/types.ts), [api.ts](../../../src/lib/api.ts), [miesta/+page.svelte](../../../src/routes/), [sk/index.ts](../../../src/lib/i18n/sk/index.ts), [en/index.ts](../../../src/lib/i18n/en/index.ts) | Home icon button |
| Create [utils/mcp.ts](../../../tests/integration/utils/), [tier2/mcp-endpoint.spec.ts](../../../tests/integration/specs/tier2/), [tier2/home-place.spec.ts](../../../tests/integration/specs/tier2/) | Integration tests |
| Create [docs/features/mcp-endpoint.md](../../../docs/features/); modify [place-book.md](../../../docs/features/place-book.md), other feature docs, [README.md](../../../README.md), [README.en.md](../../../README.en.md), [DECISIONS.md](../../../DECISIONS.md), [CHANGELOG.md](../../../CHANGELOG.md), [CLAUDE.md](../../../CLAUDE.md), [rust-backend.md](../../../.claude/rules/rust-backend.md) | Docs |

---
---

## Phase A: before task 88 is merged

### Task A1: Journey grouping (pure function)

**Files:**
- Create: [journeys/mod.rs](../../../src-tauri/core/src/), [journeys/tests.rs](../../../src-tauri/core/src/)
- Modify: [lib.rs](../../../src-tauri/core/src/lib.rs) (add `pub mod journeys;`)

**Interfaces:**
- Consumes: nothing. Phase A: do not import `crate::models::Trip`.
- Produces:
  - `pub struct Leg { id: Uuid, start: NaiveDateTime, odometer: f64, origin_place_id: Uuid, destination_place_id: Uuid, destination_name: String, distance_km: f64, purpose: String }` (Task B2 adds `Leg::from_trip`)
  - `pub struct Journey { vehicle_id: Uuid, start: NaiveDateTime, end: Option<NaiveDateTime>, nights: Option<i64>, total_km: f64, places: Vec<String>, purposes: Vec<String>, complete: bool, leg_ids: Vec<Uuid> }`
  - `pub fn group_journeys(vehicle_id: Uuid, legs: &[Leg], home: Uuid, round_trip_ids: &HashSet<Uuid>) -> Vec<Journey>`
  - `pub fn overlaps(journey: &Journey, from: NaiveDate, to: NaiveDate) -> bool`

- [ ] **Step 1: Create the module skeleton**

`src-tauri/core/src/journeys/mod.rs`:
```rust
//! Journey grouping (task 89, BIZ entry "Journeys away from home").
//!
//! A trip row is one leg. A journey is a chain of legs away from home. This
//! module is pure: no DB, no clock. `commands_internal::journeys_cmd` reads the
//! legs and the home mark, and calls `group_journeys` once per vehicle.

use std::collections::HashSet;

use chrono::{NaiveDate, NaiveDateTime};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Leg {
    pub id: Uuid,
    pub start: NaiveDateTime,
    pub odometer: f64,
    pub origin_place_id: Uuid,
    pub destination_place_id: Uuid,
    pub destination_name: String,
    pub distance_km: f64,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Journey {
    pub vehicle_id: Uuid,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub nights: Option<i64>,
    pub total_km: f64,
    pub places: Vec<String>,
    pub purposes: Vec<String>,
    pub complete: bool,
    pub leg_ids: Vec<Uuid>,
}

pub fn group_journeys(
    _vehicle_id: Uuid,
    _legs: &[Leg],
    _home: Uuid,
    _round_trip_ids: &HashSet<Uuid>,
) -> Vec<Journey> {
    Vec::new()
}

pub fn overlaps(_journey: &Journey, _from: NaiveDate, _to: NaiveDate) -> bool {
    false
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
```
Add `pub mod journeys;` to `lib.rs`, in alphabetical order after `pub mod invoice;`.

- [ ] **Step 2: Write the failing tests (the spec table)**

`src-tauri/core/src/journeys/tests.rs`:
```rust
//! The unit-test table of task 89. Invented addresses only.

use super::*;
use std::collections::HashMap;

const HOME: &str = "Home St 1, Hometown";

/// Gives each invented place name a stable ID and builds legs in order.
struct Fixture {
    vehicle_id: Uuid,
    ids: HashMap<String, Uuid>,
    legs: Vec<Leg>,
    round_trips: HashSet<Uuid>,
    odometer: f64,
}

impl Fixture {
    fn new() -> Self {
        Self {
            vehicle_id: Uuid::new_v4(),
            ids: HashMap::new(),
            legs: Vec::new(),
            round_trips: HashSet::new(),
            odometer: 10_000.0,
        }
    }

    fn place(&mut self, name: &str) -> Uuid {
        *self.ids.entry(name.to_string()).or_insert_with(Uuid::new_v4)
    }

    /// `at` is "YYYY-MM-DD HH:MM". The odometer grows by `km` per leg.
    fn leg(&mut self, at: &str, from: &str, to: &str, km: f64, purpose: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.odometer += km;
        let leg = Leg {
            id,
            start: NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M").unwrap(),
            odometer: self.odometer,
            origin_place_id: self.place(from),
            destination_place_id: self.place(to),
            destination_name: to.to_string(),
            distance_km: km,
            purpose: purpose.to_string(),
        };
        self.legs.push(leg);
        id
    }

    fn round_trip(&mut self, leg_id: Uuid) {
        self.round_trips.insert(leg_id);
    }

    fn run(&mut self) -> Vec<Journey> {
        let home = self.place(HOME);
        group_journeys(self.vehicle_id, &self.legs, home, &self.round_trips)
    }
}

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn trip_with_local_legs_and_return_two_days_later() {
    let mut fx = Fixture::new();
    let out_id = fx.leg("2026-03-02 07:00", HOME, "City A", 357.0, "Customer visit");
    fx.leg("2026-03-02 13:00", "City A", "City A Plant", 8.0, "Customer visit");
    fx.leg("2026-03-03 09:00", "City A Plant", "City A Office", 5.0, "Meeting");
    fx.leg("2026-03-03 16:00", "City A Office", "City A", 6.0, "Meeting");
    let back_id = fx.leg("2026-03-04 08:00", "City A", HOME, 357.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    let j = &journeys[0];
    assert_eq!(j.nights, Some(2));
    assert!(j.complete);
    assert_eq!(j.start, dt("2026-03-02 07:00"));
    assert_eq!(j.end, Some(dt("2026-03-04 08:00")));
    assert_eq!(j.total_km, 733.0);
    assert_eq!(j.leg_ids.len(), 5);
    assert_eq!(j.leg_ids[0], out_id);
    assert_eq!(j.leg_ids[4], back_id);
    assert_eq!(j.purposes, vec!["Customer visit", "Meeting", "Return"]);
    assert_eq!(j.vehicle_id, fx.vehicle_id);
}

#[test]
fn places_are_distinct_and_in_order() {
    let mut fx = Fixture::new();
    fx.leg("2026-04-06 06:00", HOME, "City A", 300.0, "Trip");
    fx.leg("2026-04-07 08:00", "City A", "City B", 120.0, "Trip");
    fx.leg("2026-04-08 08:00", "City B", "City A", 120.0, "Trip");
    fx.leg("2026-04-09 15:00", "City A", HOME, 300.0, "Trip");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].places, vec!["City A", "City B"]);
    assert_eq!(journeys[0].nights, Some(3));
}

#[test]
fn single_leg_day_trip_followed_by_departure_is_not_a_journey() {
    // Spec row 3 ("home -> Village, one leg, the next leg starts at home").
    // The second departure has a return leg, so it is the only journey; the
    // first leg must not show up, also not as an incomplete journey.
    let mut fx = Fixture::new();
    fx.leg("2026-05-04 08:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-05-05 08:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-05-05 12:00", "Village", HOME, 26.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].start, dt("2026-05-05 08:00"));
    assert!(journeys[0].complete);
}

#[test]
fn round_trip_map_on_newest_row_is_a_day_trip() {
    let mut fx = Fixture::new();
    let id = fx.leg("2026-05-04 08:00", HOME, "Village", 52.0, "Delivery");
    fx.round_trip(id);

    assert!(fx.run().is_empty());
}

#[test]
fn home_to_home_loop_is_not_a_journey() {
    let mut fx = Fixture::new();
    fx.leg("2026-05-04 08:00", HOME, HOME, 204.0, "Errands");

    assert!(fx.run().is_empty());
}

#[test]
fn same_day_chain_has_zero_nights() {
    let mut fx = Fixture::new();
    fx.leg("2026-06-01 07:00", HOME, "Workshop", 30.0, "Service");
    fx.leg("2026-06-01 10:00", "Workshop", "Other Town", 40.0, "Service");
    fx.leg("2026-06-01 15:00", "Other Town", HOME, 22.0, "Service");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].nights, Some(0));
    assert_eq!(journeys[0].total_km, 92.0);
}

#[test]
fn long_same_day_trip_counts_all_km() {
    let mut fx = Fixture::new();
    fx.leg("2026-06-02 04:00", HOME, "City A", 505.0, "Customer visit");
    fx.leg("2026-06-02 15:00", "City A", HOME, 506.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].nights, Some(0));
    assert_eq!(journeys[0].total_km, 1011.0);
}

#[test]
fn journey_across_month_boundary_overlaps_both_months() {
    let mut fx = Fixture::new();
    fx.leg("2026-01-27 07:00", HOME, "City A", 357.0, "Project");
    fx.leg("2026-02-10 16:00", "City A", HOME, 357.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    let j = &journeys[0];
    assert!(overlaps(j, d("2026-01-01"), d("2026-01-31")));
    assert!(overlaps(j, d("2026-02-01"), d("2026-02-28")));
    assert!(!overlaps(j, d("2026-03-01"), d("2026-03-31")));
    assert!(!overlaps(j, d("2025-12-01"), d("2025-12-31")));
}

#[test]
fn last_leg_leaves_home_without_return_is_incomplete() {
    let mut fx = Fixture::new();
    let id = fx.leg("2026-07-01 07:00", HOME, "City A", 357.0, "Project");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    let j = &journeys[0];
    assert!(!j.complete);
    assert_eq!(j.end, None);
    assert_eq!(j.nights, None);
    assert_eq!(j.leg_ids, vec![id]);
}

#[test]
fn incomplete_journey_is_open_at_the_end() {
    let mut fx = Fixture::new();
    fx.leg("2026-07-01 07:00", HOME, "City A", 357.0, "Project");
    let j = fx.run().remove(0);

    assert!(overlaps(&j, d("2026-12-01"), d("2026-12-31")));
    assert!(!overlaps(&j, d("2026-06-01"), d("2026-06-30")));
}

#[test]
fn broken_chain_is_incomplete_and_next_departure_starts_a_new_one() {
    // User decision: home -> A, A -> B, then home -> C. The first chain has no
    // return leg, so it is an incomplete journey, not dropped.
    let mut fx = Fixture::new();
    let first = fx.leg("2026-09-01 07:00", HOME, "City A", 357.0, "Project");
    let second = fx.leg("2026-09-02 07:00", "City A", "City B", 100.0, "Project");
    let next = fx.leg("2026-09-05 07:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-09-05 12:00", "Village", HOME, 52.0, "Return");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 2);
    assert_eq!(journeys[0].leg_ids, vec![first, second]);
    assert!(!journeys[0].complete);
    assert_eq!(journeys[0].end, None);
    assert_eq!(journeys[0].nights, None);
    assert_eq!(journeys[0].total_km, 457.0);
    assert_eq!(journeys[1].leg_ids[0], next);
    assert!(journeys[1].complete);
}

#[test]
fn loop_after_open_chain_closes_it_as_incomplete() {
    let mut fx = Fixture::new();
    fx.leg("2026-09-01 07:00", HOME, "City A", 357.0, "Project");
    fx.leg("2026-09-02 07:00", "City A", "City B", 100.0, "Project");
    fx.leg("2026-09-05 07:00", HOME, HOME, 40.0, "Errands");

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert!(!journeys[0].complete);
    assert_eq!(journeys[0].leg_ids.len(), 2);
}

#[test]
fn single_leg_then_loop_is_a_day_trip() {
    // A one-leg chain is a day trip when the next leg starts at home, also
    // when that next leg is a loop.
    let mut fx = Fixture::new();
    fx.leg("2026-09-01 07:00", HOME, "Village", 52.0, "Delivery");
    fx.leg("2026-09-02 07:00", HOME, HOME, 40.0, "Errands");

    assert!(fx.run().is_empty());
}

#[test]
fn same_start_datetime_is_ordered_by_odometer() {
    let mut fx = Fixture::new();
    // Inserted in the wrong order on purpose: the return leg first.
    let back = fx.leg("2026-08-03 09:00", "City A", HOME, 50.0, "Return");
    let out = fx.leg("2026-08-03 09:00", HOME, "City A", 50.0, "Errand");
    // Give the outbound leg the lower odometer.
    fx.legs[1].odometer = 1.0;

    let journeys = fx.run();

    assert_eq!(journeys.len(), 1);
    assert_eq!(journeys[0].leg_ids, vec![out, back]);
}

#[test]
fn return_leg_without_departure_is_ignored() {
    let mut fx = Fixture::new();
    fx.leg("2026-01-02 08:00", "City A", HOME, 357.0, "Return");

    assert!(fx.run().is_empty());
}

#[test]
fn empty_purpose_is_not_listed() {
    let mut fx = Fixture::new();
    fx.leg("2026-10-01 07:00", HOME, "City A", 100.0, "");
    fx.leg("2026-10-01 17:00", "City A", HOME, 100.0, "Return");

    assert_eq!(fx.run()[0].purposes, vec!["Return"]);
}
```
The case "home place not set" is a wrapper test in Task B2: the pure function always has a home ID.

- [ ] **Step 3: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core journeys::`
Expected: FAIL. Most tests fail on `assert_eq!(journeys.len(), 1)` (left: 0). The four tests that expect an empty list pass already, because the stub returns an empty list.

- [ ] **Step 4: Implement the grouping**

Replace the two stubs in `journeys/mod.rs`:
```rust
/// Group the legs of ONE vehicle into journeys away from `home`.
///
/// Rules (BIZ entry "Journeys away from home"):
/// 1. A journey starts at a leg home -> not home.
/// 2. It ends at the next leg not home -> home.
/// 3. A home -> X leg is a day trip if it has a round-trip map, or if the next
///    leg starts at home (a departure or a loop).
/// 4. A home -> home leg is never a journey.
/// 5. A chain with no return leg is incomplete: at the end of the data, or when
///    a chain of two or more legs meets a new leg from home.
/// 6. Legs outside a chain are ignored.
pub fn group_journeys(
    vehicle_id: Uuid,
    legs: &[Leg],
    home: Uuid,
    round_trip_ids: &HashSet<Uuid>,
) -> Vec<Journey> {
    let mut sorted: Vec<&Leg> = legs.iter().collect();
    sorted.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.odometer.total_cmp(&b.odometer))
    });

    let mut journeys = Vec::new();
    let mut open: Option<Vec<&Leg>> = None;
    for leg in sorted {
        let from_home = leg.origin_place_id == home;
        let to_home = leg.destination_place_id == home;
        if from_home {
            // A leg from home closes an open chain without a return leg.
            // One leg: a day trip (rule 3). Two or more: incomplete (rule 5).
            if let Some(chain) = open.take() {
                if chain.len() >= 2 {
                    journeys.push(build(vehicle_id, &chain, home, false));
                }
            }
            if !to_home && !round_trip_ids.contains(&leg.id) {
                open = Some(vec![leg]);
            }
        } else if to_home {
            if let Some(mut chain) = open.take() {
                chain.push(leg);
                journeys.push(build(vehicle_id, &chain, home, true));
            }
        } else if let Some(chain) = open.as_mut() {
            chain.push(leg);
        }
    }
    if let Some(chain) = open {
        journeys.push(build(vehicle_id, &chain, home, false));
    }
    journeys
}

fn build(vehicle_id: Uuid, chain: &[&Leg], home: Uuid, complete: bool) -> Journey {
    let start = chain[0].start;
    let end = complete.then(|| chain[chain.len() - 1].start);
    let nights = end.map(|e| (e.date() - start.date()).num_days());

    let mut places: Vec<String> = Vec::new();
    let mut purposes: Vec<String> = Vec::new();
    for leg in chain {
        if leg.destination_place_id != home && !places.contains(&leg.destination_name) {
            places.push(leg.destination_name.clone());
        }
        let purpose = leg.purpose.trim();
        if !purpose.is_empty() && !purposes.iter().any(|p| p == purpose) {
            purposes.push(purpose.to_string());
        }
    }

    Journey {
        vehicle_id,
        start,
        end,
        nights,
        total_km: chain.iter().map(|l| l.distance_km).sum(),
        places,
        purposes,
        complete,
        leg_ids: chain.iter().map(|l| l.id).collect(),
    }
}

/// True if any day of the journey is inside `from..=to`. An incomplete
/// journey has no end, so it is open towards the future.
pub fn overlaps(journey: &Journey, from: NaiveDate, to: NaiveDate) -> bool {
    journey.start.date() <= to && journey.end.map_or(true, |e| e.date() >= from)
}
```

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core journeys::`
Expected: PASS, 16 tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/core/src/journeys src-tauri/core/src/lib.rs
git commit -m "feat(journeys): group trip legs into journeys away from home"
```

---

### Task A2: MCP skeleton at `/mcp` with `list_vehicles`

**Files:**
- Modify: [core/Cargo.toml](../../../src-tauri/core/Cargo.toml), [src-tauri/Cargo.toml](../../../src-tauri/Cargo.toml) (`rust-version`), [Dockerfile.web](../../../Dockerfile.web), [CONTRIBUTING.md](../../../CONTRIBUTING.md), [CHANGELOG.md](../../../CHANGELOG.md) (upgrade notes only)
- Create: [mcp/mod.rs](../../../src-tauri/core/src/), [mcp/tests.rs](../../../src-tauri/core/src/), [commands_internal/journeys_cmd.rs](../../../src-tauri/core/src/commands_internal/), [utils/mcp.ts](../../../tests/integration/utils/), [tier2/mcp-endpoint.spec.ts](../../../tests/integration/specs/tier2/)
- Modify: [lib.rs](../../../src-tauri/core/src/lib.rs) (`pub mod mcp;`), [commands_internal/mod.rs](../../../src-tauri/core/src/commands_internal/mod.rs), [server/mod.rs](../../../src-tauri/core/src/server/mod.rs)

**Interfaces:**
- Consumes: Task A1 `journeys/mod.rs` (only as a file for the source guard); existing `get_vehicles_internal(db: &Database) -> Result<Vec<Vehicle>, String>` ([vehicles.rs:10](../../../src-tauri/core/src/commands_internal/vehicles.rs#L10)), `crate::db_tests::create_test_vehicle(name: &str) -> Vehicle`.
- Produces:
  - `pub enum ReadError { Invalid(String), Internal(String) }` (`Debug`, `Clone`, `PartialEq`)
  - `#[derive(Clone)] pub struct LogbookReader` with `new(db: Arc<Database>) -> Self` and `vehicles(&self) -> Result<Vec<Vehicle>, ReadError>`. Task B2 adds `trips_in_range` and `journeys`.
  - `pub fn mcp_service(db: Arc<Database>) -> StreamableHttpService<KnihaJazdMcp, NeverSessionManager>`, mounted at `/mcp` in both branches of `HttpServer::start`
  - Tool `list_vehicles`. Task B3 adds `list_trips` and `list_journeys`.
  - Integration helper `mcpRequest<T>(method, params?)`

The `rmcp` API below was checked on 2026-10-05 against `rmcp` 3.5.0 with a probe server: `tools/list` without `initialize`, a stale `Mcp-Session-Id`, and a foreign `Host` header all return HTTP 200 with this config. An `async` tool with `spawn_blocking` works. `Err(ErrorData::invalid_params(..))` gives JSON-RPC code -32602, `Err(ErrorData::internal_error(..))` gives -32603. A missing argument gives a result with `isError: true`.

- [ ] **Step 1: Add the dependencies and raise the Rust version**

[core/Cargo.toml](../../../src-tauri/core/Cargo.toml), under `[dependencies]`:
```toml
rmcp = { version = "3.5", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }
schemars = "1"
```
- [src-tauri/Cargo.toml](../../../src-tauri/Cargo.toml): `rust-version = "1.88"` (was `"1.77.2"`).
- [Dockerfile.web](../../../Dockerfile.web): `FROM rust:1.88-bookworm AS rust-builder` (was `rust:1.86-bookworm`).
- [CONTRIBUTING.md](../../../CONTRIBUTING.md) line 10: `- [Rust](https://rustup.rs/) 1.88+` (was `1.77+`).

Run: `cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: builds. `cargo tree --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core -i axum` shows one `axum v0.8.x` (rmcp uses `axum` only as a dev-dependency).

- [ ] **Step 2: Write the failing Rust HTTP tests**

`src-tauri/core/src/mcp/tests.rs`:
```rust
//! `/mcp` over real HTTP, through `HttpServer::start` (the same router the
//! container serves), plus the read-only source guard.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::db::Database;
use crate::server::HttpServer;

async fn start(db: Arc<Database>) -> (String, tokio::sync::oneshot::Sender<()>) {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let tmp = std::env::temp_dir();
    let addr = HttpServer::start(db, Arc::new(AppState::new()), tmp.clone(), tmp, 0, false, rx)
        .await
        .unwrap();
    (format!("http://{addr}/mcp"), tx)
}

async fn post(url: &str, body: Value, extra: &[(&str, &str)]) -> Value {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
    for (k, v) in extra {
        req = req.header(*k, *v);
    }
    let resp = req.body(body.to_string()).send().await.unwrap();
    assert_eq!(resp.status(), 200, "HTTP status");
    resp.json().await.unwrap()
}

fn call(id: u32, name: &str, args: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
           "params": {"name": name, "arguments": args}})
}

fn tool_names(body: &Value) -> Vec<String> {
    let mut names: Vec<String> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn tools_list_returns_the_read_only_tools() {
    let (url, stop) = start(Arc::new(Database::in_memory().unwrap())).await;

    let body = post(&url, json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}), &[]).await;

    // Phase A: one tool. Task B3 changes this to the three tools.
    assert_eq!(tool_names(&body), vec!["list_vehicles"]);
    for tool in body["result"]["tools"].as_array().unwrap() {
        let description = tool["description"].as_str().unwrap();
        assert!(description.contains("Read-only"), "{description}");
    }
    let _ = stop.send(());
}

#[tokio::test]
async fn stale_session_id_and_public_host_are_accepted() {
    let (url, stop) = start(Arc::new(Database::in_memory().unwrap())).await;

    let body = post(&url, call(4, "list_vehicles", json!({})),
        &[("Mcp-Session-Id", "from-before-a-restart"), ("Host", "logbook.example.org")]).await;

    assert_eq!(body["result"]["structuredContent"]["vehicles"], json!([]));
    let _ = stop.send(());
}

#[tokio::test]
async fn list_vehicles_returns_the_vehicle_fields() {
    let db = Arc::new(Database::in_memory().unwrap());
    let v = crate::db_tests::create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    let (url, stop) = start(db).await;

    let body = post(&url, call(6, "list_vehicles", json!({})), &[]).await;

    let vehicles = &body["result"]["structuredContent"]["vehicles"];
    assert_eq!(vehicles[0]["id"], v.id.to_string());
    assert_eq!(vehicles[0]["name"], "Car");
    assert_eq!(vehicles[0]["license_plate"], v.license_plate);
    assert_eq!(vehicles[0]["is_active"], v.is_active);
    let _ = stop.send(());
}

#[test]
fn mcp_read_path_has_no_write_access() {
    // Read-only by construction (ADR "Read-only MCP endpoint"). The MCP module
    // holds only a LogbookReader; these files must not name a write function,
    // the raw connection, a restore, or raw SQL.
    let sources = [
        ("mcp/mod.rs", include_str!("mod.rs")),
        ("commands_internal/journeys_cmd.rs", include_str!("../commands_internal/journeys_cmd.rs")),
        ("journeys/mod.rs", include_str!("../journeys/mod.rs")),
    ];
    let banned = [
        "check_read_only", "connection", "restore", "sql_query", "execute", "transaction",
        "create_", "update_", "delete_", "save_", "set_", "upsert", "insert",
    ];
    for (file, source) in sources {
        for word in banned {
            assert!(!source.contains(word), "{file} must not contain `{word}`");
        }
    }
}
```

- [ ] **Step 3: Write the integration helper and spec (test first)**

`tests/integration/utils/mcp.ts`:
```typescript
/**
 * Raw JSON-RPC calls to the read-only MCP endpoint (task 89).
 *
 * The endpoint is stateless, so no initialize call and no session ID are
 * needed. It answers with application/json.
 */

const SERVER_URL = process.env.WDIO_SERVER_URL || 'http://localhost:3457';

export interface McpResponse<T> {
  jsonrpc: '2.0';
  id: number;
  result?: T;
  error?: { code: number; message: string };
}

export async function mcpRequest<T>(
  method: string,
  params: Record<string, unknown> = {}
): Promise<McpResponse<T>> {
  const resp = await fetch(`${SERVER_URL}/mcp`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  if (!resp.ok) {
    throw new Error(`MCP '${method}' failed (${resp.status}): ${await resp.text()}`);
  }
  return (await resp.json()) as McpResponse<T>;
}
```
`tests/integration/specs/tier2/mcp-endpoint.spec.ts`:
```typescript
/**
 * Tier 2: Read-only MCP endpoint (task 89)
 *
 * Covers: the built image serves /mcp, and tools/list returns exactly the
 * read-only tools. The tool results and the journey rules are covered by the
 * Rust tests (mcp/tests.rs, journeys/tests.rs), not here.
 */

import { waitForAppReady } from '../../utils/app';
import { mcpRequest } from '../../utils/mcp';

interface ToolsList {
  tools: { name: string; description: string }[];
}

/** Phase A of task 89 has one tool. Task B3 changes this to the three tools. */
const EXPECTED_TOOLS = ['list_vehicles'];

describe('MCP endpoint', () => {
  beforeEach(async () => {
    await waitForAppReady();
  });

  it('tools/list returns exactly the read-only tools', async () => {
    const resp = await mcpRequest<ToolsList>('tools/list');

    expect(resp.error).toBeUndefined();
    const names = resp.result!.tools.map((t) => t.name).sort();
    expect(names).toEqual(EXPECTED_TOOLS);
  });
});
```

- [ ] **Step 4: Create the stubs and see both tests fail**

Add `pub mod mcp;` to `lib.rs` (after `pub mod journeys;`). Create `src-tauri/core/src/mcp/mod.rs` with only the test hook:
```rust
//! Read-only MCP endpoint (task 89).

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
```
Create `src-tauri/core/src/commands_internal/journeys_cmd.rs` as an empty file with one line: `//! Read-only logbook queries for the MCP endpoint (task 89).`

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core mcp::`
Expected: `mcp_read_path_has_no_write_access` passes. The three HTTP tests fail with `assertion left == right failed: HTTP status` (left: 404).

Run (note the port, see "Phases"):
```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
WDIO_SERVER_PORT=3467 xvfb-run -a -s "-screen 0 1280x1024x24" npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/mcp-endpoint.spec.ts
```
Expected: FAIL. The SPA fallback answers `/mcp` with `index.html`, so `resp.json()` throws `Unexpected token '<'` (or `MCP 'tools/list' failed (405)` for a POST to the static service).

- [ ] **Step 5: Implement the read-only facade (vehicles only)**

Replace `src-tauri/core/src/commands_internal/journeys_cmd.rs`:
```rust
//! Read-only logbook queries for the MCP endpoint (task 89).
//!
//! `LogbookReader` is the only thing the `mcp` module holds. It keeps its
//! `Database` private and exposes reads only, so the MCP module has no path
//! to a write. The source guard in `mcp/tests.rs` checks this file too.

use std::sync::Arc;

use crate::commands_internal::get_vehicles_internal;
use crate::db::Database;
use crate::models::Vehicle;

/// `Invalid`: the caller can fix it (bad range, unknown vehicle, no home).
/// `Internal`: a DB error.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    Invalid(String),
    Internal(String),
}

#[derive(Clone)]
pub struct LogbookReader {
    db: Arc<Database>,
}

impl LogbookReader {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn vehicles(&self) -> Result<Vec<Vehicle>, ReadError> {
        get_vehicles_internal(&self.db).map_err(ReadError::Internal)
    }
}
```
In `commands_internal/mod.rs` add:
```rust
pub mod journeys_cmd;
pub use journeys_cmd::*;
```

- [ ] **Step 6: Implement the MCP server (one tool)**

Replace `src-tauri/core/src/mcp/mod.rs`:
```rust
//! Read-only MCP endpoint at `/mcp` (task 89, ADR "Read-only MCP endpoint").
//!
//! Stateless streamable HTTP with `rmcp`: no session IDs, so a client that
//! cached one keeps working after a restart. The tools call `LogbookReader`
//! directly, not `/api/rpc`. This module never holds a `Database`; the test
//! `mcp_read_path_has_no_write_access` checks the sources.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Json;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::Serialize;

use crate::commands_internal::{LogbookReader, ReadError};
use crate::db::Database;

// ---------------------------------------------------------------- outputs

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VehicleOut {
    pub id: String,
    pub name: String,
    pub license_plate: String,
    /// True for the one vehicle that is selected in the app UI.
    pub is_active: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VehicleList {
    pub vehicles: Vec<VehicleOut>,
}

fn to_error(e: ReadError) -> ErrorData {
    match e {
        ReadError::Invalid(m) => ErrorData::invalid_params(m, None),
        ReadError::Internal(m) => ErrorData::internal_error(m, None),
    }
}

/// Run a read on the blocking pool, the same as `rpc_handler` does for DB work.
async fn blocking<T, F>(read: F) -> Result<T, ErrorData>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ReadError> + Send + 'static,
{
    tokio::task::spawn_blocking(read)
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(to_error)
}

// ---------------------------------------------------------------- server

#[derive(Clone)]
pub struct KnihaJazdMcp {
    reader: LogbookReader,
    tool_router: ToolRouter<Self>,
}

impl KnihaJazdMcp {
    pub fn new(reader: LogbookReader) -> Self {
        Self { reader, tool_router: Self::tool_router() }
    }
}

#[tool_router]
impl KnihaJazdMcp {
    #[tool(description = "Read-only. Lists all vehicles of the logbook: id, name, license_plate, is_active (true for the vehicle selected in the app UI).")]
    async fn list_vehicles(&self) -> Result<Json<VehicleList>, ErrorData> {
        let reader = self.reader.clone();
        let vehicles = blocking(move || reader.vehicles()).await?;
        Ok(Json(VehicleList {
            vehicles: vehicles
                .into_iter()
                .map(|v| VehicleOut {
                    id: v.id.to_string(),
                    name: v.name,
                    license_plate: v.license_plate,
                    is_active: v.is_active,
                })
                .collect(),
        }))
    }
}

#[tool_handler]
impl ServerHandler for KnihaJazdMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("kniha-jazd", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Read-only access to a vehicle logbook (Kniha jázd). No tool can change data.",
            )
    }
}

/// The `/mcp` service. Stateless: no session store, no SSE keep-alive, JSON
/// answers. The host check is off: behind a reverse proxy the Host header is
/// the public name, and `/api/rpc` (which can write) has no host check either.
pub fn mcp_service(db: Arc<Database>) -> StreamableHttpService<KnihaJazdMcp, NeverSessionManager> {
    let reader = LogbookReader::new(db);
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .disable_allowed_hosts();
    StreamableHttpService::new(
        move || Ok(KnihaJazdMcp::new(reader.clone())),
        Default::default(),
        config,
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
```
Check the guard words against this source before you run the test: `with_server_info`, `with_instructions`, `disable_allowed_hosts` and the comments contain none of them.

- [ ] **Step 7: Mount `/mcp` in both router branches**

In `server/mod.rs`, `HttpServer::start`, build the service once before the `if`:
```rust
        let mcp = crate::mcp::mcp_service(state.db.clone());
```
Then add `.nest_service("/mcp", mcp.clone())` after `.nest("/api", api_router)` in the SPA branch, and `.nest_service("/mcp", mcp)` in the `else` branch. The route must come before `.fallback_service(static_service)` in the chain, as `/api` does.

- [ ] **Step 8: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core mcp::`
Expected: PASS, 4 tests.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS. The existing `spa_fallback_serves_index_html` test still passes.

Run:
```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
WDIO_SERVER_PORT=3467 xvfb-run -a -s "-screen 0 1280x1024x24" npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/mcp-endpoint.spec.ts
npm run typecheck:tests
```
Expected: 1 passing; 0 type errors.

- [ ] **Step 9: Check the image build**

Run: `docker build -f Dockerfile.web -t kniha-jazd-web:local .`
Expected: the build succeeds on `rust:1.88-bookworm`.

- [ ] **Step 10: Update the upgrade notes**

The project rule is: a change to `Dockerfile.web` updates `### Pokyny k aktualizácii` in the same commit (project [CLAUDE.md](../../../CLAUDE.md), "Release notes are the integrator contract"). In `## [Unreleased]` of [CHANGELOG.md](../../../CHANGELOG.md), change only the image line:
```markdown
- **Obraz, zväzok, port:** nová cesta `/mcp` na tom istom porte (MCP len na čítanie, bez prihlásenia, ako `/api/rpc`). Ak reverzná proxy prepúšťa len vybrané cesty, pridajte `/mcp`. Obraz sa zostavuje s Rust 1.88.
```
Task B5 adds the user-visible entries.

- [ ] **Step 11: Commit**

```bash
git add src-tauri/core/Cargo.toml src-tauri/Cargo.toml src-tauri/Cargo.lock Dockerfile.web CONTRIBUTING.md CHANGELOG.md src-tauri/core/src/lib.rs src-tauri/core/src/mcp src-tauri/core/src/commands_internal/journeys_cmd.rs src-tauri/core/src/commands_internal/mod.rs src-tauri/core/src/server/mod.rs tests/integration/utils/mcp.ts tests/integration/specs/tier2/mcp-endpoint.spec.ts
git commit -m "feat(mcp): read-only MCP endpoint at /mcp with list_vehicles"
```

---

## Phase B: after task 88 is merged

### Task B0: Rebase on 88 and check its interfaces

**Files:** none (read only).

- [ ] **Step 1: Rebase the branch on `main` with 88**

Run:
```bash
git fetch origin
git checkout feat/89-home-place-mcp
git rebase origin/main
ls src-tauri/core/migrations | grep places_as_entities
cargo test --manifest-path src-tauri/Cargo.toml --workspace
```
Expected: the rebase ends without conflicts (phase A touched no file that 88 rewrites, except maybe `CHANGELOG.md`: keep both upgrade-note lines in one block). The `ls` prints `2026-10-05-100000_places_as_entities`. All tests pass, also the phase A tests.

- [ ] **Step 2: Confirm the interfaces this plan uses**

Run:
```bash
grep -n "pub struct PlaceRow" -A10 src-tauri/core/src/models.rs
grep -n "pub struct Place\b" -A12 src-tauri/core/src/models.rs
grep -n "pub fn from_row" src-tauri/core/src/models.rs
grep -n "origin_place_id\|destination_place_id" src-tauri/core/src/models.rs | head
grep -n "fn place_names\|pub fn get_place\b\|pub fn get_place_by_key\|pub fn all_places\|pub fn place_uses\|pub fn get_trips_for_vehicle_in_year\|fn ensure_place_for_test\|fn create_trip_named_for_test" src-tauri/core/src/db.rs
grep -n "pub fn get_vehicles_internal" src-tauri/core/src/commands_internal/vehicles.rs
grep -n "export async function ensurePlace\|export function clearPlaceCache" tests/integration/utils/db.ts
grep -n "onclick=\|on:click" src/routes/miesta/+page.svelte | head -3
grep -n "data-testid=\"place-row\"\|data-place-id" src/routes/miesta/+page.svelte
```
Expected:
- `PlaceRow { id: String, name: String, normalised_name: String, lat: Option<f64>, lon: Option<f64>, source: Option<String>, created_at: String }`, with `Queryable, Selectable`
- `Place { id: Uuid, name, normalised_name, lat, lon, source: Option<PlaceSource>, uses: i64 }` with `#[serde(rename_all = "camelCase")]`, and `Place::from_row(row: PlaceRow, uses: i64) -> Self`
- `Trip::from_row(row: TripRow, names: &HashMap<String, String>) -> Self`, and `Trip.origin_place_id: Uuid`, `Trip.destination_place_id: Uuid`, plus the display strings `origin` and `destination`
- the private `fn place_names(conn: &mut SqliteConnection) -> QueryResult<HashMap<String, String>>` in `db.rs`
- `#[cfg(test)] Database::ensure_place_for_test(&self, name: &str) -> Uuid`
- `get_vehicles_internal(db: &Database) -> Result<Vec<Vehicle>, String>`
- `ensurePlace(name: string): Promise<string>` and `clearPlaceCache()` in `utils/db.ts`
- each Miesta row has `data-testid="place-row"` and `data-place-id`; note which event syntax the page uses (`onclick=` or `on:click`)

If a name differs, use the real name in every task below. Do not change 88 code to fit this plan.

---

### Task B1: Home mark on places (migration, DB, command)

**Files:**
- Create: [migrations/2026-10-05-110000_add_place_is_home/](../../../src-tauri/core/migrations/) `up.sql`, `down.sql`
- Modify: [schema.rs](../../../src-tauri/core/src/schema.rs) (table `places`), [models.rs](../../../src-tauri/core/src/models.rs) (`PlaceRow`, `Place`, `Place::from_row`), [db.rs](../../../src-tauri/core/src/db.rs), [places_cmd.rs](../../../src-tauri/core/src/commands_internal/places_cmd.rs), [dispatcher.rs](../../../src-tauri/core/src/server/dispatcher.rs)
- Test: [db_tests.rs](../../../src-tauri/core/src/db_tests.rs), [places_cmd_tests.rs](../../../src-tauri/core/src/commands_internal/places_cmd_tests.rs)

**Interfaces:**
- Consumes (from task 88): `PlaceRow`, `Place`, `Place::from_row`, `Database::all_places`, `Database::ensure_place_for_test`, `list_places_internal`.
- Produces:
  - `PlaceRow.is_home: bool` (last field), `Place.is_home: bool` (JSON `isHome`)
  - `Database::get_home_place(&self) -> QueryResult<Option<PlaceRow>>`
  - `Database::set_home_place(&self, id: Option<&str>) -> QueryResult<()>` (unknown ID: `Err(diesel::result::Error::NotFound)`)
  - `set_home_place_internal(db: &Database, app_state: &AppState, id: Option<String>) -> Result<(), String>`
  - RPC `set_home_place { id: string | null }`

- [ ] **Step 1: Write the migration**

`up.sql`:
```sql
-- Task 89: one place can be the home of the logbook. The journey grouping
-- (journeys module) matches trip endpoints against it by place ID.
ALTER TABLE places ADD COLUMN is_home BOOLEAN NOT NULL DEFAULT 0;

-- At most one home. A partial index ignores every row with is_home = 0.
CREATE UNIQUE INDEX idx_places_single_home ON places(is_home) WHERE is_home = 1;
```

`down.sql`:
```sql
DROP INDEX IF EXISTS idx_places_single_home;
ALTER TABLE places DROP COLUMN is_home;
```

- [ ] **Step 2: Add the column to `schema.rs` and the models**

In `schema.rs`, table `places`, add `is_home -> Bool,` as the **last** column, with a comment like the ones on `trip_routes`: `PlaceRow` is `Queryable` and binds by position.

In `models.rs`:
- add `pub is_home: bool,` as the last field of `PlaceRow`;
- add `pub is_home: bool,` to `Place`, after `source`;
- in `Place::from_row`, set `is_home: row.is_home,`. Every path that builds a `Place` (`list_places_internal`, `create_place_internal`) then reports the mark.

Do not add `is_home` to `NewPlaceRow`: the DB default `0` applies on insert.

- [ ] **Step 3: Write the failing DB tests**

Add to `db_tests.rs`:
```rust
// ============================================================================
// Home mark (task 89)
// ============================================================================

#[test]
fn home_place_is_none_by_default() {
    let db = Database::in_memory().unwrap();
    db.ensure_place_for_test("Home St 1, Hometown");
    assert!(db.get_home_place().unwrap().is_none());
}

#[test]
fn set_home_place_moves_the_mark() {
    let db = Database::in_memory().unwrap();
    let a = db.ensure_place_for_test("Home St 1, Hometown").to_string();
    let b = db.ensure_place_for_test("City A").to_string();

    db.set_home_place(Some(&a)).unwrap();
    assert_eq!(db.get_home_place().unwrap().unwrap().id, a);

    db.set_home_place(Some(&b)).unwrap();
    assert_eq!(db.get_home_place().unwrap().unwrap().id, b);
    let homes = db.all_places().unwrap().into_iter().filter(|p| p.is_home).count();
    assert_eq!(homes, 1);
}

#[test]
fn set_home_place_none_clears_the_mark() {
    let db = Database::in_memory().unwrap();
    let a = db.ensure_place_for_test("Home St 1, Hometown").to_string();
    db.set_home_place(Some(&a)).unwrap();
    db.set_home_place(None).unwrap();
    assert!(db.get_home_place().unwrap().is_none());
}

#[test]
fn set_home_place_unknown_id_fails_and_keeps_the_old_mark() {
    let db = Database::in_memory().unwrap();
    let a = db.ensure_place_for_test("Home St 1, Hometown").to_string();
    db.set_home_place(Some(&a)).unwrap();

    let err = db.set_home_place(Some("no-such-id"));
    assert!(matches!(err, Err(diesel::result::Error::NotFound)));
    assert_eq!(db.get_home_place().unwrap().unwrap().id, a);
}

#[test]
fn second_home_violates_unique_index() {
    let db = Database::in_memory().unwrap();
    db.ensure_place_for_test("Home St 1, Hometown");
    db.ensure_place_for_test("City A");
    let result = diesel::sql_query("UPDATE places SET is_home = 1")
        .execute(&mut *db.connection());
    assert!(result.is_err(), "the partial unique index must allow one home only");
}
```

- [ ] **Step 4: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core home_place`
Expected: compile error, `no method named get_home_place` / `set_home_place`.

- [ ] **Step 5: Implement the DB functions**

In `db.rs`, in the place-book section:
```rust
    /// The place marked as home, if any (task 89). The partial unique index
    /// `idx_places_single_home` guarantees at most one row.
    pub fn get_home_place(&self) -> QueryResult<Option<PlaceRow>> {
        let conn = &mut *self.conn.lock().unwrap();
        places::table
            .filter(places::is_home.eq(true))
            .select(PlaceRow::as_select())
            .first(conn)
            .optional()
    }

    /// Move the home mark to `id`, or clear it with `None`. One transaction:
    /// the old mark goes first, so the unique index never sees two homes.
    /// An unknown ID rolls back and returns `NotFound`, so the old mark stays.
    pub fn set_home_place(&self, id: Option<&str>) -> QueryResult<()> {
        let conn = &mut *self.conn.lock().unwrap();
        conn.transaction::<_, diesel::result::Error, _>(|tx| {
            diesel::update(places::table.filter(places::is_home.eq(true)))
                .set(places::is_home.eq(false))
                .execute(tx)?;
            if let Some(id) = id {
                let changed = diesel::update(places::table.filter(places::id.eq(id)))
                    .set(places::is_home.eq(true))
                    .execute(tx)?;
                if changed == 0 {
                    return Err(diesel::result::Error::NotFound);
                }
            }
            Ok(())
        })
    }
```

- [ ] **Step 6: Run the DB tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core home_place`
Expected: PASS, 4 tests.
Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core second_home`
Expected: PASS, 1 test.

- [ ] **Step 7: Write the failing command tests**

Add to `places_cmd_tests.rs`:
```rust
#[test]
fn set_home_place_internal_marks_and_list_places_reports_it() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let id = db.ensure_place_for_test("Home St 1, Hometown");

    set_home_place_internal(&db, &app_state, Some(id.to_string())).unwrap();

    let places = list_places_internal(&db).unwrap();
    let home: Vec<_> = places.iter().filter(|p| p.is_home).collect();
    assert_eq!(home.len(), 1);
    assert_eq!(home[0].id, id);
}

#[test]
fn set_home_place_internal_unknown_id_is_an_error() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    let err = set_home_place_internal(&db, &app_state, Some(uuid::Uuid::new_v4().to_string()));
    assert_eq!(err.unwrap_err(), "Place not found");
}

#[test]
fn set_home_place_internal_refuses_in_read_only_mode() {
    let db = Database::in_memory().unwrap();
    let app_state = AppState::new();
    app_state.enable_read_only("test");
    let id = db.ensure_place_for_test("Home St 1, Hometown");
    assert!(set_home_place_internal(&db, &app_state, Some(id.to_string())).is_err());
    assert!(db.get_home_place().unwrap().is_none());
}
```

- [ ] **Step 8: Run them and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core set_home_place_internal`
Expected: compile error, `cannot find function set_home_place_internal`.

- [ ] **Step 9: Implement the command and the dispatcher arm**

In `places_cmd.rs`:
```rust
/// Mark a place as home, or clear the mark with `None` (task 89). The journey
/// grouping and the MCP tool `list_journeys` read this mark.
pub fn set_home_place_internal(
    db: &Database,
    app_state: &AppState,
    id: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    match db.set_home_place(id.as_deref()) {
        Ok(()) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err("Place not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}
```

In `dispatcher.rs`, in the place-book section:
```rust
        "set_home_place" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: Option<String>,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::set_home_place_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }
```

- [ ] **Step 10: Run the whole backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --workspace`
Expected: PASS, no failures. `test_migrated_schema_identical_to_fresh_schema` covers the new DDL. If a migration test lists the `places` columns, add `is_home` there.

- [ ] **Step 11: Update the upgrade notes**

The project rule is: a migration updates `### Pokyny k aktualizácii` in the same commit. In `## [Unreleased]` of [CHANGELOG.md](../../../CHANGELOG.md), add `places.is_home` to the migrations line (task 88 already has its own migration there; write "2 nové"):
```markdown
- **Migrácie databázy:** ... `places.is_home`: žiadne miesto nie je označené ako domov, kým ho používateľ neoznačí na karte Miesta. Návrat na starší obraz otvorí databázu len na čítanie.
```

- [ ] **Step 12: Commit**

```bash
git add src-tauri/core/migrations/2026-10-05-110000_add_place_is_home src-tauri/core/src/schema.rs src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs src-tauri/core/src/commands_internal/places_cmd.rs src-tauri/core/src/commands_internal/places_cmd_tests.rs src-tauri/core/src/server/dispatcher.rs CHANGELOG.md
git commit -m "feat(places): mark one place as home"
```

---

### Task B2: `Leg::from_trip`, DB range read and the rest of the facade

**Files:**
- Modify: [db.rs](../../../src-tauri/core/src/db.rs) (`get_trips_for_vehicle_in_range`)
- Modify: [journeys/mod.rs](../../../src-tauri/core/src/) (`Leg::from_trip`), [commands_internal/journeys_cmd.rs](../../../src-tauri/core/src/commands_internal/)
- Create: `commands_internal/journeys_cmd_tests.rs`

**Interfaces:**
- Consumes: Task B1 `Database::get_home_place`; Task A1 `Leg`, `group_journeys`, `overlaps`; Task A2 `LogbookReader`, `ReadError`; from task 88 `place_names(conn)`, `Trip::from_row(row, &names)`, `Database::ensure_place_for_test`; existing `get_vehicles_internal(&Database)`, `Database::get_vehicle(&str)`, `get_trips_for_vehicle(&str)`, `get_route_maps_for_trips(&[String]) -> QueryResult<HashMap<String, RouteMap>>` (`RouteMap.round_trip: bool`).
- Produces:
  - `Leg::from_trip(trip: &Trip) -> Leg`
  - `Database::get_trips_for_vehicle_in_range(&self, vehicle_id: &str, from: NaiveDate, to: NaiveDate) -> QueryResult<Vec<Trip>>`, ordered by `start_datetime` ASC, then `odometer` ASC
  - `pub const HOME_NOT_SET: &str = "Home place is not set. Mark a place as home on the Miesta page.";`
  - `pub struct JourneyList { pub home_place: String, pub journeys: Vec<Journey> }`
  - Two more methods on `LogbookReader` (Task A2 has `new` and `vehicles`):
    - `trips_in_range(&self, from: NaiveDate, to: NaiveDate, vehicle_id: Option<&str>) -> Result<Vec<Trip>, ReadError>`
    - `journeys(&self, from: NaiveDate, to: NaiveDate, vehicle_id: Option<&str>) -> Result<JourneyList, ReadError>`

- [ ] **Step 1: Write the failing tests**

`src-tauri/core/src/commands_internal/journeys_cmd_tests.rs`:
```rust
use super::*;
use crate::models::{Trip, Vehicle};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn setup() -> (Arc<Database>, LogbookReader) {
    let db = Arc::new(Database::in_memory().unwrap());
    let reader = LogbookReader::new(db.clone());
    (db, reader)
}

fn vehicle(db: &Database, name: &str, active: bool) -> Vehicle {
    let mut v = Vehicle::new_ice(name.into(), "TEST-1".into(), 50.0, 6.5, 0.0);
    v.is_active = active;
    db.create_vehicle(&v).unwrap();
    v
}

fn trip(db: &Database, v: &Vehicle, at: &str, odo: f64, from: Uuid, to: Uuid, km: f64) -> Trip {
    let mut t = Trip::test_ice_trip(d("2026-01-01"), km, None, false);
    t.vehicle_id = v.id;
    t.start_datetime = NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M").unwrap();
    t.odometer = odo;
    t.origin_place_id = from;
    t.destination_place_id = to;
    t.purpose = "Project".into();
    db.create_trip(&t).unwrap();
    t
}

fn mark_round_trip(db: &Database, trip_id: &str) {
    diesel::sql_query(
        "INSERT INTO trip_routes (trip_id, waypoints, polyline, target_km, road_km, created_at, round_trip) \
         VALUES (?, '[]', '', 0, 0, '2026-01-01T00:00:00', 1)",
    )
    .bind::<diesel::sql_types::Text, _>(trip_id)
    .execute(&mut *db.connection())
    .unwrap();
}

#[test]
fn trips_in_range_are_inclusive_and_ascending() {
    let (db, _) = setup();
    let v = vehicle(&db, "Car", true);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    trip(&db, &v, "2026-01-31 23:59", 100.0, home, a, 10.0);
    trip(&db, &v, "2026-02-01 00:00", 110.0, a, home, 10.0);
    trip(&db, &v, "2026-02-28 12:00", 120.0, home, a, 10.0);
    trip(&db, &v, "2026-03-01 00:00", 130.0, a, home, 10.0);

    let trips = db
        .get_trips_for_vehicle_in_range(&v.id.to_string(), d("2026-02-01"), d("2026-02-28"))
        .unwrap();

    let odos: Vec<f64> = trips.iter().map(|t| t.odometer).collect();
    assert_eq!(odos, vec![110.0, 120.0]);
    assert_eq!(trips[0].origin, "City A", "the range read fills the place names");
}

#[test]
fn trips_without_vehicle_cover_all_vehicles_sorted_by_time() {
    let (db, reader) = setup();
    let selected = vehicle(&db, "Selected", true);
    let other = vehicle(&db, "Other", false);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    trip(&db, &selected, "2026-02-02 08:00", 100.0, home, a, 10.0);
    trip(&db, &other, "2026-02-03 08:00", 500.0, home, a, 10.0);
    trip(&db, &selected, "2026-02-04 08:00", 110.0, a, home, 10.0);

    let trips = reader.trips_in_range(d("2026-02-01"), d("2026-02-28"), None).unwrap();

    let owners: Vec<Uuid> = trips.iter().map(|t| t.vehicle_id).collect();
    assert_eq!(owners, vec![selected.id, other.id, selected.id]);
}

#[test]
fn unknown_vehicle_is_an_error() {
    let (_, reader) = setup();
    let id = Uuid::new_v4().to_string();
    let err = reader.trips_in_range(d("2026-02-01"), d("2026-02-28"), Some(&id));
    assert_eq!(err.unwrap_err(), ReadError::Invalid("Vehicle not found".into()));
}

#[test]
fn date_from_after_date_to_is_an_error() {
    let (_, reader) = setup();
    let err = reader.trips_in_range(d("2026-03-01"), d("2026-02-01"), None);
    assert_eq!(err.unwrap_err(), ReadError::Invalid("date_from is after date_to".into()));
}

#[test]
fn journeys_without_home_is_an_error() {
    let (db, reader) = setup();
    vehicle(&db, "Car", true);
    let err = reader.journeys(d("2026-01-01"), d("2026-12-31"), None);
    assert_eq!(err.unwrap_err(), ReadError::Invalid(HOME_NOT_SET.into()));
}

#[test]
fn journey_that_starts_before_range_is_found() {
    let (db, reader) = setup();
    let v = vehicle(&db, "Car", true);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    db.set_home_place(Some(&home.to_string())).unwrap();
    trip(&db, &v, "2026-01-27 07:00", 100.0, home, a, 357.0);
    trip(&db, &v, "2026-02-10 16:00", 457.0, a, home, 357.0);

    let list = reader.journeys(d("2026-02-01"), d("2026-02-28"), None).unwrap();

    assert_eq!(list.home_place, "Home St 1, Hometown");
    assert_eq!(list.journeys.len(), 1);
    assert_eq!(list.journeys[0].nights, Some(14));
}

#[test]
fn round_trip_map_from_db_makes_a_day_trip() {
    let (db, reader) = setup();
    let v = vehicle(&db, "Car", true);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let village = db.ensure_place_for_test("Village");
    db.set_home_place(Some(&home.to_string())).unwrap();
    let t = trip(&db, &v, "2026-05-04 08:00", 100.0, home, village, 52.0);
    mark_round_trip(&db, &t.id.to_string());

    let list = reader.journeys(d("2026-05-01"), d("2026-05-31"), None).unwrap();

    assert!(list.journeys.is_empty());
}

#[test]
fn journeys_of_one_vehicle_only_when_vehicle_given() {
    let (db, reader) = setup();
    let car = vehicle(&db, "Car", true);
    let van = vehicle(&db, "Van", false);
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    db.set_home_place(Some(&home.to_string())).unwrap();
    for v in [&car, &van] {
        trip(&db, v, "2026-06-01 07:00", 100.0, home, a, 50.0);
        trip(&db, v, "2026-06-02 07:00", 150.0, a, home, 50.0);
    }

    let all = reader.journeys(d("2026-06-01"), d("2026-06-30"), None).unwrap();
    assert_eq!(all.journeys.len(), 2, "None means all vehicles, also not selected ones");

    let id = van.id.to_string();
    let one = reader.journeys(d("2026-06-01"), d("2026-06-30"), Some(&id)).unwrap();
    assert_eq!(one.journeys.len(), 1);
    assert_eq!(one.journeys[0].vehicle_id, van.id);
}
```
`ensure_place_for_test` returns a `Uuid`. If task 88 changed `Vehicle::new_ice`, use the constructor that `db_tests.rs` uses.

- [ ] **Step 2: Add `Leg::from_trip` and the facade stubs**

In `journeys/mod.rs`, add `use crate::models::Trip;` to the imports, and after `pub struct Leg { ... }`:
```rust
impl Leg {
    pub fn from_trip(trip: &Trip) -> Self {
        Self {
            id: trip.id,
            start: trip.start_datetime,
            odometer: trip.odometer,
            origin_place_id: trip.origin_place_id,
            destination_place_id: trip.destination_place_id,
            destination_name: trip.destination.clone(),
            distance_km: trip.distance_km,
            purpose: trip.purpose.clone(),
        }
    }
}
```

Replace `src-tauri/core/src/commands_internal/journeys_cmd.rs` (it keeps the A2 `vehicles` body):
```rust
//! Read-only logbook queries for the MCP endpoint (task 89).
//!
//! `LogbookReader` is the only thing the `mcp` module holds. It keeps its
//! `Database` private and exposes three reads, so the MCP module has no path
//! to a write. The source guard in `mcp/tests.rs` checks this file too.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::commands_internal::get_vehicles_internal;
use crate::db::Database;
use crate::journeys::{group_journeys, overlaps, Journey, Leg};
use crate::models::{Trip, Vehicle};

pub const HOME_NOT_SET: &str =
    "Home place is not set. Mark a place as home on the Miesta page.";

/// `Invalid`: the caller can fix it (bad range, unknown vehicle, no home).
/// `Internal`: a DB error.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    Invalid(String),
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct JourneyList {
    pub home_place: String,
    pub journeys: Vec<Journey>,
}

#[derive(Clone)]
pub struct LogbookReader {
    db: Arc<Database>,
}

impl LogbookReader {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn vehicles(&self) -> Result<Vec<Vehicle>, ReadError> {
        get_vehicles_internal(&self.db).map_err(ReadError::Internal)
    }

    pub fn trips_in_range(
        &self,
        _from: NaiveDate,
        _to: NaiveDate,
        _vehicle_id: Option<&str>,
    ) -> Result<Vec<Trip>, ReadError> {
        Err(ReadError::Internal("not implemented".into()))
    }

    pub fn journeys(
        &self,
        _from: NaiveDate,
        _to: NaiveDate,
        _vehicle_id: Option<&str>,
    ) -> Result<JourneyList, ReadError> {
        Err(ReadError::Internal("not implemented".into()))
    }
}

#[cfg(test)]
#[path = "journeys_cmd_tests.rs"]
mod tests;
```
`commands_internal/mod.rs` already has `pub mod journeys_cmd;` from Task A2.

- [ ] **Step 3: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core journeys_cmd`
Expected: compile error, `no method named get_trips_for_vehicle_in_range`.

- [ ] **Step 4: Implement the DB range read**

In `db.rs`, next to `get_trips_for_vehicle_in_year`. It uses the same name map as the other trip reads after task 88:
```rust
    /// Trips of a vehicle whose start is in `from..=to` (both days inclusive),
    /// oldest first. Same "YYYY-MM-DDTHH:MM:SS" string range as the year query.
    pub fn get_trips_for_vehicle_in_range(
        &self,
        vehicle_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> QueryResult<Vec<Trip>> {
        use crate::schema::trips::dsl;
        let conn = &mut *self.conn.lock().unwrap();

        let start = format!("{}T00:00:00", from.format("%Y-%m-%d"));
        let end = format!("{}T23:59:59", to.format("%Y-%m-%d"));

        let rows = dsl::trips
            .filter(dsl::vehicle_id.eq(vehicle_id))
            .filter(dsl::start_datetime.ge(&start))
            .filter(dsl::start_datetime.le(&end))
            .order((dsl::start_datetime.asc(), dsl::odometer.asc()))
            .load::<TripRow>(conn)?;
        let names = place_names(conn)?;

        Ok(rows.into_iter().map(|r| Trip::from_row(r, &names)).collect())
    }
```
Import `chrono::NaiveDate` in `db.rs` if it is not imported.

- [ ] **Step 5: Implement the facade**

Replace the `impl LogbookReader` block in `journeys_cmd.rs`:
```rust
fn internal(e: impl std::fmt::Display) -> ReadError {
    ReadError::Internal(e.to_string())
}

impl LogbookReader {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn vehicles(&self) -> Result<Vec<Vehicle>, ReadError> {
        get_vehicles_internal(&self.db).map_err(ReadError::Internal)
    }

    /// The vehicles a query covers: the one asked for, or ALL vehicles.
    /// `is_active` marks only the vehicle selected in the UI, so it is not a filter.
    fn vehicles_for(&self, vehicle_id: Option<&str>) -> Result<Vec<Vehicle>, ReadError> {
        match vehicle_id {
            Some(id) => self
                .db
                .get_vehicle(id)
                .map_err(internal)?
                .map(|v| vec![v])
                .ok_or_else(|| ReadError::Invalid("Vehicle not found".into())),
            None => self.vehicles(),
        }
    }

    fn check_range(from: NaiveDate, to: NaiveDate) -> Result<(), ReadError> {
        if from > to {
            return Err(ReadError::Invalid("date_from is after date_to".into()));
        }
        Ok(())
    }

    /// Trips of the range, all covered vehicles merged, oldest first.
    pub fn trips_in_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        vehicle_id: Option<&str>,
    ) -> Result<Vec<Trip>, ReadError> {
        Self::check_range(from, to)?;
        let mut trips = Vec::new();
        for vehicle in self.vehicles_for(vehicle_id)? {
            trips.extend(
                self.db
                    .get_trips_for_vehicle_in_range(&vehicle.id.to_string(), from, to)
                    .map_err(internal)?,
            );
        }
        trips.sort_by(|a, b| {
            a.start_datetime
                .cmp(&b.start_datetime)
                .then(a.odometer.total_cmp(&b.odometer))
        });
        Ok(trips)
    }

    /// Journeys that overlap `from..=to`. All legs of each vehicle are grouped
    /// first and filtered after, so a journey that starts before `from` is found.
    pub fn journeys(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        vehicle_id: Option<&str>,
    ) -> Result<JourneyList, ReadError> {
        Self::check_range(from, to)?;
        let home = self
            .db
            .get_home_place()
            .map_err(internal)?
            .ok_or_else(|| ReadError::Invalid(HOME_NOT_SET.into()))?;
        let home_id = Uuid::parse_str(&home.id).map_err(internal)?;

        let mut journeys = Vec::new();
        for vehicle in self.vehicles_for(vehicle_id)? {
            let trips = self
                .db
                .get_trips_for_vehicle(&vehicle.id.to_string())
                .map_err(internal)?;
            let trip_ids: Vec<String> = trips.iter().map(|t| t.id.to_string()).collect();
            let round_trip_ids: HashSet<Uuid> = self
                .db
                .get_route_maps_for_trips(&trip_ids)
                .map_err(internal)?
                .into_iter()
                .filter(|(_, map)| map.round_trip)
                .filter_map(|(id, _)| Uuid::parse_str(&id).ok())
                .collect();
            let legs: Vec<Leg> = trips.iter().map(Leg::from_trip).collect();
            journeys.extend(
                group_journeys(vehicle.id, &legs, home_id, &round_trip_ids)
                    .into_iter()
                    .filter(|j| overlaps(j, from, to)),
            );
        }
        journeys.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(JourneyList {
            home_place: home.name,
            journeys,
        })
    }
}
```
Check the guard words against this file: `get_vehicles_internal`, `get_home_place`, `get_route_maps_for_trips` and the comments contain none of them.

- [ ] **Step 6: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core journeys_cmd`
Expected: PASS, 8 tests.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/core/src/db.rs src-tauri/core/src/journeys/mod.rs src-tauri/core/src/commands_internal/journeys_cmd.rs src-tauri/core/src/commands_internal/journeys_cmd_tests.rs
git commit -m "feat(journeys): read-only facade for trips and journeys in a date range"
```

---

### Task B3: `list_trips` and `list_journeys` tools

**Files:**
- Modify: [mcp/mod.rs](../../../src-tauri/core/src/), [mcp/tests.rs](../../../src-tauri/core/src/), [tier2/mcp-endpoint.spec.ts](../../../tests/integration/specs/tier2/)

**Interfaces:**
- Consumes: Task A2 `KnihaJazdMcp`, `mcp_service`, `blocking`, `to_error`, the test helpers `start`, `post`, `call`, `tool_names`; Task B2 `LogbookReader::trips_in_range`, `LogbookReader::journeys`, `HOME_NOT_SET`; from task 88 `Database::ensure_place_for_test`, `Trip.origin_place_id`, `Trip.destination_place_id`.
- Produces: tools `list_trips(date_from, date_to, vehicle_id?)` and `list_journeys(date_from, date_to, vehicle_id?)`. With `list_vehicles` from A2, `tools/list` returns exactly three tools.

- [ ] **Step 1: Write the failing tests**

In `src-tauri/core/src/mcp/tests.rs`, change the assertion in `tools_list_returns_the_read_only_tools`:
```rust
    assert_eq!(tool_names(&body), vec!["list_journeys", "list_trips", "list_vehicles"]);
```
and remove the comment line `// Phase A: one tool. Task B3 changes this to the three tools.` Then add these tests before `mcp_read_path_has_no_write_access`:
```rust
#[tokio::test]
async fn list_journeys_without_home_is_invalid_params() {
    let (url, stop) = start(Arc::new(Database::in_memory().unwrap())).await;

    let body = post(&url, call(2, "list_journeys",
        json!({"date_from": "2026-01-01", "date_to": "2026-01-31"})), &[]).await;

    assert_eq!(body["error"]["code"], -32602);
    assert_eq!(
        body["error"]["message"].as_str().unwrap(),
        crate::commands_internal::HOME_NOT_SET
    );
    let _ = stop.send(());
}

#[tokio::test]
async fn malformed_date_is_invalid_params() {
    let (url, stop) = start(Arc::new(Database::in_memory().unwrap())).await;

    let body = post(&url, call(3, "list_trips",
        json!({"date_from": "2026-1-1", "date_to": "2026-01-31"})), &[]).await;

    assert_eq!(body["error"]["code"], -32602);
    assert!(body["error"]["message"].as_str().unwrap().contains("YYYY-MM-DD"));
    let _ = stop.send(());
}

#[tokio::test]
async fn list_trips_returns_place_names_and_minutes() {
    let db = Arc::new(Database::in_memory().unwrap());
    let v = crate::models::Vehicle::new_ice("Car".into(), "TEST-1".into(), 50.0, 6.5, 0.0);
    db.create_vehicle(&v).unwrap();
    let home = db.ensure_place_for_test("Home St 1, Hometown");
    let a = db.ensure_place_for_test("City A");
    let day = chrono::NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
    let mut t = crate::models::Trip::test_ice_trip(day, 357.0, None, false);
    t.vehicle_id = v.id;
    t.start_datetime = day.and_hms_opt(7, 30, 0).unwrap();
    t.origin_place_id = home;
    t.destination_place_id = a;
    t.purpose = "Customer visit".into();
    db.create_trip(&t).unwrap();
    let (url, stop) = start(db).await;

    let body = post(&url, call(5, "list_trips",
        json!({"date_from": "2026-03-01", "date_to": "2026-03-31"})), &[]).await;

    let trips = &body["result"]["structuredContent"]["trips"];
    assert_eq!(trips[0]["start"], "2026-03-02T07:30");
    assert_eq!(trips[0]["origin"], "Home St 1, Hometown");
    assert_eq!(trips[0]["destination"], "City A");
    assert_eq!(trips[0]["distance_km"], 357.0);
    assert_eq!(trips[0]["purpose"], "Customer visit");
    let _ = stop.send(());
}

```
In `tests/integration/specs/tier2/mcp-endpoint.spec.ts`, replace the constant and its comment:
```typescript
const EXPECTED_TOOLS = ['list_journeys', 'list_trips', 'list_vehicles'];
```

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core mcp::`
Expected: FAIL. `tools_list_returns_the_read_only_tools` fails (left: `["list_vehicles"]`). The three new tests fail: a call to an unknown tool gives a JSON-RPC `error` with a different code and message, so `list_journeys_without_home_is_invalid_params` fails on the message and `list_trips_returns_place_names_and_minutes` fails on `trips[0]["start"]` (null).

- [ ] **Step 3: Implement the two tools**

Replace `src-tauri/core/src/mcp/mod.rs`:
```rust
//! Read-only MCP endpoint at `/mcp` (task 89, ADR "Read-only MCP endpoint").
//!
//! Stateless streamable HTTP with `rmcp`: no session IDs, so a client that
//! cached one keeps working after a restart. The tools call `LogbookReader`
//! directly, not `/api/rpc`. This module never holds a `Database`; the test
//! `mcp_read_path_has_no_write_access` checks the sources.

use std::sync::Arc;

use chrono::{NaiveDate, NaiveDateTime};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::{Deserialize, Serialize};

use crate::commands_internal::{LogbookReader, ReadError};
use crate::db::Database;

// ---------------------------------------------------------------- inputs

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RangeArgs {
    /// First day, YYYY-MM-DD, inclusive.
    pub date_from: String,
    /// Last day, YYYY-MM-DD, inclusive.
    pub date_to: String,
    /// Vehicle ID. Leave out for all vehicles.
    #[serde(default)]
    pub vehicle_id: Option<String>,
}

// ---------------------------------------------------------------- outputs

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VehicleOut {
    pub id: String,
    pub name: String,
    pub license_plate: String,
    /// True for the one vehicle that is selected in the app UI.
    pub is_active: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VehicleList {
    pub vehicles: Vec<VehicleOut>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TripOut {
    pub id: String,
    pub vehicle_id: String,
    /// YYYY-MM-DDTHH:MM
    pub start: String,
    /// YYYY-MM-DDTHH:MM, or null if the trip has no end time.
    pub end: Option<String>,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TripList {
    pub trips: Vec<TripOut>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct JourneyOut {
    pub vehicle_id: String,
    /// Start of the first leg, YYYY-MM-DDTHH:MM.
    pub start: String,
    /// Start of the return leg, or null if the journey is incomplete.
    pub end: Option<String>,
    /// Calendar days between the start date and the end date, or null.
    pub nights: Option<i64>,
    pub total_km: f64,
    /// Distinct destinations in order, without home.
    pub places: Vec<String>,
    pub purposes: Vec<String>,
    pub complete: bool,
    pub leg_ids: Vec<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct JourneyListOut {
    pub home_place: String,
    pub journeys: Vec<JourneyOut>,
}

fn minute(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M").to_string()
}

/// Strict YYYY-MM-DD: "2026-1-1" is refused, not read as January 1.
fn parse_day(field: &str, value: &str) -> Result<NaiveDate, ErrorData> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .filter(|d| d.format("%Y-%m-%d").to_string() == value)
        .ok_or_else(|| {
            ErrorData::invalid_params(format!("{field} must be YYYY-MM-DD, got '{value}'"), None)
        })
}

fn to_error(e: ReadError) -> ErrorData {
    match e {
        ReadError::Invalid(m) => ErrorData::invalid_params(m, None),
        ReadError::Internal(m) => ErrorData::internal_error(m, None),
    }
}

/// Run a read on the blocking pool, the same as `rpc_handler` does for DB work.
async fn blocking<T, F>(read: F) -> Result<T, ErrorData>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ReadError> + Send + 'static,
{
    tokio::task::spawn_blocking(read)
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(to_error)
}

// ---------------------------------------------------------------- server

#[derive(Clone)]
pub struct KnihaJazdMcp {
    reader: LogbookReader,
    tool_router: ToolRouter<Self>,
}

impl KnihaJazdMcp {
    pub fn new(reader: LogbookReader) -> Self {
        Self { reader, tool_router: Self::tool_router() }
    }
}

#[tool_router]
impl KnihaJazdMcp {
    #[tool(description = "Read-only. Lists all vehicles of the logbook: id, name, license_plate, is_active (true for the vehicle selected in the app UI).")]
    async fn list_vehicles(&self) -> Result<Json<VehicleList>, ErrorData> {
        let reader = self.reader.clone();
        let vehicles = blocking(move || reader.vehicles()).await?;
        Ok(Json(VehicleList {
            vehicles: vehicles
                .into_iter()
                .map(|v| VehicleOut {
                    id: v.id.to_string(),
                    name: v.name,
                    license_plate: v.license_plate,
                    is_active: v.is_active,
                })
                .collect(),
        }))
    }

    #[tool(description = "Read-only. Lists the trip rows (legs) whose start is between date_from and date_to (YYYY-MM-DD, both inclusive), sorted by start time. Without vehicle_id: all vehicles.")]
    async fn list_trips(&self, Parameters(args): Parameters<RangeArgs>) -> Result<Json<TripList>, ErrorData> {
        let from = parse_day("date_from", &args.date_from)?;
        let to = parse_day("date_to", &args.date_to)?;
        let reader = self.reader.clone();
        let trips = blocking(move || reader.trips_in_range(from, to, args.vehicle_id.as_deref())).await?;
        Ok(Json(TripList {
            trips: trips
                .into_iter()
                .map(|t| TripOut {
                    id: t.id.to_string(),
                    vehicle_id: t.vehicle_id.to_string(),
                    start: minute(t.start_datetime),
                    end: t.end_datetime.map(minute),
                    origin: t.origin,
                    destination: t.destination,
                    distance_km: t.distance_km,
                    purpose: t.purpose,
                })
                .collect(),
        }))
    }

    #[tool(description = "Read-only. Groups trip legs into journeys away from the home place and returns each journey that overlaps date_from..date_to (YYYY-MM-DD, both inclusive). Without vehicle_id: all vehicles. A journey starts at a leg from home and ends at the next leg back home. Single-leg day trips and home-to-home loops are not journeys. complete=false means no return leg exists: the car has not come back yet, or it left home again before a return leg was recorded. Error if no home place is set. The app applies no accounting rule: filter on nights and total_km yourself.")]
    async fn list_journeys(&self, Parameters(args): Parameters<RangeArgs>) -> Result<Json<JourneyListOut>, ErrorData> {
        let from = parse_day("date_from", &args.date_from)?;
        let to = parse_day("date_to", &args.date_to)?;
        let reader = self.reader.clone();
        let list = blocking(move || reader.journeys(from, to, args.vehicle_id.as_deref())).await?;
        Ok(Json(JourneyListOut {
            home_place: list.home_place,
            journeys: list
                .journeys
                .into_iter()
                .map(|j| JourneyOut {
                    vehicle_id: j.vehicle_id.to_string(),
                    start: minute(j.start),
                    end: j.end.map(minute),
                    nights: j.nights,
                    total_km: j.total_km,
                    places: j.places,
                    purposes: j.purposes,
                    complete: j.complete,
                    leg_ids: j.leg_ids.iter().map(|id| id.to_string()).collect(),
                })
                .collect(),
        }))
    }
}

#[tool_handler]
impl ServerHandler for KnihaJazdMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("kniha-jazd", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Read-only access to a vehicle logbook (Kniha jázd). No tool can change data.",
            )
    }
}

/// The `/mcp` service. Stateless: no session store, no SSE keep-alive, JSON
/// answers. The host check is off: behind a reverse proxy the Host header is
/// the public name, and `/api/rpc` (which can write) has no host check either.
pub fn mcp_service(db: Arc<Database>) -> StreamableHttpService<KnihaJazdMcp, NeverSessionManager> {
    let reader = LogbookReader::new(db);
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .disable_allowed_hosts();
    StreamableHttpService::new(
        move || Ok(KnihaJazdMcp::new(reader.clone())),
        Default::default(),
        config,
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
```
Check the guard words against this source before you run the test: `with_server_info`, `with_instructions`, `disable_allowed_hosts` and the comments contain none of them.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core mcp::`
Expected: PASS, 7 tests.

Run:
```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
xvfb-run -a -s "-screen 0 1280x1024x24" npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/mcp-endpoint.spec.ts
npm run typecheck:tests
```
Expected: 1 passing; 0 type errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/core/src/mcp tests/integration/specs/tier2/mcp-endpoint.spec.ts
git commit -m "feat(mcp): list_trips and list_journeys tools"
```

---

### Task B4: Home icon on the Miesta tab

**Files:**
- Modify: [types.ts](../../../src/lib/types.ts) (`Place.isHome`), [api.ts](../../../src/lib/api.ts) (`setHomePlace`), [miesta/+page.svelte](../../../src/routes/), [sk/index.ts](../../../src/lib/i18n/sk/index.ts), [en/index.ts](../../../src/lib/i18n/en/index.ts)
- Create: [tier2/home-place.spec.ts](../../../tests/integration/specs/tier2/)

**Interfaces:**
- Consumes: Task B1 RPC `set_home_place { id: string | null }`, `list_places` with `isHome`; from task 88 the Miesta page, `data-testid="place-row"`, `data-place-id`, the tab link `a[href="/miesta"]`, `ensurePlace(name)`, and `seedTrip` that creates the places by name.
- Produces: button `data-testid="place-home-toggle"` with `aria-pressed="true|false"` on each place row.

- [ ] **Step 1: Write the failing integration test**

`tests/integration/specs/tier2/home-place.spec.ts`:
```typescript
/**
 * Tier 2: Home place mark on the Miesta tab (task 89)
 *
 * Covers the UI -> backend -> display flow of the home icon:
 * - A click on the home icon of a place marks it, and a reload keeps the mark
 * - A click on another place moves the mark
 *
 * NOT covered here: the one-home rule and the journey grouping. The Rust unit
 * tests own them (db_tests.rs, journeys/tests.rs).
 */

import { waitForAppReady } from '../../utils/app';
import { seedVehicle, seedTrip, setActiveVehicle, rpc, ensurePlace } from '../../utils/db';

const HOME = 'Home St 1, Hometown';
const CITY = 'City A, Testland';

async function openMiesta(): Promise<void> {
  const link = await $('a[href="/miesta"]');
  await link.click();
  await $('[data-testid="place-row"]').waitForDisplayed({ timeout: 5000 });
}

function homeToggle(id: string) {
  return $(`[data-testid="place-row"][data-place-id="${id}"] [data-testid="place-home-toggle"]`);
}

describe('Home place mark', () => {
  beforeEach(async () => {
    await waitForAppReady();
    const vehicle = await seedVehicle({
      name: 'Home Test Car',
      licensePlate: 'HOME-1',
      initialOdometer: 10000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);
    await seedTrip({
      vehicleId: vehicle.id as string,
      startDatetime: `${new Date().getFullYear()}-03-02T07:00`,
      origin: HOME,
      destination: CITY,
      distanceKm: 120,
      odometer: 10120,
      purpose: 'Test',
    });
    await rpc('set_home_place', { id: null });
  });

  it('marks a place as home and keeps the mark after a reload', async () => {
    const homeId = await ensurePlace(HOME);
    await openMiesta();

    await (await homeToggle(homeId)).click();
    await browser.waitUntil(
      async () => (await (await homeToggle(homeId)).getAttribute('aria-pressed')) === 'true',
      { timeout: 5000, timeoutMsg: 'home toggle did not switch on' }
    );

    await browser.refresh();
    await $('[data-testid="place-row"]').waitForDisplayed({ timeout: 5000 });
    expect(await (await homeToggle(homeId)).getAttribute('aria-pressed')).toBe('true');
  });

  it('moves the mark to another place', async () => {
    const homeId = await ensurePlace(HOME);
    const cityId = await ensurePlace(CITY);
    await rpc('set_home_place', { id: homeId });
    await openMiesta();

    await (await homeToggle(cityId)).click();
    await browser.waitUntil(
      async () => (await (await homeToggle(cityId)).getAttribute('aria-pressed')) === 'true',
      { timeout: 5000, timeoutMsg: 'home mark did not move' }
    );
    expect(await (await homeToggle(homeId)).getAttribute('aria-pressed')).toBe('false');
  });
});
```
The `seedVehicle` fields come from `SeedVehicleData` in [utils/db.ts](../../../tests/integration/utils/db.ts) (`initialOdometer` is required by `create_vehicle`, and an ICE vehicle needs `tankSizeLiters` and `tpConsumption`). Check the `seedTrip` field names there after task 88.

- [ ] **Step 2: Build, run the spec, and see it fail**

Run:
```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
xvfb-run -a -s "-screen 0 1280x1024x24" npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/home-place.spec.ts
```
Expected: FAIL in the `it` body (not in `beforeEach`): `element ("[data-testid=\"place-row\"][data-place-id=\"...\"] [data-testid=\"place-home-toggle\"]") still not existing`.

- [ ] **Step 3: Add the type, the API call and the i18n keys**

[types.ts](../../../src/lib/types.ts), interface `Place`: add `isHome: boolean;`.

[api.ts](../../../src/lib/api.ts), next to the other place calls:
```typescript
export async function setHomePlace(id: string | null): Promise<void> {
	return apiCall('set_home_place', { id });
}
```

[sk/index.ts](../../../src/lib/i18n/sk/index.ts), in `places`:
```typescript
		markHome: 'Označiť ako domov',
		unmarkHome: 'Zrušiť označenie domova',
		homeSet: 'Domov je nastavený',
		homeCleared: 'Označenie domova je zrušené',
		homeError: 'Domov sa nepodarilo nastaviť: {error:string}',
```
[en/index.ts](../../../src/lib/i18n/en/index.ts), in `places`:
```typescript
		markHome: 'Mark as home',
		unmarkHome: 'Remove the home mark',
		homeSet: 'Home is set',
		homeCleared: 'The home mark is removed',
		homeError: 'Could not set home: {error:string}',
```
Run: `npm run i18n`
Expected: `src/lib/i18n/i18n-types.ts` regenerates with the five keys.

- [ ] **Step 4: Add the button to each place row**

In `src/routes/miesta/+page.svelte`, add the handler next to the other place handlers. Use the place-list reload function of the page (Task B0 Step 2 shows its name):
```typescript
	async function toggleHome(place: Place) {
		const next = place.isHome ? null : place.id;
		try {
			await api.setHomePlace(next);
			toast.success(next ? $LL.places.homeSet() : $LL.places.homeCleared());
			await loadPlaces();
		} catch (error) {
			toast.error($LL.places.homeError({ error: String(error) }));
		}
	}
```
Import `appModeStore` from `$lib/stores/appMode` if the page does not import it. In the row markup (the element with `data-testid="place-row"`), add as the first action button. Use the event syntax that the page already uses (Task B0 Step 2): Svelte 5 rejects a mix of `onclick=` and `on:click` in one component. The example uses `onclick=`:
```svelte
<button
	type="button"
	class="place-home-toggle"
	class:is-home={place.isHome}
	data-testid="place-home-toggle"
	aria-pressed={place.isHome}
	title={place.isHome ? $LL.places.unmarkHome() : $LL.places.markHome()}
	aria-label={place.isHome ? $LL.places.unmarkHome() : $LL.places.markHome()}
	disabled={$appModeStore.isReadOnly}
	onclick={() => toggleHome(place)}
>
	<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24"
		fill={place.isHome ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2"
		stroke-linecap="round" stroke-linejoin="round"><path d="M3 10.5 12 3l9 7.5"/><path d="M5 9.5V21h14V9.5"/></svg>
</button>
```
Add the style. Use the colour variables that the page already uses if these two do not exist:
```css
	.place-home-toggle {
		background: none;
		border: none;
		cursor: pointer;
		color: var(--text-secondary);
		opacity: 0.5;
		padding: 0.25rem;
	}
	.place-home-toggle:hover,
	.place-home-toggle.is-home {
		opacity: 1;
		color: var(--accent-primary);
	}
```

- [ ] **Step 5: Run the spec and see it pass**

Run:
```bash
npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
xvfb-run -a -s "-screen 0 1280x1024x24" npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/home-place.spec.ts
npm run check && npm run typecheck:tests
```
Expected: 2 passing. `npm run check` and `typecheck:tests` report 0 errors.

- [ ] **Step 6: Commit**

```bash
git add src/lib/types.ts src/lib/api.ts src/routes/miesta/+page.svelte src/lib/i18n/sk/index.ts src/lib/i18n/en/index.ts src/lib/i18n/i18n-types.ts tests/integration/specs/tier2/home-place.spec.ts
git commit -m "feat(places): home icon on the Miesta tab"
```

---

### Task B5: Decisions, changelog, feature docs, READMEs, project guides

**Files:**
- Modify: [DECISIONS.md](../../../DECISIONS.md), [CHANGELOG.md](../../../CHANGELOG.md), [README.md](../../../README.md), [README.en.md](../../../README.en.md), [CLAUDE.md](../../../CLAUDE.md), [rust-backend.md](../../../.claude/rules/rust-backend.md), [place-book.md](../../../docs/features/place-book.md), [server-mode.md](../../../docs/features/server-mode.md), other docs in [docs/features/](../../../docs/features/) as found in Step 4
- Create: [docs/features/mcp-endpoint.md](../../../docs/features/)

- [ ] **Step 1: Record the decisions with `/decision`**

Take the numbers from `DECISIONS.md` at write time. Task 88 takes ADR-055, ADR-056 and BIZ-025, so expect ADR-057 and BIZ-026, but check:
```bash
grep -o "^### ADR-[0-9]*" DECISIONS.md | sort -t- -k2 -n | tail -1
grep -o "^### BIZ-[0-9]*" DECISIONS.md | sort -t- -k2 -n | tail -1
```
Run `/decision` two times:
- **ADR: Read-only MCP endpoint at `/mcp`.** Content:
  - stateless streamable HTTP with `rmcp` 3.5 (`legacy_session_mode(false)`, `NeverSessionManager`, JSON answers); why stateless (a cached session ID keeps working after a restart);
  - read-only by construction: the MCP module holds only `LogbookReader`, which exposes three reads; the source guard `mcp_read_path_has_no_write_access`;
  - no auth, the same as `/api/rpc`; host check off and why (reverse proxy; `/api/rpc` has none and can write);
  - `vehicle_id = None` means all vehicles, because `is_active` marks only the vehicle selected in the UI;
  - outputs are JSON objects because `structuredContent` must be an object;
  - error codes: invalid input and "no home place" are `invalid_params`, DB errors are `internal_error`;
  - DB work in `spawn_blocking`, the same as `/api/rpc`; Rust 1.88 for the build.
  Related: task 89, ADR-008.
- **BIZ: Journeys away from home.** Content: the six rules of the spec; the home match by place ID; the round-trip map makes a day trip; ordering by `start_datetime` then `odometer`; overlap with an open end for an incomplete journey; "no home place" is an error, not an empty list; **a chain of two or more legs with no return leg is an incomplete journey when the car leaves home again or drives a home -> home loop; a single leg followed by a leg from home is a day trip** (user decision, 2026-10-05); no accounting rule in the app. Related: task 89, task 88.

- [ ] **Step 2: Update the changelog with `/changelog`**

Task A2 already changed the image line, and Task B1 the migrations line, of `### Pokyny k aktualizácii`. Check that the block is still one block and complete. Add entries under `### Pridané`:
```markdown
- **Domov na karte Miesta** - ikona domčeka pri mieste ho označí ako domov. Domov môže byť len jeden.
- **MCP rozhranie len na čítanie (`/mcp`)** - AI asistent môže čítať vozidlá, jazdy a cesty mimo domova (`list_vehicles`, `list_trips`, `list_journeys`). Nič nemôže zmeniť.
```

- [ ] **Step 3: Write `docs/features/mcp-endpoint.md`**

Follow the template in [docs/CLAUDE.md](../../../docs/CLAUDE.md):
- Title "Feature: Read-Only MCP Endpoint".
- User Flow: mark home on Miesta, connect an MCP client to `https://<host>/mcp`, call the tools.
- Technical Implementation: [mcp/mod.rs](../../../src-tauri/core/src/), [journeys/mod.rs](../../../src-tauri/core/src/), [journeys_cmd.rs](../../../src-tauri/core/src/commands_internal/) (`LogbookReader`), the router line, the rmcp config.
- The three tools with their input and output fields; `vehicle_id = None` means all vehicles.
- The journey rules with two worked examples (invented addresses only): a complete journey, and a broken chain that gives an incomplete journey.
- Design Decisions (link the two new entries), Key Files, Related ([place-book.md](../../../docs/features/place-book.md), [server-mode.md](../../../docs/features/server-mode.md)).
Use ASD-STE100 and clickable links.

- [ ] **Step 4: Check every feature doc**

Run: `ls docs/features/*.md` and read each one. For each doc, decide "updated" or "checked, no change". The expected updates:
- [place-book.md](../../../docs/features/place-book.md): the home icon, `places.is_home`, the one-home index, `set_home_place`.
- [server-mode.md](../../../docs/features/server-mode.md): the `/mcp` route in the router section and in Docker deployment (proxy path), no auth.
- [route-maps.md](../../../docs/features/route-maps.md): one line that `trip_routes.round_trip` also makes a day trip for the journey grouping.
- [read-only-mode.md](../../../docs/features/read-only-mode.md): `set_home_place` is blocked in read-only mode; `/mcp` works in read-only mode.
- All other docs: check for a list of routes, a list of RPC commands, or a list of migrations that now misses an entry.

- [ ] **Step 5: Update the READMEs and the project guides**

- [README.md](../../../README.md) section `## Funkcie` and [README.en.md](../../../README.en.md) section "Features": one bullet each for the home mark and the read-only MCP endpoint, with a link to `docs/features/mcp-endpoint.md`. Keep the two files in sync.
- [CLAUDE.md](../../../CLAUDE.md), section "Architecture: Backend-Only Calculations": add `journeys` and `mcp` boxes to the `kniha-jazd-core` diagram, and add `/mcp` to the HTTP line (for example `HTTP  -  POST /api/rpc { command, args }  |  /mcp (read-only MCP)`).
- [rust-backend.md](../../../.claude/rules/rust-backend.md), table "Key Files Reference": add rows for `journeys/mod.rs` (journey grouping, pure) and `mcp/mod.rs` (read-only MCP tools; never hold a `Database`).

- [ ] **Step 6: Verify and commit**

Check only the added lines of all touched files (the old text of `DECISIONS.md` and `CHANGELOG.md` already contains em-dashes):
```bash
git diff -U0 | grep '^+' | grep -P "[\x{2014}\x{2013}\x{2192}\x{201C}\x{201D}\x{2026}]" || echo clean
```
Expected: `clean`.

Commit. Put the per-doc result of Step 4 in the commit message body:
```bash
git add DECISIONS.md CHANGELOG.md README.md README.en.md CLAUDE.md .claude/rules/rust-backend.md docs/features/
git commit -m "docs: home place, journeys and the read-only MCP endpoint" -m "Feature docs checked:
- place-book.md: updated
- server-mode.md: updated
- <one line per doc in docs/features/: updated | checked, no change>"
```

---

### Task B6: Full verification

- [ ] **Step 1: Run `/verify`**

It runs the backend tests, the integration suite, checks `git status` and the changelog. Expected: all green. Report a failure with its output; do not call the task done.

- [ ] **Step 2: Update the task status**

Set `**Status:** Complete` in [01-task.md](./01-task.md) and this file. Add or update the row of task 89 in [_tasks/index.md](../../index.md). Commit:
```bash
git add _tasks/89-home-place-mcp _tasks/index.md
git commit -m "docs(tasks): task 89 complete"
```

- [ ] **Step 3: Push and check the publish job**

Push to `main` only when the user says so. Then check the publish job:
```bash
RUN=$(gh run list --workflow test.yml --branch main --limit 1 --json databaseId --jq '.[0].databaseId')
gh run view "$RUN" --json jobs --jq '.jobs[] | select(.name|test("Publish")) | "\(.name) \(.conclusion)"'
```
Expected: `Publish main Docker Image success`. This repo does not deploy. The homelab side redeploys the stack.

