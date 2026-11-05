# Feature: Phone Pages (/m)

> Phone pages under `/m`: see the trips of a year, open one trip, add a trip from a Fuelio drive, and add a trip by hand.

The PC page copies the layout of the legal export, so it is too wide for a phone. The
phone pages do not replace it. They cover the tasks that happen away from the desk: a
Fuelio drive that just ended, a trip that must go in now, and a look at the book. Edit,
delete, fuel and invoices stay on the PC page.

The pages use the same RPC commands as the PC page where a command exists. Three new
commands give what only the phone needs: the places near a GPS point, the route
estimate of a manual trip, and the origin default of a manual trip.

## User Flow

### 1. Start page (`/m`)

1. Open `/m`. On a narrow screen (640 px or less), the PC header has the link
   **Mobilná verzia** to this page.
2. The bar shows the active vehicle.
3. Three buttons:
   - **Pridať z Fuelio** -- only when the three `DROPBOX_*` secrets are set
     (`is_fuelio_available`).
   - **Pridať ručne**.
   - **Všetky jazdy**, with the count and the km of the selected year.
4. In read-only mode, the two add buttons do not show.
5. Under the buttons, a card shows the year stats of the PC page: the km, the fuel
   (liters and EUR), the fuel remaining, the average consumption and the deviation of
   the year. The vehicle settings row of the PC page (name, plate, tank, TP) is not on
   the phone.
6. With Home Assistant set, the card also shows the real odometer with the km that are
   missing in the book, and the liters to refill. The PC page and the phone use one
   helper for these values, [haReadings.ts](../../src/lib/haReadings.ts).
7. The suggestions of the PC page show too: the compensation banner when the year is
   over the limit, and the suggested fill-up of the open period ("Návrh tankovania",
   `legendSuggestedFillup` of `get_trip_grid_data`).
8. **Otvoriť tabuľku (PC)** opens the PC page.

### 2. All trips (`/m/jazdy`)

