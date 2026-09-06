**Date:** 2026-09-06
**Subject:** Place book — a curated list of places with coordinates
**Status:** Planning

Requirements grew out of a brainstorm on 2026-09-05 and the place-name cleanup that
preceded it (the cleanup writeup is local only — see [01-task.md](./01-task.md)). This design
supersedes Phase 1 of
[task 72](../72-route-map-origin-destination/02-design.md), which is reduced to its
routing phases.

## What this is

Trips record `origin` and `destination` as free text. The book gives each distinct
place a coordinate, once, confirmed by a human. Two things read it:

- **Trip entry** — the autocomplete stops being fed by per-vehicle route pairs and is
  fed by the book instead.
- **Route maps** ([task 72](../72-route-map-origin-destination/)) — a row can only be
  routed A→B if both endpoints have coordinates. The book is where they come from.

Nothing about trips changes. The book sits beside them and points at them.

## What the real data changed

The brainstorm designed against an imagined dataset. Profiling the production database
contradicted it three times, and each contradiction removed work:

**The list is closed and small — 47 places.** Not hundreds, and it grows by roughly a
handful a year. Every design choice below leans on that number.

**They are street addresses, not town names.** So the auto-accept rule sketched during
the brainstorm — trust the geocoder when its returned name equals the query — cannot
work: a geocoder's name for an address never equals the typed string. Rather than
invent a replacement heuristic, the book has **no auto-accept at all** (below).

**There are no spelling variants left.** All 47 strings normalise to 47 distinct keys,
zero collisions. The cleanup that ran on 2026-09-06 removed the duplicates as a one-off
data fix, so the book needs no merge, no rename, and no alias indirection.

## Every place is confirmed by a human

There is no conclusiveness heuristic, no confidence threshold, no `checked_at`, and no
review queue as a distinct concept. You open the section once, walk 47 rows, and for
each one accept the geocoder's suggestion or drop a pin.

This is the central simplification. The alternative — a rule deciding when a machine's
guess about an address is trustworthy — is the hardest thing in the feature, is
untestable without either a network dependency or an invented threshold, and buys
back roughly fifteen minutes of clicking, once, on a list that is already complete.

A wrong pin that a heuristic accepted silently would surface later as a map of the
wrong place on a document that is legal evidence. That is the failure this feature
must not have, and the cheapest way not to have it is to never guess.

## Data model

One new table. One Diesel migration, per
[migration conventions](../../.claude/rules/migrations.md).

### `places`

| column | type | notes |
|---|---|---|
| `normalised_name` | TEXT PK | lookup key: lowercased, diacritics folded, whitespace collapsed |
| `display_name` | TEXT NOT NULL | **the spelling trips use**, verbatim |
| `lat` / `lon` | REAL **NULL** | null = not yet placed |
| `source` | TEXT NOT NULL | `geocoder` (a suggestion accepted) or `manual` (a pin dropped) |
| `confirmed_at` | TEXT NULL | when a human last placed it |

**`display_name` is the trip's own spelling, not the geocoder's.** This is easy to get
wrong and expensive if you do. The autocomplete offers `display_name`; if that were the
geocoder's official rendering, picking it would write a *new* string into the trip —
one differing from the 200 rows already using the plain form — and the fragmentation
the cleanup just removed would grow straight back. The geocoder's name is shown while
choosing and then discarded.

**No `unplaceable` state.** A pin can always be dropped by hand, so nothing is truly
unplaceable; a row with null coordinates simply has not been done yet.

**Not vehicle-scoped.** A place is in the same spot whichever car drove there.

## The list is derived, never maintained

The section's list is a join: the distinct normalised place strings in `trips`, left
joined onto `places`. It is not a stored list that write paths keep in step.

This is a direct lesson from the `routes` table, whose stored `usage_count` is wrong in
52 of its 96 rows because three write paths were supposed to maintain it and none of
them fully did — see [task 76](../76-route-usage-counter-drift/). A derived list cannot
drift: rename a place in a trip and it appears; delete the last trip using it and it
disappears. The volume is tens of rows, so the query costs nothing worth measuring.

The consequence to accept: a `places` row whose trips have all been deleted becomes an
orphan, invisible in the list but still on disk. Harmless — it is a coordinate nobody
asks for — and it means a place is not re-confirmed if that string comes back.

## Geocoding

`GeocodeProvider` as an injected trait, so no test touches the network — mirroring
`RouteProvider` and `TileFetcher` as task 72 established.

```rust
pub struct Candidate {
    pub lat: f64,
    pub lon: f64,
    /// What the geocoder calls it. Shown while choosing, never stored.
    pub label: String,
}

#[async_trait::async_trait]
pub trait GeocodeProvider: Send + Sync {
    /// Best matches first. An empty vec is a valid answer, not an error.
    async fn search(&self, query: &str) -> Result<Vec<Candidate>, String>;
}
```

