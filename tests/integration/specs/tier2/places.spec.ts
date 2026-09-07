/**
 * Tier 2: Place Book (Miesta) Integration Tests
 *
 * Covers the UI → backend → display flows of the place book:
 * - The places a seeded trip names turn up in Settings → Miesta, marked unplaced
 * - A coordinate confirmed in the map dialog is stored and survives a reload
 * - The book is one list for the whole database, so a place one vehicle used is
 *   offered in another vehicle's trip form (the old per-vehicle `routes`
 *   autocomplete could not do this — that is the behaviour worth pinning)
 * - A pin dropped on the map by hand is stored as a `manual` coordinate, and
 *   Clear puts the row back to unplaced
 * - An ASCII query in the list filter finds a name written with diacritics
 *
 * NOT covered here on purpose — all of it lives in the Rust unit tests:
 * `places::normalise`, how the derived list folds spellings and orders itself,
 * save/clear semantics, the read-only refusal, and how a Nominatim response is
 * parsed into candidates.
 *
 * Nothing here reaches Nominatim. `KNIHA_JAZD_MOCK_GEOCODER_DIR` (set for the
 * spawned server in wdio.server.conf.ts, and passed to the container as an -e
 * flag in .github/workflows/test.yml) points the provider at
 * `tests/integration/data/geocoder/`, where a canned `format=jsonv2` body is
 * filed under the *normalised* query — `places::normalise(query) + ".json"`, so
 * "Gamma Warehouse, Testville" is answered by "gamma warehouse, testville.json".
 * A query with no file gets an empty answer, never a request.
 *
 * What the geocoder replies for a real address is Nominatim's property, not this
 * codebase's, so no test here asserts that a search finds the right place — only
 * that the candidate a human picked travels from dialog to database to list.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, rpc } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

/** What `place-coords` renders for a row with no coordinate yet (em dash). */
const NO_COORDS = '—';

/**
 * The place the geocoder is asked about. Its fixture lives at
 * `data/geocoder/gamma warehouse, testville.json`; renaming one without the
 * other turns the search into "nothing found".
 */
const GEOCODED_PLACE = 'Gamma Warehouse, Testville';

/** First candidate in that fixture, as both the dialog and the list render it. */
const GEOCODED_COORDS = '48.500, 17.250';

/** Places used by the "seeded trip shows up unplaced" test. */
const UNPLACED_ORIGIN = 'Alpha Office, Testville';
const UNPLACED_DESTINATION = 'Beta Depot, Testville';

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
 * diacritics, so the row's own rendered text never contains "kosice" — the match
 * can only come through the backend's `normalisedName`.
 */
const DIACRITIC_PLACE = 'Košice Sklad';
const FILTER_OTHER_PLACE = 'Lambda Gate, Testville';

/**
 * Every place name this spec puts in the book, one set per test so a coordinate
 * stored by one can never colour another's assertions.
 *
 * They are cleared before each test as well as after the spec: `afterTest` in
 * wdio.server.conf.ts resets trips and vehicles but not the places table, and
 * spec files retry — so a save from an earlier attempt would otherwise still be
 * on disk when the "unplaced" assertions run.
 */
const SPEC_PLACES = [
  UNPLACED_ORIGIN,
  UNPLACED_DESTINATION,
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
];

/** Trips are seeded into the running year so the grid and the reset both see them. */
const YEAR = new Date().getFullYear();

async function forgetSpecPlaces(): Promise<void> {
  for (const displayName of SPEC_PLACES) {
    await rpc<null>('clear_place', { displayName });
  }
}

/**
 * Open Settings and wait for the Miesta section.
 *
 * Navigates away first: SvelteKit keeps a page component mounted when the route
 * does not change, and the list is filled by `onMount` — so going straight to
 * /settings from /settings would render whatever was loaded before the seeding
 * (see .claude/rules/integration-tests.md, "SvelteKit Component Caching").
 */
async function openPlacesSection(): Promise<void> {
  await navigateTo('trips');
  await navigateTo('settings');
  const section = await $('[data-testid="places-section"]');
  await section.waitForDisplayed({ timeout: 10000 });
}

