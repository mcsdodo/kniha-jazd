**Date:** 2026-11-23
**Subject:** Fuelio "Pridať" dialog: one place field instead of a select plus a name input
**Status:** Complete

## Goal

In the PC dialog "Pridať jazdu z Fuelio", each end ("Odkiaľ", "Kam") has a `<select>` with
"+ Nové miesto tu" as its first option and, for that option, a second input "Názov nového
miesta". The new-place option sits in the list of real places, and the select shows it as
if it were a place. The user finds this confusing.

Replace the two controls with one text field per end. The field suggests known places and
creates a new place when the text is not a known name.

## Requirements

- Focus on an empty field: the list shows the places nearest to the GPS point, with the distance.
- Typed text filters the list (part of the name, any case). The last row is
  `+ Vytvoriť nové miesto "<text>"`.
- Text that is a known name in another case or without diacritics is that place. Rust
  `find_place` does the match (ADR-008). The list then has no create row.
- Text that is not a known name is a new place. The field shows the mark "nové miesto" and
  the line "Nové miesto sa vytvorí v bode začiatku/konca GPS jazdy.".
- The backend preselect (BIZ-031) fills the field with the place name. No place near the
  point: the field opens empty, with "Žiadne známe miesto nablízku.".
- The RPC payload of `add_fuelio_trip` does not change.

Non-goals: the phone page (`/m/fuelio`, `MobilePlacePicker.svelte`) keeps its own picker.
It is listed as an open point in the mockup. "Účel" already has the purpose autocomplete.

## Design

Frames 5 and 5b of [docs/mockups/fuelio.html](../../../docs/mockups/fuelio.html), marked "Návrh (task 97)".

## Technical Notes

- The text is the only state of a field. `proceed()` resolves it once, to a `placeId` or a
  `newPlaceName`, and the dry run and the commit use the same result (as
  `TripRow.svelte:350-360`). A debounced `find_place` lookup drives only the display (the
  mark and the hidden create row). The submit does not depend on it.
- `preview.origin` / `preview.destination` hold only places with coordinates.
  `find_place` searches the whole place book. `add_fuelio_trip` accepts any place id
  (`TripEnd::Place`, `fuelio_cmd.rs`), so a hit on a place without coordinates is valid; it
  shows without a distance.
- No diacritics fold in TypeScript. The list filter is a plain lower-case substring match,
  plus the `find_place` hit.
- A new runes component. `Autocomplete.svelte` stays as it is: `TripRow` and the phone pages
  use it, and it shows nothing while the field is empty.
- The only test of this dialog is `tests/integration/specs/env/fuelio.spec.ts` (the
  env-pinned suite, `WDIO_ENV_PINNED=1`).
