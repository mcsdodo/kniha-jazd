/**
 * Tier 2: Place Book (Miesta) Integration Tests
 *
 * Covers the UI -> backend -> display flows of the place book:
 * - A coordinate confirmed in the map dialog is stored and survives a reload
 * - A place is added, renamed and deleted on the Miesta tab (/miesta)
 * - The book is one list for the whole database, so a place one vehicle used is
 *   offered in another vehicle's trip form (the old per-vehicle `routes`
 *   autocomplete could not do this - that is the behaviour worth pinning)
 * - A pin dropped on the map by hand is stored as a `manual` coordinate
 * - An ASCII query in the list filter finds a name written with diacritics
 *
 * NOT covered here: the "needs a position" marker of a legacy unplaced place.
 * Seeding cannot create an unplaced place, so only the code path exists.
 *
 * NOT covered here on purpose - all of it lives in the Rust unit tests:
 * `places::normalise`, how the derived list folds spellings and orders itself,
 * rename and delete rules, the read-only refusal, and how a Nominatim response is
 * parsed into candidates.
 *
 * Nothing here reaches Nominatim. `KNIHA_JAZD_MOCK_GEOCODER_DIR` (set for the
 * spawned server in wdio.server.conf.ts, and passed to the container as an -e
 * flag in .github/workflows/test.yml) points the provider at
 * `tests/integration/data/geocoder/`, where a canned `format=jsonv2` body is
 * filed under the *normalised* query - `places::normalise(query) + ".json"`, so
 * "Gamma Warehouse, Testville" is answered by "gamma warehouse, testville.json".
 * A query with no file gets an empty answer, never a request.
 *
 * What the geocoder replies for a real address is Nominatim's property, not this
 * codebase's, so no test here asserts that a search finds the right place - only
 * that the candidate a human picked travels from dialog to database to list.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, rpc, getTripGridData, ensurePlace } from '../../utils/db';
import { waitForTripGrid, TripGrid } from '../../utils/assertions';
import { fillTripForm, fillField } from '../../utils/forms';

/**
 * The place the geocoder is asked about. Its fixture lives at
 * `data/geocoder/gamma warehouse, testville.json`; renaming one without the
 * other turns the search into "nothing found".
 */
const GEOCODED_PLACE = 'Gamma Warehouse, Testville';

/** First candidate in that fixture, as both the dialog and the list render it. */
const GEOCODED_COORDS = '48.500, 17.250';

/** A place no trip uses, so it can be deleted. */
const DELETE_PLACE = 'Unused Corner, Testville';

/** The place vehicle A uses and vehicle B must still be offered. */
const SHARED_PLACE = 'Delta Yard, Testville';

/**
 * The place that gets its pin by hand. Deliberately absent from the geocoder
 * fixture directory: the manual path never searches, and a name the mock could
 * answer would leave open which of the two paths actually produced the pin.
 */
const MANUAL_PLACE = 'Theta Quarry, Testville';
const MANUAL_COMPANION = 'Iota Bend, Testville';

/**
 * The place the filter has to find from an ASCII query. Its display name carries
 * diacritics, so the row's own rendered text never contains "kosice" - the match
 * can only come through the backend's `normalisedName`.
 */
const DIACRITIC_PLACE = 'Košice Sklad';
const FILTER_OTHER_PLACE = 'Lambda Gate, Testville';

/** Every place name this spec creates by name; deleted again before each test. */
const SPEC_PLACES = [
  GEOCODED_PLACE,
  'Gamma Sidings, Testville',
  SHARED_PLACE,
  'Epsilon Site, Testville',
  'Zeta Hub, Otherton',
  'Eta Ramp, Otherton',
  MANUAL_PLACE,
  MANUAL_COMPANION,
  DIACRITIC_PLACE,
  FILTER_OTHER_PLACE,
  'Nitra',
  'Nitra - centrum',
  DELETE_PLACE,
];

