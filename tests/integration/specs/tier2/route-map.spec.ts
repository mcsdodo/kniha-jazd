/**
 * Tier 2: Route Map Integration Tests
 *
 * Covers the UI → backend → display flows for generated route maps:
 * - The map action renders on trip rows (capability-gated, server mode only)
 * - Saving a route flips that row's pin to its saved state
 * - Deleting the route clears it again
 * - Deleting the trip takes its route map with it (FK cascade)
 *
 * NOT covered here on purpose — all of it lives in the Rust unit tests:
 * route generation (genetic algorithm, tile geometry, polyline codec),
 * distance/deviation math, PNG rendering and export HTML.
 *
 * Two deliberate constraints on this file:
 * 1. `generate_route` is never called. It hits the public OSRM demo server —
 *    network-dependent, rate-limited and non-deterministic. Routes are seeded
 *    with `save_trip_route` and a canned polyline instead.
 * 2. No export is triggered for a vehicle that has a saved route map. Export
 *    renders map PNGs from live OSM tiles (15s timeout), which would stall
 *    this suite on an offline or throttled CI box.
 * 3. Candidate picking, alternative promotion and drag editing are NOT covered.
 *    Each needs a live geocoder/router mid-flow, which constraint 1 rules out,
 *    and the providers are constructed inside the dispatcher arms so there is
 *    nothing to stub. Covering them needs a test-mode provider override first
 *    -- see _tasks/72-route-map-origin-destination/03-plan.md.
 *
 *    An unplaced endpoint opening the shared place dialog IS covered below,
 *    though: `start_route_for_trip` is DB-only (a place-book lookup, not a
 *    geocode -- see ADR-032), and the page returns as soon as it finds an
 *    unplaced field, before `route_direct` is ever called. Only the second
 *    half of that flow -- placing the pin there and watching the route get
 *    drawn -- needs the router and stays deferred with the rest of this list.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
  deleteTrip,
  rpc,
} from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

/**
 * Pin button in a trip row's actions cell. Neither the pin nor the row carries
 * a data-testid, so the class pair is the contract; `has-map` is added on top
 * when the trip has a saved route.
 */
const MAP_PIN = 'td.col-actions button.icon-btn.map';

/**
 * Canned polyline5 holding three geometry points between Bratislava and Trnava.
 * Encoding/decoding is proven in the Rust unit tests — these tests never assert
 * on the geometry, they only need a persisted route to exist.
 */
const CANNED_POLYLINE = 'w_{dHcjlgBg}L{pd@wv]_bw@';

/** The two stops the polyline above runs between. */
const CANNED_WAYPOINTS = [
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
  { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
];

/**
 * Bratislava -> Trnava with a via in the middle. A separate constant from
 * `CANNED_WAYPOINTS` -- the existing tests seed that one and must not see it
 * grow a third stop. `save_trip_route_internal` never checks waypoints
 * against the polyline's own decoded points, so reusing `CANNED_POLYLINE`
 * here is fine -- these tests assert presence, not geometry.
 */
const CANNED_VIA_WAYPOINTS = [
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
  { lat: 48.26, lon: 17.34, name: 'Via stop' },
  { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
];

/**
 * Bratislava -> Trnava -> Bratislava: the ALREADY-CLOSED shape a round trip
 * persists as (Task 19). This is exactly what `save_trip_route` stores once
 * a user ticks "Round trip" and saves -- the third waypoint is a clone of the
 * first, same coordinates AND name, mirroring what `route_direct_internal`
 * actually appends.
 */
const CANNED_ROUND_TRIP_WAYPOINTS = [
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
  { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
];

/** Only the identity fields matter here — the tests assert presence, not values. */
interface SavedRouteMap {
  tripId: string;
  polyline: string;
}

/** Persist a route against a trip without touching OSRM. */
async function saveRoute(tripId: string, targetKm: number): Promise<void> {
  await rpc<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm: targetKm,
    // These are canned home-loop fixtures (Task 72, Phase 2): mode is a
    // required field on the wire, and 'loop' is the value RouteMode's serde
    // form pins it to.
    mode: 'loop',
  });
}

/**
 * Persist a direct route with a via stop against a trip, without touching
 * OSRM. Waypoint count (not the decoded polyline) is what the map view reads
 * to decide a route "has vias" -- see `hasVias` in mapa/+page.svelte.
 */
async function saveDirectRouteWithVia(tripId: string, targetKm: number): Promise<void> {
  await rpc<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_VIA_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm: targetKm,
    mode: 'direct',
  });
}

