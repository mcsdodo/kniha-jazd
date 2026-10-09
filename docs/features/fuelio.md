# Feature: Fuelio Cross-Check

> Compares the drives that the Fuelio Android app recorded (GPS) with the logbook, finds
> the drives the logbook does not have, and can correct a trip or add a missing one from
> the GPS data.

## User Flow

1. **Setup.** The operator sets `DROPBOX_APP_KEY`, `DROPBOX_APP_SECRET` and
   `DROPBOX_REFRESH_TOKEN`. Without all three, the nav has no "Fuelio" link, `/fuelio`
   shows a notice, and every Fuelio command fails, even if `<DATA_DIR>/fuelio` holds
   drives.
2. **Sync.** On `/fuelio` the user clicks **Synchronizovať z Dropboxu (rok)**. The server
   downloads the drives of the selected year that are not in `<DATA_DIR>/fuelio` yet.
3. **Read the table.** One row per run of Fuelio drives (newest first):
   - The **Stav** column: green check = matched to a logbook trip, red exclamation mark =
     missing in the logbook, crossed-out eye = ignored. The warning icons follow in the
     same cell: time differs, km differ, different route, partial GPS, loose match.
   - Logbook time, route and km, next to GPS time, km, minutes above 100 km/h and max
     speed, and the differences (start minutes, km %, off-route %).
   - A legend above the table explains every icon. Each icon also has a tooltip and an
     `aria-label`.
4. **Filter.** Min. km (15 by default: the page is for long drives), the state pills
   (matched / missing / ignored), "Len problémy" and "Len diaľnica" (GPS km >= 30 or >= 5
   minutes above 100 km/h). With no state pill selected, the table shows matched and
   missing rows; an ignored row shows only under its own pill. "Len problémy" never
   counts an ignored row.
   Two more fields set when drives join one row: the longest stop (60 min by default)
   and the largest distance (2 km by default) from the end of a drive to the start of
   the next. A change loads the report again. Raise them to join a drive that Fuelio
   split at a long stop, then "Pridať" adds it as one trip.
5. **Map.** The map shows the selected row: the GPS track (red) and the stored route
   (blue). After a load, and when a filter hides the selected row, the first shown row
   is selected.
6. **Ignorovať** (missing row, crossed-out eye button). The drive does not belong in the
   logbook, for example a private drive. The row leaves the default view. Under the
   "Ignorované" pill the eye button un-ignores it.
7. **Prepísať** (matched row). A popup selects which trip fields the GPS values replace:
   start, end, distance, route. A distance change shows the odometer cascade confirmation
   (the same one as on `/mapa`).
8. **Pridať** (missing row). A popup proposes the origin and destination (the nearest
   places to the GPS start and end, with their distance), the GPS times and km. The user
   types the purpose and saves. A new trip that moves later odometers shows the insert
   cascade confirmation.

## Technical Implementation

### Frontend
- Page: [src/routes/fuelio/+page.svelte](../../src/routes/fuelio/+page.svelte) -- table,
  filters, legend, Leaflet map. It only renders the report (ADR-008); the filters only
  hide rows.
- Popups: [FuelioOverwriteModal.svelte](../../src/lib/components/FuelioOverwriteModal.svelte),
  [FuelioAddModal.svelte](../../src/lib/components/FuelioAddModal.svelte).
- Nav: [src/routes/+layout.svelte](../../src/routes/+layout.svelte) asks
  `is_fuelio_available` once at start and shows the link only on `true`.

### Backend (Rust)
| Command | What it does |
|---------|--------------|
| `is_fuelio_available` | `true` only when the three `DROPBOX_*` secrets are set |
| `get_fuelio_crosscheck { vehicleId, year }` | Scans the folder, matches, returns the rows |
| `get_fuelio_track { driveIds, tripId? }` | Full GPS track + stored route for the map |
| `apply_fuelio_to_trip { tripId, driveIds, fields, dryRun }` | Prepísať, one transaction |
| `get_fuelio_add_preview { driveIds }` | Times, km and the places sorted by distance |
| `add_fuelio_trip { ..., withRoute, dryRun }` | Pridať, through `create_trip_cascade_internal` |
| `set_fuelio_drives_ignored { vehicleId, driveIds, ignored }` | Ignore / un-ignore the drives of a row |
| `sync_fuelio_dropbox { year }` | Async: Dropbox download (dispatcher_async) |

Every sync command calls `require_fuelio_internal` after its argument parse, so a
missing `dryRun` still fails first. Read-only mode blocks the writes, not the dry runs:
`apply_fuelio_to_trip` calls `check_read_only!` itself, `add_fuelio_trip` through
`create_trip_cascade_internal`.

