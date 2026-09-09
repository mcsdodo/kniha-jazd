/**
 * Tier 1: Smart Trip Defaults Integration Tests
 *
 * Covers two related conveniences for the trip grid:
 *   1. ODO on a NEW row - no clamp (Task 8 deleted it; ADR-042). Typing an
 *      ODO below the anchor just shows what was typed. More than that: a NEW
 *      row's odometer is never even sent to the backend (createTripCascade
 *      takes only `distanceKm`; see `plan_insert_cascade`, which derives
 *      `new_odometer = anchor + distance_km` unconditionally) -- so whatever
 *      the ODO field shows is a live preview only, never the saved value.
 *   2. Time inference — on a NEW row, picking origin + destination that match
 *      a previous trip auto-fills start/end datetimes (jittered) from the
 *      most recent matching trip. Editing existing rows must NOT trigger this.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
  getTripGridData,
  rpc,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { waitForTripGrid } from '../../utils/assertions';

async function openNewTripRow(): Promise<void> {
  const newTripBtn = await $('button.new-record');
  await newTripBtn.waitForClickable({ timeout: 5000 });
  await newTripBtn.click();

  await browser.waitUntil(
    async () => {
      const editingRow = await $('tr.editing');
      return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
    },
    { timeout: 10000, timeoutMsg: 'Editing row did not appear' }
  );
}

async function selectFromAutocomplete(
  inputTestId: string,
  value: string
): Promise<void> {
  const input = await $(`[data-testid="${inputTestId}"]`);
  await input.waitForDisplayed({ timeout: 5000 });
  await input.click();
  await input.setValue(value);

  // Wait for THIS input's dropdown (scoped to its .autocomplete container).
  const container = await input.parentElement();
  const dropdown = await container.$('.dropdown');

  await dropdown.waitForDisplayed({
    timeout: 5000,
    timeoutMsg: `Autocomplete dropdown for ${inputTestId} did not appear`,
  });

  const suggestion = await dropdown.$('.suggestion');
  await suggestion.waitForClickable({ timeout: 5000 });
  await suggestion.click();
}

/**
 * Fire `input` events with cumulative values to mimic a user typing one
 * character at a time. Atomic setValue() hides bugs that only manifest
 * when handlers see intermediate values — see: the "KM fills with last ODO"
 * regression where delta-based KM recalculation accumulated wrongly on
 * keystroke-by-keystroke input.
 */
async function simulateTyping(selector: string, text: string): Promise<void> {
  for (let i = 1; i <= text.length; i++) {
    const partial = text.slice(0, i);
    await browser.execute((sel: string, val: string) => {
      const input = document.querySelector(sel) as HTMLInputElement;
      if (input) {
        input.value = val;
        input.dispatchEvent(new Event('input', { bubbles: true }));
      }
    }, selector, partial);
    await browser.pause(10);
  }
}

