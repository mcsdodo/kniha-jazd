# Feature: Place Book (Miesta)

> Every place the logbook's trips name, listed once, each with a coordinate a human confirmed — the shared source for trip autocomplete and, later, route maps.

Trips record `origin` and `destination` as free text. The place book sits beside them
and points at them: it gives each distinct place string a latitude and longitude,
confirmed once, in a **Miesta** section in Settings. Nothing about trips changes.

Two things read the book:

- **Trip autocomplete** — the „Odkiaľ" / „Kam" suggestions used to come from the
  per-vehicle `routes` table; they now come from the book, which spans every vehicle.
- **Route maps** ([task 72](../../_tasks/72-route-map-origin-destination/)) — a row can
  only be routed A→B once both endpoints have coordinates. This is where they come from.

## User Flow

1. **Open Settings → Miesta.** The list is already populated: it is derived from the
   trips in the database, so no place has to be added by hand and none can be missed.
2. **Read the counter.** The heading carries `N/M umiestnených` — how many of the
   book's places already have a coordinate. It is the progress bar for the one pass
   through the list.
3. **Work top-down.** Unplaced places sort first (they are the work left to do), then
   by how heavily they are used, then by name. An unplaced row is marked with a ⚠ and
   shows `—` where its coordinates would be; a placed row shows them to three decimals
   (~100 m — enough to recognise a place, short enough to read).
4. **Filter** with the search box when looking for one particular place.
5. **Click Upraviť** on a row. A dialog opens with a map and a search field
   pre-filled with the place's own name, so walking the list is one click per row.
6. **Search.** Submitting sends the query to Nominatim and returns at most five
   candidates. Picking one drops the pin and centres the map on it.
7. **Or place it by hand.** Clicking the map, or dragging the pin, sets the coordinate
   directly — this is the path for a place the geocoder cannot find, and for
   fine-tuning a candidate that landed close but not exactly.
8. **Save.** Only now is anything written. The dialog holds a *pending* pin until
   then; closing it discards the pin.
9. **Remove a location** with the dialog's clear button — offered only for a place
   that already has one. The place itself stays in the list (it is still named by
   trips); it simply goes back to being unplaced.

The place's own spelling never changes. What the geocoder calls the address is shown
while choosing and then thrown away.

**Read-only mode**: saving and clearing are blocked (`check_read_only!`). Listing and
searching still work.

## Technical Implementation

### The lookup key: `normalise()`

One function decides what "the same place" means. It lowercases, folds diacritics,
collapses whitespace and trims:

```
"  Hlavná  ulica 12, Košice " → "hlavna ulica 12, kosice"
```

Digits and punctuation survive — they are what distinguishes one street number from
another, and the book's entries are addresses.

The folding is a **closed table, not Unicode NFD**. NFD would pull in a dependency for
a 44-letter problem, and being a one-liner in JavaScript it would invite the frontend
to grow a second implementation that disagrees (ADR-008). The price is that a letter
the table does not know keeps its accent and gets a key of its own — a duplicate row
in the book, never a wrong coordinate.

This is **not** [`db::normalize_location`](../../src-tauri/core/src/db.rs), which
already existed. The two do different jobs: `normalize_location` only collapses
whitespace and keeps case and diacritics, because it rewrites the string a *trip
stores*; `normalise` throws that information away to make a *matching key*.

### The derived list

The `places` table stores coordinates and nothing else — there is no list of places in
it. The list is computed at read time (ADR-033):

```
raw = SELECT origin, COUNT(*) FROM trips GROUP BY origin
      UNION ALL
      SELECT destination, COUNT(*) FROM trips GROUP BY destination
      (summed per raw spelling)

for each (spelling, uses) in raw:
    key = normalise(spelling)
    skip if key is empty
    fold onto the entry for key:
        total += uses
        if uses beats the leading spelling's own count: this spelling leads
    left join the stored coordinate for key

sort: unplaced first, then uses descending, then display name
```

The **join happens in Rust, not SQL**, because the key is `normalise()` and SQLite
cannot call it. At tens of rows the cost is nil.

Two details in the fold are load-bearing:

- The leading spelling is chosen against the leader's **own** count, never the running
  total — the total has already absorbed other spellings, so it outgrows any single one
  and would freeze the display on whichever spelling SQLite happened to return first.
- Equal counts are settled on the spelling itself (byte-wise smaller wins), so row
  order does not decide those either.

`uses` counts trip **endpoints**, not trips: A → B adds one to each, and a trip whose
origin and destination are the same place adds two to it. Hence the Slovak label
„výskytov" (occurrences) rather than „jázd".

