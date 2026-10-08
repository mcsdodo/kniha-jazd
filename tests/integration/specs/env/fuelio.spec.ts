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
import { getTripGridData, rpc, seedTrip, seedVehicle, setActiveVehicle } from '../../utils/db';

const YEAR = new Date().getFullYear();

async function rowCount(): Promise<number> {
  return (await $$('[data-testid="fuelio-row"]').getElements()).length;
}

async function waitForRows(count: number, msg: string): Promise<void> {
  await browser.waitUntil(async () => (await rowCount()) === count, {
    timeout: 5000,
    timeoutMsg: `${msg}: expected ${count} rows, got ${await rowCount()}`,
  });
}

async function statusCount(status: string): Promise<number> {
  return (await $$(`[data-testid="fuelio-status-${status}"]`).getElements()).length;
}

async function setMinKm(km: number): Promise<void> {
  const input = $('[data-testid="fuelio-min-km"]');
  await input.clearValue();
  await input.setValue(String(km));
}

/** The places at the two ends of the fixture's 22 km drives. */
async function seedPlaces(): Promise<void> {
  await rpc('create_place', { name: 'Fuelio South', lat: 48.0, lon: 17.0, source: 'manual' });
  await rpc('create_place', { name: 'Fuelio North', lat: 48.2, lon: 17.0, source: 'manual' });
}

/** A trip after the fixture drives: an earlier change moves its odometer. */
async function seedLaterTrip(vehicleId: string, odometer: number): Promise<void> {
  await seedTrip({
    vehicleId,
    startDatetime: `${YEAR}-01-20T08:00`,
    endDatetime: `${YEAR}-01-20T08:30`,
    origin: 'Fuelio North',
    destination: 'Fuelio South',
    distanceKm: 22,
    odometer,
    purpose: 'later',
  });
}

describe('Fuelio page (Dropbox configured)', () => {
  let vehicleId: string;

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
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
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

  describe('with a logbook trip', () => {
    // Seeded after the page loaded: reload so the page reads the new state.
    async function reload(): Promise<void> {
      await browser.url('/fuelio');
      await $('[data-testid="fuelio-table"]').waitForDisplayed({ timeout: 10000 });
    }

    it('keeps both rows of a trip that has a match and a loose fragment', async () => {
      await seedPlaces();
      await seedTrip({
        vehicleId,
        startDatetime: `${YEAR}-01-10T10:00`,
        endDatetime: `${YEAR}-01-10T10:20`,
        origin: 'Fuelio South',
        destination: 'Fuelio North',
        distanceKm: 22,
        odometer: 10022,
        purpose: 'trip',
      });
      await reload();

      // The 3 km fragment is under the default 15 km filter
      await waitForRows(2, 'default 15 km filter');
      await setMinKm(0);
      await waitForRows(3, 'no km filter');
      expect(await statusCount('matched')).toBe(2);
      expect((await $$('[data-testid="fuelio-flag-looseMatch"]').getElements()).length).toBe(1);

      // A filter change must keep each row once: no lost and no stale rows
      await setMinKm(15);
      await waitForRows(2, 'back to 15 km');
      await setMinKm(0);
      await waitForRows(3, 'no km filter again');
      expect(await statusCount('matched')).toBe(2);
      expect(await statusCount('missing')).toBe(1);
    });

    it('overwrites a trip from its GPS drive and moves the later odometer', async () => {
      await seedPlaces();
      await seedTrip({
        vehicleId,
        startDatetime: `${YEAR}-01-10T10:00`,
        endDatetime: `${YEAR}-01-10T10:30`,
        origin: 'Fuelio South',
        destination: 'Fuelio North',
        distanceKm: 20,
        odometer: 10020,
        purpose: 'trip',
      });
      await seedLaterTrip(vehicleId, 10042);
      await reload();
      await $('[data-testid="fuelio-flag-kmDiffers"]').waitForDisplayed({ timeout: 5000 });

      await $('[data-testid="fuelio-overwrite"]').click();
      await $('[data-testid="fuelio-overwrite-modal"]').waitForDisplayed({ timeout: 5000 });
      await $('[data-testid="fuelio-overwrite-continue"]').click();
      await $('[data-testid="cascade-modal"]').waitForDisplayed({ timeout: 5000 });
      await $('[data-testid="cascade-confirm"]').click();

      await browser.waitUntil(
        async () => !(await $('[data-testid="fuelio-flag-kmDiffers"]').isExisting()),
        { timeout: 5000, timeoutMsg: 'the km flag stays after the overwrite' }
      );
      const grid = await getTripGridData(vehicleId, YEAR);
      const [trip, later] = [...grid.trips].sort((a, b) =>
        a.startDatetime.localeCompare(b.startDatetime)
      );
      expect(trip.distanceKm).toBe(22);
      expect(trip.endDatetime?.slice(11, 16)).toBe('10:18');
      expect(later.odometer).toBe(10044);
      expect(await $('.toast-error').isExisting()).toBe(false);
    });

    it('adds a missing drive as a trip and moves the later odometer', async () => {
      await seedPlaces();
      await seedLaterTrip(vehicleId, 10022);
      await reload();
      await waitForRows(2, 'both 22 km drives missing');

      // Newest first: the first row is the drive of 11 January
      await $('[data-testid="fuelio-add"]').click();
      await $('[data-testid="fuelio-add-modal"]').waitForDisplayed({ timeout: 5000 });
      await browser.waitUntil(
        async () => (await $('[data-testid="fuelio-add-origin"]').getValue()) !== '',
        { timeout: 5000, timeoutMsg: 'the add preview did not preselect a place' }
      );
      await $('[data-testid="fuelio-add-purpose"]').setValue('from Fuelio');
      await $('[data-testid="fuelio-add-continue"]').click();
      await $('[data-testid="cascade-modal"]').waitForDisplayed({ timeout: 5000 });
      await $('[data-testid="cascade-confirm"]').click();

      await browser.waitUntil(async () => (await statusCount('matched')) === 1, {
        timeout: 5000,
        timeoutMsg: 'the added drive is not matched to its new trip',
      });
      expect(await statusCount('missing')).toBe(1);
      const grid = await getTripGridData(vehicleId, YEAR);
      const [added, later] = [...grid.trips].sort((a, b) =>
        a.startDatetime.localeCompare(b.startDatetime)
      );
      expect(added.startDatetime.slice(0, 16)).toBe(`${YEAR}-01-11T10:00`);
      expect(added.origin).toBe('Fuelio South');
      expect(added.destination).toBe('Fuelio North');
      expect(added.distanceKm).toBe(22);
      expect(later.odometer).toBe(10044);
      expect(await $('.toast-error').isExisting()).toBe(false);
    });
  });
});
