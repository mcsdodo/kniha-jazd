/**
 * Tier 2: Home place mark on the Miesta tab (task 89)
 *
 * Covers the UI -> backend -> display flow of the home icon:
 * - A click on the home icon of a place marks it, and a reload keeps the mark
 * - A click on another place moves the mark
 * - A click on the current home removes the mark
 *
 * NOT covered here: the one-home rule and the journey grouping. The Rust unit
 * tests own them (db_tests.rs, journeys/tests.rs).
 */

import { waitForAppReady } from '../../utils/app';
import { seedVehicle, seedTrip, setActiveVehicle, rpc, ensurePlace } from '../../utils/db';

const HOME = 'Home St 1, Hometown';
const CITY = 'City A, Testland';

async function openMiesta(): Promise<void> {
  const link = await $('[data-testid="nav-places"]');
  await link.click();
  await $('[data-testid="place-row"]').waitForDisplayed({ timeout: 5000 });
}

function homeToggle(id: string) {
  return $(`[data-testid="place-row"][data-place-id="${id}"] [data-testid="place-home-toggle"]`);
}

describe('Home place mark', () => {
  beforeEach(async () => {
    await waitForAppReady();
    const vehicle = await seedVehicle({
      name: 'Home Test Car',
      licensePlate: 'HOME-1',
      initialOdometer: 10000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);
    await seedTrip({
      vehicleId: vehicle.id as string,
      startDatetime: `${new Date().getFullYear()}-03-02T07:00`,
      origin: HOME,
      destination: CITY,
      distanceKm: 120,
      odometer: 10120,
      purpose: 'Test',
    });
    await rpc('set_home_place', { id: null });
  });

  it('marks a place as home and keeps the mark after a reload', async () => {
    const homeId = await ensurePlace(HOME);
    await openMiesta();

    await (await homeToggle(homeId)).click();
    await browser.waitUntil(
      async () => (await (await homeToggle(homeId)).getAttribute('aria-pressed')) === 'true',
      { timeout: 5000, timeoutMsg: 'home toggle did not switch on' }
    );

    await browser.refresh();
    await $('[data-testid="place-row"]').waitForDisplayed({ timeout: 5000 });
    expect(await (await homeToggle(homeId)).getAttribute('aria-pressed')).toBe('true');
  });

  it('moves the mark to another place', async () => {
    const homeId = await ensurePlace(HOME);
    const cityId = await ensurePlace(CITY);
    await rpc('set_home_place', { id: homeId });
    await openMiesta();

    await (await homeToggle(cityId)).click();
    await browser.waitUntil(
      async () => (await (await homeToggle(cityId)).getAttribute('aria-pressed')) === 'true',
      { timeout: 5000, timeoutMsg: 'home mark did not move' }
    );
    expect(await (await homeToggle(homeId)).getAttribute('aria-pressed')).toBe('false');
  });

  it('removes the mark when the current home is clicked', async () => {
    const homeId = await ensurePlace(HOME);
    await rpc('set_home_place', { id: homeId });
    await openMiesta();
    // The previous test can leave the page on /miesta, and the nav click does
    // not reload it. Reload, so the page shows the mark set above.
    await browser.refresh();
    await $('[data-testid="place-row"]').waitForDisplayed({ timeout: 5000 });
    expect(await (await homeToggle(homeId)).getAttribute('aria-pressed')).toBe('true');

    await (await homeToggle(homeId)).click();
    await browser.waitUntil(
      async () => (await (await homeToggle(homeId)).getAttribute('aria-pressed')) === 'false',
      { timeout: 5000, timeoutMsg: 'home mark was not removed' }
    );
    const places = await rpc<Array<{ isHome: boolean }>>('list_places', {});
    expect(places.some((p) => p.isHome)).toBe(false);
  });
});