### Data source

Fuelio writes each drive as `route-<epoch ms>.data`: a zip with one headerless CSV, one
GPS fix per row: timestamp (ms UTC), lat, lon, distance from the previous fix (m), speed
(m/s), altitude, accuracy. [parse.rs](../../src-tauri/core/src/fuelio/parse.rs) reads it
and computes the local start/end (Europe/Bratislava), GPS km, minutes above 100 km/h,
max speed, and a track thinned to one point per 100 m. The scanner keeps one drive per ID.
rclone and Drive leave copies such as `route-<id>(1).data`: the scan and every later read
(`read_drive`) take the canonical file first, then a copy, and use the first one that reads.
A drive belongs to the local year of the epoch in its file name, as in the sync. A page
load skips the files of the other years before it unzips them.

### Matching ([matching.rs](../../src-tauri/core/src/fuelio/matching.rs))

Fuelio is the reference: every row is a run of drives. A logbook trip without a drive is
normal (Fuelio does not record every drive) and gets no row. Three passes, each one only
on the drives and trips that the earlier passes left:

1. **Complete.** A run of consecutive drives (gaps <= 90 min) that starts within 2 km of
   the origin place, ends within 2 km of the destination, starts within 12 h of the
   logbook start and has 50-150% of the trip km. A round trip must end at its origin, and
   its stop at the turnaround can last the whole trip. If more runs fit, the nearest start
   time wins. A drive belongs to one trip only.
2. **Partial** (flag "Čiastočné GPS"). The trip has a stored route, >= 80% of the track is
   within 500 m of it, the run starts or ends near a trip place, starts between 3 h before
   the trip start and 3 h after its end, and has <= 150% of the trip km.
3. **Loose** (flag "Voľné spárovanie"). The rest joins into chains (gap <= 60 min, next
   start within 2 km of the previous end). A chain goes to the nearest trip in time inside
   the same 3 h window if a chain end is within 5 km of a trip place or >= 50% of it is
   within 1 km of the stored route, and the trip stays at <= 150% of its km.

A chain that fits no trip is a **missing** row.

**Merge rules (`MergeRules`).** The page sends `maxGapMin` and `maxJumpKm` with
`get_fuelio_crosscheck`; absent values use the defaults 60 min and 2 km, and a negative
value is an error. They replace the gap and the distance of the pass 3 chains. The
passes 1 and 2 use the gap or 90 min, whichever is longer, so a small value never
splits a trip. A round trip keeps its own longer stop rule.

**Ignored rows.** The table `fuelio_ignored_drives (vehicle_id, drive_id)` holds the drives
the user ignored. Matching does not read it: the ignored drives stay in the input, so a
drive that later gets its trip is a normal match. After the match,
`get_fuelio_crosscheck` sets `ignored` on a **missing** row only, and only when **every**
drive of the row is ignored: a drive that a later sync adds to the chain shows the row
again. `set_fuelio_drives_ignored` writes all drives of the row in one transaction; it is
blocked in read-only mode and checks every drive ID. Flags on a match: start differs by more
than 30 min, km differ by more than 10%, more than 10% of the track is over 500 m from
the stored route ("Iná trasa").

### Writes
- **Prepísať:** start and end in whole minutes (a start that moves the trip past another
  trip is refused, because the order is the odometer chain); the distance in whole km
  through `plan_route_distance`, so later odometers move; the route as the full GPS track
  (`direct`, no provider). A complete match preselects every field; a partial or loose
  match preselects none and warns.
- **Pridať:** the trip goes through the normal create cascade, then the full GPS track is
  saved as its route. The route is a second write: if it fails, the trip stays, the
  command still succeeds with `routeError`, and the dialog closes with an error toast.
  So a second click cannot add the trip again.
- Both store **every GPS fix** as the route. The 100 m track cut corners (up to 93 m off
  the road on the highway) and stays only in the "Iná trasa" check.

### Dropbox sync ([dropbox.rs](../../src-tauri/core/src/fuelio/dropbox.rs))

The refresh token buys an access token. The server lists `FUELIO_DROPBOX_FOLDER`
(default `/Apps/Fuelio/routes`, 2000 entries per page), keeps the files whose start year
(the epoch in the file name, in local time) is the selected year, and downloads only the missing ones, 8
at a time, to a temporary name and then a rename. A body that does not open as a zip is
not saved and counts as failed, so the next sync downloads it again. No cursor: a full listing is a few
calls. The Dropbox app needs "Full Dropbox" access, because Fuelio writes to its own app
folder.