/**
 * Persist an already-closed round trip against a trip, without touching
 * OSRM (Task 19, fix round 1). This is the exact shape that exposed the
 * bug the review caught: re-ticking the checkbox against an already-closed
 * `[A, B, A]` list could append a SECOND closing point in Rust -- proving
 * that end to end needs a live `route_direct` call to OSRM, which this file
 * deliberately never makes (see the header). The guard itself is pinned
 * directly in Rust
 * (`round_trip_does_not_double_close_an_already_closed_route` in
 * `route_maps_tests.rs`); this fixture instead lets the UI-level test below
 * pin what CAN be asserted without the network -- that the reopened state
 * renders the persisted round trip correctly.
 */
async function saveDirectRoundTrip(tripId: string, targetKm: number): Promise<void> {
  await rpc<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_ROUND_TRIP_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm: targetKm,
    mode: 'direct',
    roundTrip: true,
  });
}

/**
 * Persist a plain one-way direct route against a trip, without touching
 * OSRM. The mirror fixture to `saveDirectRoundTrip` above -- explicitly
 * sends `roundTrip: false` (rather than omitting it) so the test below pins
 * the OTHER direction of the persisted flag: a saved one-way route must not
 * come back ticked, the same way a saved round trip must not come back
 * unticked (Task 20).
 */
async function saveDirectOneWay(tripId: string, targetKm: number): Promise<void> {
  await rpc<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm: targetKm,
    mode: 'direct',
    roundTrip: false,
  });
}

/** `get_trip_route` returns null when the trip has no saved route. */
async function getRoute(tripId: string): Promise<SavedRouteMap | null> {
  return rpc<SavedRouteMap | null>('get_trip_route', { tripId });
}

async function deleteRoute(tripId: string): Promise<void> {
  await rpc<null>('delete_trip_route', { tripId });
}

/**
 * Open the map view the same way production does -- `TripGrid` opens
 * `/mapa?trip=<id>` via `window.open`, so a direct navigation to that URL is
 * the real entry point, not a shortcut around it.
 */
async function openMap(tripId: string): Promise<void> {
  await browser.url(`/mapa?trip=${tripId}`);
}

/**
 * Path elements Leaflet's SVG renderer draws for polyline layers, scoped to
 * the overlay pane. A plain `path` selector under the canvas also matches
 * the attribution control's own flag icon -- three `<path>`s that exist on
 * every map instance whether or not a route is drawn -- so counting those
 * would make this helper pass even on a blank map.
 */
async function drawnPathCount(): Promise<number> {
  const paths = await $$('[data-test="route-map-canvas"] .leaflet-overlay-pane path');
  return paths.length;
}

/** Wait until the map view has either rendered a loaded route or reported an
 *  error -- whichever this fixture is expected to reach without a network
 *  call. Polls instead of pausing so a slow render never turns into flake. */
async function waitForMapOutcome(kind: 'route' | 'error'): Promise<void> {
  const selector = kind === 'route' ? '[data-test="deviation"]' : '[data-test="route-map-error"]';
  const el = await $(selector);
  await el.waitForDisplayed({ timeout: 10000 });
}

/** Wait for the shared place dialog (PlaceModal) to appear in place, and
 *  return the field name it is asking about -- read off `data-place-name`,
 *  the attribute the modal stamps with `place.displayName` verbatim, rather
 *  than parsing the header text mixed in with i18n copy. */
async function waitForPlaceDialog(): Promise<string | null> {
  const modal = await $('[data-testid="place-modal"]');
  await modal.waitForDisplayed({ timeout: 10000 });
  return modal.getAttribute('data-place-name');
}

/** Reload the grid so it re-reads `routeMapTripIds` from the backend. */
async function reloadTripGrid(): Promise<void> {
  await browser.refresh();
  await waitForAppReady();
  await navigateTo('trips');
  await waitForTripGrid();
}

/** Whatever `element.$(...)` hands back in this WebdriverIO version. */
type GridElement = ReturnType<WebdriverIO.Element['$']>;