### Geocoding

Nominatim sits behind a `GeocodeProvider` trait so tests can stand in a fake and never
touch the network. `HttpGeocodeProvider` asks the public instance for `format=jsonv2`,
`limit=5`, `accept-language=sk`, with an identifying User-Agent (Nominatim's usage
policy requires one; a generic or absent agent gets the whole application blocked
rather than just one request). The timeout is 15 seconds — a person is watching a
dialog for the answer, so a slow search is a stuck one.

A row whose latitude or longitude will not parse is dropped, not the whole response:
the candidates are independent alternatives, so one unreadable row costs that row and
not the user's other four options. An empty answer is valid and already means "place it
by hand".

Deliberately absent: retries, caching, any country filter (ADR-035), and any rate
limiter (see Design Decisions).

**Mock mode** — `KNIHA_JAZD_MOCK_GEOCODER_DIR` points the provider at a directory of
canned `jsonv2` bodies ([tests/integration/data/geocoder/](../../tests/integration/data/geocoder/)), filed under `normalise(query) + ".json"`. So
`"Gamma Warehouse, Testville"` is answered by [gamma warehouse, testville.json](../../tests/integration/data/geocoder/gamma%20warehouse,%20testville.json). A
missing file is an empty answer; a file that is present but unreadable is an error —
a broken fixture must shout rather than quietly turn a test green. The integration
harness sets the variable for you, both for the spawned server
([wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts)) and for the
container (`-e` flag in [test.yml](../../.github/workflows/test.yml)).

### Backend (Rust)

Four RPC commands, registered in
[server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) except the one
that awaits the network:

| Command | Where | Args | Does |
|---------|-------|------|------|
| `list_places` | dispatcher | — | The derived list, ordered |
| `geocode_place` | dispatcher_async | `query` | Candidates. **Writes nothing** |
| `save_place` | dispatcher | `displayName`, `lat`, `lon`, `source` | Upserts the coordinate |
| `clear_place` | dispatcher | `displayName` | Forgets it |

Looking and committing are separate calls on purpose: a geocode that saved its first
guess would make that guess permanent before anyone saw it, and the whole point of the
book is that no coordinate is stored without a human confirming it.

`save_place` derives the key itself — not the caller, not the db layer — so a save and
the list that reads it back can never disagree about what "the same place" is. It
refuses an empty name: the read side has already decided an empty key is not a place,
and storing one would leave a row no list can show. The guard cannot live in the
caller, because `POST /api/rpc` is reachable without the UI.

`upsert_place` is a delete + insert in one transaction, the same shape
`save_route_map` uses for the same "primary key that is not an id" problem. An
`AsChangeset` update would not do: it reads `None` as "leave this column alone", so it
could set a coordinate but never clear one, and the row would end up a merge of two
answers rather than the latest one.

`clear_place` on a place that was never placed is a **no-op, not an error** — the
caller asked for the book to hold no coordinate for it, and it already holds none.

`source` is `geocoder` or `manual` — recording whether a coordinate is a suggestion
someone accepted or a pin someone dropped. An unrecognised string parses to `None`
rather than panicking, so a row written by a newer build cannot crash an older one.

### Frontend

**Miesta section** — [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte).
Renders the list in the order it arrives and never re-sorts it (ADR-008). The filter
matches against **both** spellings the row carries: a query typed with diacritics
matches `displayName`, an ASCII one matches `normalisedName`. That asymmetry is
deliberate — folding the needle in TypeScript would mean a second, divergent copy of
`normalise`, which ADR-008 forbids. The case that actually occurs is the ASCII one:
production data shows users type "Kosice", not "Košice".

**PlaceModal** — [src/lib/components/PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte).
Leaflet is imported lazily (it touches `window` at import time), the pin is a `divIcon`
so no image asset has to survive the bundler's URL rewriting, and tiles carry the
attribution the OpenStreetMap tile policy requires. The pending coordinate is seeded
from the prop exactly once and then free to diverge — a dialog that re-mirrored the
prop would throw the user's edits away. The dialog imports **no write command at all**;
it hands the confirmed coordinate back to the page, which owns the write. A geocoder
answer therefore physically cannot reach the database before someone presses Save.

Because the seeding is once-only, the settings page wraps the dialog in
`{#key place.normalisedName}`. The `{#if}` alone is not enough: nothing traps focus, so
a keyboard user can tab to another row's edit button behind the open dialog, and the
target would go straight from place A to place B without ever being null — leaving A's
coordinate in the dialog under B's name, which is exactly the wrong-pin outcome ADR-032
exists to prevent.

