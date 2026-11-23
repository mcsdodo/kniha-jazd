# Fuelio Place Field Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** One text field with suggestions for each end of the Fuelio "Pridať" dialog.

**Architecture:** Frontend only. A new component `FuelioPlaceField.svelte` owns the text, the
list and the display lookup. `FuelioAddModal.svelte` resolves the text at submit with the
existing `find_place` RPC. No backend change, so no new backend test.

---

## Task 1: Integration tests first (red)

**Files:** Modify `tests/integration/specs/env/fuelio.spec.ts`

1. Change the three existing add flows to the single field: `fuelio-add-origin` and
   `fuelio-add-destination` are text inputs. A preselect is a non-empty value. No preselect
   is an empty value plus `fuelio-add-destination-no-nearby`. Remove `-new-name` and `__new__`.
2. In the join flow, check that typed unknown text shows `fuelio-add-destination-create` and
   `fuelio-add-destination-new-mark`.
3. Add one flow: type `fuelio NORTH` into the destination. The list shows the option
   "Fuelio North" and no create row. Add the trip. The trip destination is "Fuelio North" and
   `list_places` still has 2 places.

**Verification:** the spec fails on the old dialog.

## Task 2: The field component

**Files:** Create `src/lib/components/FuelioPlaceField.svelte`

- Props: `id`, `testId`, `options` (`FuelioPlaceOption[]`), bindable `text`, `noNearby`,
  `newHint`.
- List on focus: empty text, all options; else substring matches plus the `find_place` hit
  first; the create row when the text is not empty and there is no hit.
- Arrow keys, Enter, Escape, Tab; mousedown on a row picks it without a blur.
- Field adornment: the distance of a known place, or the mark "nové miesto".

## Task 3: Use it in the dialog

**Files:** Modify `src/lib/components/FuelioAddModal.svelte`, `src/lib/i18n/{sk,en}/index.ts`

- The text starts as the name of the preselected place, or empty.
- `proceed()` resolves both texts once (exact option name, else `find_place`, else a new name)
  and passes the result to the dry run and to the commit.
- i18n: add `placeholder`, `createPlace`, `newMark`, `distanceShort`; shorten `noNearby`; remove
  `newPlace` and `newPlaceName`. Run `npm run i18n`.

**Verification:** `npm run check`, `npm run typecheck:tests`, the env spec is green.

## Task 4: Docs and wrap-up

- CHANGELOG `[Unreleased]` (no upgrade notes: no env var, migration or image change).
- After the merge: remove the "Návrh (task 97)" marks in `fuelio.html`, match the labels to the
  code, publish the canvas, run `scripts/mockup-drift.sh`.