**Not restricted to Slovakia.** Task 72's design pinned Nominatim to `countrycodes=sk`.
The data says that would fail outright: five of the 47 places are in Czechia and
Hungary, and one of them alone accounts for 39 trips. No country filter is applied.

Nominatim's usage policy is binding the same way the OSM tile policy already is in
[tiles.rs](../../src-tauri/core/src/route_map/tiles.rs): an identifying User-Agent, and
at most one request per second.

Nine of the 47 strings are bare names with no city component and will geocode poorly.
That is expected and needs no special handling — they are exactly the rows where you
drop a pin instead.

## Commands

In [core/src/commands_internal/](../../src-tauri/core/src/commands_internal/)`places.rs`, dispatcher-only, following
[ADR-016](../../DECISIONS.md#adr-016-_internal-extraction-pattern-for-command-reuse).

| command | dispatcher | |
|---|---|---|
| `list_places` | [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | the derived join: name, uses, coordinates, source |
| `geocode_place` | [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | candidates for one query. **Writes nothing.** |
| `save_place` | [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | write — `check_read_only!` |
| `clear_place` | [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | write — un-place a pin put in the wrong spot |

Looking and committing stay separate calls, for the reason task 72 separated generating
from saving: a geocode that wrote its first guess would make that guess permanent before
anyone saw it.

Normalisation lives in Rust and has exactly one implementation. Task 72's route-map
work needs the same function; the frontend gets no copy of it
([ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication)).

## UI

A `Miesta` section in [settings/+page.svelte](../../src/routes/settings/+page.svelte),
modelled on the vehicles section already there. Not a new nav tab: this is reference
data curated once, not something opened daily.

```
▾ Miesta                                    43 / 47 umiestnených
   [hľadať…]

   Office, City A                 48.148, 17.107   ✎
   Warehouse, City B              48.716, 21.261   ✎
 ⚠ Depot, City C                  —                ✎
```

The edit modal carries a Leaflet map, a search box and a draggable pin. Leaflet is
already a dependency and is lazily imported in
[mapa/+page.svelte](../../src/routes/mapa/+page.svelte); the same pattern applies, since
it touches `window` at import time. Searching lists candidates with their labels;
clicking one places the pin; dragging the pin sets `source = 'manual'`.

Placing is one request at a time, driven from the browser and persisted before the next
begins. No background job: closing the page mid-way loses nothing and reopening resumes
where it stopped, which also keeps the one-request-per-second obligation in the one
place that already honours it.

## Autocomplete

[TripRow.svelte](../../src/lib/components/TripRow.svelte) currently builds its
suggestions from the per-vehicle `routes` table, flattening origins and destinations
into a `Set` and sorting alphabetically. It switches to the book.

Two things improve: suggestions become global rather than per-vehicle, and two spellings
that normalise alike can never both be offered. Since the book is derived from trips,
a place typed today appears immediately — unplaced, but present — so nothing a user can
type ever goes missing from the list.

## Testing

Backend owns the logic; integration owns the flow; neither touches the network.

**Rust unit** — `normalise` exhaustively (case, diacritics, whitespace, empty); the
derived join, including a place used by trips but absent from `places`, and an orphan
row that must not appear; `save_place` and `clear_place` round-trips; both refused under
`check_read_only!`; geocoder response parsing against a fake provider — candidates,
empty result, HTTP error, malformed JSON.

**Integration** — one spec: the section lists places with unplaced ones marked, placing
one persists it, and it survives a reload. This needs the **provider-override
environment variable** that task 72 deferred, so it is pulled forward here; without it
the geocoder cannot be stubbed and the flow is untestable. It also unblocks the three
tests task 72 deferred for the same reason.

**Not tested:** whether a geocoder returns the right place for a given address. That is
a property of Nominatim, not of this code, and asserting it would be a network test
wearing a unit test's clothes.

## Non-goals

- **No merge, rename or alias UI.** The cleanup did that once as a data fix. If
  duplicates recur in practice, that is the moment to reconsider — not before.
- **No writing coordinates back into trips.** Trips keep text; the book keeps points.
- **No auto-accept.** See above.
- **No desktop support.** `route_maps` is server-only and no Tauri wrappers exist.
- **No Settings screen for anything task 72 needs beyond coordinates.**

## Deferred

- A provider-override env var is built here, but pointing the *router* at a stub as well
  is task 72's business.
- What task 72 does when an endpoint has no coordinates yet — its design decides that;
  the book only guarantees a lookup that can answer "not placed".

## Decisions to record via [/decision](../../DECISIONS.md)

- Every place is confirmed by a human; no confidence heuristic decides for the user.
- The book's list is derived from trips rather than maintained on write paths, on the
  evidence of `routes`' 52-of-96 drift.
- `display_name` is the spelling trips already use, never the geocoder's rendering.
- The geocoder is not restricted by country.
