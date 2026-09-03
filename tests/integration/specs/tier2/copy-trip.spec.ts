/**
 * Tier 2: Copy Trip Row Integration Tests (Task 71)
 *
 * Covers the UI flow only:
 * - Copy opens a new row in edit mode with the route fields prefilled
 * - Fuel fields are NOT prefilled
 * - The row saves with a recalculated ODO
 * - The button is disabled while a new row is open
 * - Changing the route replaces the copied KM (regression)
 * - Editing the start time preserves an overnight span (regression)
 *
 * The date-resolution and day-offset rules are backend-owned and exhaustively
 * covered in src-tauri/core/src/calculations/trip_copy.rs — do not retest here.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, invokeTauri } from '../../utils/db';

/** Set an input's value and fire input+change, which setValue does not do. */
async function setFieldByTestId(testId: string, value: string): Promise<void> {
  await browser.execute(
    (sel: string, newValue: string) => {
      const input = document.querySelector(sel) as HTMLInputElement | null;
      if (input) {
        input.value = newValue;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
      }
    },
    `[data-testid="${testId}"]`,
    value
  );
}

/** Type into an autocomplete and click its first suggestion, firing onSelect. */
async function selectFromAutocomplete(inputTestId: string, value: string): Promise<void> {
  const input = await $(`[data-testid="${inputTestId}"]`);
  await input.waitForDisplayed({ timeout: 5000 });
  await input.click();
  await input.setValue(value);

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

// Seeded chronologically ascending; the grid shows newest first, so the copy
// buttons read: [0] = Trnava (47 km), [1] = overnight Vieden, [2] = Kosice.
// The newest ODO (50527) is what a new top row derives from.
const YEAR = new Date().getFullYear();
const LAST_ODO = 50527;

describe('Tier 2: Copy Trip Row', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    // hidden_columns persists per app-dir and survives across spec files, and
    // sibling specs hide 'time'. Without this reset, trip-end-datetime may not
    // render and this spec becomes order-dependent.
    await invokeTauri<void>('set_hidden_columns', { columns: [] });

    const vehicle = await seedVehicle({
      name: 'Copy Test Vehicle',
      licensePlate: 'COPY001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    const vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);

    // Registers the Bratislava->Kosice route at 400 km, so changing a copied
    // row's destination has a known distance to auto-fill from.
    await seedTrip({
      vehicleId,
      startDatetime: `${YEAR}-01-10T07:00`,
      endDatetime: `${YEAR}-01-10T11:00`,
      origin: 'Bratislava',
      destination: 'Kosice',
      distanceKm: 400,
      odometer: 50400,
      purpose: 'Long haul',
    });

    // Overnight trip: 22:00 -> 02:00 the following day.
    await seedTrip({
      vehicleId,
      startDatetime: `${YEAR}-01-15T22:00`,
      endDatetime: `${YEAR}-01-16T02:00`,
      origin: 'Bratislava',
      destination: 'Vieden',
      distanceKm: 80,
      odometer: 50480,
      purpose: 'Night drive',
    });

    // Newest trip — the default copy source for most tests.
    await seedTrip({
      vehicleId,
      startDatetime: `${YEAR}-01-20T08:30`,
      endDatetime: `${YEAR}-01-20T09:15`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 47,
      odometer: LAST_ODO,
      purpose: 'Client visit',
    });

    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);
  });

  it('should prefill the route fields and leave fuel empty', async () => {
    const copyBtn = await $('.icon-btn.copy');
    expect(await copyBtn.isExisting()).toBe(true);
    await copyBtn.click();
    await browser.pause(700);

    const editingRow = await $('tr.editing');
    expect(await editingRow.isExisting()).toBe(true);

    expect(await (await editingRow.$('[data-testid="trip-origin"]')).getValue()).toBe('Bratislava');
    expect(await (await editingRow.$('[data-testid="trip-destination"]')).getValue()).toBe('Trnava');
    expect(await (await editingRow.$('[data-testid="trip-distance"]')).getValue()).toBe('47');
    expect(await (await editingRow.$('[data-testid="trip-purpose"]')).getValue()).toBe('Client visit');

    // Time-of-day carries over from the source row; the date is today's.
    const start = await (await editingRow.$('[data-testid="trip-start-datetime"]')).getValue();
    expect(start).toContain('08:30');
    const today = new Date();
    const todayStr = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`;
    expect(start).toContain(todayStr);

    // ODO is derived: 50527 (previous) + 47 (copied km)
    expect(await (await editingRow.$('[data-testid="trip-odometer"]')).getValue()).toBe(
      String(LAST_ODO + 47)
    );

    // Fuel is a one-off event and must NOT be copied.
    expect(await (await editingRow.$('[data-testid="trip-fuel-liters"]')).getValue()).toBe('');
  });

  it('should disable the copy button while a new row is open', async () => {
    await (await $('.icon-btn.copy')).click();
    await browser.pause(700);

    const copyButtons = await $$('.icon-btn.copy');
    // Guard against a vacuous pass: an empty collection would satisfy the loop
    // while asserting nothing, hiding the very regression this test exists for.
    expect(copyButtons.length).toBeGreaterThan(0);
    for (const btn of copyButtons) {
      expect(await btn.isEnabled()).toBe(false);
    }
  });

  it('should save the copied row with a recalculated ODO', async () => {
    await (await $('.icon-btn.copy')).click();
    await browser.pause(700);

    const editingRow = await $('tr.editing');

    // Fine-tune the distance before applying.
    await (await editingRow.$('[data-testid="trip-distance"]')).setValue('60');
    await browser.pause(300);

    await (await editingRow.$('.icon-btn.save')).click();
    await browser.pause(1500);

    const expected = String(LAST_ODO + 60);
    const odoCells = await $$('td.col-odo');
    const odoTexts: string[] = [];
    for (const cell of odoCells) {
      odoTexts.push(await cell.getText());
    }
    expect(odoTexts.some((t) => t.includes(expected))).toBe(true);
  });

  it('should replace the copied KM when the route is changed', async () => {
    // Regression: the seeded distance used to block tryAutoFillDistance forever
    // (its guard was `distanceKm === null`), so picking a new destination
    // re-inferred the times but left the OLD route's km and ODO in place — a
    // 400 km journey silently saved as 47 km in a legal logbook.
    await (await $('.icon-btn.copy')).click();
    await browser.pause(700);

    const kmInput = await $('[data-testid="trip-distance"]');
    expect(await kmInput.getValue()).toBe('47');

    await selectFromAutocomplete('trip-destination', 'Kosice');
    await browser.pause(700);

    expect(await kmInput.getValue()).toBe('400');
    // ODO must follow the corrected distance.
    expect(await (await $('[data-testid="trip-odometer"]')).getValue()).toBe(String(LAST_ODO + 400));
  });

  it('should keep an overnight span when the start time is edited', async () => {
    // Regression: handleStartDatetimeChange collapsed end onto start for any
    // new row, destroying the +1 day offset the backend computed for an
    // overnight copy (BIZ-024) on the user's very first edit.
    const copyButtons = await $$('.icon-btn.copy');
    expect(copyButtons.length).toBeGreaterThanOrEqual(2);
    await copyButtons[1].click(); // the overnight Vieden trip
    await browser.pause(700);

    const startInput = await $('[data-testid="trip-start-datetime"]');
    const endInput = await $('[data-testid="trip-end-datetime"]');
    const seededStart = await startInput.getValue();
    const seededEnd = await endInput.getValue();
    expect(seededStart).toContain('22:00');
    expect(seededEnd).toContain('02:00');
    // Seeded end is already on the following day.
    expect(seededEnd.slice(0, 10)).not.toBe(seededStart.slice(0, 10));

    // Nudge the departure 15 minutes earlier.
    const shifted = `${seededStart.slice(0, 11)}21:45`;
    await setFieldByTestId('trip-start-datetime', shifted);
    await browser.pause(500);

    // Duration preserved (4h): the end moves back 15 min and STAYS on the
    // following day rather than collapsing onto the start.
    const newEnd = await endInput.getValue();
    expect(newEnd).toContain('01:45');
    expect(newEnd.slice(0, 10)).not.toBe(shifted.slice(0, 10));
  });

  // The copied times are explicit user intent, so the Task 56 / BIZ-014 time
  // inference must not jitter them away. Inference only fires from the
  // autocomplete onSelect handlers, so the path that matters is re-selecting
  // the SAME destination on a copied row. Inference is opt-in (default OFF),
  // hence the explicit enable — without it this test would pass vacuously.
  describe('with time inference enabled', () => {
    beforeEach(async () => {
      await invokeTauri<void>('set_infer_trip_times', { enabled: true });
    });

    afterEach(async () => {
      await invokeTauri<void>('set_infer_trip_times', { enabled: false });
    });

    it('should keep the copied times when the same destination is re-selected', async () => {
      await (await $('.icon-btn.copy')).click();
      await browser.pause(700);

      const startInput = await $('[data-testid="trip-start-datetime"]');
      const endInput = await $('[data-testid="trip-end-datetime"]');
      const beforeStart = await startInput.getValue();
      const beforeEnd = await endInput.getValue();
      expect(beforeStart).toContain('08:30');

      // Would trigger tryInferTimes() and jitter the times, were the copied
      // route pair not already marked as inferred.
      await selectFromAutocomplete('trip-destination', 'Trnava');
      await browser.pause(700);

      expect(await startInput.getValue()).toBe(beforeStart);
      expect(await endInput.getValue()).toBe(beforeEnd);
    });
  });
});
