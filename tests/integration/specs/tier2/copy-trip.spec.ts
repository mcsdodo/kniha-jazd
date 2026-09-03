/**
 * Tier 2: Copy Trip Row Integration Tests (Task 71)
 *
 * Covers the UI flow only:
 * - Copy opens a new row in edit mode with the route fields prefilled
 * - Fuel fields are NOT prefilled
 * - The row saves with a recalculated ODO
 * - The button is disabled while a new row is open
 *
 * The date-resolution and day-offset rules are backend-owned and exhaustively
 * covered in src-tauri/core/src/calculations/trip_copy.rs — do not retest here.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';

describe('Tier 2: Copy Trip Row', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    const vehicle = await seedVehicle({
      name: 'Copy Test Vehicle',
      licensePlate: 'COPY001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);

    // Source trip, dated earlier this year so "today" is the newest date.
    const year = new Date().getFullYear();
    await seedTrip({
      vehicleId: vehicle.id as string,
      startDatetime: `${year}-01-15T08:30`,
      endDatetime: `${year}-01-15T09:15`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 47,
      odometer: 50047,
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

    // ODO is derived: 50047 (previous) + 47 (copied km) = 50094
    expect(await (await editingRow.$('[data-testid="trip-odometer"]')).getValue()).toBe('50094');

    // Fuel is a one-off event and must NOT be copied.
    expect(await (await editingRow.$('[data-testid="trip-fuel-liters"]')).getValue()).toBe('');
  });

  it('should disable the copy button while a new row is open', async () => {
    await (await $('.icon-btn.copy')).click();
    await browser.pause(700);

    const copyButtons = await $$('.icon-btn.copy');
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

    // 50047 (previous ODO) + 60 = 50107
    const odoCells = await $$('td.col-odo');
    const odoTexts: string[] = [];
    for (const cell of odoCells) {
      odoTexts.push(await cell.getText());
    }
    expect(odoTexts.some((t) => t.includes('50107'))).toBe(true);
  });
});