function placeRowSelector(displayName: string): string {
  return `[data-testid="place-item"][data-place-name="${displayName}"]`;
}

/**
 * Wait for one place's row, optionally until it reports the expected placed
 * state. `data-place-placed` is rendered as the string "true"/"false" on every
 * row, so both states are observable.
 */
async function waitForPlaceRow(displayName: string, placed?: boolean) {
  const selector = placeRowSelector(displayName);
  await browser.waitUntil(
    async () => {
      const row = await $(selector);
      if (!(await row.isExisting())) return false;
      if (placed === undefined) return true;
      return (await row.getAttribute('data-place-placed')) === String(placed);
    },
    {
      timeout: 10000,
      timeoutMsg:
        `Place row '${displayName}'` +
        (placed === undefined ? '' : ` with placed=${placed}`) +
        ' never appeared',
    }
  );
  return await $(selector);
}

/** The two numbers the section header reports: "{placed}/{total} placed". */
async function placedCounter(): Promise<{ placed: number; total: number }> {
  const text = await $('[data-testid="places-counter"]').getText();
  const match = text.match(/(\d+)\s*\/\s*(\d+)/);
  if (!match) {
    throw new Error(`Unrecognised places counter: '${text}'`);
  }
  return { placed: Number(match[1]), total: Number(match[2]) };
}

/**
 * The names of the rows the list is currently showing, in render order.
 *
 * The filter's assertions go through this rather than through the rows' text:
 * a row can match on its normalised name while nothing it renders contains the
 * query, which is exactly the case worth pinning.
 */
