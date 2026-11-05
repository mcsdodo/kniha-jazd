/**
 * Tier 2: the route of a trip added by hand on the phone (Task 94)
 *
 * /m/nova shows the route between the two places on a map, offers the
 * routing services the server has, and saves the chosen route with the trip.
 * The mock router (KNIHA_JAZD_MOCK_ROUTER) answers OSRM with 90 km and Sygic
 * with 100 km, one route each. The alternatives and the km math are in the
 * backend tests (mobile_tests.rs); this spec checks the page flow only.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { rpc, seedVehicle, setActiveVehicle } from '../../utils/db';

interface Trip {
  id: string;
  distanceKm: number;
}
interface RouteMap {
  provider: string | null;
  roadKm: number;
  coordinates: [number, number][];
}

/** Pick a place in one of the two pickers of /m/nova. */
async function pick(testId: string, placeId: string): Promise<void> {
  const select = await $(`[data-testid="${testId}-select"]`);
  await select.waitForDisplayed({ timeout: 10000 });
  await select.selectByAttribute('value', placeId);
}

describe('Tier 2: phone manual trip route', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  it('shows the route, switches the routing service and saves the route with the trip', async () => {
    const vehicle = await seedVehicle({
      name: 'Phone Route Test',
      licensePlate: 'MOB-094',
      initialOdometer: 50000,
      vehicleType: 'Ice',
      tankSizeLiters: 50,
      tpConsumption: 6.0,
    });
    const vehicleId = vehicle.id as string;
    const home = await rpc<{ id: string }>('create_place', {
      name: 'Route Test Home',
      lat: 48.1486,
      lon: 17.1077,
      source: 'manual',
    });
    const office = await rpc<{ id: string }>('create_place', {
      name: 'Route Test Office',
      lat: 48.3774,
      lon: 17.5872,
      source: 'manual',
    });

    await setActiveVehicle(vehicleId);
    // "Kam" reads the GPS when the page opens: stand at the office, so the
    // picker selects it (a place within 300 m).
    await browser.emulate('geolocation', { latitude: 48.3774, longitude: 17.5872, accuracy: 10 });
    await browser.url('/m/nova');
    await $('[data-testid="m-nova"]').waitForDisplayed({ timeout: 15000 });

    const destination = await $('[data-testid="m-nova-destination-select"]');
    await browser.waitUntil(async () => (await destination.getValue()) === office.id, {
      timeout: 15000,
      timeoutMsg: 'the GPS did not select the office as Kam',
    });
    await pick('m-nova-origin', home.id);

    // The map and the km of the default service (mock OSRM: 90 km).
    await $('[data-testid="m-nova-map"]').waitForDisplayed({ timeout: 10000 });
    const km = await $('[data-testid="m-nova-km"]');
    await browser.waitUntil(async () => (await km.getValue()) === '90', {
      timeout: 10000,
      timeoutMsg: 'km did not fill from the OSRM route',
    });
    // One route only: no alternative chips.
    expect(await $('[data-testid="m-nova-alternatives"]').isExisting()).toBe(false);

    // The mock offers two services: switch to Sygic (100 km).
    const sygic = await $('[data-testid="m-nova-provider-sygic"]');
    await sygic.waitForDisplayed({ timeout: 10000 });
    await sygic.click();
    await browser.waitUntil(async () => (await km.getValue()) === '100', {
      timeout: 10000,
      timeoutMsg: 'km did not change to the Sygic route',
    });
    expect(await sygic.getAttribute('aria-pressed')).toBe('true');

    await $('[data-testid="m-nova-save"]').click();
    await browser.waitUntil(async () => (await browser.getUrl()).endsWith('/m'), {
      timeout: 15000,
      timeoutMsg: 'the page did not return to /m after the save',
    });
    expect(await $('.toast-error').isExisting()).toBe(false);

    // The trip has the route the page chose.
    const year = new Date().getFullYear();
    const trips = await rpc<Trip[]>('get_trips_for_year', { vehicleId, year });
    expect(trips.length).toBe(1);
    expect(trips[0].distanceKm).toBe(100);
    const route = await rpc<RouteMap | null>('get_trip_route', { tripId: trips[0].id });
    expect(route).not.toBeNull();
    expect(route!.provider).toBe('sygic');
    expect(route!.roadKm).toBe(100);
  });
});