**Trip autocomplete** — [TripGrid.svelte](../../src/lib/components/TripGrid.svelte)
loads the book on mount and again after a trip is created or updated, so a place just
typed into a trip is offered on the next row without a page reload.
[TripRow.svelte](../../src/lib/components/TripRow.svelte) offers `displayName` (never
`normalisedName`, which is a folded key and would be written verbatim into the trip),
re-sorted alphabetically because a datalist wants alphabetical order rather than the
book's work-queue order. `routes` is still passed to the row — it carries the
per-vehicle kilometres for a known origin/destination pair — but it no longer feeds the
suggestions.

### Data Flow

```
Settings → Miesta
  │
  ├─ list_places ──► dispatch_sync ──► list_places_internal
  │                                      │
  │                                      ├─ distinct_trip_places()   (SQL: trips)
  │                                      ├─ all_places()             (SQL: places)
  │                                      └─ fold on normalise(), join, sort  (Rust)
  │                                                      │
  │                            ◄─────────────────────────┘  Place[]
  │
  └─ open PlaceModal
        │
        ├─ geocode_place ─► dispatch_async ─► HttpGeocodeProvider ─► Nominatim
        │                                     (or mock dir)
        │                    ◄───────────────  Candidate[]     nothing written
        │
        │   user picks a candidate, clicks the map, or drags the pin
        │   → pending {lat, lon, source} lives in the dialog only
        │
        └─ Save ─► save_place ─► save_place_internal ─► upsert places row
                                  (check_read_only!, key = normalise(displayName))
                        │
                        └─► list_places again ─► list re-renders, counter ticks up
```

## Key Files

