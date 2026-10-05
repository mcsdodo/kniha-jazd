# Feature: Place Book (Miesta)

> Every place a trip can name is a record with an ID, a name and a position. The Miesta tab manages the records. A trip accepts only a place that exists.

A place is an entity. A trip and a saved route point at a place by ID
(`origin_place_id`, `destination_place_id`). The free-text origin and destination columns
do not exist any more. A read still returns the display names: the backend joins the
place name onto the trip. See [ADR-055](../../DECISIONS.md#adr-055-places-are-entities-trips-and-routes-reference-them-by-id) for the decision.

Three things read the book:

- **Trip form** - "Odkiaľ" and "Kam" accept only an existing place. See
  [trip-entry-defaults.md](./trip-entry-defaults.md).
- **Distance auto-fill and time inference** - both match a trip on place IDs.
- **Route maps** - the endpoints of a route come from the book. See
  [route-maps.md](./route-maps.md).

## User Flow

1. **Open the Miesta tab** (`/miesta` in the top navigation). The list shows every place
   of every vehicle. The heading shows `N/M umiestnených`: how many places have a
   position.
2. **Read the order.** Places without a position come first. Then places with more
   trips come first, then the name decides.
3. **Filter** with the search box.
4. **Add a place.** Click *Pridať miesto*, type the name and click *Ďalej*. A dialog with
   a map opens. A name and a position are both required, so the place is saved only when
   the user presses *Uložiť miesto*. If another place has the same normalised name, the
   save fails with an error.
5. **Set a position.** Click *Upraviť* on a row. The dialog opens with a search field
   filled with the place name. Search (Nominatim returns at most five candidates), click
   the map or drag the pin. Only *Uložiť miesto* writes the position.
6. **Rename a place.** Click *Premenovať*, type the new name and save. The new name shows
   on every trip of the place, also in the past years and in new prints. If another
   place already has the same normalised name, the rename fails with an error. The app
   does not merge two places.
7. **Delete a place.** Click *Zmazať* and confirm. The button is disabled when a trip
   uses the place. The tooltip says how many times the place is used. Saved routes that
   point at the place go with it (see Delete rules below).
8. **Mark the home place.** Click the house icon on a row. The icon is filled for the home
   place and outlined for the others. A click on the home place removes the mark. A new
   mark replaces the old one, because only one place can be home. The button has
   `data-testid="place-home-toggle"` and `aria-pressed`. The journey grouping and the MCP
   tool `list_journeys` read this mark. See [mcp-endpoint.md](./mcp-endpoint.md).
9. **Fix a migrated place.** A place from an old database can have no position. The row
   shows a warning marker with the text *Treba doplniť polohu*. Set a position as in
   step 5.

Settings has no place section. The Settings page and the Miesta tab are separate.

**Read-only mode**: create, rename, set position, delete and the home mark are blocked
(`check_read_only!`). The list, `find_place` and the search still work.

## Technical Implementation

### The data model

The `places` table:

| Column | Meaning |
|--------|---------|
| `id` | UUID, primary key |
| `name` | The spelling shown to the user |
| `normalised_name` | `normalise(name)`, `UNIQUE`. The identity of a place for matching. |
| `lat`, `lon` | Position. `NULL` only for a legacy place from the migration. |
| `source` | `geocoder` or `manual`. `NULL` when there is no position. |
| `created_at` | Creation time |
| `is_home` | `BOOLEAN NOT NULL DEFAULT 0`. `1` for the home place (task 89). |

The migration `2026-10-05-110000_add_place_is_home` adds `is_home` and the partial unique
index `idx_places_single_home` (`WHERE is_home = 1`). The index allows at most one home.
No place is home until the user marks one. The migration changes no data.

`trips` and `routes` have `origin_place_id` and `destination_place_id` with a foreign key
to `places(id)`. `routes` is `UNIQUE(vehicle_id, origin_place_id, destination_place_id)`.
`trip_routes` (the saved maps) and `paperless_trip_links` still point at the trip.

The `Place` model returned by the RPCs adds `uses`: how many trip endpoints point at the
place. It is computed at read time with a query, not stored (ADR-033 still holds for the
counter). A trip with the same place at both ends counts two.

### The matching key: `normalise()`

One function in [places/normalise.rs](../../src-tauri/core/src/places/normalise.rs)
decides what "the same place" means. It lowercases, folds diacritics, collapses white
space and trims:

```
"  Hlavná  ulica 12, Košice " -> "hlavna ulica 12, kosice"
```

Digits and punctuation stay: they tell one street number from another. The folding is a
closed table, not Unicode NFD. A letter that the table does not know keeps its accent
and gets its own key. The result is a duplicate place, never a wrong position.

The key is stored in `places.normalised_name`, so the unique index enforces it. The same
function runs in the migration, through the SQL function `kj_normalise`.

### Backend (Rust)

The commands live in
[commands_internal/places_cmd.rs](../../src-tauri/core/src/commands_internal/places_cmd.rs).

| Command | Args | Does |
|---------|------|------|
| `list_places` | none | All places with `uses` and `isHome`. Unplaced first, then `uses` descending, then name. |
| `create_place` | `name`, `lat`, `lon`, `source` | Creates a place. Position required. Error if the key exists. |
| `rename_place` | `id`, `name` | Changes `name` and `normalised_name`. Error if another place has the key. |
| `set_place_position` | `id`, `lat`, `lon`, `source` | Sets the position. |
| `delete_place` | `id` | Deletes the place only if no trip uses it. |
| `set_home_place` | `id` or `null` | Moves the home mark to the place, or clears it with `null`. One transaction: the old mark goes first. Error if the place does not exist. Guarded by `check_read_only!`. |
| `find_place` | `name` | Returns the place whose key equals `normalise(name)`, or nothing. |
| `geocode_place` | `query` | Returns candidates. **Writes nothing.** |

`save_place` and `clear_place` do not exist any more. A place keeps its position: the
user can change it, not remove it.

Details:

- **Name rules.** The backend trims the name and collapses the spaces. A blank name is an
  error.
- **Rename.** A rename applies to all trips, because the trips hold the ID. A key that
  another place holds is an error (`ensure_key_free`). The place that is renamed does
  not collide with its own key, so a rename that changes only the case or the accents is
  allowed.
- **Delete rules.** `delete_place_if_unused` counts the trips that use the place. If the
  count is more than zero, the command returns the error
  `Miesto používa N jázd, nedá sa zmazať`. If no trip uses the place, the command deletes
  the place and the routes that point at it.
- **`find_place`.** The trip form calls it for typed text that does not match a name
  exactly. A text that differs only in case, spaces or diacritics finds the place. The
  fold stays in Rust (ADR-008): the frontend has no second copy of `normalise`.
- **Geocoding.** `geocode_place` is unchanged. Looking and saving are separate calls, so
  no position is stored before a human confirms it (ADR-032). Nominatim sits behind the
  `GeocodeProvider` trait in [places/geocode.rs](../../src-tauri/core/src/places/geocode.rs).
  The request has `format=jsonv2`, `limit=5`, `accept-language=sk`, an identifying
  User-Agent and a 15 second timeout. There is no country filter (ADR-035).
  `KNIHA_JAZD_MOCK_GEOCODER_DIR` points the provider at canned answers for the tests.
  A row that Nominatim returns with a bad `lat` or `lon` is dropped, not the whole answer.

### The migration

The migration `2026-10-05-100000_places_as_entities`
([up.sql](../../src-tauri/core/migrations/2026-10-05-100000_places_as_entities/up.sql))
is plain SQL. It uses the SQL function `kj_normalise`. The function is registered from
Rust in `db::prepare_connection`, before the migrations, on every connection that
migrates, also on a restore. The `diesel migration run` command from the CLI fails with
`no such function: kj_normalise`: only the app can run this migration.

The rules:

1. **One place for each key.** All spellings that give one `normalise()` key become one
   place. The name is the spelling that trips use most. If the counts are equal, the
   byte-wise smaller spelling wins.
2. **Positions stay.** The place takes the position of the old `places` row with the same
   key.
3. **Unused places stay.** An old `places` row that no trip names stays as a place.
4. **A blank endpoint** becomes the place `Neznáme miesto`, so the trip keeps a target.
5. **A place without a position** is marked *Treba doplniť polohu* in the tab.
6. **Routes.** The migration drops routes that no trip uses. If two routes of one vehicle
   fall on one place pair, it keeps the route that the latest trip on that pair used by
   exact spelling. If none matches, it keeps the first by `id`.
7. **Child rows.** Foreign keys are ON. `DROP TABLE trips` would delete `trip_routes` and
   `paperless_trip_links` rows by cascade. The migration copies both tables to TEMP
   tables first and copies them back after the new `trips` table exists.

The migration is **one-way for the app**. An image that is older than the migration opens
the migrated database in read-only mode. See [read-only-mode.md](./read-only-mode.md).

### Frontend

**Miesta tab** - [src/routes/miesta/+page.svelte](../../src/routes/miesta/+page.svelte).
It shows the list in the order that the backend returns (ADR-008). The filter matches
both `name` and `normalisedName`: a query with diacritics matches the name and an ASCII
query matches the key. The tab never folds the query itself.

**PlaceModal** - [src/lib/components/PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte).
The dialog imports no write command. It gives the confirmed position to the page, and
the page owns the write (`setPlacePosition` or `createPlace`). A geocoder answer cannot
reach the database before the user presses save. The page wraps the dialog in
`{#key ...}`, so a dialog never keeps the pin of a previous place.

**Trip form** - [TripRow.svelte](../../src/lib/components/TripRow.svelte) offers the place
names from `list_places`. [TripGrid.svelte](../../src/lib/components/TripGrid.svelte)
loads the list on mount and after every trip write.

### Data Flow

```
Miesta tab
  |
  +- list_places -> list_places_internal -> places + uses (SQL), sort (Rust)
  |
  +- Pridat miesto: name -> PlaceModal
  |     +- geocode_place -> Nominatim        nothing written
  |     +- Save -> create_place (name, lat, lon, source)
  |
  +- Upravit: PlaceModal -> Save -> set_place_position
  +- Premenovat: rename_place   (error on a key collision)
  +- Zmazat: delete_place       (error if a trip uses the place)

Trip form (save)
  +- typed text equals a place name -> use that ID
  +- else find_place(text) -> ID, or the save is blocked with a link to /miesta
```

## Key Files

| File | Purpose |
|------|---------|
| [places/normalise.rs](../../src-tauri/core/src/places/normalise.rs) | `normalise()`, the matching key |
| [places/geocode.rs](../../src-tauri/core/src/places/geocode.rs) | `GeocodeProvider` trait, Nominatim client, mock mode |
| [commands_internal/places_cmd.rs](../../src-tauri/core/src/commands_internal/places_cmd.rs) | The place commands |
| [db.rs](../../src-tauri/core/src/db.rs) | `kj_normalise` registration, `delete_place_if_unused`, place queries |
| [models.rs](../../src-tauri/core/src/models.rs) | `Place`, `PlaceRow`, `NewPlaceRow`, `PlaceSource` |
| [migrations/2026-10-05-100000_places_as_entities](../../src-tauri/core/migrations/2026-10-05-100000_places_as_entities/up.sql) | The migration |
| [migrations/2026-10-05-110000_add_place_is_home](../../src-tauri/core/migrations/2026-10-05-110000_add_place_is_home/up.sql) | The `is_home` column and the one-home index |
| [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | The RPC arms |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | The `geocode_place` arm |
| [miesta/+page.svelte](../../src/routes/miesta/+page.svelte) | The Miesta tab |
| [PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte) | Map dialog: search, candidates, draggable pin |
| [TripRow.svelte](../../src/lib/components/TripRow.svelte) | Place choice and `find_place` on save |
| [types.ts](../../src/lib/types.ts) / [api.ts](../../src/lib/api.ts) | `Place`, `PlaceSource`; the RPC wrappers |
| [places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts) | Tier 2 flows |
| [home-place.spec.ts](../../tests/integration/specs/tier2/home-place.spec.ts) | Tier 2 flow: the home icon |

## Design Decisions

- **Why entities?** A free-text place needed a fold (`normalise`) at every read, and a
  rename of a place was impossible. With an ID, a rename is one update and the match is
  an equality. See ADR-055 in [DECISIONS.md](../../DECISIONS.md#adr-055-places-are-entities-trips-and-routes-reference-them-by-id). It supersedes ADR-033
  for places (the route counters stay derived) and ADR-034 (the display name is now the
  stored name).
- **Why is a collision an error, not a merge?** A merge changes the trips of two places
  and cannot be undone. The user can see the other place and decide.
- **Why can a place be deleted only if unused?** A delete of a used place would leave
  trips without an endpoint. The tooltip tells the user how many trips use it.
- **Why does a trip accept only an existing place?** Typed text that matches no place
  was the source of duplicate spellings. The save is blocked and the message links to
  Miesta.
- **Why one home place?** The journey grouping needs one start point. A partial unique
  index enforces it in the database, and `set_home_place` clears the old mark first. The
  match is by place ID, so a rename does not break it. See
  [BIZ-026](../../DECISIONS.md#biz-026-journeys-away-from-home).
- **Every place is confirmed by a human (ADR-032).** The app never accepts a geocoder
  answer without a click. A wrong pin shows up later as a wrong map on a legal document.
- **The geocoder has no country filter (ADR-035).** Real logbooks name Czech and
  Hungarian places.
- **`normalise()` is not `db::normalize_location`.** The first makes a matching key. The
  second only cleans the spaces of a spelling that is stored.
- **No rate limiter in the client.** One address at a time is submitted by a person, so
  the Nominatim limit of one request each second is met by the use itself.

## Testing

- **Backend unit tests** own the rules: `normalise` and its table, create, rename (also
  the collision error), set position, delete (also the in-use error), `find_place`, the
  read-only refusal, the migration cases (spelling fold, blank endpoint, route collapse,
  child rows) and the parse of a Nominatim answer (`wiremock`).
- **Integration tests** ([places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts))
  own the UI flows: add, rename, delete, the disabled delete of a used place and the
  trip form that accepts only an existing place.
- **Integration test** ([home-place.spec.ts](../../tests/integration/specs/tier2/home-place.spec.ts))
  owns the home icon flow: mark, replace and reload.

No test reaches Nominatim. What the geocoder answers for a real address is not this
project's behaviour.

## Related

- [ADR-055](../../DECISIONS.md#adr-055-places-are-entities-trips-and-routes-reference-them-by-id): Places are entities, trips and routes reference them by ID
- [BIZ-026](../../DECISIONS.md#biz-026-journeys-away-from-home): Journeys away from home (the home mark)
- [mcp-endpoint.md](./mcp-endpoint.md): the read-only MCP endpoint that reads the home mark
- [ADR-032](../../DECISIONS.md): Places are placed by a human, never by a confidence heuristic
- [ADR-033](../../DECISIONS.md): Aggregates over trips are computed, not stored
- [ADR-034](../../DECISIONS.md): The display spelling (superseded by ADR-055: the name is stored)
- [ADR-035](../../DECISIONS.md): The geocoder is not restricted by country
- [ADR-008](../../DECISIONS.md): All business logic lives in the Rust backend
- [_tasks/88-places-as-entities/](../../_tasks/88-places-as-entities/): task and plan
- [_tasks/_done/75-place-book/](../../_tasks/_done/75-place-book/): the first place book
