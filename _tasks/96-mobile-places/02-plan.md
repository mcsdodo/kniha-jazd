**Date:** 2026-11-06
**Subject:** A places editor on the phone (`/m/miesta`)
**Status:** Planning

# Mobile Places Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A phone page `/m/miesta` with a filtered list of the places and an edit screen
(name, position by search, map or GPS, home mark, delete, new place).

**Architecture:** Frontend only. One route holds the list. The edit screen opens on top of it
through a shallow-routing history entry, the same as the trip detail of `/m/jazdy`. A new
mobile pin map component draws the pin. The page calls the existing place commands. No Rust
change.

**Tech Stack:** SvelteKit 2 + Svelte 5 runes, TypeScript, Leaflet, typesafe-i18n, WebdriverIO.

**Spec:** [01-task.md](01-task.md) and screens 1, 10, 11 and 12 of
[docs/mockups/mobil.html](../../docs/mockups/mobil.html) ("Návrh (task 96)").

## Global Constraints

- All business logic stays in Rust (ADR-008). The page does not sort, and it does not fold
  diacritics: the filter matches `name.toLowerCase()` or `normalisedName`, as on
  [src/routes/miesta/+page.svelte](../../src/routes/miesta/+page.svelte).
- No write before "Uložiť miesto" (ADR-032). The stored name is the name field, never a
  geocoder label (ADR-034).
- Do not change [PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte),
  [MobilePlacePicker.svelte](../../src/lib/components/mobile/MobilePlacePicker.svelte) or any
  file outside the Mobil mockup row, except the i18n files and the docs.
- Every UI string goes through i18n, in `sk` and `en`. Run `npm run i18n` after each edit of
  the i18n files.
- Position text: 5 decimals, `"{lat.toFixed(5)}, {lon.toFixed(5)}"`. No position: `-`.
- Touch targets have a minimum height of 44px, as on the other `/m` pages.
- Work on the branch `feat/96-mobile-places` (a `.worktrees/` checkout), not on `main`.

## Review Focus

1. **Rename to a name that another place has.** The backend refuses it. Expect the error
   toast, the edit screen stays open, and the position and home mark stay unchanged in the
   database. Test in Task 2.
2. **A second tap on "Uložiť miesto" after a partial failure.** Expect the same end state as
   one good save. The edit path is safe to repeat: a rename to its own name passes
   `ensure_key_free(..., Some(&id))`, and the position and home writes overwrite. Compare
   against a fresh `list_places` row, not against the snapshot from when the screen opened.
3. **`create_place` succeeds and `set_home_place` fails.** A second tap would fail with the
   duplicate-name error. Expect the page to leave the new-place screen after a successful
   create, go back to the list, and show the home error toast. No test (the failure needs a
   broken backend); the rule is in Task 3, Step 3.
4. **The phone back gesture on the edit screen.** Expect the list, with no write. Test in
   Task 2.
5. **An empty or blank name.** Expect "Uložiť miesto" disabled. Test in Task 3.

---

### Task 1: The list page and the start page button

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts) (new `mobile.places` section)
- Modify: [src/lib/components/mobile/MobileFormat.ts](../../src/lib/components/mobile/MobileFormat.ts)
- Modify: [src/routes/m/+page.svelte](../../src/routes/m/+page.svelte)
- Create: [src/routes/m/miesta/+page.svelte](../../src/routes/m/miesta/+page.svelte)
- Test: [tests/integration/specs/tier2/mobile-places.spec.ts](../../tests/integration/specs/tier2/mobile-places.spec.ts)

**Interfaces:**
- Produces: `coords5(lat: number | null, lon: number | null): string` in `MobileFormat.ts`
  (returns `-` if one value is null).
- Produces: the i18n keys under `mobile.places` (sk / en):
  - `homeButton`: `Miesta` / `Places`
  - `homeHint`: `Názov, poloha a domov` / `Name, position and home`
  - `back`: `Späť` / `Back`
  - `editTitle`: `Upraviť miesto` / `Edit place`
  - `position`: `Poloha` / `Position`
  - `mapHint`: `Ťuknite na mapu alebo posuňte špendlík.` / `Tap the map or drag the pin.`
  - `positionManual`: `Poloha: {coords:string} (ručne)` / `Position: {coords:string} (manual)`
  - The page reuses `places.title`, `places.filterPlaceholder`, `places.add`, `places.addName`,
    `places.uses`, `places.empty`, `places.noMatches`, `places.search`, `places.searching`,
    `places.noResults`, `places.searchError`, `places.save`, `places.saved`, `places.markHome`,
    `places.delete`, `places.deleteInUse`, `places.deleteConfirmTitle`,
    `places.deleteConfirmMessage`, and the `mobile.add.picker` GPS keys.
