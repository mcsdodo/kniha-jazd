/**
 * Fuelio page with Dropbox configured.
 *
 * Runs ONLY in the env suite (`npm run test:integration:docker:env`): Fuelio is
 * on only with DROPBOX_* set (see ENV_PINNED_FIXTURE). The harness puts two
 * missing drives of the current year into `<DATA_DIR>/fuelio`
 * (fixtures/fuelio-drives.mjs); CI does it before the container starts.
 *
 * Covers the UI flow of ignore / un-ignore. The ignore rules (only missing
 * rows, all drives of the row, per vehicle) are proven in fuelio_cmd_tests.rs.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, setActiveVehicle } from '../../utils/db';

async function rowCount(): Promise<number> {
  return (await $$('[data-testid="fuelio-row"]').getElements()).length;
}

async function waitForRows(count: number, msg: string): Promise<void> {
  await browser.waitUntil(async () => (await rowCount()) === count, {
    timeout: 5000,
    timeoutMsg: `${msg}: expected ${count} rows, got ${await rowCount()}`,
  });
}

describe('Fuelio page (Dropbox configured)', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    const vehicle = await seedVehicle({
      name: 'Fuelio Test Vehicle',
      licensePlate: 'FUEL001',
      initialOdometer: 10000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);
    // A fresh load: the layout reads the active vehicle on start
    await browser.url('/fuelio');
    await $('[data-testid="fuelio-table"]').waitForDisplayed({ timeout: 10000 });
  });

  it('lists the missing drives and selects the first one on the map', async () => {
    await waitForRows(2, 'both fixture drives');
    expect((await $$('[data-testid="fuelio-status-missing"]').getElements()).length).toBe(2);
    await $('[data-testid="fuelio-map"]').waitForDisplayed({ timeout: 5000 });
  });

  it('hides an ignored drive, shows it under the Ignored pill, and un-ignores it', async () => {
    await waitForRows(2, 'before ignore');

    await $('[data-testid="fuelio-ignore"]').click();
    await waitForRows(1, 'after ignore, default view');

    await $('[data-testid="fuelio-pill-ignored"]').click();
    await waitForRows(1, 'Ignored pill');
    await $('[data-testid="fuelio-status-ignored"]').waitForDisplayed({ timeout: 5000 });

    await $('[data-testid="fuelio-unignore"]').click();
    await waitForRows(0, 'Ignored pill after un-ignore');

    await $('[data-testid="fuelio-pill-ignored"]').click();
    await waitForRows(2, 'default view after un-ignore');
    expect(await $('.toast-error').isExisting()).toBe(false);
  });
});
