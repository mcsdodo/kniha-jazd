**Date:** 2026-10-05
**Subject:** Places as entities: a Miesta tab, trips and routes reference places by ID, only existing places on a trip
**Status:** Complete

## Goal

A place is a real record with an ID, a name and a position. Trips and routes point to a
place by ID. They do not hold a free-text copy of the name.

Today the place list is computed from the free text in `trips.origin` and
`trips.destination`. The coordinates live in a side table that is keyed by a normalised
string ([ADR-033](../../../DECISIONS.md#adr-033-aggregates-over-trips-are-computed-not-stored)).
Every feature that needs "the same place" must fold strings with `places::normalise`. A
second fold, `normalize_location`, keeps case and diacritics, so the two disagree.
[Task 89](../89-home-place-mcp/01-task.md) needs a home place, and a home flag does not fit
the current table. This task removes the string model.

Out of scope, follow-up task: create a new place from inside the trip form.

## Current state

| Item | Today |
|------|-------|
| Place strings | Free text on each trip: `trips.origin`, `trips.destination` ([models.rs:191](../../../src-tauri/core/src/models.rs#L191)). Trimmed by `normalize_location` ([db.rs:43](../../../src-tauri/core/src/db.rs#L43)). |
| Place list | Derived on each read: `distinct_trip_places` ([db.rs:1207](../../../src-tauri/core/src/db.rs#L1207)) plus a fold in Rust in `list_places_internal` ([places_cmd.rs:29](../../../src-tauri/core/src/commands_internal/places_cmd.rs#L29)). The most-used spelling is the display name, and a tie goes to the byte-wise smaller spelling. |
| Coordinates | Table `places`, key `normalised_name`, columns `display_name`, `lat`, `lon`, `source` ([schema.rs:134](../../../src-tauri/core/src/schema.rs#L134)). A row exists only for a place with coordinates. `clear_place` and `upsert_place` delete the row. |
| Routes | `routes` has `UNIQUE(vehicle_id, origin, destination)` on raw strings. `find_or_create_route` ([db.rs:798](../../../src-tauri/core/src/db.rs#L798)), `get_routes_for_vehicle` ([db.rs:720](../../../src-tauri/core/src/db.rs#L720)) joins trips on the strings. |
| Time inference | `find_most_recent_trip_times_for_route` ([db.rs:763](../../../src-tauri/core/src/db.rs#L763)) matches the strings exactly. |
| Route maps | `mode_for` and `placed_endpoint` ([route_maps.rs:985](../../../src-tauri/core/src/commands_internal/route_maps.rs#L985)) fold the strings to find coordinates. |
| Trip form | `Autocomplete` with free text, suggestions from `listPlaces()` ([TripRow.svelte:652](../../../src/lib/components/TripRow.svelte#L652)). Distance auto-fill matches `r.origin === formData.origin` ([TripRow.svelte:314](../../../src/lib/components/TripRow.svelte#L314)). |
| Place UI | Section "Miesta" inside Settings ([settings/+page.svelte:1379](../../../src/routes/settings/+page.svelte#L1379)). |
| Migrations | Run on three paths: `Database::new`, `in_memory`, `restore_from_file` ([db.rs:89](../../../src-tauri/core/src/db.rs#L89), [:126](../../../src-tauri/core/src/db.rs#L126), [:167](../../../src-tauri/core/src/db.rs#L167)). Foreign keys are **on** for every connection, also during migrations: the bundled `libsqlite3-sys` 0.30.1 builds SQLite with `-DSQLITE_DEFAULT_FOREIGN_KEYS=1` (its `build.rs:123`). So `DROP TABLE trips` cascades into `trip_routes` and `paperless_trip_links`. |

**Real data** (read-only copy of the live DB, 2026-10-05): 339 trips, 2 vehicles, 95 routes.
The trips use 48 distinct normalised place keys, and all 48 have coordinates. One more
`places` row has coordinates and no trip. No trip has an empty origin or destination.

## Requirements

### 1. Schema

- `places`:

  | Column | Type | Note |
  |--------|------|------|
  | `id` | TEXT PK | UUID |
  | `name` | TEXT NOT NULL | Display name, as the user wrote it |
  | `normalised_name` | TEXT NOT NULL UNIQUE | `places::normalise(name)` |
  | `lat`, `lon` | REAL NULL | NULL only for a migrated legacy place (see 2) |
  | `source` | TEXT NULL | `geocoder` or `manual`; NULL when there are no coordinates |
  | `created_at` | TEXT NOT NULL | |

- `trips`: drop `origin` and `destination`. Add `origin_place_id` and
  `destination_place_id`, both TEXT NOT NULL, REFERENCES `places(id)`.
- `routes`: drop `origin` and `destination`. Add `origin_place_id` and
  `destination_place_id`. Unique on `(vehicle_id, origin_place_id, destination_place_id)`.
- Foreign keys are already on (see Current state). The new `REFERENCES places(id)`
  clauses are enforced with no extra pragma.

### 2. Migration

- Register `places::normalise` as a SQLite scalar function (for example `kj_normalise`) on
  each connection **before** the migrations run. The migration is then plain SQL, and
  Diesel runs it in one transaction. A crash leaves the old schema in place.
- `diesel migration run` from the CLI cannot run this migration: it fails with
  "no such function: kj_normalise". Only the app runs it.
- One helper opens a connection and registers the function. Then the migrations run. `Database::new`, `in_memory` and `restore_from_file` all use it. So an old
  backup also migrates on restore.
- Place rows:
  - One place per distinct normalised key in `trips.origin` and `trips.destination`.
  - The name is the spelling that Miesta shows today: the most-used spelling, then the
    byte-wise smaller spelling.
  - Coordinates and `source` come from the old `places` row with the same key.
  - A key without an old row becomes a place with NULL coordinates. The UI flags it.
  - An old `places` row that no trip uses stays as a place.
- A blank or whitespace-only trip endpoint maps to one place named `Neznáme miesto`, so
  its trip keeps a place to point to. The live data has none.
- Trips and routes get the IDs through `kj_normalise(old string)`. The trip copy uses
  LEFT JOINs: if a trip key has no place, the NOT NULL column aborts the migration. A
  trip is never dropped without notice.
- **Orphan routes are dropped.** A `routes` row whose place pair no trip uses is not
  copied. Today `get_routes_for_vehicle` already hides such a row, so nothing visible
  changes. The live copy has 9 route strings like this, with no coordinates.
- If two `routes` rows collapse onto one place pair, keep the row whose old strings the
  most recent trip on that pair used. The first row by `id` breaks a tie.
- **Child rows survive the trips rebuild.** Copy `trip_routes` and `paperless_trip_links`
  into TEMP tables before `DROP TABLE trips`, and insert them back after the rename.
  Do not turn foreign keys off: Diesel writes the version row outside the SQL file.
- `down.sql` rebuilds `origin` and `destination` from the place names, with the same
  TEMP copy of the child rows. It cannot give back the other spellings of a place.
- The existing pre-migration backup at startup stays ([db.rs:66-86](../../../src-tauri/core/src/db.rs#L66)).

### 3. Backend API

- `Trip` (read): keep `origin` and `destination` as display strings, filled by a join.
  Add `originPlaceId` and `destinationPlaceId`. Display, export, statistics and most of
  the frontend keep working.
- `create_trip`, `update_trip`, `create_trip_cascade`, `update_trip_cascade` take
  `originPlaceId` and `destinationPlaceId`. An unknown ID is an error.
- Switch these from strings to IDs: `find_or_create_route`, `get_routes_for_vehicle`,
  `find_most_recent_trip_times_for_route` and its RPC
  `get_inferred_trip_time_for_route`, `mode_for`, `placed_endpoint`.
- Remove `normalize_location` and `distinct_trip_places`.
- Place commands:
  - `list_places` -> places with `id`, `name`, `lat`, `lon`, `source`, `uses` (count of
    trip endpoints, computed).
  - `create_place(name, lat, lon, source)` -> the new place. Name and position are
    required. A name whose normalised key exists is an error.
  - `rename_place(id, name)`. A collision with another place's key is an error. There is
    no merge.
  - `set_place_position(id, lat, lon, source)` (replaces `save_place`).
  - `delete_place(id)`. Allowed only if no trip uses the place. Orphan routes on the
    place are deleted with it: a route with no trip is not visible anywhere.
  - `find_place(name)` -> the place whose normalised key equals `normalise(name)`, or
    null. The trip form calls it for typed text, so the fold stays in Rust (ADR-008).
  - `clear_place` goes away: a place keeps its position.
  - All writes guard with `check_read_only!`.

### 4. UI

- New top-level tab **Miesta** (`/miesta`), next to the existing tabs. The place list, the
  filter and the position editor move there from Settings. Settings no longer shows places.
- On the tab:
  - Add a place: name and position (geocoder search or a manual pin) are both required.
  - Rename a place.
  - Delete a place. The button is disabled with a tooltip if the place is in use.
  - A legacy place without a position shows a "needs a position" marker.
- Trip form: the origin and destination autocomplete accept only an existing place. If
  the text matches no place, the save is blocked and a message tells the user to add the
  place on Miesta first.
- The distance auto-fill and the time inference in `TripRow.svelte` match on place IDs.
- All strings go through i18n, Slovak first.

### 5. Behaviour rules

- **A rename applies to all trips**, including the printed logbook of past years. The
  logbook shows the current place name.
- A rename to a name whose normalised key belongs to another place is an error.
- A place can be deleted only if nothing uses it.

## Tests

### Backend unit tests

- Migration, on a legacy DB built with `open_db_legacy_before`:
  - two spellings of one place fold into one place, with the most-used name;
  - a string without coordinates becomes a place with NULL coordinates;
  - an unused old `places` row stays;
  - two routes that collapse onto one pair become one row;
  - a route that no trip uses is dropped;
  - a blank endpoint maps to `Neznáme miesto`, and the trip survives;
  - route maps and Paperless links survive the trips rebuild;
  - a restore of an old backup file migrates it (trips get place IDs);
  - `down.sql` gives back the strings.
- Trip create and update with an unknown place ID fail.
- `rename_place` collision fails, and a rename that only changes case or diacritics
  succeeds. `delete_place` on a used place fails, and on a place that only an orphan
  route uses it succeeds. `find_place` folds case and diacritics.
- `list_places` returns a place without coordinates with `lat = null`. This is the only
  test of the "needs a position" marker data: no RPC can create an unplaced place, and a
  test-only RPC must not exist on a live server. The marker has no integration test.
- `find_or_create_route` and time inference work on IDs.
- Test helpers: a test-only `Database::ensure_place_for_test(name) -> Uuid` gives the ID
  of a place by name and creates it (with coordinates) if it is missing. The about 60
  Rust test call sites that insert a trip set `origin_place_id` and
  `destination_place_id` from it. Foreign keys are on, so a trip with a nil place ID
  fails.

### Integration tests

- Helpers: `seedTrip` and `fillTripForm` create the place by name if it is missing
  (`find_place`, then `create_place`). The DB reset in `wdio.server.conf.ts` also deletes
  the places. The about 217 `seedTrip` call sites stay. The place specs that use
  `places-section` in Settings move to the Miesta tab.
- New flows:
  - Add a place on Miesta, then pick it in the trip form.
  - Text that matches no place blocks the trip save.
  - A rename on Miesta shows in the trip grid.

## Documentation

Check **every** feature doc in [docs/features/](../../../docs/features/). Update each doc that
describes places, trip endpoints, routes or the Settings layout. In the PR, list each doc
as "updated" or "checked, no change".

| Doc | Action |
|-----|--------|
| [backup-system.md](../../../docs/features/backup-system.md) | restore runs the new migration with the registered SQLite function |
| [export-system.md](../../../docs/features/export-system.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [home-assistant.md](../../../docs/features/home-assistant.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [magic-fill.md](../../../docs/features/magic-fill.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [multi-invoice.md](../../../docs/features/multi-invoice.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [multi-year-state.md](../../../docs/features/multi-year-state.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [paperless-integration.md](../../../docs/features/paperless-integration.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [place-book.md](../../../docs/features/place-book.md) | rewrite for the new model (entity, Miesta tab, migration, rename and delete rules) |
| [read-only-mode.md](../../../docs/features/read-only-mode.md) | an older image opens the migrated DB read-only |
| [route-maps.md](../../../docs/features/route-maps.md) | endpoints by place ID, not by a folded string |
| [server-mode.md](../../../docs/features/server-mode.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [settings-architecture.md](../../../docs/features/settings-architecture.md) | Miesta is no longer in Settings |
| [trip-entry-defaults.md](../../../docs/features/trip-entry-defaults.md) | autocomplete accepts only existing places; distance auto-fill and time inference on IDs |
| [trip-grid-calculation.md](../../../docs/features/trip-grid-calculation.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [trip-odometer-cascade.md](../../../docs/features/trip-odometer-cascade.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |
| [unified-invoice-picker.md](../../../docs/features/unified-invoice-picker.md) | Check for place strings, `origin`/`destination` as text, or the Settings place section. Fix what is out of date. |

Also update [README.md](../../../README.md) and [README.en.md](../../../README.en.md) for the
Miesta tab.

## Decisions to record (`/decision`)

- **ADR:** places are entities, trips and routes reference them by ID. Supersedes
  [ADR-033](../../../DECISIONS.md#adr-033-aggregates-over-trips-are-computed-not-stored) for
  places (the route counters stay derived) and ADR-034 (the display spelling is now the
  stored name). Check ADR-032 for the place-book endpoints on the map.
- **ADR:** the migration uses a SQLite function registered from Rust; the TEMP copy of
  the child rows (foreign keys are on); orphan routes are dropped; the rule for routes
  that collapse onto one pair; `Neznáme miesto` for a blank endpoint.
- **BIZ:** a rename applies to all trips; delete only if no trip uses the place, and
  orphan routes go with it; a trip accepts only an existing place.

## Upgrade notes (`### Pokyny k aktualizácii`)

Use the five fields of the template in [CHANGELOG.md](../../../CHANGELOG.md):

- **Potrebný zásah:** take a backup before the redeploy (the app also writes a
  pre-migration backup).
- **Premenné prostredia:** no change.
- **Migrácie databázy:** `2026-10-05-100000_places_as_entities`. It is one-way: an older
  image opens the migrated DB read-only.
- **Strata údajov:**
  - The other spellings of a place are lost. Each trip shows the place name (the
    most-used spelling). `down.sql` cannot give the spellings back.
  - Saved routes that no trip uses are dropped.
  - A blank trip endpoint becomes the place `Neznáme miesto`.
  - A trip place without coordinates becomes a place marked "needs a position".
- **Image, volume, port:** no change.

## Done when

- [ ] The schema and the migration are in, with the migration tests.
- [ ] Trips and routes use place IDs on every read and write path.
- [ ] The Miesta tab exists, and Settings has no place section.
- [ ] The trip form accepts only existing places.
- [ ] The backend and integration tests pass.
- [ ] Feature docs, READMEs, `DECISIONS.md` and `CHANGELOG.md` (with upgrade notes) are updated.

This repo is public. Do not add a homelab address, host, IP or real trip data.