- Produces test ids: `mobile-home-places` (start button), `m-miesta` (page root),
  `m-miesta-filter`, `m-miesta-add`, `m-miesta-card` (attribute `data-place-id`),
  `m-miesta-card-name`, `m-miesta-card-uses`, `m-miesta-card-coords`, `m-miesta-card-home`,
  `m-miesta-empty`, `m-miesta-no-matches`.

- [ ] **Step 1: Write the failing spec.** Seed with `ensurePlace` and `seedTrip` from
  [tests/integration/utils/db.ts](../../tests/integration/utils/db.ts). `ensurePlace` creates
  at `48.15, 17.11`. Copy the `SPEC_PLACES` clean-up pattern of
  [places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts).

```ts
describe('Tier 2: Mobile places (/m/miesta)', () => {
  it('opens the list from the start page', async () => {
    await ensurePlace('Alpha Depot, Testville');
    await browser.url('/m');
    await (await $('[data-testid="mobile-home-places"]')).click();
    await $('[data-testid="m-miesta"]').waitForDisplayed({ timeout: 10000 });
    expect(await browser.getUrl()).toContain('/m/miesta');
    const card = await cardByName('Alpha Depot, Testville');
    expect(await card.$('[data-testid="m-miesta-card-coords"]').getText()).toBe('48.15000, 17.11000');
  });

  it('finds a name with diacritics from an ASCII query', async () => {
    await ensurePlace('Košice Sklad');
    await ensurePlace('Lambda Gate, Testville');
    await browser.url('/m/miesta');
    await (await $('[data-testid="m-miesta-filter"]')).setValue('kosice');
    const names = await $$('[data-testid="m-miesta-card-name"]').map((n) => n.getText());
    expect(names).toEqual(['Košice Sklad']);
  });
});
```

  `cardByName(name)` is a local helper: it waits for the card whose `m-miesta-card-name` text
  equals `name` and returns the card.

- [ ] **Step 2: Run it and see it fail.**
  `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web && npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier2/mobile-places.spec.ts`
  Expected: FAIL, `mobile-home-places` not found.

- [ ] **Step 3: Implement.**
  - Add the i18n keys, then `npm run i18n`.
  - Add `coords5` to `MobileFormat.ts`.
  - Start page: a `<a class="big alt" href="/m/miesta" data-testid="mobile-home-places">`
    after "Všetky jazdy", outside the `{#if !$appModeStore.isReadOnly}` block.
  - `/m/miesta`: `MobileBar` with `title={$LL.places.title()}`, `backHref="/m"`,
    `backLabel={$LL.mobile.places.back()}`. Not wrapped in `MobileVehicleGate`: places belong to
    the whole database, not to a vehicle. Load with `listPlaces()` on mount. Render in the
    order received. The filter is the `visiblePlaces` rule of the PC page. The house icon is
    the SVG of the PC page. Hide "Pridať miesto" in read-only mode.

- [ ] **Step 4: Run the spec and see it pass.** Same command. Expected: 2 passing.

- [ ] **Step 5: Commit.** `git add` the files above, then
  `git commit -m "feat(mobile): place list on /m/miesta (task 96)"`.

---

### Task 2: The pin map and the edit screen of an existing place

**Files:**
- Create: [src/lib/components/mobile/PlacePinMap.svelte](../../src/lib/components/mobile/PlacePinMap.svelte)
- Create: [src/lib/components/mobile/PlaceEditor.svelte](../../src/lib/components/mobile/PlaceEditor.svelte)
- Modify: [src/routes/m/miesta/+page.svelte](../../src/routes/m/miesta/+page.svelte)
- Test: [tests/integration/specs/tier2/mobile-places.spec.ts](../../tests/integration/specs/tier2/mobile-places.spec.ts)

**Interfaces:**
- Consumes: `coords5`, the `mobile.places` keys (Task 1).
- Produces `PlacePinMap` props:
  `{ position: { lat: number; lon: number } | null; onpick: (lat: number, lon: number) => void }`.
  Copy the Leaflet setup of [PlaceModal.svelte](../../src/lib/components/PlaceModal.svelte)
  (lazy import, the `place-pin` div icon, a draggable marker, a click on the map calls
  `onpick`), with `scrollWheelZoom: false`. The start view is the position at zoom 15, or
  `[48.7, 19.7]` at zoom 7. When `position` changes from outside (search, GPS), the map pans
  to it at zoom 15. Root test id: `m-place-map`.
