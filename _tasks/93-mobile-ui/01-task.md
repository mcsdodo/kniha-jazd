**Date:** 2026-11-05
**Subject:** Phone pages under `/m`: the trip list, add from Fuelio, add by hand
**Status:** In Progress -- implemented on branch `feat/93-mobile-ui`, in review on the local compose. Not merged.

## Goal

The PC page copies the layout of the legal export. It is not usable on a phone. The user
must be able to do the basic tasks on a phone:

1. **Add a trip from a Fuelio drive.** The main case: the user is in the car, Fuelio
   has uploaded the drive, and the user adds it at once.
2. **Add a trip by hand**, with the places from the phone GPS.
3. **See all trips** in a list for the phone, with a read-only detail on tap.

## Decisions from the brainstorm

| Topic | Decision | Who |
|-------|----------|-----|
| Where | Separate pages under `/m`, with a link from the PC header on a narrow screen. The PC page does not change. | Claude, accepted |
| Place preselect | A place within 300 m of the GPS point is selected. | Claude (the user did not answer) |
| New place | "+ Nové miesto" is always offered, also when a place matched. The user types the name; the position comes from the GPS point. | User |
| Fuel fields | None on the phone. Invoices are assigned on the PC from Paperless. | User |
| Manual entry | "Odkiaľ" first, then "Kam". Each is an existing place or a new place. A new place gets its position from "Moja poloha" or from an address search. | User (second design) |
| Manual end time | Starts at now, and the user can change it. | User |
| Manual start and km | The route fills them. The user can change them. | Claude (the user did not object) |
| Manual origin | Starts with the destination of the last trip before the end time. | Claude |
| Trip detail | Read-only, with a zoomable route map. | User |
| Fuelio add | Shows the GPS track on a map. | User |
| Dropbox sync | The local list shows at once; the sync downloads only new files and does not hide the list. | User |

The business rules are [BIZ-031](../../DECISIONS.md#biz-031-a-new-trip-preselects-a-place-only-within-300-m-and-a-new-place-is-always-offered)
and [BIZ-032](../../DECISIONS.md#biz-032-the-phone-manual-entry-fills-the-origin-the-start-and-the-km).
The feature doc is [docs/features/mobile.md](../../docs/features/mobile.md). The screen
mockups of the current state are in a private claude.ai artifact of the user.

## Backend

New RPC commands, all in [mobile.rs](../../src-tauri/core/src/commands_internal/mobile.rs):

- `get_places_near` -- the places sorted by distance from a point, and the preselect.
- `get_manual_trip_estimate` -- the route between two ends (a place or a point): the
  route km, the trip km, the drive time and the start time.
- `get_manual_trip_defaults` -- the origin default.

Changed: `add_fuelio_trip` takes a new place name per end and creates the place at the
GPS point ([fuelio_cmd.rs](../../src-tauri/core/src/commands_internal/fuelio_cmd.rs)).
No migration, no env var, no change to the image.

## Done

- [x] Backend commands and tests ([mobile_tests.rs](../../src-tauri/core/src/commands_internal/mobile_tests.rs), [fuelio_cmd_tests.rs](../../src-tauri/core/src/commands_internal/fuelio_cmd_tests.rs))
- [x] Phone pages: start page, trip list and detail, Fuelio list and add, manual add
- [x] "+ Nové miesto tu" and the 300 m preselect in the PC Fuelio dialog
- [x] CHANGELOG, README sk/en, DECISIONS (BIZ-031, BIZ-032), feature doc

## Open

- [ ] WebdriverIO specs for the `/m` flows
- [ ] Run the changed [fuelio.spec.ts](../../tests/integration/specs/env/fuelio.spec.ts) (it now types a new
      place name, because the PC dialog no longer selects a far place)
- [ ] Test on a real phone: the GPS, the back gesture in the detail, the odometer
      dialog at phone width
- [ ] Merge to `main`

## Known limits

- The phone Fuelio list shows only the current year.
- If the user cancels the odometer dialog, a new place that the manual page made stays
  in the place book.
- The GPS position needs HTTPS (or localhost).