/** Trips are seeded into the running year so the grid and the reset both see them. */
const YEAR = new Date().getFullYear();

/**
 * Delete the spec's places that no trip uses. A place that is still in use
 * (a retry left a trip behind) cannot be deleted, so errors are ignored.
 */
async function forgetSpecPlaces(): Promise<void> {
  for (const name of SPEC_PLACES) {
    const found = await rpc<{ id: string } | null>('find_place', { name });
    if (!found) continue;
    try {
      await rpc<null>('delete_place', { id: found.id });
    } catch {
      // in use: a later test reuses it
    }
  }
}

/**
 * Open the Miesta tab and wait for the list.
 *
 * Navigates away first: SvelteKit keeps a page component mounted when the route
 * does not change, and the list is filled by `onMount` - so going straight to
 * /miesta from /miesta would render whatever was loaded before the seeding
 * (see .claude/rules/integration-tests.md, "SvelteKit Component Caching").
 */
async function openPlacesPage(): Promise<void> {
  await navigateTo('trips');
  await navigateTo('miesta');
  const list = await $('[data-testid="places-list"]');
  await list.waitForDisplayed({ timeout: 10000 });
}

function placeRowSelector(id: string): string {
  return `[data-testid="place-row"][data-place-id="${id}"]`;
}

/** Wait for one place's row, found by place id. */
async function waitForPlaceRow(id: string) {
  const row = await $(placeRowSelector(id));
  await row.waitForDisplayed({ timeout: 10000, timeoutMsg: `Place row '${id}' never appeared` });
  return row;
}

/**
 * The names of the rows the list is currently showing, in render order.
 *
 * The filter's assertions go through this rather than through the rows' text:
 * a row can match on its normalised name while nothing it renders contains the
 * query, which is exactly the case worth pinning.
 */
async function visiblePlaceNames(): Promise<string[]> {
  const cells = await $$('[data-testid="place-row"] [data-testid="place-name"]');
  const names: string[] = [];
  for (const cell of cells) {
    names.push((await cell.getText()).trim());
  }
  return names;
}

/**
 * Wait for the dialog's map div to actually be a Leaflet map.
 *
 * Leaflet is imported lazily (it touches `window` at import time), so the
 * container exists for a while before it can answer a click. `leaflet-container`
 * is the class `L.map()` puts on the element it takes over.
 */
async function waitForLeafletMap() {
  const map = await $('[data-testid="place-map"]');
  await map.waitForDisplayed({ timeout: 10000 });
  await browser.waitUntil(
    async () => ((await map.getAttribute('class')) ?? '').includes('leaflet-container'),
    { timeout: 10000, timeoutMsg: 'Leaflet never took over the map container' }
  );
  return map;
}

/** Open the new-trip row in the grid and wait for its inputs. */
async function openNewTripRow(): Promise<void> {
  const newTripBtn = await $('button.new-record');
  await newTripBtn.waitForClickable({ timeout: 5000 });
  await newTripBtn.click();

  await browser.waitUntil(
    async () => {
      const editingRow = await $('tr.editing');
      return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
    },
    { timeout: 10000, timeoutMsg: 'New trip row did not open' }
  );
}