- Produces `PlaceEditor` props:
  `{ place: Place | null; readOnly: boolean; onsave: (draft: PlaceDraft) => Promise<void>; ondelete: () => void }`,
  with `type PlaceDraft = { name: string; lat: number; lon: number; source: PlaceSource; isHome: boolean }`
  exported from the component module script. `place === null` is a new place.
- Produces test ids: `m-place-editor`, `m-place-name`, `m-place-search`, `m-place-search-btn`,
  `m-place-candidate`, `m-place-gps`, `m-place-gps-error`, `m-place-position`, `m-place-home`,
  `m-place-save`, `m-place-delete`, `m-place-in-use`.
- Open and close: the page calls `pushState('', { placeId })` (or `{ newPlace: true }` in
  Task 3), and reads `page.state`, the same as `openDetail` in
  [src/routes/m/jazdy/+page.svelte](../../src/routes/m/jazdy/+page.svelte). The editor bar has
  `onback={() => history.back()}`, label `places.title`.

- [ ] **Step 1: Write the failing tests.**

```ts
it('saves a renamed place at a geocoder result', async () => {
  const id = await ensurePlace('Beta Shed, Testville');
  await openEditor(id);
  await (await $('[data-testid="m-place-name"]')).setValue('Beta Shed North, Testville');
  await (await $('[data-testid="m-place-search"]')).setValue('Gamma Warehouse, Testville');
  await (await $('[data-testid="m-place-search-btn"]')).click();
  await (await $$('[data-testid="m-place-candidate"]')[0]).click();
  await (await $('[data-testid="m-place-save"]')).click();
  await $('[data-testid="m-place-editor"]').waitForDisplayed({ reverse: true });
  const card = await cardByName('Beta Shed North, Testville');
  expect(await card.$('[data-testid="m-miesta-card-coords"]').getText()).toBe('48.50000, 17.25000');
  const row = (await rpc<Place[]>('list_places', {})).find((p) => p.id === id)!;
  expect(row.source).toBe('geocoder');
});

it('moves the pin by a tap on the map', async () => {
  const id = await ensurePlace('Theta Quarry, Testville');
  await openEditor(id);
  const map = await waitForLeaflet('m-place-map');
  await map.click({ x: 80, y: 40 }); // off the centre: the old pin is there
  await expect($('[data-testid="m-place-position"]')).toHaveText(expect.stringContaining('(ručne)'));
  await (await $('[data-testid="m-place-save"]')).click();
  const row = (await rpc<Place[]>('list_places', {})).find((p) => p.id === id)!;
  expect(row.source).toBe('manual');
  expect(row.lat).not.toBe(48.15);
});

it('takes the position from the phone GPS', async () => {
  const id = await ensurePlace('Kappa Lot, Testville');
  await browser.emulate('geolocation', { latitude: 48.3774, longitude: 17.5872, accuracy: 10 });
  await openEditor(id);
  await (await $('[data-testid="m-place-gps"]')).click();
  await (await $('[data-testid="m-place-save"]')).click();
  const card = await cardByName('Kappa Lot, Testville');
  expect(await card.$('[data-testid="m-miesta-card-coords"]').getText()).toBe('48.37740, 17.58720');
});

it('marks a place as home', async () => {
  const id = await ensurePlace('Mu Flat, Testville');
  await openEditor(id);
  await (await $('[data-testid="m-place-home"]')).click();
  await (await $('[data-testid="m-place-save"]')).click();
  const card = await cardByName('Mu Flat, Testville');
  await expect(card.$('[data-testid="m-miesta-card-home"]')).toBeDisplayed();
});

it('keeps the screen open and writes nothing else if the new name is taken', async () => {
  await ensurePlace('Nu Gate, Testville');
  const id = await ensurePlace('Xi Gate, Testville');
  await openEditor(id);
  await (await $('[data-testid="m-place-name"]')).setValue('Nu Gate, Testville');
  await (await $('[data-testid="m-place-home"]')).click();
  await (await $('[data-testid="m-place-save"]')).click();
  await expect($('.toast.error')).toBeDisplayed(); // use the toast selector of places.spec.ts
  await expect($('[data-testid="m-place-editor"]')).toBeDisplayed();
  const row = (await rpc<Place[]>('list_places', {})).find((p) => p.id === id)!;
  expect(row.name).toBe('Xi Gate, Testville');
  expect(row.isHome).toBe(false);
});

it('closes the edit screen with the back gesture', async () => {
  const id = await ensurePlace('Omicron Pier, Testville');
  await openEditor(id);
  await browser.back();
  await $('[data-testid="m-place-editor"]').waitForDisplayed({ reverse: true });
  await expect($('[data-testid="m-miesta"]')).toBeDisplayed();
});
```

  `openEditor(id)` opens `/m/miesta`, clicks the card with `data-place-id=id`, and waits for
  `m-place-editor`. `waitForLeaflet(testId)` is the `waitForLeafletMap` helper of
  `places.spec.ts` with the test id as a parameter. Check the toast selector in
  `places.spec.ts` before use.

