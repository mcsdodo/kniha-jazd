/**
 * Tier 1: Smart Trip Defaults Integration Tests
 *
 * Covers two related conveniences for the trip grid:
 *   1. ODO clamp — any ODO entered below the previous row's ODO is silently
 *      snapped to (previousOdometer + 1) on blur. Applies to all rows.
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
      return editingRow.isExisting() && (await editingRow.isDisplayed());
    },
    { timeout: 10000, timeoutMsg: 'Editing row did not appear' }
  );
}

async function selectFromAutocomplete(
  inputTestId: string,
  value: string,
  alreadyVisibleDropdownCount = 0
): Promise<void> {
  const input = await $(`[data-testid="${inputTestId}"]`);
  await input.waitForDisplayed({ timeout: 5000 });
  await input.click();
  await input.setValue(value);

  // Wait for a NEW dropdown to appear (more than the count already visible).
  await browser.waitUntil(
    async () => {
      const dropdowns = await $$('.autocomplete .dropdown');
      let visible = 0;
      for (const d of dropdowns) {
        if (await d.isDisplayed()) visible++;
      }
      return visible > alreadyVisibleDropdownCount;
    },
    { timeout: 5000, timeoutMsg: `Autocomplete dropdown for ${inputTestId} did not appear` }
  );

  // Click the visible dropdown's first suggestion.
  const dropdowns = await $$('.autocomplete .dropdown');
  for (const d of dropdowns) {
    if (await d.isDisplayed()) {
      const suggestion = await d.$('.suggestion');
      await suggestion.click();
      return;
    }
  }
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

  describe('ODO auto-clamp', () => {
    it('clamps ODO entered below previousOdometer up to previous + 1', async () => {
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

      // Existing trip — establishes previousOdometer = 51000 for next new row.
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
      const clampedValue = await odoInput.getValue();
      // previousOdometer is 51000, so the clamped value must be 51001.
      expect(parseFloat(clampedValue)).toBe(51001);
    });
  });

  describe('KM derivation on NEW rows (regression: "KM fills with last ODO")', () => {
    it('computes KM from (newODO − previousODO) on every keystroke, not via delta accumulation', async () => {
      // Regression: when a user typed an ODO value digit-by-digit into a fresh
      // new row, the KM field would accumulate via the delta branch of
      // handleOdoChange and land at ~previousOdometer (e.g., 60194 when the
      // last row's ODO was 60000) — the user described this as "KM fills with
      // last ODO". The fix derives KM directly from the current ODO on new
      // rows so intermediate keystrokes cannot accumulate.
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

      // No prior trips — previousOdometer on the new row will equal the
      // vehicle's initialOdometer (60000).
      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      await openNewTripRow();

      // Simulate a user typing "60200" one character at a time.
      await simulateTyping('[data-testid="trip-odometer"]', '60200');
      await browser.pause(150);

      const odoInput = await $('[data-testid="trip-odometer"]');
      const distanceInput = await $('[data-testid="trip-distance"]');
      expect(parseFloat(await odoInput.getValue())).toBe(60200);
      // Correct KM = 60200 − 60000 = 200. Pre-fix value was ≈60194.
      expect(parseFloat(await distanceInput.getValue())).toBe(200);
    });

    it('leaves KM blank when previousOdometer is 0 (vehicle without initialOdometer)', async () => {
      // When a user creates a vehicle without an initial odometer and enters
      // their first trip, previousOdometer is 0. Auto-deriving KM from
      // (ODO − 0) surfaces the raw ODO value in the KM field, which looks
      // identical to "the last ODO ended up in KM". Guard: skip auto-derive
      // and let the user type KM explicitly.
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
      await selectFromAutocomplete('trip-origin', 'Bratislava', 0);
      await browser.pause(200);
      await selectFromAutocomplete('trip-destination', 'Žilina', 1);

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
          return editingRow.isExisting() && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear' }
      );

      const startInput = await $('[data-testid="trip-start-datetime"]');
      const beforeStart = await startInput.getValue();

      // Trigger a "change" to origin/destination (here just re-select the same
      // values via the autocomplete) — for an existing row this must NOT call
      // the inference backend, so the start time must be unchanged.
      await selectFromAutocomplete('trip-origin', 'Trnava', 0);
      await browser.pause(200);
      await selectFromAutocomplete('trip-destination', 'Nitra', 1);
      await browser.pause(500);

      const afterStart = await startInput.getValue();
      expect(afterStart).toBe(beforeStart);
    });
  });
});
