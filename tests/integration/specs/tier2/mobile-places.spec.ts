/**
 * Tier 2: the place list on the phone (Task 96)
 *
 * /m/miesta lists the places of the database with a filter. This spec checks
 * the start page button, the card and the diacritics-blind filter.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { ensurePlace, rpc, seedVehicle, setActiveVehicle } from '../../utils/db';

interface Place {
  id: string;
  name: string;
  lat: number | null;
  lon: number | null;
  source: string | null;
  isHome: boolean;
}

/** Every place name this spec creates; deleted again before each test. */
const SPEC_PLACES = [
  'Alpha Depot, Testville',
  'Košice Sklad',
  'Lambda Gate, Testville',
  'Beta Shed, Testville',
  'Beta Shed North, Testville',
  'Theta Quarry, Testville',
  'Kappa Lot, Testville',
  'Mu Flat, Testville',
  'Nu Gate, Testville',
  'Xi Gate, Testville',
  'Omicron Pier, Testville',
];

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

/** Wait for the card whose name is `name` and return it. */
async function cardByName(name: string) {
  let found: WebdriverIO.Element | undefined;
  await browser.waitUntil(
    async () => {
      for (const card of await $$('[data-testid="m-miesta-card"]').getElements()) {
        if ((await card.$('[data-testid="m-miesta-card-name"]').getText()) === name) {
          found = card;
          return true;
        }
      }
      return false;
    },
    { timeout: 10000, timeoutMsg: `no card named ${name}` }
  );
  return found as WebdriverIO.Element;
}

/** The leaflet map with this test id, once Leaflet has taken over the container. */
async function waitForLeaflet(testId: string) {
  const map = await $(`[data-testid="${testId}"]`);
  await map.waitForDisplayed({ timeout: 10000 });
  await browser.waitUntil(
    async () => ((await map.getAttribute('class')) ?? '').includes('leaflet-container'),
    { timeout: 10000, timeoutMsg: 'Leaflet never took over the map container' }
  );
  return map;
}

/** Open /m/miesta, tap the card of the place and wait for the edit screen. */
async function openEditor(id: string): Promise<void> {
  await browser.url('/m/miesta');
  const card = await $(`[data-testid="m-miesta-card"][data-place-id="${id}"]`);
  await card.waitForDisplayed({ timeout: 10000 });
  await card.click();
  await $('[data-testid="m-place-editor"]').waitForDisplayed({ timeout: 10000 });
}

describe('Tier 2: Mobile places (/m/miesta)', () => {
  beforeEach(async () => {
    // The phone pages have no h1, which waitForAppReady waits for.
    await browser.url('/');
    await waitForAppReady();
    await ensureLanguage('en');
    await forgetSpecPlaces();
  });

  after(async () => {
    await forgetSpecPlaces();
  });

  it('opens the list from the start page', async () => {
    // The start page shows its buttons only with an active vehicle.
    const vehicle = await seedVehicle({
      name: 'Phone Places Test',
      licensePlate: 'MOB-096',
      initialOdometer: 1000,
      vehicleType: 'Ice',
      tankSizeLiters: 50,
      tpConsumption: 7.0,
    });
    await setActiveVehicle(vehicle.id as string);
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
    await cardByName('Lambda Gate, Testville');
    await (await $('[data-testid="m-miesta-filter"]')).setValue('kosice');
    await browser.waitUntil(
      async () => {
        const names = await $$('[data-testid="m-miesta-card-name"]').map((n) => n.getText());
        return names.length === 1 && names[0] === 'Košice Sklad';
      },
      { timeout: 5000, timeoutMsg: 'filter did not narrow to Kosice Sklad' }
    );
  });

  it('saves a renamed place at a geocoder result', async () => {
    const id = await ensurePlace('Beta Shed, Testville');
    await openEditor(id);
    await (await $('[data-testid="m-place-name"]')).setValue('Beta Shed North, Testville');
    await (await $('[data-testid="m-place-search"]')).setValue('Gamma Warehouse, Testville');
    await (await $('[data-testid="m-place-search-btn"]')).click();
    const first = await $('[data-testid="m-place-candidate"]');
    await first.waitForDisplayed({ timeout: 10000 });
    await first.click();
    await (await $('[data-testid="m-place-save"]')).click();
    await $('[data-testid="m-place-editor"]').waitForDisplayed({ reverse: true, timeout: 10000 });
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
    await expect($('[data-testid="m-place-position"]')).toHaveText(expect.stringContaining('(manual)'));
    await (await $('[data-testid="m-place-save"]')).click();
    await $('[data-testid="m-place-editor"]').waitForDisplayed({ reverse: true, timeout: 10000 });
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
    await expect($('.toast-error')).toBeDisplayed();
    await expect($('[data-testid="m-place-editor"]')).toBeDisplayed();
    const row = (await rpc<Place[]>('list_places', {})).find((p) => p.id === id)!;
    expect(row.name).toBe('Xi Gate, Testville');
    expect(row.isHome).toBe(false);
  });

  it('closes the edit screen with the back gesture', async () => {
    const id = await ensurePlace('Omicron Pier, Testville');
    await openEditor(id);
    await browser.back();
    await $('[data-testid="m-place-editor"]').waitForDisplayed({ reverse: true, timeout: 10000 });
    await expect($('[data-testid="m-miesta"]')).toBeDisplayed();
  });
});