- [ ] **Step 2: Run the spec and see the new tests fail** (`m-miesta-card` click opens nothing).

- [ ] **Step 3: Implement `PlacePinMap` and `PlaceEditor`, and wire them into the page.**
  - The editor keeps a local draft: name, `lat`, `lon`, `source`, `isHome`. It seeds them once
    from `place` (wrap the editor in `{#key}` on the place id, as the PC page does).
  - Search: the box starts with `place.name`. `geocodePlace(query)` lists the candidates. A tap
    on a candidate sets the pin, `source = 'geocoder'`, and clears the list. Show
    `places.noResults` for an empty answer and `places.searchError` for an error.
  - "Moja poloha": `navigator.geolocation.getCurrentPosition` with
    `{ enableHighAccuracy: true, timeout: 15000, maximumAge: 30000 }`, `source = 'manual'`.
    No secure context: `mobile.add.picker.insecure`. The error codes map to `denied`,
    `timeout` and `unavailable`, as `geoMessage` in `MobilePlacePicker.svelte`.
  - Under the map (`m-place-position`): after a tap or a drag, `positionManual` with
    `coords5`; otherwise `mapHint`.
  - "Uložiť miesto" is disabled while the name is blank, the pin is not set, or a save runs.
    Read-only: no save button, no delete button, the inputs are disabled.
  - The page's `onsave(draft)` for an existing place:
    1. `current = (await listPlaces()).find((p) => p.id === id)`.
    2. If `draft.name.trim() !== current.name`: `renamePlace(id, draft.name.trim())`.
    3. If the pin differs from `current.lat` / `current.lon`, or `draft.source !== current.source`:
       `setPlacePosition(id, draft.lat, draft.lon, draft.source)`.
    4. If `draft.isHome !== current.isHome`: `setHomePlace(draft.isHome ? id : null)`.
    5. Stop at the first error: toast it (`places.renameError` / `places.saveError` /
       `places.homeError`) and keep the screen open. On success: `history.back()`, reload the
       list, toast `places.saved`.

- [ ] **Step 4: Run the spec and see all tests pass.**

- [ ] **Step 5: Commit.**
  `git commit -m "feat(mobile): edit a place on the phone - name, search, map, GPS, home (task 96)"`.

---

### Task 3: Add a new place and delete a place

**Files:**
- Modify: [src/routes/m/miesta/+page.svelte](../../src/routes/m/miesta/+page.svelte), [src/lib/components/mobile/PlaceEditor.svelte](../../src/lib/components/mobile/PlaceEditor.svelte)
- Test: [tests/integration/specs/tier2/mobile-places.spec.ts](../../tests/integration/specs/tier2/mobile-places.spec.ts)

**Interfaces:**
- Consumes: `PlaceEditor` with `place === null`, `PlaceDraft` (Task 2).
- Uses [ConfirmModal.svelte](../../src/lib/components/ConfirmModal.svelte) for the delete
  question, with `danger={true}`, as the PC page.

- [ ] **Step 1: Write the failing tests.**