describe('Tier 1: Smart Trip Defaults', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('ODO on a new row: no clamp, and never sent to the backend', () => {
    it('does not clamp an ODO entered below the anchor', async () => {
      // Task 8 deleted handleOdoBlur, the only place this clamp lived. The
      // field now shows exactly what was typed, however far below the
      // anchor (ADR-042: no silent correction).
      const vehicleData = createTestIceVehicle({
        name: 'ODO Clamp Test',
        licensePlate: 'CLMP-001',
        initialOdometer: 50000,
      });
      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      const year = new Date().getFullYear();

      // Existing trip - the new row's anchor is its odometer, 51000.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-02-01T08:00`,
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 1000,
        odometer: 51000,
        purpose: 'Business',
      });

      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      await openNewTripRow();

      // Enter an obviously-too-low ODO and fire a `change` event (blur).
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-odometer"]', '40000');

      await browser.pause(150);

      const odoInput = await $('[data-testid="trip-odometer"]');
      // No clamp: the field shows exactly what was typed, even below the
      // anchor (51000).
      expect(await odoInput.getValue()).toBe('40000');

      // Saving a NEW row never even sends this field -- createTripCascade
      // takes only distanceKm (plan_insert_cascade always derives the
      // odometer as anchor + distance_km). km was never typed here, so it
      // is saved as 0 and the odometer is the anchor, untouched by the
      // typed-but-unsent 40000.
      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const grid = await getTripGridData(vehicle.id as string, year);
      const newRow = grid.trips.find((t) => t.destination !== 'Trnava');
      expect(newRow).toBeDefined();
      expect(newRow!.distanceKm).toBe(0);
      expect(newRow!.odometer).toBe(51000);
    });
  });

  describe('ODO on a new row never derives KM (regression: "KM fills with last ODO")', () => {
    it('leaves KM alone while ODO is typed, and never sends the ODO on save', async () => {
      // Original regression: typing an ODO digit-by-digit into a fresh new
      // row made the KM field accumulate via a delta branch and land at
      // ~the anchor (60194 for an anchor of 60000) — "KM fills with last
      // ODO". Task 8 deleted that whole derivation. It is not merely
      // disabled live: a NEW row's ODO is never sent to the backend at all
      // (createTripCascade takes only distanceKm; plan_insert_cascade always
      // derives new_odometer = anchor + distance_km). So typing only the ODO
      // and saving must leave distanceKm at 0 and the odometer at the
      // anchor, regardless of what the ODO field showed.
      const vehicleData = createTestIceVehicle({
        name: 'KM-from-ODO Regression',
        licensePlate: 'KMBUG-01',
        initialOdometer: 60000,
      });
      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // No prior trips - the new row's anchor is the vehicle's
      // initialOdometer (60000).
      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      await openNewTripRow();

      const distanceInput = await $('[data-testid="trip-distance"]');
      const kmBefore = await distanceInput.getValue();

      // Simulate a user typing "60200" one character at a time. Even
      // keystroke-by-keystroke, the km field must never move.
      await simulateTyping('[data-testid="trip-odometer"]', '60200');
      await browser.pause(150);

      const odoInput = await $('[data-testid="trip-odometer"]');
      // No clamp, no live derivation: the ODO field just shows what was typed.
      expect(parseFloat(await odoInput.getValue())).toBe(60200);
      expect(await distanceInput.getValue()).toBe(kmBefore);

      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      // The typed 60200 never reached the backend. km saved as 0 (nothing
      // typed into the km field), and the odometer is the untouched anchor.
      const grid = await getTripGridData(vehicle.id as string, new Date().getFullYear());
      expect(grid.trips).toHaveLength(1);
      expect(grid.trips[0].distanceKm).toBe(0);
      expect(grid.trips[0].odometer).toBe(60000);
    });

    it('leaves KM alone the same way when the anchor is 0 (no initialOdometer)', async () => {
      // Folded from the old "leaves KM blank when the anchor is 0" case: that
      // guarded specifically against (ODO - 0) surfacing the raw ODO value
      // in the km field. There is no more derivation of any kind now, from
      // any anchor, so this is the same invariant as the case above --
      // restated here only because the vehicle shape (anchor 0) differs
      // enough to be worth its own regression guard.
      const vehicleData = createTestIceVehicle({
        name: 'No Initial ODO',
        licensePlate: 'NOINI-01',
        initialOdometer: 0,
      });
      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      await openNewTripRow();

      await simulateTyping('[data-testid="trip-odometer"]', '60200');
      await browser.pause(150);

      const distanceInput = await $('[data-testid="trip-distance"]');
      const kmValue = await distanceInput.getValue();
      // Must NOT equal 60200 — that would be "ODO in KM field".
      expect(kmValue).not.toBe('60200');
    });
  });

  describe('Time inference for new rows', () => {
    // Time inference is opt-in (default OFF) since v0.34 — see BIZ-014 in DECISIONS.md.
    // These tests assume the feature is ON.
    beforeEach(async () => {
      await rpc<void>('set_infer_trip_times', { enabled: true });
    });

    afterEach(async () => {
      await rpc<void>('set_infer_trip_times', { enabled: false });
    });

    it('auto-fills start/end datetimes from the most recent matching route', async () => {
      const vehicleData = createTestIceVehicle({
        name: 'Time Inference Test',
        licensePlate: 'TIME-001',
        initialOdometer: 60000,
      });
      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      const year = new Date().getFullYear();

      // Seed a completed trip so the inference has a base to learn from.
      // Bratislava → Žilina, 09:30 → 11:00 (90-minute duration).
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-03-10T09:30`,
        endDatetime: `${year}-03-10T11:00`,
        origin: 'Bratislava',
        destination: 'Žilina',
        distanceKm: 200,
        odometer: 60200,
        purpose: 'Business',
      });

      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      await openNewTripRow();

      // Capture pre-inference start datetime so we can detect the change.
      const startInput = await $('[data-testid="trip-start-datetime"]');
      const beforeStart = await startInput.getValue();

      // Pick origin then destination — second selection triggers tryInferTimes.
      await selectFromAutocomplete('trip-origin', 'Bratislava');
      await browser.pause(200);
      await selectFromAutocomplete('trip-destination', 'Žilina');

      // Allow the async invoke to resolve and Svelte to re-render.
      await browser.pause(500);

      const afterStart = await startInput.getValue();
      const endInput = await $('[data-testid="trip-end-datetime"]');
      const afterEnd = await endInput.getValue();

      // Start time must have been changed by inference (jittered around 09:30).
      expect(afterStart).not.toBe(beforeStart);

      // Parse HH:MM from "YYYY-MM-DDTHH:MM" and assert within jitter bounds:
      //   start within ±15 minutes of 09:30
      //   duration within ±15% of 90 minutes (76–104 min)
      const toMinutes = (dt: string): number => {
        const [, time] = dt.split('T');
        const [h, m] = time.split(':').map(Number);
        return h * 60 + m;
      };
      const startMins = toMinutes(afterStart);
      const endMins = toMinutes(afterEnd);
      const baseStart = 9 * 60 + 30; // 09:30 = 570
      expect(startMins).toBeGreaterThanOrEqual(baseStart - 15);
      expect(startMins).toBeLessThanOrEqual(baseStart + 15);

      const duration = endMins - startMins;
      expect(duration).toBeGreaterThanOrEqual(Math.floor(90 * 0.85)); // 76
      expect(duration).toBeLessThanOrEqual(Math.ceil(90 * 1.15));     // 104
    });

    it('does not re-infer times when editing an existing trip', async () => {
      const vehicleData = createTestIceVehicle({
        name: 'No-Reinfer Test',
        licensePlate: 'NORE-001',
        initialOdometer: 70000,
      });
      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      const year = new Date().getFullYear();

      // Earlier completed trip on the same route with a distinctive 06:15 start.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-01T06:15`,
        endDatetime: `${year}-04-01T07:00`,
        origin: 'Trnava',
        destination: 'Nitra',
        distanceKm: 65,
        odometer: 70065,
        purpose: 'Business',
      });

      // Trip we will edit — also Trnava → Nitra but starts at 14:00.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-15T14:00`,
        endDatetime: `${year}-04-15T14:45`,
        origin: 'Trnava',
        destination: 'Nitra',
        distanceKm: 65,
        odometer: 70130,
        purpose: 'Business',
      });

      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Double-click the most recent trip row to edit it.
      const tripRow = await $('tbody tr:not(.first-record):not(.editing)');
      await tripRow.waitForDisplayed({ timeout: 5000 });
      await browser.execute(() => {
        const row = document.querySelector('tbody tr:not(.first-record):not(.editing)') as HTMLElement;
        if (row) row.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      });
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear' }
      );

      const startInput = await $('[data-testid="trip-start-datetime"]');
      const beforeStart = await startInput.getValue();

      // Trigger a "change" to origin/destination (here just re-select the same
      // values via the autocomplete) — for an existing row this must NOT call
      // the inference backend, so the start time must be unchanged.
      await selectFromAutocomplete('trip-origin', 'Trnava');
      await browser.pause(200);
      await selectFromAutocomplete('trip-destination', 'Nitra');
      await browser.pause(500);

      const afterStart = await startInput.getValue();
      expect(afterStart).toBe(beforeStart);
    });
  });
});