### Data Flow
```
Dropbox --sync_fuelio_dropbox--> <DATA_DIR>/fuelio/route-*.data
                                           |
/fuelio --get_fuelio_crosscheck--> scan + match (fuelio::) --> rows --> table + legend
   |                                                     trips + places (db)
   +-- row click  --get_fuelio_track------> GPS track + stored route --> map
   +-- Prepísať   --apply_fuelio_to_trip--> dry run -> cascade modal -> write
   +-- Pridať     --add_fuelio_trip-------> dry run -> cascade modal -> write
```

## Key Files

| File | Purpose |
|------|---------|
| [fuelio/parse.rs](../../src-tauri/core/src/fuelio/parse.rs) | Zip + CSV parse, drive stats, folder scan |
| [fuelio/matching.rs](../../src-tauri/core/src/fuelio/matching.rs) | The three passes, flags, highway rule |
| [fuelio/geo.rs](../../src-tauri/core/src/fuelio/geo.rs) | Haversine, distance to a polyline |
| [fuelio/dropbox.rs](../../src-tauri/core/src/fuelio/dropbox.rs) | `DropboxConfig::from_env`, `sync_year` |
| [commands_internal/fuelio_cmd.rs](../../src-tauri/core/src/commands_internal/fuelio_cmd.rs) | The commands, `require_fuelio_internal` |
| [src/routes/fuelio/+page.svelte](../../src/routes/fuelio/+page.svelte) | Page, icon snippet, legend |
| [migrations/2026-10-08-100000_fuelio_ignored_drives](../../src-tauri/core/migrations/2026-10-08-100000_fuelio_ignored_drives/up.sql) | The ignored drives, per vehicle (FK with `ON DELETE CASCADE`) |
| [tests/integration/fixtures/fuelio-drives.mjs](../../tests/integration/fixtures/fuelio-drives.mjs) | Writes three drives of the current year for the env suite |
| [tests/integration/specs/env/fuelio.spec.ts](../../tests/integration/specs/env/fuelio.spec.ts) | Rows, map, ignore / un-ignore, a trip with two rows, Prepísať and Pridať with the odometer dialog |
| [tests/integration/specs/tier3/empty-states.spec.ts](../../tests/integration/specs/tier3/empty-states.spec.ts) | No Dropbox: no nav link, notice on `/fuelio` |
| [tests/integration/specs/env/env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts) | Dummy `DROPBOX_*`: the nav link shows |

## Known Limits

- **New Year:** each year is checked alone. A drive that starts on 31 December cannot
  match a trip that starts on 1 January: the drive is "missing" in the old year, and the
  trip has no drive in the new year (`a_drive_and_a_trip_on_both_sides_of_new_year_do_not_match`).
- **Load time:** a page load parses every drive of the year and compares each candidate
  run with the stored route. With full-fix routes (every GPS fix), a long highway trip
  costs more than before. Not measured on the production data.

## Design Decisions

- **Why is Fuelio the reference, not the logbook?** The question is "which highway drive
  is missing in the logbook". A trip without a drive is normal, so listing those would
  only add noise.
- **Why three passes?** The complete pass is strict on places and km and loose on time,
  because a wrong logbook time is what it must find. The partial and loose passes are
  loose on places and km, so they stay inside 3 h of the trip: with 12 h, an evening
  city drive got joined to a morning trip.
- **Why only Dropbox enables the feature?** The operator decides that Fuelio is part of
  the deployment by setting the secrets. A folder that happens to exist in the volume
  does not turn on a page that writes trips.
- **Why ignore per vehicle?** The drives folder is shared, the cross-check runs for one
  vehicle. A drive that is "not a trip of this car" must not hide it for another vehicle.
- **Why does the default view hide ignored rows?** The list must get shorter as the user
  works through it. The "Ignorované" pill shows them, so an ignore can be undone.
- **Why whole minutes and whole km?** The logbook stores both that way; the GPS
  precision does not survive the save anyway, and a whole-km distance follows ADR-054.

## Related

- ADR-008 (backend-only calculations), ADR-054 (whole-km distance writeback), BIZ-027 (Fuelio is the reference), ADR-058 (the Dropbox gate)
- [trip-odometer-cascade.md](./trip-odometer-cascade.md) -- the cascade the writes use
- [route-maps.md](./route-maps.md) -- stored routes, `plan_route_distance`
- [_tasks/90-fuelio-crosscheck/](../../_tasks/90-fuelio-crosscheck/01-task.md) -- planning, measured facts from the production data