```ts
it('adds a new place with a pin from the map', async () => {
  await browser.url('/m/miesta');
  await (await $('[data-testid="m-miesta-add"]')).click();
  await expect($('[data-testid="m-place-save"]')).toBeDisabled();
  await (await $('[data-testid="m-place-name"]')).setValue('Pi Corner, Testville');
  await expect($('[data-testid="m-place-search"]')).toHaveValue('Pi Corner, Testville');
  await expect($('[data-testid="m-place-save"]')).toBeDisabled(); // no pin yet
  const map = await waitForLeaflet('m-place-map');
  await map.click({ x: 100, y: 60 });
  await (await $('[data-testid="m-place-save"]')).click();
  await cardByName('Pi Corner, Testville');
  const found = await rpc<Place | null>('find_place', { name: 'Pi Corner, Testville' });
  expect(found?.source).toBe('manual');
});

it('keeps save disabled for a blank name', async () => {
  await browser.url('/m/miesta');
  await (await $('[data-testid="m-miesta-add"]')).click();
  await (await $('[data-testid="m-place-name"]')).setValue('   ');
  const map = await waitForLeaflet('m-place-map');
  await map.click({ x: 100, y: 60 });
  await expect($('[data-testid="m-place-save"]')).toBeDisabled();
});

it('deletes an unused place after the question', async () => {
  const id = await ensurePlace('Rho Shed, Testville');
  await openEditor(id);
  await (await $('[data-testid="m-place-delete"]')).click();
  await (await $('.modal .button-danger')).click(); // use the ConfirmModal selector of places.spec.ts
  await $('[data-testid="m-place-editor"]').waitForDisplayed({ reverse: true });
  expect(await rpc<Place | null>('find_place', { name: 'Rho Shed, Testville' })).toBeNull();
});

it('disables delete for a place that a trip uses', async () => {
  // seedTrip with origin 'Sigma Yard, Testville' (see seedTrip in utils/db.ts)
  await openEditor(await ensurePlace('Sigma Yard, Testville'));
  await expect($('[data-testid="m-place-delete"]')).toBeDisabled();
  await expect($('[data-testid="m-place-in-use"]')).toHaveText(expect.stringContaining('1-krát'));
});
```

- [ ] **Step 2: Run the spec and see the new tests fail.**

- [ ] **Step 3: Implement.**
  - "Pridať miesto": `pushState('', { newPlace: true })`. The editor gets `place = null`: an
    empty name, no pin, the Slovakia view, no "Zmazať", title `places.add`. The search box
    copies the name until the user types into the search box.
  - The page's `onsave(draft)` for a new place: `createPlace(draft.name.trim(), draft.lat, draft.lon, draft.source)`.
    If `draft.isHome`, then `setHomePlace(created.id)`. **After a successful create, always
    leave the screen** (`history.back()`, reload the list). If the home write fails, toast
    `places.homeError` on the list. If the create fails, toast `places.saveError` and keep
    the screen.
  - "Zmazať": disabled if `place.uses > 0`, with `places.deleteInUse({ count })` under it. A
    tap opens `ConfirmModal` (`places.deleteConfirmTitle`, `places.deleteConfirmMessage`). On
    confirm: `deletePlace(id)`, `history.back()`, reload, toast `places.deleted`. On error:
    toast `places.deleteError`, keep the screen.

- [ ] **Step 4: Run the spec and see all tests pass.** Then `npm run check` and
  `npm run typecheck:tests`: zero errors.

- [ ] **Step 5: Commit.** `git commit -m "feat(mobile): add and delete a place on the phone (task 96)"`.

---

### Task 4: Docs and the full check

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) (via `/changelog`), [README.md](../../README.md),
  [README.en.md](../../README.en.md), [docs/features/mobile.md](../../docs/features/mobile.md),
  [_tasks/index.md](../index.md)

- [ ] **Step 1:** Run `/changelog`. The `### Pokyny k aktualizácii` block says: no migration,
  no env var, no image change.
- [ ] **Step 2:** Add `/m/miesta` to both READMEs and to [docs/features/mobile.md](../../docs/features/mobile.md) (the user
  flow and the files; the save order and the failure rules of Task 2 and Task 3).
- [ ] **Step 3:** Run `npm run test:backend`, then the full integration suite
  `npm run test:integration`. Expected: all pass. Run `/verify`.
- [ ] **Step 4:** Set the task status to 🟡 In Progress in `_tasks/index.md`, commit.

### After the merge to `main`

Use the `mockup` skill, "Steps: after the merge to `main`": remove the `open` boxes and the
"Návrh (task 96)" tags in [docs/mockups/mobil.html](../../docs/mockups/mobil.html), match the frames to the merged labels,
commit, and publish `mobil.html`. `bash` [scripts/mockup-drift.sh](../../scripts/mockup-drift.sh) must not list Mobil.

## Not tested

- Read-only mode: no spec can start the server in read-only mode (the only trigger is an
  unknown migration at start, [src-tauri/web/src/main.rs](../../src-tauri/web/src/main.rs)).
  The PC places spec has the same gap.
- A drag of the pin: the tap test covers the same `onpick` path and the same `manual` source.
