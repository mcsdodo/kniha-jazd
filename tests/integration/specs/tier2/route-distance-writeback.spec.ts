/**
 * Tier 2: writing a routed distance back onto a trip (Task 78).
 *
 * This flow is fully testable here, unlike leg routing: a SAVED route is drawn
 * with no call to the routing service, so the Apply button, its modal and the
 * write are all reachable without the network. The same constraint as
 * route-map.spec.ts still applies -- nothing in this file calls
 * `generate_route`, `route_direct` or `route_round_trip`.
 *
 * NOT covered here on purpose: the period rate, the margin and the odometer
 * cascade are proven in Rust (`period_margin_impact` and
 * `apply_route_distance_internal` in commands_tests.rs). This file proves the
 * UI reaches them and shows what they answered.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, rpc } from '../../utils/db';

const CANNED_POLYLINE = 'w_{dHcjlgBg}L{pd@wv]_bw@';
const CANNED_WAYPOINTS = [
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
  { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
];

/** Persist a one-way direct route whose road distance differs from the row. */
async function saveRouteWithRoadKm(tripId: string, targetKm: number, roadKm: number) {
  await rpc<null>('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    targetKm,
    roadKm,
    mode: 'direct',
    roundTrip: false,
  });
}

/**
 * Open the map view the same way production does -- `TripGrid` opens
 * `/mapa?trip=<id>` via `window.open`, so a direct navigation to that URL is
 * the real entry point, not a shortcut around it (mirrors route-map.spec.ts).
 */
async function openMap(tripId: string) {
  await browser.url(`/mapa?trip=${tripId}`);
  await $('[data-test="route-map-page"]').waitForExist({ timeout: 10000 });
  await $('[data-test="actual-km"]').waitForDisplayed({ timeout: 15000 });
}

describe('Route distance write-back', () => {
  let vehicleId: string;

  before(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  beforeEach(async () => {
    const vehicle = await seedVehicle({
      name: 'Writeback Car',
      licensePlate: 'BA-WB-1',
      tankSizeLiters: 60,
      tpConsumption: 5.0,
      initialOdometer: 50000,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  it('writes the routed distance onto the trip and shifts the later rows', async () => {
    const first = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-01T08:00',
      endDatetime: '2026-04-01T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 50050,
      purpose: 'Business trip',
    });
    const second = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-02T08:00',
      endDatetime: '2026-04-02T10:00',
      origin: 'Trnava',
      destination: 'Bratislava',
      distanceKm: 40,
      odometer: 50090,
      purpose: 'Business trip',
    });

    await saveRouteWithRoadKm(first.id as string, 50, 61.5);
    await openMap(first.id as string);

    await $('[data-test="apply-distance-btn"]').click();

    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    const summary = await $('[data-testid="cascade-summary"]').getText();
    expect(summary).toContain('50');
    expect(summary).toContain('61.5');

    await $('[data-testid="cascade-confirm"]').click();
    await modal.waitForDisplayed({ timeout: 5000, reverse: true });

    // The row moved, and so did the one after it.
    const trips = await rpc<Array<Record<string, unknown>>>('get_trips', { vehicleId });
    const a = trips.find((t) => t.id === first.id)!;
    const b = trips.find((t) => t.id === second.id)!;
    expect(a.distanceKm).toBe(61.5);
    expect(a.odometer).toBe(50061.5);
    expect(b.odometer).toBe(50101.5);

    // The map now measures against the new distance, so the deviation is gone.
    await browser.waitUntil(
      async () => (await $('[data-test="target-km"]').getText()).includes('61.5'),
      { timeout: 5000, timeoutMsg: 'the target distance did not follow the write' }
    );
    expect(await $('[data-test="deviation"]').getText()).toContain('0.0');
  });

  it('writes nothing when the confirmation is dismissed', async () => {
    const trip = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-03T08:00',
      endDatetime: '2026-04-03T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 50050,
      purpose: 'Business trip',
    });

    await saveRouteWithRoadKm(trip.id as string, 50, 61.5);
    await openMap(trip.id as string);

    await $('[data-test="apply-distance-btn"]').click();
    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    await $('[data-testid="cascade-cancel"]').click();
    await modal.waitForDisplayed({ timeout: 5000, reverse: true });

    const trips = await rpc<Array<Record<string, unknown>>>('get_trips', { vehicleId });
    expect(trips.find((t) => t.id === trip.id)!.distanceKm).toBe(50);
  });

  it('names the 20 % legal limit before it is crossed', async () => {
    // 100 km, then 100 km closing on 12 litres: 200 km on 12 l is 6.0
    // l/100km, exactly 20 % over the 5.0 l/100km TP rate. Shortening the
    // first row to 90 km pushes the same 12 litres over 190 km, which is
    // 26.3 % -- over the limit.
    const first = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-04T08:00',
      endDatetime: '2026-04-04T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 100,
      odometer: 50100,
      purpose: 'Business trip',
    });
    await seedTrip({
      vehicleId,
      startDatetime: '2026-04-05T08:00',
      endDatetime: '2026-04-05T10:00',
      origin: 'Trnava',
      destination: 'Bratislava',
      distanceKm: 100,
      odometer: 50200,
      purpose: 'Business trip',
      fuelLiters: 12,
      fullTank: true,
    });

    await saveRouteWithRoadKm(first.id as string, 100, 90);
    await openMap(first.id as string);

    await $('[data-test="apply-distance-btn"]').click();
    await $('[data-testid="cascade-modal"]').waitForDisplayed({ timeout: 5000 });

    expect(await $('[data-testid="writeback-margin"]').isDisplayed()).toBe(true);
    expect(await $('[data-testid="writeback-crosses-limit"]').isDisplayed()).toBe(true);
    const marginText = await $('[data-testid="writeback-margin"]').getText();
    expect(marginText).toContain('20.0');
    expect(marginText).toContain('26.3');
  });
});
