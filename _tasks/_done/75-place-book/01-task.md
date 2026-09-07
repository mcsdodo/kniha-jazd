**Date:** 2026-09-06
**Subject:** Place book — give every place in the logbook a coordinate
**Status:** Planning

## Goal

Trips record `origin` and `destination` as free text. Give each distinct place a
coordinate, confirmed once by a human, and let two things read it:

- **Trip entry** — the autocomplete is fed by the book instead of per-vehicle route
  pairs, so it is global and cannot offer two spellings of one place.
- **Route maps** ([task 72](../72-route-map-origin-destination/)) — a row can only be
  routed A→B if both endpoints resolve to points. The book is where they come from.

Nothing about trips changes. The book sits beside them and points at them.

## Requirements

- A **Miesta** section in [Settings](../../src/routes/settings/+page.svelte), listing
  every place used by a trip and whether it has been placed yet.
- An edit dialog with a map, a search box and a draggable pin. Searching offers
  candidates; a pin can always be dropped by hand.
- **Nothing is placed without a human confirming it.** No confidence heuristic decides
  on the user's behalf.
- Geocoding sits behind an injected trait, so no test touches the network.
- The list of places is **derived from trips**, not a stored list kept in step by write
  paths.
- Coordinates are stored per normalised name; the displayed name is the spelling trips
  already use.
- Write commands guard with `check_read_only!`; all user-facing strings go through i18n.

Design, with the reasoning behind each: [02-design.md](./02-design.md).

## Context: the cleanup that came first

The place strings were cleaned up before this task starts, as a one-off data fix rather
than a feature — three spellings of one petrol station, abbreviations no geocoder
resolves, two typos. Applied to production on **2026-09-06**: 17 rewrites across 62 trip
endpoints, 52 → 47 distinct places, `routes` 96 → 91.

Consequently the book needs **no merge, rename or alias machinery**. The 47 remaining
strings normalise to 47 distinct keys with zero collisions.

The cleanup's scripts, its editable fix list and its writeup are **not in this
repository**. They name real home and business addresses alongside trip ids and dates,
and this repository is public. They live at [_tmp/75-place-cleanup/](../../_tmp/75-place-cleanup/) on the maintainer's
machine, which is gitignored. That part of the work cannot be reproduced from the repo
alone, and is not meant to be — it ran once.

## Technical notes

- **47 places, closed set.** Every design choice leans on that number; it is what makes
  confirming each one by hand cheaper than a rule for trusting a machine's guess.
- **They are street addresses, not town names**, which is why name-matching heuristics
  do not apply.
- **The geocoder must not be restricted to Slovakia.** Five of the 47 places are Czech
  or Hungarian, and one alone accounts for 39 trips.
- **Derived, not maintained**, on the evidence of the `routes` table — see
  [task 76](../76-route-usage-counter-drift/), whose stored counter is wrong in 52 of 96
  rows because three write paths were meant to keep it current and none did.
- The integration test needs a **provider-override environment variable** so the
  geocoder can be stubbed. Task 72 deferred building it; this task builds it, and it
  unblocks the three tests task 72 deferred for the same reason.

## Related

- [Task 72](../72-route-map-origin-destination/) — its Phase 1 is superseded by this
  task; it keeps the routing phases.
- [Task 76](../76-route-usage-counter-drift/) — shares the stored-versus-derived
  decision.
