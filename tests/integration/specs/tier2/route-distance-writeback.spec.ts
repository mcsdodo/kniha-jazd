/**
 * Tier 2: "Uložiť a použiť vzdialenosť" -- a map save writes its distance to
 * the trip (Task 78, Task 87).
 *
 * One button saves the route and writes its whole-km distance to the trip, in
 * every route mode, behind the dry-run modal. Routing runs offline here:
 * `KNIHA_JAZD_MOCK_ROUTER` answers every route (OSRM, the default: 90.0 km),
 * so the button, its modal and the write are all reachable without the network.
 *
 * NOT covered here on purpose: the rounding, the period rate, the margin and
 * the odometer cascade are proven in Rust (`logbook_km`, `plan_route_distance`
 * and `period_margin_impact` in commands_tests.rs, the save commands in
 * route_maps_tests.rs). This file proves the UI reaches them and shows what
 * they answered.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, rpc, updateTrip } from '../../utils/db';

const CANNED_POLYLINE = 'w_{dHcjlgBg}L{pd@wv]_bw@';
const CANNED_WAYPOINTS = [
  { lat: 48.1486, lon: 17.1077, name: 'Bratislava' },
  { lat: 48.3774, lon: 17.5872, name: 'Trnava' },
];

/** Persist a one-way direct route whose road km equals the trip km (no trip change). */
async function saveSyncedRoute(tripId: string, km: number) {
  await rpc('save_trip_route', {
    tripId,
    waypoints: CANNED_WAYPOINTS,
    polyline: CANNED_POLYLINE,
    roadKm: km,
    mode: 'direct',
    roundTrip: false,
    dryRun: false,
  });
}

/** A map saved before task 87, or a trip km edited after the save. */
async function editTripKm(trip: Record<string, unknown>, distanceKm: number) {
  await updateTrip({ ...trip, id: trip.id, distanceKm });
}

async function getTrip(vehicleId: string, tripId: unknown) {
  const trips = await rpc<Array<Record<string, unknown>>>('get_trips', { vehicleId });
  return trips.find((t) => t.id === tripId)!;
}

async function getRoute(tripId: unknown) {
  return rpc<Record<string, unknown> | null>('get_trip_route', { tripId });
}