| File | Purpose |
|------|---------|
| [places/normalise.rs](../../src-tauri/core/src/places/normalise.rs) | `normalise()` — the one lookup key |
| [places/geocode.rs](../../src-tauri/core/src/places/geocode.rs) | `GeocodeProvider` trait, Nominatim client, mock mode |
| [commands_internal/places_cmd.rs](../../src-tauri/core/src/commands_internal/places_cmd.rs) | `list_places_internal`, `save_place_internal`, `clear_place_internal` |
| [db.rs](../../src-tauri/core/src/db.rs) | `distinct_trip_places`, `all_places`, `upsert_place`, `delete_place` |
| [models.rs](../../src-tauri/core/src/models.rs) | `Place`, `PlaceRow`, `NewPlaceRow`, `PlaceSource` |
| [migrations/2026-09-07-100000_add_places](../../src-tauri/core/migrations/2026-09-07-100000_add_places/up.sql) | The `places` table |
| [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | `list_places`, `save_place`, `clear_place` arms |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | `geocode_place` arm |
| [settings/+page.svelte](../../src/routes/settings/+page.svelte) | Miesta section: list, counter, filter |
| [PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte) | Map dialog: search, candidates, draggable pin |
| [TripGrid.svelte](../../src/lib/components/TripGrid.svelte) / [TripRow.svelte](../../src/lib/components/TripRow.svelte) | Autocomplete fed from the book |
| [types.ts](../../src/lib/types.ts) / [api.ts](../../src/lib/api.ts) | `Place`, `GeocodeCandidate`, `PlaceSource`; the four wrappers |
| [places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts) | Tier 2 flows against a stubbed geocoder |

## Design Decisions

### Every place is confirmed by a human — no auto-accept, no confidence threshold (ADR-032)

The brainstorm had proposed trusting the geocoder when its returned name equalled the
query. Profiling the production database killed that outright: the book's entries are
**street addresses**, not town names, and a geocoder's name for an address never equals
the typed string. The rule could not fire.

Rather than invent a replacement heuristic, there is none. Any threshold would be tuned
to one geocoder's scoring and would break silently when the provider is swapped — which
the `GeocodeProvider` trait exists to allow. Decisively: a wrong pin accepted silently
does not announce itself. It surfaces later as a map of the wrong place on a document
that is **legal evidence**. The cheapest way not to have that failure is to never guess.

The cost is roughly fifteen minutes of clicking, once, on a closed list of 47 places
that grows by a handful a year.

### The list is derived from trips, never stored (ADR-033)

A direct lesson from the `routes` table. Its stored `usage_count` was maintained by
three write paths that all had to agree forever — and did not: the counter was **wrong
in 52 of its 96 production rows** ([task 76](../../_tasks/_done/76-route-usage-counter-drift/)).
`update_trip` counted a second time, deleting a trip never decremented, and rows
outlived every trip that justified them.

A derived list cannot drift. A place appears when a trip names it and disappears when
the last one stops, at no cost at tens of rows and with no backfill migration.

The accepted consequence: a `places` row whose trips are all deleted becomes an
invisible orphan — its coordinates stay on disk but nothing lists them. That is a point
nobody asks for, not a wrong answer.

### `display_name` is the trip's own spelling, never the geocoder's (ADR-034)

A geocoder returns an official rendering — full diacritics, canonical street form,
country suffix. The logbook's strings are plain ASCII, made consistent by a one-off
cleanup.

The autocomplete offers `display_name`. If that were the geocoder's rendering, picking
a suggestion would write a *new* string into the trip — one differing from every row
already using the plain form — and the spelling fragmentation the cleanup had just
removed would grow straight back, one autocomplete selection at a time. The book's job
is to point at places, not to rename them.

### The geocoder is not restricted by country (ADR-035)

[Task 72](../../_tasks/72-route-map-origin-destination/)'s earlier design pinned
Nominatim to `countrycodes=sk`, assuming a Slovak logbook names Slovak places. The
production data contradicts it: five of the 47 places are Czech or Hungarian, and a
single Czech address accounts for **39 trips**. The restriction would fail those
outright, leaving a pin-by-hand as the only route to the most-travelled foreign
destination in the book.

The usage obligations that actually matter — an identifying User-Agent and one request
per second — are unaffected by dropping it.

### The join happens in Rust, not SQL

The list's key is `normalise()`, and SQLite cannot call it. So SQL groups the raw trip
strings and Rust does the folding and the join. At tens of rows the cost is nil, and
keeping the key in one language keeps ADR-008's "one implementation of the rule"
intact.

### `normalise()` is not `db::normalize_location`

Two functions, two jobs, on purpose. `normalize_location` collapses whitespace but
keeps case and diacritics, because it produces the **canonical spelling a trip stores**.
`normalise` folds case and diacritics away, because it produces a **matching key**.
Merging them would either put folded ASCII into trips or make the book blind to
"Kosice" vs "Košice".

### `uses` counts endpoints, not trips

A → B contributes one use to each of A and B; a trip that starts and ends at the same
place contributes two to it. The number answers "how much does this place appear in the
book", which is what makes it a sensible sort key for the work queue. The Slovak label
says „výskytov" rather than „jázd" for exactly this reason.

### No rate limiter in the client

Nominatim's usage policy caps requests at one per second. Placing is one address at a
time, driven from the browser, submit-only (no keystroke starts a request), and
persisted before the next begins — so the obligation is already met by the only thing
that can pace it. A limiter in `HttpGeocodeProvider` would be machinery guarding a
queue that never has two items in it.

## Testing

- **Backend unit tests** own the logic: `places::normalise` and its folding table, how
  the derived list folds spellings and orders itself, the display-name tiebreak,
  save/clear semantics, the empty-name refusal, the read-only refusal, and how a
  Nominatim `jsonv2` response is parsed into candidates (via `wiremock`).
- **Integration tests** ([places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts))
  own the three UI flows and nothing else: a seeded trip's places turn up marked
  unplaced; a coordinate confirmed in the dialog is stored and survives a reload; and a
  place one vehicle used is offered in another vehicle's trip form — the behaviour the
  old per-vehicle `routes` autocomplete could not provide, and the one worth pinning.

Nothing in either layer reaches Nominatim. What the geocoder replies for a real address
is Nominatim's property, not this codebase's, so no test asserts that a search finds the
right place — only that the candidate a human picked travels from dialog to database to
list.

The spec shares one backend with the rest of tier 2 and the harness's `afterTest` reset
does not clear the `places` table, so the spec clears its own place rows in
`beforeEach` as well as `after` — a save from an earlier retry would otherwise still be
on disk when the "unplaced" assertions run.

## Related

- [ADR-032](../../DECISIONS.md): Places are placed by a human, never by a confidence heuristic
- [ADR-033](../../DECISIONS.md): Aggregates over trips are computed, not stored
- [ADR-034](../../DECISIONS.md): The book displays the spelling trips already use, not the geocoder's
- [ADR-035](../../DECISIONS.md): The geocoder is not restricted by country
- [ADR-008](../../DECISIONS.md): All business logic lives in the Rust backend
- [_tasks/_done/75-place-book/](../../_tasks/_done/75-place-book/) — task, design and plan
- [_tasks/72-route-map-origin-destination/](../../_tasks/72-route-map-origin-destination/) — the next reader of the book
- [_tasks/_done/76-route-usage-counter-drift/](../../_tasks/_done/76-route-usage-counter-drift/) — the stored-counter drift that shaped ADR-033
