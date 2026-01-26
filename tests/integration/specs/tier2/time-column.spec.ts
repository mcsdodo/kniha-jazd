/**
 * Tier 2: Trip Time Column Integration Tests
 *
 * Tests the time column functionality:
 * - Create trip with time → verify displays
 * - Edit trip time → verify saves
 * - Default time is 00:00
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

describe('Tier 2: Time Column', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    // Seed test vehicle (fresh data for each test)
    const vehicle = await seedVehicle({
      name: 'Time Test Vehicle',
      licensePlate: 'TIME-001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  describe('Time Display', () => {
    it('should display time column with seeded time value', async () => {
      // Seed a trip with specific time
      await seedTrip({
        vehicleId,
        date: '2026-01-15',
        time: '08:30',
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

      // Find the time column header
      const timeHeader = await $('[data-testid="column-header-time"]');
      expect(await timeHeader.isExisting()).toBe(true);

      // Find trip row and verify time displays
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      // The time should be displayed in the row
      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('08:30');
    });

    it('should display 00:00 for trip seeded without time', async () => {
      // Seed a trip without time (should default to 00:00)
      await seedTrip({
        vehicleId,
        date: '2026-01-15',
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

      // Find trip row and verify time displays as 00:00
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('00:00');
    });
  });

  describe('Time Entry', () => {
    it('should allow entering time when creating a new trip', async () => {
      // Navigate to trips page
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button.new-record');
      await newRecordBtn.waitForClickable({ timeout: 5000 });
      await newRecordBtn.click();
      await browser.pause(300);

      // Check that time input exists
      const timeInput = await $('[data-testid="trip-time"]');
      expect(await timeInput.isExisting()).toBe(true);

      // Default time should be 00:00
      const defaultTime = await timeInput.getValue();
      expect(defaultTime).toBe('00:00');

      // Set a specific time using atomic method
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-time"]', '14:45');

      // Verify the time was set
      const timeValue = await timeInput.getValue();
      expect(timeValue).toBe('14:45');
    });

    it('should save and display the entered time', async () => {
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

      // Date
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-date"]', today);

      // Time
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-time"]', '09:15');

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

      // Verify the trip appears with the correct time
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('09:15');
    });
  });
});
