# Task 90: Fuelio Cross-Check (POC)

**Date:** 2026-10-06
**Subject:** Read-only cross-check of Fuelio GPS drives against the logbook
**Status:** In Progress

## Goal

Compare the drives that the Fuelio Android app recorded with the trips in the
logbook. The main interest is the long (highway) drives. The logbook must have
each of them, with correct times and the correct route.

## Scope of the POC

Read-only. Nothing writes to the logbook.

- List the Fuelio drives, with a filter by length.
- Match each drive (or a run of drives split by a stop) to a logbook trip.
- Detect a drive with no trip ("missing in the logbook").
- Detect a matched trip whose stored route (`trip_routes`) is not the road
  that the GPS track shows ("different route").
- Show the time and km differences of a matched trip.
- Show the GPS track next to the stored route on a map.

Out of scope (later): add a trip from a drive, an automatic Google Drive sync
inside the app.

**Overwrite (added 2026-10-06).** A matched row has a "Prepísať" button. A
popup selects which fields the GPS values replace: start, end, distance,
route. `apply_fuelio_to_trip { tripId, driveIds, fields, dryRun }`:

- start / end: the GPS start and end in whole minutes. A new start that moves
  the trip past another trip is refused (the order is the odometer chain).
- distance: the GPS km in whole km, through `plan_route_distance` (task 87),
  so the later odometers move and the margin impact is in the plan. The page
  shows `OdometerCascadeModal` (kind `writeback`) when the distance changes.
- route: the GPS track becomes the stored route (`direct`, no provider,
  waypoints at the GPS start and end).
- Everything selected is written in one transaction; read-only mode blocks
  the write, not the dry run. A complete match preselects all fields; a
  partial or loose match preselects none and shows a warning.

## Data source

Fuelio backs up each drive to Google Drive (`Android/Fuelio/routes`) as two
files with the same ID (`route-<epoch ms>`):

| File | Content |
|------|---------|
| `route-<id>.route` | Base64 of a Google encoded polyline5 (a simplified track) |
| `route-<id>.data` | A zip with one CSV, `route-<id>.csv`, one GPS fix per row |

The CSV has no header. Columns, verified on real files:

| # | Field | Unit |
|---|-------|------|
| 1 | timestamp | epoch ms, UTC |
| 2 | latitude | degrees |
| 3 | longitude | degrees |
| 4 | distance from the previous fix | m (matches haversine) |
| 5 | speed | m/s |
| 6 | altitude | m |
| 7 | accuracy | m |

Drive contains duplicates of the same file. The scanner keeps one drive per ID.

Something outside the app (rclone, Syncthing) copies the folder to
`<KNIHA_JAZD_DATA_DIR>/fuelio`. The app only reads that folder.

## Facts from the production data (2026-10-06)

- 2026-09-28: the logbook trip SNV -> Bratislava (04:25, 357 km) is three Fuelio
  drives (04:30, 05:35, 06:51). One trip can be several drives.
- The same day, Bratislava -> Brno (290 km) has no drive. A trip with no drive
  is normal (Fuelio did not record).
- Bratislava -> OMV Zamarovce is 17:00 in the logbook and 17:29 in Fuelio. The
  next trip starts at the same OMV 7 minutes after the drive ends. So a drive
  chain must not join over a logbook endpoint.

## Design (approach A, approved 2026-10-06)

**Fuelio is the reference (changed 2026-10-06).** Every row is a run of
Fuelio drives. A logbook trip without a drive is no problem (Fuelio does not
record every drive), so it has no row. The problem to find is a Fuelio
highway drive that the logbook does not have, so the matching can be loose.

All logic is in a new core module [fuelio](../../src-tauri/core/src/fuelio/)
(ADR-008). The page only renders the result.

1. **Scan.** Read every `route-<id>.data` in the folder, one per ID. Unzip,
   parse the CSV, and compute: local start and end (Europe/Bratislava), GPS
   km, minutes above 100 km/h, max speed, start and end point, and a thinned
   track.
2. **Match per trip.** For each trip of the vehicle and year, find the run of
   consecutive drives that:
   - starts within 2 km of the origin place and ends within 2 km of the
     destination place,
   - has gaps of at most 90 minutes between the drives,
   - starts within 12 hours of the logbook start,
   - has a GPS km between 50% and 150% of the trip km.

   A round trip (the stored route has `round_trip`, or the origin is the
   destination) must end back at the origin, and its stop at the turnaround
   can last the whole trip (`end_datetime - start_datetime`, or 12 hours
   without an end time). The turnaround point is not checked. Example:
   2026-08-27 SNV -> Poprad -> SNV, two drives with a 3-hour stop.

   If more than one run fits, the one with the start time nearest to the
   logbook time wins. A drive belongs to one trip only.
3. **Partial match.** A second pass, after all complete matches, for the
   trips left without drives. A run matches part of a trip if: the trip has
   a stored route, >= 80% of the track is within 500 m of it, the drives
   start between 12 hours before the trip start and 12 hours after the trip
   end, the run starts or ends near one of the trip's places, and its km is
   at most 150% of the trip km. The run with the most km wins. The row gets
   the flag "partial GPS" instead of "km differs", and a start time
   difference only if the run starts at the origin. Example: 2026-08-18
   BA -> Brno -> BA, Fuelio recorded only the way back.
4. **Loose match.** Drives that no trip uses are joined into chains (gap of
   at most 60 minutes, next start within 2 km of the previous end). A chain
   matches the nearest trip in time if: it starts between 12 hours before the
   trip start and 12 hours after the trip end; a chain end is within 5 km of
   a trip place, or >= 50% of the chain is within 1 km of the stored route;
   and the trip's GPS km stays at most 150% of its km (so a second full drive
   of the same route is still missing). Flag "loose match".
   A chain that fits no trip is a "missing in the logbook" row.
5. **Highway flag.** A row is a highway row if GPS km >= 30 or minutes above
   100 km/h >= 5. The page filters by min km and by "highway only".
6. **Route check.** For a matched trip with a stored route: the share of track
   points more than 500 m from the stored polyline. More than 10% is
   "different route".
7. **Flags on a match:** start time differs by more than 30 minutes; km
   differs by more than 10%.

RPC: `get_fuelio_crosscheck { vehicleId, year }` returns the rows, and
`get_fuelio_track { driveIds, tripId? }` returns the GPS track and the
stored route for the map.