async function visiblePlaceNames(): Promise<string[]> {
  const rows = await $$('[data-testid="place-item"]');
  const names: string[] = [];
  for (const row of rows) {
    names.push((await row.getAttribute('data-place-name')) ?? '');
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

  describe('Derived List', () => {
    /**
     * The list is derived from the trips, not stored (ADR-033) — so a trip is
     * all it takes for its two endpoints to become rows, and they arrive with
     * no coordinate until somebody confirms one.
     */
    it('should list the places of a seeded trip, marked unplaced', async () => {
      const vehicle = await seedVehicle({
        name: 'Place Book Derived List',
        licensePlate: 'PLC-001',
        initialOdometer: 10000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${YEAR}-05-12T08:00`,
        endDatetime: `${YEAR}-05-12T09:00`,
        origin: UNPLACED_ORIGIN,
        destination: UNPLACED_DESTINATION,
        distanceKm: 40,
        odometer: 10040,
        purpose: 'Business trip',
      });

      await openPlacesSection();

      for (const displayName of [UNPLACED_ORIGIN, UNPLACED_DESTINATION]) {
        const row = await waitForPlaceRow(displayName, false);

        // The warning icon renders only on unplaced rows...
        const icon = await row.$('[data-testid="place-unplaced-icon"]');
        expect(await icon.isExisting()).toBe(true);

        // ...while the coordinate cell is on every row, showing the placeholder
        // until there is something to put in it.
        expect((await row.$('[data-testid="place-coords"]').getText()).trim()).toBe(
          NO_COORDS
        );
      }

      // Nothing has been confirmed yet, and the header says so.
      expect((await placedCounter()).placed).toBe(0);
    });
  });

  describe('Placing From The Dialog', () => {
    /**
     * The dialog itself writes nothing (ADR-032): it hands the coordinate back
     * to the page, which calls `save_place`. This test follows that hand-off all
     * the way to a reloaded page, which is the only proof the coordinate reached
     * the database rather than the component's own state.
     */
    it('should store a picked candidate and still show it after a reload', async () => {
      const vehicle = await seedVehicle({
        name: 'Place Book Dialog',
        licensePlate: 'PLC-002',
        initialOdometer: 20000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${YEAR}-05-13T08:00`,
        endDatetime: `${YEAR}-05-13T09:00`,
        origin: GEOCODED_PLACE,
        destination: 'Gamma Sidings, Testville',
        distanceKm: 30,
        odometer: 20030,
        purpose: 'Business trip',
      });

      await openPlacesSection();
      const row = await waitForPlaceRow(GEOCODED_PLACE, false);
      await row.$('[data-testid="place-edit"]').click();

      const modal = await $('[data-testid="place-modal"]');
      await modal.waitForDisplayed({ timeout: 10000 });

      // Prefilled with the row's own spelling — which is exactly the key the
      // mock files its answer under — so walking the list is submit, pick, save.
      expect(await $('[data-testid="place-search-input"]').getValue()).toBe(
        GEOCODED_PLACE
      );

      // Save is inert until there is a pin to save.
      const save = await $('[data-testid="place-modal-save"]');
      expect(await save.isEnabled()).toBe(false);
      expect(await $('[data-testid="place-modal-coords"]').getText()).toBe(NO_COORDS);

      await $('[data-testid="place-search-submit"]').click();

      const candidateList = await $('[data-testid="place-candidates"]');
      await candidateList.waitForDisplayed({
        timeout: 10000,
        timeoutMsg: 'Geocoder candidates never rendered — is the mock directory wired up?',
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
      expect(await $('[data-testid="place-modal-coords"]').getText()).toBe(
        GEOCODED_COORDS
      );

      await save.click();
      await modal.waitForDisplayed({ timeout: 10000, reverse: true });

      // The page re-reads the list from the backend after the save.
      const placedRow = await waitForPlaceRow(GEOCODED_PLACE, true);
      expect(await placedRow.$('[data-testid="place-coords"]').getText()).toBe(
        GEOCODED_COORDS
      );
      expect(
        await placedRow.$('[data-testid="place-unplaced-icon"]').isExisting()
      ).toBe(false);
      expect((await placedCounter()).placed).toBe(1);

      // Reload the whole SPA: what comes back can only have come from the DB.
      await browser.refresh();
      await waitForAppReady();
      await openPlacesSection();

      const reloadedRow = await waitForPlaceRow(GEOCODED_PLACE, true);
      expect(await reloadedRow.$('[data-testid="place-coords"]').getText()).toBe(
        GEOCODED_COORDS
      );
    });
  });

  describe('Shared Across Vehicles', () => {
    /**
     * The trip form's origin/destination suggestions come from `list_places`,
     * which is database-wide. Before the place book they came from the `routes`
     * table, which is keyed by vehicle — so this assertion is the one that fails
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

      // Vehicle B has its own, entirely different places — nothing it has ever
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
     * geocoder cannot resolve — so this test never searches. It ends by clearing
     * the coordinate it just stored, which is also the only way to reach the
     * Clear button: it is rendered only for a place that already has one.
     */
    it('should pin a place by hand and then clear it again', async () => {
      const vehicle = await seedVehicle({
        name: 'Place Book Manual Pin',
        licensePlate: 'PLC-003',
        initialOdometer: 50000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${YEAR}-05-16T08:00`,
        endDatetime: `${YEAR}-05-16T09:00`,
        origin: MANUAL_PLACE,
        destination: MANUAL_COMPANION,
        distanceKm: 20,
        odometer: 50020,
        purpose: 'Business trip',
      });

      await openPlacesSection();
      const row = await waitForPlaceRow(MANUAL_PLACE, false);
      await row.$('[data-testid="place-edit"]').click();

      const modal = await $('[data-testid="place-modal"]');
      await modal.waitForDisplayed({ timeout: 10000 });

      const map = await waitForLeafletMap();

      // Nothing pinned yet: no coordinate to show, and nothing to save.
      const save = await $('[data-testid="place-modal-save"]');
      expect(await save.isEnabled()).toBe(false);
      expect(await $('[data-testid="place-modal-coords"]').getText()).toBe(NO_COORDS);

      // The whole point of this path: a click on the map, no search, no
      // candidate, no request. WebdriverIO clicks the element's centre, which
      // for a Leaflet container is the map's own centre — "somewhere" is all
      // this test needs, since which point was hit is the map's business.
      await map.click();

      // This is the assertion the feature had nowhere else: the UI can produce
      // a `manual` source at all. The Rust tests only prove one round-trips
      // once it has been handed over.
      await browser.waitUntil(
        async () => (await modal.getAttribute('data-place-source')) === 'manual',
        {
          timeout: 10000,
          timeoutMsg: 'Clicking the map did not turn the pending pin into a manual one',
        }
      );

      expect(await save.isEnabled()).toBe(true);

      const pinned = await $('[data-testid="place-modal-coords"]').getText();
      expect(pinned).not.toBe(NO_COORDS);
      expect(pinned).toMatch(/^-?\d+\.\d{3}, -?\d+\.\d{3}$/);

      await save.click();
      await modal.waitForDisplayed({ timeout: 10000, reverse: true });

      // Same string the dialog showed, now coming back from the list the page
      // re-read after the save — so the hand-dropped pin reached the database.
      const placedRow = await waitForPlaceRow(MANUAL_PLACE, true);
      expect(await placedRow.$('[data-testid="place-coords"]').getText()).toBe(pinned);
      expect(
        await placedRow.$('[data-testid="place-unplaced-icon"]').isExisting()
      ).toBe(false);
      expect((await placedCounter()).placed).toBe(1);

      // Reopen the now-placed row: only now does the dialog offer Clear.
      await placedRow.$('[data-testid="place-edit"]').click();
      const reopened = await $('[data-testid="place-modal"]');
      await reopened.waitForDisplayed({ timeout: 10000 });

      const clear = await $('[data-testid="place-modal-clear"]');
      await clear.waitForClickable({ timeout: 10000 });
      await clear.click();
      await reopened.waitForDisplayed({ timeout: 10000, reverse: true });

      // Back to where the row started, header included.
      const clearedRow = await waitForPlaceRow(MANUAL_PLACE, false);
      expect((await clearedRow.$('[data-testid="place-coords"]').getText()).trim()).toBe(
        NO_COORDS
      );
      expect(
        await clearedRow.$('[data-testid="place-unplaced-icon"]').isExisting()
      ).toBe(true);
      expect((await placedCounter()).placed).toBe(0);
    });
  });

  describe('Filtering The List', () => {
    /**
     * Rows are matched against two spellings: the one they display and the
     * backend's `normalisedName`. Only the second can answer an ASCII query for
     * a name written with diacritics, which is what production data shows users
     * type — so this asserts on *which* rows survive, by name. "The visible text
     * contains what I typed" is false here by design.
     */
    it('should match an ASCII query against a name written with diacritics', async () => {
      const vehicle = await seedVehicle({
        name: 'Place Book Filter',
        licensePlate: 'PLC-004',
        initialOdometer: 60000,
        tankSizeLiters: 50,
        tpConsumption: 6.5,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${YEAR}-05-17T08:00`,
        endDatetime: `${YEAR}-05-17T09:00`,
        origin: DIACRITIC_PLACE,
        destination: FILTER_OTHER_PLACE,
        distanceKm: 35,
        odometer: 60035,
        purpose: 'Business trip',
      });

      await openPlacesSection();
      await waitForPlaceRow(DIACRITIC_PLACE, false);
      await waitForPlaceRow(FILTER_OTHER_PLACE, false);

      const filter = await $('[data-testid="places-filter"]');
      await filter.setValue('kosice');

      await browser.waitUntil(async () => (await visiblePlaceNames()).length === 1, {
        timeout: 10000,
        timeoutMsg: "Filtering for 'kosice' did not narrow the list to one row",
      });
      expect(await visiblePlaceNames()).toEqual([DIACRITIC_PLACE]);

      // A query nothing matches leaves the book intact and says so with its own
      // message — not the one for a book with no places in it at all.
      await filter.setValue('nonesuch');
      await $('[data-testid="places-no-matches"]').waitForDisplayed({ timeout: 10000 });
      expect(await visiblePlaceNames()).toEqual([]);
      expect(await $('[data-testid="places-empty"]').isExisting()).toBe(false);
    });
  });
});
