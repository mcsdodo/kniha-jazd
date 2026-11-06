**Date:** 2026-11-06
**Subject:** A places editor on the phone (`/m/miesta`)
**Status:** Planning

## Goal

Give the phone pages a list of the places with a search box, and a screen to edit one place.
The user can change the name, the position and the home mark, delete an unused place, and
add a new place, without the PC page `/miesta`.

## Requirements

1. The start page `/m` has a new button "Miesta" with the hint "Názov, poloha a domov". It
   opens `/m/miesta`. The button shows also in read-only mode.
2. `/m/miesta` shows one card for each place, in the order of `list_places` (the most used
   first, then by name). The page does not sort.
3. A card shows the name, the number of uses and the position with 5 decimals. The home
   place has the house icon.
4. No "{n}/{total} umiestnených" counter and no warning sign for a place without a position.
   Since task 88, `create_place_internal` requires a position, and the live data (58 places on
   2026-11-06) has no place without one. A place without a position shows "-" as its position.
5. "Hľadať..." filters the cards by name, the same as the PC page: an ASCII query also
   matches `normalisedName` ("kosice" finds "Košice").
6. A tap on a card opens the edit screen. "Pridať miesto" opens the same screen for a new
   place. The phone back gesture closes the screen (a shallow-routing history entry, the same
   as the trip detail of `/m/jazdy`).
7. The edit screen has:
   - "Názov miesta" (the name).
   - "Poloha": a search box with "Hľadať" (`geocode_place`) and a list of the results. For an
     existing place, the search box starts with the place name. For a new place, the search
     box copies the name until the user changes the search box.
   - A map with the pin. A tap on the map puts the pin there. The user can drag the pin. A
     tap or a drag gives the source `manual`. A search result gives the source `geocoder`.
   - "Moja poloha": the GPS point of the phone, source `manual`.
   - "Označiť ako domov" (a checkbox).
   - "Uložiť miesto". Nothing is written before this tap (ADR-032). The stored name is the
     name in the field, never the address (ADR-034).
   - "Zmazať", only for an existing place. It is disabled if a trip uses the place, and the
     line "Miesto je použité {n}-krát." shows under it. It asks "Zmazať miesto?" first.
8. In read-only mode, "Pridať miesto" is hidden, and the edit screen has no "Uložiť miesto"
   and no "Zmazať".

## Design

Frames 1, 10, 11 and 12 of [docs/mockups/mobil.html](../../docs/mockups/mobil.html), marked
"Návrh (task 96)". The user approved them on 2026-11-06 (one comment: 5 decimals, applied).

## Technical Notes

- No backend change. The page uses the existing commands: `list_places`, `geocode_place`,
  `create_place`, `rename_place`, `set_place_position`, `set_home_place`, `delete_place`.
- "Uložiť miesto" calls the commands in sequence from the page, not one atomic command. The
  approved mockup names these commands, and the order of writes is not a calculation
  (ADR-008). The rules for a failure are in the plan.
- [PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte) does not change. It is a source path of the Miesta and Mapa mockups,
  so a change to it makes [scripts/mockup-drift.sh](../../scripts/mockup-drift.sh) list both pages. The phone gets its own
  pin map in [src/lib/components/mobile/](../../src/lib/components/mobile/).
- The GPS errors use the existing labels `mobile.add.picker.{insecure,denied,timeout,unavailable}`.
