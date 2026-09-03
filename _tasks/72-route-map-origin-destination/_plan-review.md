**Date:** 2026-09-03
**Subject:** Plan review — Route maps V2 (origin/destination routing, alternatives, manual editing)
**Reviewed:** [03-plan.md](./03-plan.md) against [01-task.md](./01-task.md), [02-design.md](./02-design.md) and the codebase as built
**Status:** Needs revisions

## Verdict

**3 Critical · 6 Important · 7 Minor — Needs Revisions.**

The architecture is sound and the backend is the strongest part of the plan: the
provider-trait hedge, the alias cache, the "looking is not committing" split and the
Rust-side waypoint insertion all hold up, and I traced the `insert_waypoint` arithmetic
against all five of its tests by hand — it produces the asserted slots. The three
Critical findings are not about the design; they are places where the plan does not
actually deliver what the design specifies (mode selection), leans on repository
machinery that does not exist (the migration harness), or contradicts a constraint the
target test file states in its own header (integration tests and the network).

---

## Critical

### [ ] C1 — `mode_for()` is built, tested, and never called; mode selection is duplicated in TypeScript

Task 5 (plan L1011–1112) implements `mode_for` in Rust with four unit tests. **No later
task ever calls it.** It appears in no dispatcher arm, no `_internal` function, and not
in `route_direct_internal`. Instead Task 13 adds `normaliseForCompare()` to
[src/routes/mapa/+page.svelte](../../src/routes/mapa/+page.svelte) (plan L2337, L2353) — a second implementation of
`normalise()`, written as `NFD` + combining-mark strip.

The two implementations will disagree. Rust's `fold_diacritic` is a closed hand-written
table (Slovak/Czech/Polish); JS `NFD` strips **every** combining mark. Hungarian border
towns a Slovak logbook plausibly names — `Győr`, `Kőszeg` — fold to `gyor`/`koszeg` in
the browser but keep `ő` in Rust, because `ő` is not in the table. That is precisely the
failure the design's own comment forbids:

> "if these two ever disagreed, a row could be routed as A→B while its endpoints
> collided onto one cache entry" — 02-design.md, Mode selection

02-design.md promises "One pure function, one call site, one rule." The plan delivers
zero Rust call sites and one TypeScript reimplementation — a direct ADR-008 violation of
the kind [CLAUDE.md](../../CLAUDE.md) lists first under Common Pitfalls.

**Fix:** give the map view a command to ask. The cheapest shape that also fixes I6 is a
single `start_route_for_trip(tripId)` returning `{ mode, origin: PlaceResolution,
destination: PlaceResolution }` — one round trip on page open (the page already awaits
`get_trip_route` there), mode decided by `mode_for`, both endpoints resolved in Rust.
Then delete `normaliseForCompare` entirely.

### [ ] C2 — Task 16's integration tests hit the live network, contradicting the target file's stated constraint

[tests/integration/specs/tier2/route-map.spec.ts](../../tests/integration/specs/tier2/route-map.spec.ts) opens with an explicit rule:

> "1. `generate_route` is never called. It hits the public OSRM demo server —
> network-dependent, rate-limited and non-deterministic. Routes are seeded with
> `save_trip_route` and a canned polyline instead."

Proposed tests 1, 2 and 3 all open `/mapa?trip={id}` on an A→B row, which fires
`resolve_place` (live Nominatim) and then `route_direct` (live OSRM) during page load.
Test 5 ("a same-place row still loops") fires `generate_route` → live OSRM. Only test 4
(missing endpoint) is offline-safe, because `startForTrip` errors before any request.
This also breaks the plan's own ground rule, "No test touches the network."

There is no injection point: Task 10 constructs `HttpGeocodeProvider::public()` and
`HttpRouteProvider::public()` inside the dispatcher arms, so an integration test cannot
stub them.

**Fix:** decide and write it down. Either (a) add a test-mode provider override (an env
var read once into `ServerState`, pointing both providers at a local stub), or (b) scope
Task 16 to what is reachable offline — pre-seed a `place_alias` through `remember_place`
so resolution is a cache hit, and seed a saved direct route through `save_trip_route` so
the page renders without routing. Under (b), test 3 (promote an alternative) is not
reachable at all and should move to the deferred list.

### [ ] C3 — Task 9's migration test calls helpers that do not exist, against a harness that cannot express the cutoff

The plan uses `legacy_db_before("2026-09-03-110000_add_trip_route_mode")` and
`run_remaining_migrations(&db)`, with the note *"it already has a way to open a DB at a
given migration and run the rest. Reuse it; do not add a second mechanism."*

It does not. What exists is:

- [db.rs:1189](../../src-tauri/core/src/db.rs) — `const MULTI_INVOICE_VERSION: &str = "2026-07-15"`, hardcoded
- [db.rs:1208](../../src-tauri/core/src/db.rs) — `open_db_legacy()`, **no parameter**
- [db.rs:1243](../../src-tauri/core/src/db.rs) — `migrate_to_current(&db)`

Worse, `open_db_legacy()`'s cutoff (2026-07-15) predates
[2026-08-10-100000_add_trip_routes](../../src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/), so the legacy DB has **no `trip_routes` table** and
the plan's `INSERT INTO trip_routes ...` fails outright.

Parameterising the cutoff is a genuine refactor — the const is consumed by ~10 existing
tests plus the schema-parity test at [migration_tests.rs:622](../../src-tauri/core/src/migration_tests.rs). Budget it as its own
step, or drop the migration test and prove the backfill with a [db_tests.rs](../../src-tauri/core/src/db_tests.rs) assertion
that a `trip_routes` row inserted without `mode` reads back as `RouteMode::Loop`.

---

## Important

### [ ] I1 — The Nominatim rate limiter never fires: the provider is rebuilt per request

`HttpGeocodeProvider` carries `last_request: Mutex<Option<Instant>>` and a `throttle()`
enforcing `MIN_REQUEST_INTERVAL`. But Task 10's dispatcher does
`let provider = HttpGeocodeProvider::public();` **inside** the `"resolve_place"` arm — a
fresh instance per RPC, so `last_request` is always `None` and `throttle()` is a no-op.

01-task.md calls the usage policy "binding". As written the plan ships the appearance of
compliance without the substance. Hold one provider in `ServerState` or a process-wide
`OnceLock`. Note that a test asserting two back-to-back searches are ≥1 s apart costs a
real second of suite time — `tokio::time::pause()` is the usual dodge but needs tokio's
`test-util` feature, which [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml) does not enable.

### [ ] I2 — Task 14's "alternatives unavailable" branch is unreachable

```svelte
{#if mode === 'direct' && alternatives.length > 0}   ← wins whenever a route exists
{:else if mode === 'direct' && hasWaypoints}          ← never reached
```

Once a via exists OSRM returns exactly one route, so `alternatives.length === 1 > 0` and
the first branch renders a one-item "Alternatívne trasy" list. The design requires the
opposite:

> "the panel **says so** rather than going quietly empty, because an empty list
> otherwise reads as 'the service failed'." — 02-design.md

Invert it: `{#if mode === 'direct' && !hasWaypoints && alternatives.length > 0}` /
`{:else if mode === 'direct' && hasWaypoints}`. Also rename `hasWaypoints` — its test is
`> 2`, so it means "has vias", and every direct route has waypoints.

### [ ] I3 — A failed `remember_place` traps the user in a picker loop; guaranteed in read-only mode

`handlePlacePicked` swallows a `rememberPlace` failure ("Remembering is a convenience"),
sets `resolvedOrigin = place`, then calls `resolveEndpoints(trip)` — which **re-reads**
the endpoint via `resolvePlace`, discarding that assignment. On a cache miss it sets
`pendingPlace` again.

`remember_place_internal` opens with `check_read_only!`, so in read-only mode every pick
is refused and the picker reappears forever with only a `console.error`. Either skip
already-resolved endpoints in `resolveEndpoints`, or surface the failure and continue
with the in-memory pick.

### [ ] I4 — Task 9 leaves the tree uncompilable, so its "whole backend suite passes" checkpoint is wrong

Task 9 changes `save_trip_route_internal` to take a trailing `mode: RouteMode`. Its only
call site outside tests is [dispatcher.rs:852](../../src-tauri/core/src/server/dispatcher.rs), which Task 10
updates. Between the two, `cargo test` cannot compile — yet Task 9 Step 4 says
*"Expected: PASS, whole backend suite."*

Separately, [dispatcher.rs:1095](../../src-tauri/core/src/server/dispatcher.rs)'s existing
`route_map_commands_round_trip_with_frontend_argument_names` dispatches
`save_trip_route` with the current five fields; once `mode` becomes a required `Args`
field serde rejects that payload. The plan never names that test as needing an update.

**Fix:** merge Tasks 9 and 10, or move the dispatcher call-site edit into Task 9 and
name the existing test's payload as part of it.

### [ ] I5 — **Prepočítať** always fails on a re-opened saved direct route

