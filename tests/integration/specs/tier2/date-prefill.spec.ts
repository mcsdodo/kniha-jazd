/**
 * Tier 2: Date Prefill Mode Integration Tests
 *
 * Tests the date prefill toggle functionality:
 * - Toggle switches between "+1" (previous date + 1) and "Today" modes
 * - Setting persists across app reload
 */

import { waitForAppReady, navigateTo, waitForTripGrid } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, clearDatabase } from '../../utils/db';

/**
 * Get date prefill mode via Tauri IPC
 */
async function getDatePrefillMode(): Promise<string> {
  const result = await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('get_date_prefill_mode');
  });
  return result as string;
}

/**
 * Set date prefill mode via Tauri IPC
 */
async function setDatePrefillModeViaIpc(mode: 'previous' | 'today'): Promise<void> {
  await browser.execute(async (m: string) => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('set_date_prefill_mode', { mode: m });
  }, mode);
}

describe('Tier 2: Date Prefill Mode', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    await clearDatabase();

    // Seed test vehicle with a trip
    const vehicle = await seedVehicle({
      name: 'Test Vehicle',
      licensePlate: 'TEST001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id;
    await setActiveVehicle(vehicleId);

    // Seed a trip from yesterday
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    const yesterdayStr = yesterday.toISOString().split('T')[0];

    await seedTrip({
      vehicleId,
      date: yesterdayStr,
      origin: 'Home',
      destination: 'Office',
      distanceKm: 25,
      odometer: 50025,
      purpose: 'Work commute',
    });

    // Reset to default mode (previous)
    await setDatePrefillModeViaIpc('previous');
  });

  describe('Toggle Functionality', () => {
    it('should toggle date prefill mode and persist across reload', async () => {
      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Find the segmented toggle
      const toggle = await $('.segmented-toggle');
      expect(await toggle.isExisting()).toBe(true);

      // Get initial mode from backend
      const initialMode = await getDatePrefillMode();
      expect(initialMode).toBe('previous');

      // Click "Today" option
      const todayButton = await toggle.$('button*=Today');
      if (await todayButton.isExisting()) {
        await todayButton.click();
        await browser.pause(300);

        // Verify mode changed via IPC
        const newMode = await getDatePrefillMode();
        expect(newMode).toBe('today');

        // Verify toggle shows active state on Today
        const isActive = await todayButton.getAttribute('class');
        expect(isActive).toContain('active');
      }

      // Reload the page
      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Verify setting persisted
      const persistedMode = await getDatePrefillMode();
      expect(persistedMode).toBe('today');

      // Verify toggle still shows Today as active
      const toggleAfterReload = await $('.segmented-toggle');
      const todayButtonAfterReload = await toggleAfterReload.$('button*=Today');
      if (await todayButtonAfterReload.isExisting()) {
        const isActiveAfterReload = await todayButtonAfterReload.getAttribute('class');
        expect(isActiveAfterReload).toContain('active');
      }

      // Switch back to previous mode
      const previousButton = await toggleAfterReload.$('button*=+1');
      if (await previousButton.isExisting()) {
        await previousButton.click();
        await browser.pause(300);

        const finalMode = await getDatePrefillMode();
        expect(finalMode).toBe('previous');
      }
    });
  });

  describe('Date Prefill Behavior', () => {
    it('should prefill with today date when in Today mode', async () => {
      // Set mode to Today
      await setDatePrefillModeViaIpc('today');

      // Navigate to trips
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button*=New record');
      if (await newRecordBtn.isExisting()) {
        await newRecordBtn.click();
        await browser.pause(300);

        // Check the date field value
        const dateInput = await $('[data-testid="trip-date"]');
        if (await dateInput.isExisting()) {
          const dateValue = await dateInput.getValue();
          const today = new Date().toISOString().split('T')[0];
          expect(dateValue).toBe(today);
        }
      }
    });

    it('should prefill with previous +1 date when in Previous mode', async () => {
      // Set mode to Previous
      await setDatePrefillModeViaIpc('previous');

      // Navigate to trips
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button*=New record');
      if (await newRecordBtn.isExisting()) {
        await newRecordBtn.click();
        await browser.pause(300);

        // Check the date field value
        const dateInput = await $('[data-testid="trip-date"]');
        if (await dateInput.isExisting()) {
          const dateValue = await dateInput.getValue();
          // Should be yesterday + 1 = today (since we seeded yesterday's trip)
          const today = new Date().toISOString().split('T')[0];
          expect(dateValue).toBe(today);
        }
      }
    });
  });
});
