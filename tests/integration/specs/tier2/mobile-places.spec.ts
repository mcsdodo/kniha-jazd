/**
 * Tier 2: the place list on the phone (Task 96)
 *
 * /m/miesta lists the places of the database with a filter. This spec checks
 * the start page button, the card and the diacritics-blind filter.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { ensurePlace, rpc, seedVehicle, setActiveVehicle } from '../../utils/db';

/** Every place name this spec creates; deleted again before each test. */
const SPEC_PLACES = ['Alpha Depot, Testville', 'Košice Sklad', 'Lambda Gate, Testville'];

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
});
