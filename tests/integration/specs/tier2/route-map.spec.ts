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
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
  deleteTrip,
  invokeTauri,
} from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';
import { describeNotInTauriMode } from '../../utils/skip';

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

/** Only the identity fields matter here — the tests assert presence, not values. */
interface SavedRouteMap {
  tripId: string;
  polyline: string;
}

/** Persist a route against a trip without touching OSRM. */
async function saveRoute(tripId: string, targetKm: number): Promise<void> {
  await invokeTauri<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm: targetKm,
  });
}

/** `get_trip_route` returns null when the trip has no saved route. */
async function getRoute(tripId: string): Promise<SavedRouteMap | null> {
  return invokeTauri<SavedRouteMap | null>('get_trip_route', { tripId });
}

async function deleteRoute(tripId: string): Promise<void> {
  await invokeTauri<null>('delete_trip_route', { tripId });
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
 * Skipped under the desktop/Tauri config: `GET /api/capabilities` reports
 * `route_maps: true` only in server mode, and the desktop build registers no
 * Tauri wrappers for the route-map commands at all — so there the pin does not
 * render and the commands do not exist.
 */
describeNotInTauriMode('Tier 2: Route Map', () => {
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

      // Present because the server reports route_maps: true — this fails if the
      // capability guard or the capabilities endpoint regresses.
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
});
