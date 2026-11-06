# UI Mockups

Each page of the app has a mockup: one HTML file in this folder that draws the screens of the
page, one frame per state, with English notes under each frame. The files are the source.
Each file is also published as a private claude.ai artifact, where the owner reviews it and
sends comments to Claude.

A change that the user will see goes into the mockup file first, marked "Návrh (task NN)",
before the plan and the code. The steps are in the `mockup` skill
([SKILL.md](../../.claude/skills/mockup-skill/SKILL.md)). The decision is
[ADR-063](../../DECISIONS.md#adr-063-a-visible-ui-change-goes-into-the-page-mockup-first).

## View a mockup

- **Owner:** open the artifact link in the table.
- **Anybody else:** GitHub Pages serves this folder from `main`:
  `https://mcsdodo.github.io/kniha-jazd/mockups/<file>`, for example
  [jazdy.html](https://mcsdodo.github.io/kniha-jazd/mockups/jazdy.html). A file in a clone also
  opens in a browser. In both cases the page nav links to the files next to it.
- Pages serves `docs/` as plain files, without Jekyll ([docs/.nojekyll](../.nojekyll)).

## Registry

[scripts/mockup-drift.sh](../../scripts/mockup-drift.sh) reads this table. For each row it lists
the commits that changed a source path after the last commit that changed the mockup file.
Keep one row per line, and keep the markers. Use the UUID for the `claude-artifacts` tools:
they refuse the short id of the link.

<!-- registry:start -->
| Page | File | Artifact | UUID | Source paths |
|---|---|---|---|---|
| Mobil (`/m`) | [mobil.html](mobil.html) | [Kniha jázd na mobile](https://claude.ai/artifact/TyYTKii3cU4ryvN98giUGW) | `da72a3f1-189b-4dcc-9329-2a83cebceb07` | `src/routes/m src/lib/components/mobile` |
| Jazdy (`/`, header) | [jazdy.html](jazdy.html) | [Kniha jázd PC: Jazdy](https://claude.ai/artifact/PCbcTRe6EybiezMCqH8rm2) | `b3c78fdc-d50c-4ab8-ad08-d429b649a9b5` | `src/routes/+page.svelte src/routes/+layout.svelte src/lib/components/TripGrid.svelte src/lib/components/TripRow.svelte src/lib/components/ColumnVisibilityDropdown.svelte src/lib/components/CompensationBanner.svelte src/lib/components/OdometerCascadeModal.svelte src/lib/components/DatetimeInput.svelte src/lib/components/Autocomplete.svelte src/lib/components/SegmentedToggle.svelte src/lib/components/InvoiceIndicator.svelte src/lib/components/Toast.svelte src/lib/components/ConfirmModal.svelte src/lib/components/GlobalConfirm.svelte src/lib/haReadings.ts src-tauri/core/src/export.rs` |
| Doklady (`/doklady`) | [doklady.html](doklady.html) | [Kniha jázd PC: Doklady](https://claude.ai/artifact/Nvtcv8rofbFtVG4BNbAoMr) | `b19624a6-101f-4676-87e7-99f7716a8fd9` | `src/routes/doklady src/lib/components/TripSelectorModal.svelte` |
| Fuelio (`/fuelio`) | [fuelio.html](fuelio.html) | [Kniha jázd PC: Fuelio](https://claude.ai/artifact/RgyA56LvfFjRHpgMcYG3ku) | `c7eff145-f9e3-43c5-ba4a-0afe5d028be2` | `src/routes/fuelio src/lib/components/FuelioAddModal.svelte src/lib/components/FuelioOverwriteModal.svelte src/lib/components/OdometerCascadeModal.svelte` |
| Mapa (`/mapa`) | [mapa.html](mapa.html) | [Kniha jázd PC: Mapa](https://claude.ai/artifact/F3fXHbFEaERHHcKC4tr215) | `71bf2b3d-da4d-4b78-89e6-5588312efea0` | `src/routes/mapa src/lib/components/PlaceModal.svelte src/lib/components/OdometerCascadeModal.svelte` |
| Miesta (`/miesta`) | [miesta.html](miesta.html) | [Kniha jázd PC: Miesta](https://claude.ai/artifact/T2jiZ3ZLGs9s1F89UZP9eE) | `d2cbac82-1a44-412b-82e7-5b3f49ce6c5f` | `src/routes/miesta src/lib/components/PlaceModal.svelte` |
| Nastavenia (`/settings`) | [nastavenia.html](nastavenia.html) | [Kniha jázd PC: Nastavenia](https://claude.ai/artifact/F5fCz7wgrunXAyQ5rRw1AJ) | `72067645-50f0-4bec-97b9-cbeaff7e902b` | `src/routes/settings src/lib/components/VehicleModal.svelte` |
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
