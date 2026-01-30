/**
 * Tier 2: Trip Datetime Column Integration Tests
 *
 * Tests the datetime column functionality:
 * - Create trip with datetime → verify displays
 * - Edit trip datetime → verify saves
 * - End datetime column visibility toggle
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

/**
 * Reset hidden columns via Tauri IPC
 */
async function resetHiddenColumns(): Promise<void> {
  await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('set_hidden_columns', { columns: [] });
  });
}

describe('Tier 2: Datetime Column', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    // Reset hidden columns to show all columns
    await resetHiddenColumns();

    // Seed test vehicle (fresh data for each test)
    const vehicle = await seedVehicle({
      name: 'Datetime Test Vehicle',
      licensePlate: 'TIME-001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  describe('Datetime Display', () => {
    it('should display start datetime with correct time value', async () => {
      // Seed a trip with specific start datetime
      await seedTrip({
        vehicleId,
        startDatetime: '2026-01-15T08:30',
        endDatetime: '2026-01-15T10:00',
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'Work commute',
      });

      // Refresh to show seeded data
      await browser.refresh();
      await waitForAppReady();

      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Find the start datetime column header
      const startHeader = await $('[data-testid="column-header-start"]');
      expect(await startHeader.isExisting()).toBe(true);

      // Find trip row and verify datetime displays
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      // The start time should be displayed in the row (formatted as "15.01. 08:30")
      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('08:30');
    });

    it('should display end datetime when seeded', async () => {
      // Seed a trip with both start and end datetime
      await seedTrip({
        vehicleId,
        startDatetime: '2026-01-15T08:00',
        endDatetime: '2026-01-15T10:30',
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'Work commute',
      });

      // Refresh to show seeded data
      await browser.refresh();
      await waitForAppReady();

      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Find trip row and verify end datetime displays
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      // The end time should be displayed in the row
      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('10:30');
    });
  });

  describe('Datetime Entry', () => {
    it('should allow entering datetime when creating a new trip', async () => {
      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button.new-record');
      await newRecordBtn.waitForClickable({ timeout: 5000 });
      await newRecordBtn.click();
      await browser.pause(300);

      // Check that start datetime input exists
      const startDatetimeInput = await $('[data-testid="trip-start-datetime"]');
      expect(await startDatetimeInput.isExisting()).toBe(true);

      // Check that end datetime input exists
      const endDatetimeInput = await $('[data-testid="trip-end-datetime"]');
      expect(await endDatetimeInput.isExisting()).toBe(true);
    });

    it('should save and display the entered datetime', async () => {
      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button.new-record');
      await newRecordBtn.waitForClickable({ timeout: 5000 });
      await newRecordBtn.click();
      await browser.pause(300);

      // Fill in required fields using atomic method
      const today = new Date().toISOString().split('T')[0];

      // Start datetime (datetime-local format: YYYY-MM-DDTHH:MM)
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-start-datetime"]', `${today}T09:15`);

      // End datetime
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-end-datetime"]', `${today}T11:30`);

      // Origin
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-origin"]', 'TestOrigin');

      // Destination
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-destination"]', 'TestDestination');

      // Distance
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-distance"]', '30');

      // Purpose
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-purpose"]', 'Test purpose');

      await browser.pause(300);

      // Save the trip
      await browser.keys('Enter');
      await browser.pause(700);

      // Check for error toast
      const toastError = await $('.toast-error');
      expect(await toastError.isExisting()).toBe(false);

      // Verify the trip appears with the correct times
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('09:15');
      expect(rowText).toContain('11:30');
    });
  });

  describe('Column Visibility Dropdown', () => {
    it('should show column visibility dropdown button', async () => {
      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Find the column visibility toggle button
      const toggleBtn = await $('[data-testid="column-visibility-toggle"]');
      expect(await toggleBtn.isExisting()).toBe(true);
    });

    it('should open dropdown and show column options', async () => {
      // Seed a trip so the grid is populated
      await seedTrip({
        vehicleId,
        startDatetime: '2026-01-15T08:00',
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'Work commute',
      });

      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click the column visibility toggle
      const toggleBtn = await $('[data-testid="column-visibility-toggle"]');
      await toggleBtn.click();
      await browser.pause(300);

      // Check that dropdown menu appears
      const dropdownMenu = await $('[data-testid="column-visibility-menu"]');
      expect(await dropdownMenu.isDisplayed()).toBe(true);

      // Check for "time" toggle option (controls end datetime visibility)
      const timeToggle = await $('[data-testid="column-toggle-time"]');
      expect(await timeToggle.isExisting()).toBe(true);
    });
  });

  describe('Toggle Columns', () => {
    it('should hide end datetime column when toggled off', async () => {
      // Seed a trip
      await seedTrip({
        vehicleId,
        startDatetime: '2026-01-15T08:00',
        endDatetime: '2026-01-15T10:00',
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'Work commute',
      });

      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Verify end datetime column is initially visible
      const endHeader = await $('[data-testid="column-header-end"]');
      expect(await endHeader.isDisplayed()).toBe(true);

      // Open column visibility dropdown
      const toggleBtn = await $('[data-testid="column-visibility-toggle"]');
      await toggleBtn.click();
      await browser.pause(300);

      // Click the time toggle label (checkbox is hidden, click the parent label)
      // The label contains the checkbox with data-testid="column-toggle-time"
      const timeToggleLabel = await $('[data-testid="column-toggle-time"]').parentElement();
      await timeToggleLabel.click();
      await browser.pause(300);

      // Close dropdown by clicking elsewhere
      await browser.keys('Escape');
      await browser.pause(300);

      // Verify end datetime column is now hidden
      const endHeaderAfter = await $('[data-testid="column-header-end"]');
      expect(await endHeaderAfter.isExisting()).toBe(false);
    });

    it('should show end datetime column when toggled back on', async () => {
      // Seed a trip
      await seedTrip({
        vehicleId,
        startDatetime: '2026-01-15T08:00',
        endDatetime: '2026-01-15T10:00',
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'Work commute',
      });

      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Open column visibility dropdown
      const toggleBtn = await $('[data-testid="column-visibility-toggle"]');
      await toggleBtn.click();
      await browser.pause(300);

      // Toggle off (click label, not hidden checkbox)
      const timeToggleLabel = await $('[data-testid="column-toggle-time"]').parentElement();
      await timeToggleLabel.click();
      await browser.pause(300);

      // Verify hidden
      await browser.keys('Escape');
      await browser.pause(300);
      let endHeader = await $('[data-testid="column-header-end"]');
      expect(await endHeader.isExisting()).toBe(false);

      // Toggle back on
      await toggleBtn.click();
      await browser.pause(300);
      await timeToggleLabel.click();
      await browser.pause(300);
      await browser.keys('Escape');
      await browser.pause(300);

      // Verify visible again
      endHeader = await $('[data-testid="column-header-end"]');
      expect(await endHeader.isDisplayed()).toBe(true);
    });
  });
});