Task 13's `loadTripAndRoute` returns early when `savedRoute` exists, setting only `mode`
— `resolvedOrigin` and `resolvedDestination` stay `null`. `handleRegenerate` /
`handleRetry` then call
`runDirect(generated?.waypoints ?? waypointsFromEndpoints(), ...)`, where `generated` is
`null` and `waypointsFromEndpoints()` returns `[]` because both resolved states are
null. `route_direct_internal` errors with *"A route needs a start and an end, got 0
point(s)."*

Task 15's `currentWaypoints()` gets this right
(`generated?.waypoints ?? savedRoute?.waypoints ?? []`). Task 13 should use the same
fallback rather than a second, shorter one.

### [ ] I6 — `resolveEndpoints` costs up to six geocode round trips for one row

Origin and destination resolve sequentially, and the whole function restarts after every
pick — so a row where both endpoints miss the cache does 2 + 2 + 2 = 6 `resolve_place`
calls, each building a fresh HTTP client. With a working rate limiter (I1) that is
several seconds of blank map on first open. Folding both endpoints into the single
`start_route_for_trip` command proposed in C1 removes this entirely.

---

## Minor

### [ ] M1 — `AppState::set_read_only(true)` does not exist

The real API is `set_read_only_reason(Option<String>)` at
[app_state.rs:150](../../src-tauri/core/src/app_state.rs). The plan already carries a note telling the
implementer to check; just name the correct call and drop the note.

### [ ] M2 — `RouteMapRow` binds positionally; `mode` must be appended in both places

`RouteMapRow` is `Queryable` and read via `first::<RouteMapRow>` ([db.rs:1127](../../src-tauri/core/src/db.rs)) and
`load::<RouteMapRow>` ([db.rs:1152](../../src-tauri/core/src/db.rs)). Task 9 adds `mode` to the `trip_routes` `table!`
block and to `RouteMapRow`. Both `mode` and `created_at` are `Text`, so appending them
in **different** positions swaps the values silently with no compile error. Worth one
explicit line: append `mode` last in both.

### [ ] M3 — Swap Tasks 11 and 12 so the `npm run check` checkpoint means something

Task 11 Step 2 says "Expected: no errors ... (i18n key errors are expected until Task 12
— ignore those for now)." A checkpoint whose expected output includes errors you are
told to ignore is not a checkpoint. i18n has no dependency on the types, so run it
first.

### [ ] M4 — The ghost handle is never removed on `mouseout`

`attachGhost` creates a ghost on `mousemove` over the polyline and removes it only on
`dragend`. Move the cursor off the line and a stray draggable dot stays on the map;
because creation is guarded by `if (!ghost)`, entering a different layer reuses the
stale one. Add a `mouseout` that clears it when not dragging.

### [ ] M5 — Three i18n keys are added but never used

`duration`, `removeWaypoint` and `placeRemembered` appear in Task 12 but no markup in
Tasks 13–15 references them (`formatDuration` renders the value bare, the remove gesture
is an unlabelled click, and there is no place-saved toast). Use them or drop them —
typesafe-i18n will not flag dead keys.

### [ ] M6 — `insert_waypoint` runs before the two-waypoint guard

In `route_direct_internal`, `insert` is applied first, so a one-waypoint list plus an
insert silently becomes a routable two-point route rather than hitting the "A route
needs a start and an end" error. Harmless with today's callers, but the guard reads as
though it prevents this.

### [ ] M7 — [.claude/rules/migrations.md](../../.claude/rules/migrations.md) will not auto-load for these migrations

Its front matter declares `paths: src-tauri/migrations/**/*.sql`, but migrations live in
`src-tauri/core/migrations/` (moved by task 58, the Tauri workspace split). Worth fixing
the glob while working in the area.

---

## What was verified and is correct

Recorded so the next pass does not re-check it:

- `insert_waypoint`'s slot arithmetic produces the asserted result for all five Task 7
  tests, including the `direct_waypoints()` / straight-line-geometry mismatch in Task 8's
  `an_insert_point_is_applied_before_routing`.
- `fold_diacritic` covers all 15 accented Slovak letters after `to_lowercase()`.
- Route-map commands really are server-only — there are no Tauri wrappers in
  [src-tauri/desktop/](../../src-tauri/desktop/), so Task 10's dispatcher-only wiring is right, and the plan's
  "Deferred" note about seven commands is accurate.
- `Dataset::bundled().version`, `Database::in_memory()`, `ServerState`'s four fields,
  `db.get_route_map(&str)`, `polyline::{encode, decode}` and the TS `Waypoint` shape
  (`name?`, `nodeIdx?` both optional) all match what the plan assumes.
- `tokio::time::sleep` is already used in [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs), so the `time` feature is
  available through unification despite not being listed in `core/Cargo.toml`.
- Migration directory naming and the `2026-09-03-100000` / `-110000` ordering match the
  existing convention.