1. A year picker loads the trips of one year.
2. A summary card shows the values of the PC page: the consumption of the closed
   periods, the year deviation as % of TP, the km and the fuel remaining. If a period
   between fill-ups is over 120 % of TP, the deviation pill turns orange and a warning
   line shows. The card has no worst-period value ([BIZ-033](../../DECISIONS.md#biz-033-the-app-shows-the-year-deviation-from-the-closed-periods)).
3. The trips show newest first, with a header per month. A card shows the day and
   times, the km, the route, the purpose and the odometer.
4. A card with a fill-up shows the liters and the rate of that fill-up period. A rate
   over the limit gets the pill **nad limitom**. With Paperless set, a fill-up without
   an invoice gets the pill **chýba doklad**.
5. Tap a card to open the detail.

### 3. Trip detail

1. The detail is read-only: the route, a map, the purpose, the odometer at the start
   and at the end, the consumption, the fuel remaining, and the fill-up, charge and
   other costs if the trip has them. With Paperless set, the invoices of the trip show
   too.
2. The map shows the stored route of the trip. Pinch to zoom, drag to move,
   double-tap to zoom in. The scroll wheel does not zoom, so the page still scrolls.
3. **< Jazdy** returns to the list. The phone back gesture also returns to the list,
   because the detail is a history entry.

### 4. Fuelio drives (`/m/fuelio`)

1. The list shows the drives of the current year that have no trip in the book.
2. Ignored drives do not show. Drives under the saved minimum km of the PC Fuelio page
   do not show.
3. The local list shows at once. A Dropbox sync of the year runs at the same time. It
   downloads only the new files. The list loads again only if a file came.
4. The status line shows the sync: "Synchronizujem s Dropbox...", then
   "Synchronizované o HH:MM". If the sync fails, the status line shows the error, and
   the local list stays.
5. Tap **Pridať >** on a drive.

### 5. Add from Fuelio

1. The panel shows the date, the times and the km (the GPS km in whole km).
2. A map shows the GPS track. A green dot marks the start, a red dot marks the end.
   Pinch and drag work.
3. **Odkiaľ** and **Kam** list the places sorted by distance from the GPS start and the
   GPS end. A place within 300 m is selected.
4. If no place is within 300 m, a name field for a new place opens. The new place gets
   the GPS point.
5. **+ Nové miesto tu** is always there, also when a place is selected. A place in a
   city can be close to a different address.
6. Type the purpose. **Uložiť GPS trasu** saves the track as the route of the trip.
7. **Uložiť jazdu** saves the trip. If later trips move, the odometer dialog opens
   first (screen 8).

### 6. Add by hand (`/m/nova`)

1. **Odkiaľ** starts with the destination of the last trip before the end time. A note
   says "Cieľ poslednej jazdy."
2. **Kam** reads the GPS when the page opens.
3. Both use the same picker: an existing place or a new place.
   - **Moja poloha** reads the GPS, sorts the list by distance and selects a place
     within 300 m. If no place is within 300 m, a new place at the GPS point opens.
   - **+ Nové miesto** opens screen 7.
4. **Koniec** starts at now. The user can set another time. **Teraz** sets now again.
5. When both places have a position, the backend computes the route. The box shows
   "Podľa trasy: X km, Y min", and fills:
   - **Začiatok**: the end minus the drive time;
   - **Vzdialenosť (km)**: the route km in whole km.
6. A value that the user types stays. A new route does not change it.
7. If the route fails, the box shows the error, and the user types both values.
8. Type the purpose and press **Uložiť jazdu**. The page has no fuel fields.

### 7. "+ Nové miesto" open

1. Type the name of the new place.
2. Set its position with one of two buttons:
   - **Moja poloha** -- the GPS point;
   - **Hľadať adresu** -- an address search (the geocoder of the Miesta page). Tap one
     result.
3. The panel shows the position: "Poloha: moja poloha GPS (presnosť N m)" or
   "Poloha: <address>". Without a position, it shows "Nové miesto ešte nemá polohu",
   and the save is refused.
4. **Vybrať existujúce miesto** goes back to the list.
5. The place is created when the trip is saved: a GPS position with source `manual`,
   an address with source `geocoder`.

### 8. Later trips move

1. Both add pages first send a dry run. If later trips move, the odometer dialog of the
   PC page opens. Below 600 px it has a phone layout.
2. **Uložiť a posunúť** saves the trip. **Zrušiť** saves no trip.
3. A trip that ends now is usually the last trip, so the dialog does not open.

## Technical Implementation

### Frontend

- **Layout:** [src/routes/+layout.svelte:24](../../src/routes/+layout.svelte) sets
  `isMobile` for `/m` and `/m/...`. The branch at line 130 renders the page without the
  PC header, but keeps the startup (vehicle, read-only mode) and the read-only banner.
  The PC nav link `nav-mobile` is at line 155. It shows only at 640 px or less.
- **Pages:**
  - [src/routes/m/+page.svelte](../../src/routes/m/+page.svelte) -- start page: the buttons, and the year stats from `calculate_trip_stats` and `get_trip_grid_data` (the suggested fill-up) with [CompensationBanner.svelte](../../src/lib/components/CompensationBanner.svelte).
    It calls `getTripsForYear`, `calculateTripStats` and `isFuelioAvailable`.
  - [src/routes/m/jazdy/+page.svelte](../../src/routes/m/jazdy/+page.svelte) -- list and
    detail. It loads `get_trip_grid_data` and `calculate_trip_stats` for the year. The
    detail is a shallow-routing entry: `openDetail()` calls `pushState('', { tripId })`
    (line 100), and `closeDetail()` calls `history.back()`. The Paperless rows load on
    the first detail open, so the list never waits for Paperless.
  - [src/routes/m/fuelio/+page.svelte](../../src/routes/m/fuelio/+page.svelte) -- the
    Fuelio list and the add panel. `syncThenLoad()` (line 63) starts the local load and
    the Dropbox sync together. `load()` filters the crosscheck rows for display only
    (line 100): `missing`, not ignored, GPS km at least the saved `minKm`.
  - [src/routes/m/nova/+page.svelte](../../src/routes/m/nova/+page.svelte) -- the manual
    trip. The effects at line 89 (`getManualTripDefaults`) and line 132
    (`getManualTripEstimate`) follow the end time and the two ends. Only the newest
    request may set a value. `placeId()` (line 191) creates a new place once per name,
    so a second save after a cancelled odometer dialog does not create it again.
- **Components** in [src/lib/components/mobile/](../../src/lib/components/mobile/):
  - [MobilePlacePicker.svelte](../../src/lib/components/mobile/MobilePlacePicker.svelte) -- one end of a trip. Fuelio mode (no `locate`): a new
    place gets the GPS point of the drive. Manual mode (`locate`): `locate_()` (line
    148) reads the phone GPS and calls `getPlacesNear`; `search()` (line 195) calls
    `geocodePlace`; `pickCandidate()` sets a `geocoder` position.
  - [FuelioTrackMap.svelte](../../src/lib/components/mobile/FuelioTrackMap.svelte) -- the GPS track of the drives (`get_fuelio_track`).
  - [TripRouteMap.svelte](../../src/lib/components/mobile/TripRouteMap.svelte) -- the stored route of a trip (`get_trip_route`).
  - [TripCard.svelte](../../src/lib/components/mobile/TripCard.svelte), [TripDetail.svelte](../../src/lib/components/mobile/TripDetail.svelte), [TripLookup.ts](../../src/lib/components/mobile/TripLookup.ts) -- the list card, the detail,
    and the per-trip values from the grid data (no calculation).
  - [MobileBar.svelte](../../src/lib/components/mobile/MobileBar.svelte), [MobileVehicleGate.svelte](../../src/lib/components/mobile/MobileVehicleGate.svelte), [MobileFormat.ts](../../src/lib/components/mobile/MobileFormat.ts) -- the bar, the
    "no vehicle" gate, and number formats.
- **Geolocation** works only in a secure context (HTTPS or localhost). The picker checks
  `window.isSecureContext` first and shows a message without it
  ([MobilePlacePicker.svelte:150](../../src/lib/components/mobile/MobilePlacePicker.svelte)).

### Backend (Rust)

The phone commands are in
[commands_internal/mobile.rs](../../src-tauri/core/src/commands_internal/mobile.rs).
None needs Fuelio.

| Command | Args | Does |
|---------|------|------|
| `get_places_near` | `lat`, `lon` | Every place with a position, nearest first, and `preselectId`: the nearest place within 300 m, or `null`. Sync, [dispatcher.rs:1207](../../src-tauri/core/src/server/dispatcher.rs). |
| `get_manual_trip_defaults` | `vehicleId`, `endDatetime` | `originPlaceId`: the destination of the latest trip of the vehicle that starts before the end. On the same start, the higher odometer counts as later. `null` without such a trip. A bad datetime is an error. Sync, [dispatcher.rs:1217](../../src-tauri/core/src/server/dispatcher.rs). |
| `get_manual_trip_estimate` | per end `originPlaceId` or `originLat` + `originLon` (the same for `destination`), `endDatetime` | Routes from the origin to the destination. Returns `routeKm` (1 decimal), `distanceKm` (whole km, `logbook_km`), `durationMin`, `startDatetime` (the end minus the drive time) and `error`. A failed route or a place without a position is not an error: the numbers are `null` and `error` says why. An unknown place id or a bad datetime is an error. Async, [dispatcher_async.rs:346](../../src-tauri/core/src/server/dispatcher_async.rs). |

The 300 m radius is `PLACE_PRESELECT_RADIUS_M` in
[fuelio_cmd.rs:407](../../src-tauri/core/src/commands_internal/fuelio_cmd.rs).
`get_places_near` reuses `places_by_distance` and `preselect_id` of the Fuelio add, so
the phone and the Fuelio dialog use one rule.

**Changed: `add_fuelio_trip`** ([dispatcher.rs:1164](../../src-tauri/core/src/server/dispatcher.rs)).
Each end takes a place id or the name of a new place: `originPlaceId` or
`newOriginName`, `destinationPlaceId` or `newDestinationName`. Exactly one per end
(`TripEnd::from_args`). See [fuelio.md](./fuelio.md) for the write.

The manual trip has no new write command. The page calls `create_place` for a new end,
then `create_trip_cascade` with a dry run first, as the PC page does.

### Data Flow

```
/m/nova
  open  -> get_manual_trip_defaults(vehicle, end)      -> Odkiaľ = last destination
        -> phone GPS -> get_places_near(lat, lon)      -> Kam = place within 300 m,
                                                          or a new place at the point
  both ends known (place id or point)
        -> get_manual_trip_estimate(ends, end)         -> Začiatok, km (if not typed)
  Uložiť jazdu
        -> create_place (each new end, once)
        -> create_trip_cascade(dryRun)                 -> later trips move?
             yes -> odometer dialog -> create_trip_cascade
             no  -> create_trip_cascade

/m/fuelio
  open  -> get_fuelio_crosscheck (local)  |  sync_fuelio_dropbox(year)
                                           +-> downloaded > 0 -> get_fuelio_crosscheck
  Pridať -> get_fuelio_add_preview -> get_fuelio_track (map)
  Uložiť -> add_fuelio_trip(dryRun) -> odometer dialog? -> add_fuelio_trip
```

## Key Files

| File | Purpose |
|------|---------|
| [src/routes/+layout.svelte](../../src/routes/+layout.svelte) | The `/m` branch without the PC header; the `nav-mobile` link |
| [src/lib/haReadings.ts](../../src/lib/haReadings.ts) | The Home Assistant readings next to the year stats, for the PC page and the phone start page |
| [src/routes/m/](../../src/routes/m/) | The four phone pages |
| [src/lib/components/mobile/](../../src/lib/components/mobile/) | Place picker, maps, trip card and detail, bar, vehicle gate |
| [OdometerCascadeModal.svelte](../../src/lib/components/OdometerCascadeModal.svelte) | The odometer dialog; a `max-width: 600px` block for the phone |
| [commands_internal/mobile.rs](../../src-tauri/core/src/commands_internal/mobile.rs) | `get_places_near`, `get_manual_trip_defaults`, `get_manual_trip_estimate` |
| [commands_internal/fuelio_cmd.rs](../../src-tauri/core/src/commands_internal/fuelio_cmd.rs) | `PLACE_PRESELECT_RADIUS_M`, `places_by_distance`, `TripEnd`, the new places of the Fuelio add |
| [server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) | Sync arms |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | The `get_manual_trip_estimate` arm |
| [src/lib/api.ts](../../src/lib/api.ts) / [src/lib/types.ts](../../src/lib/types.ts) | `getPlacesNear`, `getManualTripDefaults`, `getManualTripEstimate`, `addFuelioTrip`; `NearbyPlaces`, `ManualTripDefaults`, `ManualTripEstimate`, `FuelioAddEnd` |

## Design Decisions

- **Why separate pages under `/m`, not a responsive PC page?** The PC page copies the
  columns of the legal export. A phone needs fewer fields and another order. Separate
  pages keep the PC page as it is, and use the same commands.
- **Why 300 m for the preselect, not the 2 km of the matcher?** In a city, 2 km covers
  many addresses. A wrong preselect is worse than an empty field. The matcher radius is
  a different rule. See [BIZ-031](../../DECISIONS.md#biz-031-a-new-trip-preselects-a-place-only-within-300-m-and-a-new-place-is-always-offered).
- **Why is "+ Nové miesto" always there?** A place within 300 m can still be the wrong
  address. The user decides.
- **Why does "Odkiaľ" start with the last destination?** A trip usually starts where the
  one before it ended (the odometer chain). The backend picks it (ADR-008). See
  [BIZ-032](../../DECISIONS.md#biz-032-the-phone-manual-entry-fills-the-origin-the-start-and-the-km).
- **Why does the route fill the start and the km?** On the phone, the user knows the end
  (now) and the two places. The route gives the rest. The user can change both values,
  and a typed value stays. The km is in whole km, as a routed trip gets it (ADR-054).
- **Why does a new place need a position?** Without a position, there is no route, and
  the start and the km must be typed. The position comes from the GPS or from a tapped
  address result, so a human confirms it (ADR-032).
- **Why no fuel fields?** Fuel comes from Paperless invoices on the PC page.
- **Why does the Fuelio list show the local drives first?** The sync can take seconds. A
  drive that is already local must not wait for Dropbox.

## Testing

- **Backend unit tests:**
  - [mobile_tests.rs](../../src-tauri/core/src/commands_internal/mobile_tests.rs) -- the
    300 m preselect, the estimate (place to point, failed route, place without a
    position, unknown place, bad end), the start time across midnight, and the origin
    default (latest earlier trip, same start, other vehicles).
  - [fuelio_cmd_tests.rs](../../src-tauri/core/src/commands_internal/fuelio_cmd_tests.rs)
    -- the add preview preselect, and the new places of the Fuelio add (dry run, same
    name twice, GPS ends, failed insert).
  - [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) and
    [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) -- the
    argument parse of the new commands.
- **Integration tests:** [mobile-home-stats.spec.ts](../../tests/integration/specs/tier2/mobile-home-stats.spec.ts) checks the stats card, the compensation banner and the suggested fill-up on `/m`. The specs for the other `/m` flows are not written yet.
  [fuelio.spec.ts](../../tests/integration/specs/env/fuelio.spec.ts) types a new place
  name in the PC Fuelio dialog. The spec is changed for the 300 m preselect, but it was
  not run after the change.

## Known Limits

- **Current year only** in the phone Fuelio list. An older drive needs the PC page.
- **A new place stays** if the user cancels the odometer dialog on the manual page. The
  page creates the place before the dry run. A second save uses the same place.
- **Geolocation needs HTTPS.** Over plain HTTP (not localhost), "Moja poloha" shows a
  message, and the user picks from the list or uses the address search.
- **Not tested on a real phone:** the back gesture from the detail, and the odometer
  dialog at phone width. A headless browser checked the pages.

## Related

- [fuelio.md](./fuelio.md) -- the Fuelio cross-check and the add write
- [place-book.md](./place-book.md) -- places, sources, the geocoder
- [trip-odometer-cascade.md](./trip-odometer-cascade.md) -- the odometer dialog
- [route-maps.md](./route-maps.md) -- stored routes, the route provider
- ADR-008 (backend-only calculations), ADR-032 (a human places a place), ADR-054 (whole-km distance)
- [BIZ-031](../../DECISIONS.md#biz-031-a-new-trip-preselects-a-place-only-within-300-m-and-a-new-place-is-always-offered), [BIZ-032](../../DECISIONS.md#biz-032-the-phone-manual-entry-fills-the-origin-the-start-and-the-km)
- [_tasks/93-mobile-ui/](../../_tasks/93-mobile-ui/01-task.md) -- planning