describe('Tier 2: Place Book', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    await forgetSpecPlaces();
  });

  after(async () => {
    await forgetSpecPlaces();
  });

  describe('Placing From The Dialog', () => {
    /**
     * The dialog itself writes nothing (ADR-032): it hands the coordinate back
     * to the page, which calls `set_place_position`. This test follows that
     * hand-off all the way to a reloaded page, which is the only proof the
     * coordinate reached the database rather than the component's own state.
     */
    it('should store a picked candidate and still show it after a reload', async () => {
      const id = await ensurePlace(GEOCODED_PLACE);

      await openPlacesPage();
      const row = await waitForPlaceRow(id);
      await (await row.$('[data-testid="place-edit"]')).click();

      const modal = await $('[data-testid="place-modal"]');
      await modal.waitForDisplayed({ timeout: 10000 });

      // Prefilled with the row's own name, which is exactly the key the mock
      // files its answer under, so walking the list is submit, pick, save.
      expect(await $('[data-testid="place-search-input"]').getValue()).toBe(GEOCODED_PLACE);

      await (await $('[data-testid="place-search-submit"]')).click();

      const candidateList = await $('[data-testid="place-candidates"]');
      await candidateList.waitForDisplayed({
        timeout: 10000,
        timeoutMsg: 'Geocoder candidates never rendered - is the mock directory wired up?',
      });
      const candidates = await $$('[data-testid="place-candidate"]');
      expect(candidates.length).toBe(2);

      // The labels are the fixture's own, so a mock directory that silently
      // stopped being wired up fails here instead of quietly asking Nominatim.
      expect(await candidates[0].getText()).toContain('synthetic fixture');

      await candidates[0].click();

      // Picking a candidate is what makes the pending pin a geocoder answer;
      // dragging or clicking the map would make it "manual" instead.
      expect(await modal.getAttribute('data-place-source')).toBe('geocoder');
      expect(await $('[data-testid="place-modal-coords"]').getText()).toBe(GEOCODED_COORDS);

      await (await $('[data-testid="place-modal-save"]')).click();
      await modal.waitForDisplayed({ timeout: 10000, reverse: true });

      // The page re-reads the list from the backend after the save.
      const placedRow = await waitForPlaceRow(id);
      await browser.waitUntil(
        async () => (await placedRow.$('[data-testid="place-coords"]').getText()) === GEOCODED_COORDS,
        { timeout: 10000, timeoutMsg: 'the saved coordinate did not show in the row' }
      );

      // Reload the whole SPA: what comes back can only have come from the DB.
      await browser.refresh();
      await waitForAppReady();
      await openPlacesPage();

      const reloadedRow = await waitForPlaceRow(id);
      expect(await reloadedRow.$('[data-testid="place-coords"]').getText()).toBe(GEOCODED_COORDS);
    });
  });

  describe('Shared Across Vehicles', () => {
    /**
     * The trip form's origin/destination suggestions come from `list_places`,
     * which is database-wide. Before the place book they came from the `routes`
     * table, which is keyed by vehicle - so this assertion is the one that fails
     * if the autocomplete is ever wired back to a per-vehicle source.
     */
    it("should offer another vehicle's place in the trip form", async () => {
      const alpha = await seedVehicle({
        name: 'Place Book Vehicle A',
        licensePlate: 'PLC-00A',
        initialOdometer: 30000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });
      await seedTrip({
        vehicleId: alpha.id as string,
        startDatetime: `${YEAR}-05-14T08:00`,
        endDatetime: `${YEAR}-05-14T09:00`,
        origin: SHARED_PLACE,
        destination: 'Epsilon Site, Testville',
        distanceKm: 25,
        odometer: 30025,
        purpose: 'Business trip',
      });

      // Vehicle B has its own, entirely different places - nothing it has ever
      // driven starts with "Delta".
      const beta = await seedVehicle({
        name: 'Place Book Vehicle B',
        licensePlate: 'PLC-00B',
        initialOdometer: 40000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });
      await seedTrip({
        vehicleId: beta.id as string,
        startDatetime: `${YEAR}-05-15T08:00`,
        endDatetime: `${YEAR}-05-15T09:00`,
        origin: 'Zeta Hub, Otherton',
        destination: 'Eta Ramp, Otherton',
        distanceKm: 15,
        odometer: 40015,
        purpose: 'Business trip',
      });

      await setActiveVehicle(beta.id as string);
      await navigateTo('trips');
      await waitForTripGrid();

      await openNewTripRow();

      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.waitForDisplayed({ timeout: 5000 });
      await originInput.click();
      await originInput.setValue('Delta');

      await browser.waitUntil(
        async () => {
          const dropdown = await $('.autocomplete .dropdown');
          return (await dropdown.isExisting()) && (await dropdown.isDisplayed());
        },
        {
          timeout: 5000,
          timeoutMsg: "Origin autocomplete offered nothing for another vehicle's place",
        }
      );

      // Read the entries one at a time: in WebdriverIO 9 `ElementArray.map()`
      // is a chaining helper that returns a promise, not an array of promises.
      const suggestions = await $$('.autocomplete .dropdown .suggestion');
      const offered: string[] = [];
      for (const suggestion of suggestions) {
        offered.push((await suggestion.getText()).trim());
      }

      // Only vehicle A has ever been to a "Delta" place, and it is offered here.
      expect(offered).toEqual([SHARED_PLACE]);
    });
  });

  describe('Placing By Hand', () => {
    /**
     * The map click (and the marker drag beside it) is the only producer of a
     * `manual` source, and the documented answer for a place whose bare name the
     * geocoder cannot resolve - so this test never searches.
     */
    it('should move the pin of a place by hand', async () => {
      const id = await ensurePlace(MANUAL_PLACE);

      await openPlacesPage();
      const row = await waitForPlaceRow(id);
      const before = await row.$('[data-testid="place-coords"]').getText();
      await (await row.$('[data-testid="place-edit"]')).click();

      const modal = await $('[data-testid="place-modal"]');
      await modal.waitForDisplayed({ timeout: 10000 });

      const map = await waitForLeafletMap();

      // The click is offset from the centre: the existing pin sits there, and a
      // click on the marker does not reach the map.
      await map.click({ x: 120, y: 60 });

      await browser.waitUntil(
        async () => (await $('[data-testid="place-modal-coords"]').getText()) !== before,
        { timeout: 10000, timeoutMsg: 'Clicking the map did not move the pending pin' }
      );
      expect(await modal.getAttribute('data-place-source')).toBe('manual');

      const pinned = await $('[data-testid="place-modal-coords"]').getText();
      expect(pinned).toMatch(/^-?\d+\.\d{3}, -?\d+\.\d{3}$/);

      await (await $('[data-testid="place-modal-save"]')).click();
      await modal.waitForDisplayed({ timeout: 10000, reverse: true });

      // Same string the dialog showed, now coming back from the list the page
      // re-read after the save - so the hand-dropped pin reached the database.
      await browser.waitUntil(
        async () => (await (await waitForPlaceRow(id)).$('[data-testid="place-coords"]').getText()) === pinned,
        { timeout: 10000, timeoutMsg: 'the hand-dropped pin did not show in the row' }
      );
    });
  });

  describe('Filtering The List', () => {
    /**
     * Rows are matched against two spellings: the one they display and the
     * backend's `normalisedName`. Only the second can answer an ASCII query for
     * a name written with diacritics, which is what production data shows users
     * type - so this asserts on *which* rows survive, by name. "The visible text
     * contains what I typed" is false here by design.
     */
    it('should match an ASCII query against a name written with diacritics', async () => {
      const diacriticId = await ensurePlace(DIACRITIC_PLACE);
      const otherId = await ensurePlace(FILTER_OTHER_PLACE);

      await openPlacesPage();
      await waitForPlaceRow(diacriticId);
      await waitForPlaceRow(otherId);

      const filter = await $('[data-testid="places-filter"]');
      await filter.setValue('kosice');

      await browser.waitUntil(async () => (await visiblePlaceNames()).length === 1, {
        timeout: 10000,
        timeoutMsg: "Filtering for 'kosice' did not narrow the list to one row",
      });
      expect(await visiblePlaceNames()).toEqual([DIACRITIC_PLACE]);

      // A query nothing matches leaves the book intact and says so with its own
      // message - not the one for a book with no places in it at all.
      await filter.setValue('nonesuch');
      await $('[data-testid="places-no-matches"]').waitForDisplayed({ timeout: 10000 });
      expect(await visiblePlaceNames()).toEqual([]);
      expect(await $('[data-testid="places-empty"]').isExisting()).toBe(false);
    });
  });

  describe('Managing Places', () => {
    it('should add a place, then offer it in the trip form', async () => {
      await openPlacesPage();
      await (await $('[data-testid="place-add"]')).click();
      const nameInput = await $('[data-testid="place-add-name"]');
      await nameInput.waitForDisplayed({ timeout: 5000 });

      // The next button waits for a name.
      const next = await $('[data-testid="place-add-next"]');
      expect(await next.isEnabled()).toBe(false);

      // GEOCODED_PLACE is the name the geocoder mock has an answer for.
      await fillField('[data-testid="place-add-name"]', GEOCODED_PLACE);
      expect(await next.isEnabled()).toBe(true);
      await next.click();

      // The add flow opens the map dialog with the name as the query.
      const modal = await $('[data-testid="place-modal"]');
      await modal.waitForDisplayed({ timeout: 10000 });
      expect(await $('[data-testid="place-search-input"]').getValue()).toBe(GEOCODED_PLACE);
      const save = await $('[data-testid="place-modal-save"]');
      expect(await save.isEnabled()).toBe(false); // no position yet, no save

      await (await $('[data-testid="place-search-submit"]')).click();
      const candidateList = await $('[data-testid="place-candidates"]');
      await candidateList.waitForDisplayed({ timeout: 10000 });
      const candidates = await $$('[data-testid="place-candidate"]');
      await candidates[0].click();
      expect(await $('[data-testid="place-modal-coords"]').getText()).toBe(GEOCODED_COORDS);
      await save.click();
      await modal.waitForDisplayed({ timeout: 10000, reverse: true });

      // Not a count: the reset keeps a place that a trip outside its three
      // years still uses. Look the new place up by id.
      const added = await rpc<{ id: string } | null>('find_place', { name: GEOCODED_PLACE });
      expect(added).not.toBeNull();
      await waitForPlaceRow(added!.id);

      // The trip form offers it.
      const vehicle = await seedVehicle({
        name: 'Place Add Vehicle',
        licensePlate: 'PLC-A01',
        initialOdometer: 10000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });
      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await openNewTripRow();
      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.click();
      await originInput.setValue('Gamma');
      await browser.waitUntil(
        async () => (await $('.autocomplete .dropdown')).isDisplayed(),
        { timeout: 5000, timeoutMsg: 'the new place was not offered' }
      );
      const suggestions = await $$('.autocomplete .dropdown .suggestion');
      const offered: string[] = [];
      for (const suggestion of suggestions) {
        offered.push((await suggestion.getText()).trim());
      }
      expect(offered).toContain(GEOCODED_PLACE);
    });

    it('should rename a place and show the new name in the trip grid', async () => {
      const vehicle = await seedVehicle({
        name: 'Place Rename Vehicle',
        licensePlate: 'PLC-R01',
        initialOdometer: 1000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });
      const vehicleId = vehicle.id as string;
      const id = await ensurePlace('Nitra');
      await seedTrip({
        vehicleId,
        startDatetime: `${YEAR}-03-01T08:00`,
        origin: 'Nitra',
        destination: 'Nitra',
        distanceKm: 5,
        odometer: 1005,
        purpose: 'p',
      });
      await setActiveVehicle(vehicleId);

      await openPlacesPage();
      const row = await waitForPlaceRow(id);
      await (await row.$('[data-testid="place-rename"]')).click();
      await fillField('[data-testid="place-rename-input"]', 'Nitra - centrum');
      await (await $('[data-testid="place-rename-save"]')).click();

      await browser.waitUntil(
        async () => (await (await waitForPlaceRow(id)).$('[data-testid="place-name"]').getText()).trim() === 'Nitra - centrum',
        { timeout: 10000, timeoutMsg: 'the renamed place did not show in the list' }
      );

      await navigateTo('trips');
      await waitForTripGrid();
      await browser.waitUntil(
        async () => (await $(TripGrid.dataRows).getText()).includes('Nitra - centrum'),
        { timeout: 10000, timeoutMsg: 'the renamed place did not show in the grid' }
      );
    });

    it('should disable delete for a place in use and say how often it is used', async () => {
      const vehicle = await seedVehicle({
        name: 'Place Delete Vehicle',
        licensePlate: 'PLC-D01',
        initialOdometer: 1000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });
      const id = await ensurePlace('Nitra');
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${YEAR}-03-01T08:00`,
        origin: 'Nitra',
        destination: 'Nitra',
        distanceKm: 5,
        odometer: 1005,
        purpose: 'p',
      });

      await openPlacesPage();
      const row = await waitForPlaceRow(id);
      const del = await row.$('[data-testid="place-delete"]');
      expect(await del.isEnabled()).toBe(false);
      // A loop trip counts the place twice (once per endpoint).
      expect(await del.getAttribute('title')).toContain('2');
    });

    it('should delete an unused place after a confirmation', async () => {
      const id = await ensurePlace(DELETE_PLACE);

      await openPlacesPage();
      const row = await waitForPlaceRow(id);
      const del = await row.$('[data-testid="place-delete"]');
      expect(await del.isEnabled()).toBe(true);
      await del.click();

      // The confirmation dialog: the last button of its action row confirms.
      const confirm = await $('.modal-actions button:last-child');
      await confirm.waitForClickable({ timeout: 5000 });
      await confirm.click();

      await browser.waitUntil(
        async () => !(await $(placeRowSelector(id)).isExisting()),
        { timeout: 10000, timeoutMsg: 'the deleted place stayed in the list' }
      );
      expect(await rpc<unknown>('find_place', { name: DELETE_PLACE })).toBeNull();
    });
  });
});

describe('Trip Form Uses Existing Places', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  async function openFormForNewVehicle(plate: string): Promise<string> {
    const vehicle = await seedVehicle({
      name: `Place Form ${plate}`,
      licensePlate: plate,
      initialOdometer: 10000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);
    await navigateTo('trips');
    await waitForTripGrid();
    await openNewTripRow();
    return vehicle.id as string;
  }

  it('should block a save when the origin names no place', async () => {
    const vehicleId = await openFormForNewVehicle('PLC-F01');
    await fillTripForm({
      startDatetime: `${YEAR}-03-01T08:00`,
      origin: 'Iota Square, Testville', // fillTripForm calls ensurePlace
      destination: 'Iota Square, Testville',
      distanceKm: 5,
      purpose: 'Business trip',
    });
    // Overwrite the origin with text that matches no place.
    await fillField(TripGrid.tripForm.origin, 'Nowhere At All');
    await (await $('tr.editing .icon-btn.save')).click();

    const error = await $('[data-testid="trip-place-error"]');
    await error.waitForDisplayed({ timeout: 5000 });
    expect(await error.getText()).toContain('Nowhere At All');
    expect((await getTripGridData(vehicleId, YEAR)).trips.length).toBe(0);
  });

  it('should save typed text that matches a place in another case', async () => {
    await ensurePlace('Kappa Plaza, Testville');
    const vehicleId = await openFormForNewVehicle('PLC-F02');
    await fillTripForm({
      startDatetime: `${YEAR}-03-01T08:00`,
      origin: 'Kappa Plaza, Testville',
      destination: 'Kappa Plaza, Testville',
      distanceKm: 5,
      purpose: 'Business trip',
    });
    await fillField(TripGrid.tripForm.origin, 'KAPPA PLAZA, TESTVILLE');
    await (await $('tr.editing .icon-btn.save')).click();

    await browser.waitUntil(
      async () => (await getTripGridData(vehicleId, YEAR)).trips.length === 1,
      { timeout: 10000, timeoutMsg: 'the trip with a case-folded origin was not saved' }
    );
    expect((await getTripGridData(vehicleId, YEAR)).trips[0].origin).toBe('Kappa Plaza, Testville');
  });
});