/** Route the shown trip again through the mock (OSRM: 90.0 km). */
async function recalculate() {
  await $('[data-test="recalculate-btn"]').click();
  await expect($('[data-test="actual-km"]')).toHaveText('90.0 km');
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

  it('saves the map and writes its distance, shifting the later rows', async () => {
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

    await saveSyncedRoute(first.id as string, 50);
    await openMap(first.id as string);
    await recalculate();

    await $('[data-test="save-apply-btn"]').click();

    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    const summary = await $('[data-testid="cascade-summary"]').getText();
    expect(summary).toContain('50');
    expect(summary).toContain('90');

    await $('[data-testid="cascade-confirm"]').click();
    await modal.waitForDisplayed({ timeout: 5000, reverse: true });
    await $('[data-test="saved-notice"]').waitForDisplayed();

    // The row moved, and so did the one after it.
    const a = await getTrip(vehicleId, first.id);
    const b = await getTrip(vehicleId, second.id);
    expect(a.distanceKm).toBe(90);
    expect(a.odometer).toBe(50090);
    expect(b.odometer).toBe(50130);
    // The map was saved in the same step.
    expect((await getRoute(first.id))?.roadKm).toBe(90);

    // The map now measures against the new distance, so the deviation is gone.
    await browser.waitUntil(
      async () => (await $('[data-test="target-km"]').getText()).includes('90.0'),
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

    await saveSyncedRoute(trip.id as string, 50);
    await openMap(trip.id as string);
    await recalculate();

    await $('[data-test="save-apply-btn"]').click();
    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    await $('[data-testid="cascade-cancel"]').click();
    await modal.waitForDisplayed({ timeout: 5000, reverse: true });

    expect((await getTrip(vehicleId, trip.id)).distanceKm).toBe(50);
    // The proposal was not saved either: the stored map is the old one.
    expect((await getRoute(trip.id))?.roadKm).toBe(50);
  });

  it('saves at once when the distance already matches', async () => {
    const trip = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-04T08:00',
      endDatetime: '2026-04-04T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 90,
      odometer: 50090,
      purpose: 'Business trip',
    });

    await saveSyncedRoute(trip.id as string, 90);
    await openMap(trip.id as string);
    await recalculate();

    await $('[data-test="save-apply-btn"]').click();
    await $('[data-test="saved-notice"]').waitForDisplayed({ timeout: 5000 });
    expect(await $('[data-testid="cascade-modal"]').isExisting()).toBe(false);
  });

  it("writes a loop route's distance too", async () => {
    const trip = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-05T08:00',
      endDatetime: '2026-04-05T10:00',
      origin: 'Bratislava',
      destination: 'Bratislava',
      distanceKm: 37,
      odometer: 50037,
      purpose: 'Business trip',
    });

    // No saved map: a loop generates at once through the mock (OSRM, 90 km).
    await openMap(trip.id as string);
    await expect($('[data-test="actual-km"]')).toHaveText('90.0 km');

    await $('[data-test="save-apply-btn"]').click();
    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    await $('[data-testid="cascade-confirm"]').click();
    await $('[data-test="saved-notice"]').waitForDisplayed();

    expect((await getTrip(vehicleId, trip.id)).distanceKm).toBe(90);
    expect((await getRoute(trip.id))?.mode).toBe('loop');
  });

  it('syncs a saved map whose trip km was edited later', async () => {
    const trip = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-06T08:00',
      endDatetime: '2026-04-06T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 100,
      odometer: 50100,
      purpose: 'Business trip',
    });
    await saveSyncedRoute(trip.id as string, 100);
    await editTripKm(trip as unknown as Record<string, unknown>, 80);

    await openMap(trip.id as string);

    const button = await $('[data-test="save-apply-btn"]');
    await expect(button).toBeEnabled();
    await button.click();
    const modal = await $('[data-testid="cascade-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });
    await $('[data-testid="cascade-confirm"]').click();
    await $('[data-test="saved-notice"]').waitForDisplayed();

    expect((await getTrip(vehicleId, trip.id)).distanceKm).toBe(100);
  });

  it('disables the button on a saved map that is in sync', async () => {
    const trip = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-07T08:00',
      endDatetime: '2026-04-07T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 100,
      odometer: 50100,
      purpose: 'Business trip',
    });
    await saveSyncedRoute(trip.id as string, 100);

    await openMap(trip.id as string);

    await expect($('[data-test="save-apply-btn"]')).toBeDisabled();
  });

  it('names the 20 % legal limit before it is crossed', async () => {
    // 100 km, then 100 km closing on 12 litres: 200 km on 12 l is 6.0
    // l/100km, exactly 20 % over the 5.0 l/100km TP rate. Shortening the
    // first row to 90 km pushes the same 12 litres over 190 km, which is
    // 26.3 % -- over the limit.
    const first = await seedTrip({
      vehicleId,
      startDatetime: '2026-04-08T08:00',
      endDatetime: '2026-04-08T10:00',
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 100,
      odometer: 50100,
      purpose: 'Business trip',
    });
    await seedTrip({
      vehicleId,
      startDatetime: '2026-04-09T08:00',
      endDatetime: '2026-04-09T10:00',
      origin: 'Trnava',
      destination: 'Bratislava',
      distanceKm: 100,
      odometer: 50200,
      purpose: 'Business trip',
      fuelLiters: 12,
      fullTank: true,
    });

    await saveSyncedRoute(first.id as string, 100);
    await openMap(first.id as string);
    await recalculate();

    await $('[data-test="save-apply-btn"]').click();
    await $('[data-testid="cascade-modal"]').waitForDisplayed({ timeout: 5000 });

    expect(await $('[data-testid="writeback-margin"]').isDisplayed()).toBe(true);
    expect(await $('[data-testid="writeback-crosses-limit"]').isDisplayed()).toBe(true);
    const marginText = await $('[data-testid="writeback-margin"]').getText();
    expect(marginText).toContain('20.0');
    expect(marginText).toContain('26.3');
  });
});