/**
 * Find the map pin for the row whose destination cell matches, so assertions
 * are tied to a specific trip instead of a row index. Returns null when the row
 * exists but renders no pin (capability off) or when the row isn't there yet.
 */
async function findMapPin(destination: string): Promise<GridElement | null> {
  const rows = await $$('.trip-grid tbody tr');
  for (const row of rows) {
    const cell = row.$('td.col-destination');
    if (!(await cell.isExisting())) continue;
    if ((await cell.getText()).trim() !== destination) continue;

    const pin = row.$(MAP_PIN);
    return (await pin.isExisting()) ? pin : null;
  }
  return null;
}

/** True when the pin carries the saved-route marker class. */
async function pinShowsSavedState(pin: GridElement): Promise<boolean> {
  const classes = (await pin.getAttribute('class')) ?? '';
  return classes.split(/\s+/).includes('has-map');
}

/**
 * Wait until the pin for `destination` reports the expected saved state.
 * Polls instead of pausing so a slow grid reload doesn't turn into flake.
 */
async function waitForPinState(destination: string, saved: boolean): Promise<void> {
  await browser.waitUntil(
    async () => {
      const pin = await findMapPin(destination);
      if (!pin) return false;
      return (await pinShowsSavedState(pin)) === saved;
    },
    {
      timeout: 10000,
      timeoutMsg: `Map pin for '${destination}' never reached saved=${saved}`,
    }
  );
}

/**
 * Route maps render from commands the server dispatches directly; `GET
 * /api/capabilities` reports `route_maps: true`. This spec used to be skipped
 * under the desktop build, which registered no wrappers for these commands.
 */
