# UI Mockups

Each page of the app has a mockup: one HTML file in this folder that draws the screens of the
page, one frame per state, with English notes under each frame. The files are the source.
All the files are also published as one private claude.ai artifact, with `index.html` as its
start page. There the owner reviews them and sends comments to Claude. The artifact is per owner: a new
artifact is private to the claude.ai account that publishes it, so each person who works on the
repo publishes their own. Its UUID is in the gitignored `.claude/settings.local.json`, not in the
repo ([ADR-064](../../DECISIONS.md#adr-064-one-mockups-artifact-per-owner-its-uuid-outside-the-repo-amends-adr-063)).

A change that the user will see goes into the mockup file first, marked "Návrh (task NN)",
before the plan and the code. The steps are in the `mockup` skill
([SKILL.md](../../.claude/skills/mockup-skill/SKILL.md)). The decision is
[ADR-063](../../DECISIONS.md#adr-063-a-visible-ui-change-goes-into-the-page-mockup-first).

## View a mockup

- **Owner:** open your own artifact. The `mockup` skill shows its UUID, and publishes a new one
  if the checkout has none.
- **Anybody else:** GitHub Pages serves this folder from `main`. Start at the index,
  [mockups/](https://mcsdodo.github.io/kniha-jazd/mockups/) ([index.html](index.html)), which
  links to each page. A file in a clone also opens in a browser. In each viewer, the artifact
  too, the page nav links to the files next to it, and "All mockups" goes back to the index.
- Pages serves `docs/` as plain files, without Jekyll ([docs/.nojekyll](../.nojekyll)).
- **Local server:** use it only if the owner asks for it, or if the artifact does not open
  for the owner. Run `python3 -m http.server 8090 --bind 0.0.0.0 --directory docs/mockups`
  from the repo root, then open `http://<tailscale address of the host>:8090/`. The server
  serves the working tree, so it also shows a change that is not committed.

## Registry

[scripts/mockup-drift.sh](../../scripts/mockup-drift.sh) reads this table. For each row it lists
the commits that changed a source path after the last commit that changed the mockup file.
Keep one row per line, and keep the markers. The table has no artifact link: the artifact
belongs to its owner.

<!-- registry:start -->
| Page | File | Source paths |
|---|---|---|
| Mobil (`/m`) | [mobil.html](mobil.html) | `src/routes/m src/lib/components/mobile` |
| Jazdy (`/`, header) | [jazdy.html](jazdy.html) | `src/routes/+page.svelte src/routes/+layout.svelte src/lib/components/TripGrid.svelte src/lib/components/TripRow.svelte src/lib/components/ColumnVisibilityDropdown.svelte src/lib/components/CompensationBanner.svelte src/lib/components/OdometerCascadeModal.svelte src/lib/components/DatetimeInput.svelte src/lib/components/Autocomplete.svelte src/lib/components/SegmentedToggle.svelte src/lib/components/InvoiceIndicator.svelte src/lib/components/Toast.svelte src/lib/components/ConfirmModal.svelte src/lib/components/GlobalConfirm.svelte src/lib/haReadings.ts src-tauri/core/src/export.rs` |
| Doklady (`/doklady`) | [doklady.html](doklady.html) | `src/routes/doklady src/lib/components/TripSelectorModal.svelte` |
| Fuelio (`/fuelio`) | [fuelio.html](fuelio.html) | `src/routes/fuelio src/lib/components/FuelioAddModal.svelte src/lib/components/FuelioOverwriteModal.svelte src/lib/components/OdometerCascadeModal.svelte` |
| Mapa (`/mapa`) | [mapa.html](mapa.html) | `src/routes/mapa src/lib/components/PlaceModal.svelte src/lib/components/OdometerCascadeModal.svelte` |
| Miesta (`/miesta`) | [miesta.html](miesta.html) | `src/routes/miesta src/lib/components/PlaceModal.svelte` |
| Nastavenia (`/settings`) | [nastavenia.html](nastavenia.html) | `src/routes/settings src/lib/components/VehicleModal.svelte` |
<!-- registry:end -->

## What a mockup holds

- A page header, a link to each other mockup, and a dark / light switch.
- One frame per screen state: the app window with invented data and the real Slovak labels
  from `src/lib/i18n/sk/index.ts`.
- Notes under each frame: what the user sees and does. The trigger, the result, the conditions
  (env vars, settings, vehicle type) and the RPC command.
- A note gives a value (a limit, a distance, a percentage, a timeout) only if the screen shows
  that value, or if its code is in a source path of the page row. Then the drift check sees a
  change to it. A value that only the backend knows is named by its BIZ/ADR id, or by its
  constant if it has no id: "a known place near the GPS point is preselected (BIZ-031)", not
  "within 300 m".
- "Open points": gaps or bugs seen in the code while drawing, with `file:line`. They are not
  fixed by the mockup.

## Mockup notes and feature docs

The mockup notes own the UI behaviour: the labels, the states, the triggers and what the user
sees. The feature docs in [../features/](../features/) own the technical part: the backend
steps, the data flow and the rationale. Do not copy one into the other: link instead.