describe('Tier 2: Route Map', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    const vehicle = await seedVehicle({
      name: 'Route Map Test Vehicle',
      licensePlate: 'MAP-001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  describe('Map Action Visibility', () => {
    it('should show the map action on a trip row', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: '2026-03-10T08:00',
        endDatetime: '2026-03-10T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 60,
        odometer: 50060,
        purpose: 'Business trip',
      });

      await reloadTripGrid();

      // Present because the server dispatches the route-map commands — this
      // fails if they regress.
      const pin = await findMapPin('Trnava');
      expect(pin).not.toBeNull();
      expect(await pin!.isDisplayed()).toBe(true);

      // A trip with no route yet renders the unsaved (outline) pin.
      expect(await pinShowsSavedState(pin!)).toBe(false);
    });
  });

  describe('Saved Route Display', () => {
    it('should mark only the mapped row once a route is saved', async () => {
      const mapped = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-10T08:00',
        endDatetime: '2026-03-10T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 60,
        odometer: 50060,
        purpose: 'Business trip',
      });
      await seedTrip({
        vehicleId,
        startDatetime: '2026-03-11T08:00',
        endDatetime: '2026-03-11T09:00',
        origin: 'Trnava',
        destination: 'Nitra',
        distanceKm: 50,
        odometer: 50110,
        purpose: 'Business trip',
      });

      await saveRoute(mapped.id as string, 60);
      await reloadTripGrid();

      // The grid's routeMapTripIds must mark the mapped row and nothing else.
      await waitForPinState('Trnava', true);
      await waitForPinState('Nitra', false);
    });
  });

  describe('Route Removal', () => {
    it('should clear the saved state when the route is deleted', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-10T08:00',
        endDatetime: '2026-03-10T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 60,
        odometer: 50060,
        purpose: 'Business trip',
      });
      const tripId = trip.id as string;

      await saveRoute(tripId, 60);
      await reloadTripGrid();
      await waitForPinState('Trnava', true);

      await deleteRoute(tripId);
      expect(await getRoute(tripId)).toBeNull();

      await reloadTripGrid();
      await waitForPinState('Trnava', false);
    });
  });

  describe('Trip Deletion Cascade', () => {
    /**
     * The trip_routes → trips foreign key is declared ON DELETE CASCADE, but it
     * only fires when the connection has `PRAGMA foreign_keys` enabled. That is
     * a property of the real runtime connection, not of the schema, so it is
     * worth observing through the live RPC boundary here: an orphaned route map
     * would otherwise resurface against a recycled trip id.
     */
    it('should remove the route map when the trip is deleted', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-10T08:00',
        endDatetime: '2026-03-10T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 60,
        odometer: 50060,
        purpose: 'Business trip',
      });
      const tripId = trip.id as string;

      await saveRoute(tripId, 60);
      expect(await getRoute(tripId)).not.toBeNull();

      await deleteTrip(tripId);

      expect(await getRoute(tripId)).toBeNull();
    });
  });

  /**
   * The map view (Task 72, Phase 2 / V2). Only flows that start from an
   * already-saved route, or fail before the first network request, are
   * reachable here -- see constraint 3 in the header comment.
   */
  describe('Map View (V2, offline-reachable flows)', () => {
    // A previous test in this block leaves the browser on /mapa?trip=<id>.
    // WDIO's own beforeTest only waits for *an* h1 -- the map page has one
    // too -- so without this, the next test's seeding refreshes would keep
    // landing back on a stale map URL instead of the trip grid.
    afterEach(async () => {
      await browser.url('/');
      await waitForAppReady();
    });

    it('renders a saved direct route with a via and offers direct-mode controls', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-12T08:00',
        endDatetime: '2026-03-12T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50065,
        purpose: 'Business trip',
      });

      await saveDirectRouteWithVia(trip.id as string, 65);
      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      // A route is actually drawn, not just the info panel around it.
      expect(await drawnPathCount()).toBeGreaterThan(0);

      const deviationText = await $('[data-test="deviation"]').getText();
      expect(deviationText).toMatch(/%/);

      // Loop-only control is absent; direct-mode's own control is present.
      expect(await $('[data-test="regenerate-btn"]').isExisting()).toBe(false);
      expect(await $('[data-test="recalculate-btn"]').isDisplayed()).toBe(true);

      // The saved route already has a via -- the "alternatives unavailable"
      // branch (I2, _tasks/72-route-map-origin-destination/_plan-review.md)
      // is reachable on a cold load, not only right after a fresh proposal.
      expect(await $('[data-test="alternatives-unavailable"]').isDisplayed()).toBe(true);
      expect(await $('[data-test="alternatives"]').isExisting()).toBe(false);
    });

    it('reopens an already-closed round trip without corrupting its stop count', async () => {
      // Task 19, fix round 1 (review finding "Important 1"): reopening a
      // saved round trip yields an already-closed `[A, B, A]` list. Ticking
      // the checkbox again and re-routing that list is what exposed the bug
      // -- Rust appended a SECOND closing point, `[A, B, A, A]`. Proving that
      // exact click end to end needs a live `route_direct` call to OSRM,
      // which this file deliberately never makes (see the header); Rust's
      // own idempotence guard is pinned directly in
      // `round_trip_does_not_double_close_an_already_closed_route`
      // (route_maps_tests.rs). What this test CAN pin without the network is
      // the reopened state itself: the persisted round trip renders with its
      // correct (un-doubled) stop count, the checkbox comes back TICKED
      // because the flag is now persisted (Task 20, reversing design
      // decision 5), and the alternatives-unavailable copy (fix round 1,
      // "Important 2") now reads true for a plain round trip that has no via
      // at all.
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-16T08:00',
        endDatetime: '2026-03-16T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50165,
        purpose: 'Business trip',
      });

      await saveDirectRoundTrip(trip.id as string, 65);
      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      expect(await drawnPathCount()).toBeGreaterThan(0);

      // Three stops, closing back on the origin's own name -- not silently
      // dropped to two, and not doubled to four.
      const stopsText = await $('[data-test="stops"]').getText();
      expect(stopsText).toContain('(3)');
      expect(stopsText).toContain('Bratislava → Trnava → Bratislava');

      // Persisted (Task 20): reopening restores the checkbox from the saved
      // flag, so it comes back ticked -- not always unticked as before.
      const checkbox = await $('[data-test="round-trip-checkbox"]');
      expect(await checkbox.isExisting()).toBe(true);
      expect(await checkbox.isSelected()).toBe(true);

      // The corrected copy names the real condition (more than two points),
      // which is true here even though this route has no via -- only a
      // return leg. The old wording named intermediate stops as the cause,
      // which was false for exactly this shape.
      expect(await $('[data-test="alternatives-unavailable"]').isDisplayed()).toBe(true);
      const unavailableText = await $('[data-test="alternatives-unavailable"]').getText();
      expect(unavailableText).toContain('more than two points');
      expect(await $('[data-test="alternatives"]').isExisting()).toBe(false);
    });

    it('reopens a saved one-way route with the checkbox unticked', async () => {
      // The other half of the guard above: a saved round trip must come back
      // ticked, but a saved ONE-WAY route must come back unticked. Without
      // this test, an implementation that hardcoded the checkbox to `true`
      // on load (or ignored the stored flag and always ticked it) would
      // still pass "reopens an already-closed round trip...", since that
      // test only ever seeds `roundTrip: true`. This test seeds
      // `roundTrip: false` explicitly and checks the box reflects it.
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-17T08:00',
        endDatetime: '2026-03-17T10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50265,
        purpose: 'Business trip',
      });

      await saveDirectOneWay(trip.id as string, 65);
      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      expect(await drawnPathCount()).toBeGreaterThan(0);

      const stopsText = await $('[data-test="stops"]').getText();
      expect(stopsText).toContain('(2)');

      const checkbox = await $('[data-test="round-trip-checkbox"]');
      expect(await checkbox.isExisting()).toBe(true);
      expect(await checkbox.isSelected()).toBe(false);
    });

    it('still renders a saved loop route with the V1 controls', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-13T08:00',
        endDatetime: '2026-03-13T09:00',
        origin: 'Bratislava',
        destination: 'Bratislava',
        distanceKm: 40,
        odometer: 50040,
        purpose: 'Business trip',
      });

      await saveRoute(trip.id as string, 40);
      await openMap(trip.id as string);
      await waitForMapOutcome('route');

      expect(await drawnPathCount()).toBeGreaterThan(0);

      // Regression guard for "loop mode, unchanged": Generovat/Regenerate
      // stays, Recalculate (direct-only) does not appear.
      expect(await $('[data-test="regenerate-btn"]').isDisplayed()).toBe(true);
      expect(await $('[data-test="recalculate-btn"]').isExisting()).toBe(false);

      // Guards the hasVias fix above: a loop route's own multi-point
      // waypoint list must never light up the direct-only alternatives copy.
      expect(await $('[data-test="alternatives-unavailable"]').isExisting()).toBe(false);
    });

    it('reports a blank destination and draws nothing, without offering retry', async () => {
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-14T08:00',
        endDatetime: '2026-03-14T09:00',
        origin: 'Bratislava',
        destination: '',
        distanceKm: 10,
        odometer: 50010,
        purpose: 'Business trip',
      });

      await openMap(trip.id as string);
      await waitForMapOutcome('error');

      // The page actually mounted with the real trip, not a blank screen.
      expect(await $('[data-test="trip-summary"]').isDisplayed()).toBe(true);
      const errorText = await $('[data-test="route-map-error"]').getText();
      expect(errorText).toContain('no origin or destination');

      expect(await drawnPathCount()).toBe(0);
      // mode_for's validation failure is a data problem, not a transient
      // one -- retrying would fail identically forever.
      expect(await $('[data-test="retry-btn"]').isExisting()).toBe(false);
    });

    it('opens the shared place dialog for a row with an unplaced endpoint, in place', async () => {
      // Neither name has ever been placed (see the header comment for why
      // that is safe to assume across this suite): start_route_for_trip is a
      // place-book lookup (ADR-032), never a geocode, and it checks origin
      // before destination -- so a trip with both endpoints unplaced reaches
      // exactly the same branch a real first-time user hits, and resolves
      // deterministically on 'origin' without needing to single one out.
      const trip = await seedTrip({
        vehicleId,
        startDatetime: '2026-03-15T08:00',
        endDatetime: '2026-03-15T09:00',
        origin: 'Bratislava',
        destination: 'Kosice',
        distanceKm: 400,
        odometer: 50400,
        purpose: 'Business trip',
      });

      await openMap(trip.id as string);
      const placeName = await waitForPlaceDialog();
      expect(placeName).toBe('Bratislava');

      // In place: still the same map view, same trip, no error state --
      // route_direct was never reached to produce one either way.
      expect(await $('[data-test="route-map-page"]').isDisplayed()).toBe(true);
      expect(await $('[data-test="trip-summary"]').isDisplayed()).toBe(true);
      expect(await $('[data-test="route-map-error"]').isExisting()).toBe(false);

      // Nothing was routed yet -- confirms the page really did stop before
      // route_direct, not just that the dialog happens to render on top.
      expect(await drawnPathCount()).toBe(0);
    });
  });
});
