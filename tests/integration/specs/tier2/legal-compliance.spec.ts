/**
 * Tier 2: Legal Compliance Columns Integration Tests
 *
 * Tests the Slovak legal compliance features (effective 1.1.2026):
 * - Trip numbering (4a)
 * - Driver name display (4b)
 * - Start and end times (4c)
 * - Odometer before/after (4f)
 * - Column visibility toggles
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, getTripGridData } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

/**
 * Get hidden columns via Tauri IPC
 */
async function getHiddenColumns(): Promise<string[]> {
  const result = await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('get_hidden_columns');
  });
  return result as string[];
}

/**
 * Set hidden columns via Tauri IPC
 */
async function setHiddenColumnsViaIpc(columns: string[]): Promise<void> {
  await browser.execute(async (cols: string[]) => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('set_hidden_columns', { columns: cols });
  }, columns);
}

describe('Tier 2: Legal Compliance Columns', () => {
  let vehicleId: string;
  const year = new Date().getFullYear();

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    // Reset hidden columns to show all legal compliance columns
    await setHiddenColumnsViaIpc([]);

    // Seed test vehicle with driver name
    const vehicle = await seedVehicle({
      name: 'Legal Compliance Test Vehicle',
      licensePlate: 'LEGAL-01',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
      driverName: 'Ján Novák',
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  describe('Trip Numbering (§4a)', () => {
    it('should display trip numbers in chronological order', async () => {
      // Seed multiple trips in non-chronological order
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
        time: '10:00',
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 50,
        odometer: 50050,
        purpose: 'Second trip',
      });

      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-10T08:00`,
        time: '08:00',
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'First trip',
      });

      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-20T08:00`,
        time: '14:00',
        origin: 'Trnava',
        destination: 'Nitra',
        distanceKm: 70,
        odometer: 50120,
        purpose: 'Third trip',
      });

      // Refresh to show seeded data
      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Get grid data to verify trip numbers
      const gridData = await getTripGridData(vehicleId, year);

      expect(gridData.trips.length).toBe(3);
      expect(Object.keys(gridData.tripNumbers).length).toBe(3);

      // Find trips by date and verify numbering
      const trip1 = gridData.trips.find(t => t.startDatetime.includes('-01-10'));
      const trip2 = gridData.trips.find(t => t.startDatetime.includes('-01-15'));
      const trip3 = gridData.trips.find(t => t.startDatetime.includes('-01-20'));

      expect(gridData.tripNumbers[trip1!.id!]).toBe(1);
      expect(gridData.tripNumbers[trip2!.id!]).toBe(2);
      expect(gridData.tripNumbers[trip3!.id!]).toBe(3);
    });

    it('should display trip number column in UI', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
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

      // Check that trip number column exists
      const tripNumberHeader = await $('.col-trip-number');
      expect(await tripNumberHeader.isExisting()).toBe(true);

      // Check that trip row shows number
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('1'); // First trip should be #1
    });
  });

  describe('End Time (§4c)', () => {
    it('should display end time column with seeded value', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
        endDatetime: `${year}-01-15T09:45`,
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

      // Verify end datetime column header exists (renamed from end-time to end)
      const endTimeHeader = await $('[data-testid="column-header-end"]');
      expect(await endTimeHeader.isExisting()).toBe(true);

      // Verify the trip row contains the end time
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('09:45');
    });

    it('should allow entering end time in trip form', async () => {
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button.new-record');
      await newRecordBtn.waitForClickable({ timeout: 5000 });
      await newRecordBtn.click();
      await browser.pause(300);

      // Check that end datetime input exists (now datetime-local type)
      const endTimeInput = await $('[data-testid="trip-end-datetime"]');
      expect(await endTimeInput.isExisting()).toBe(true);

      // Set end datetime using atomic method (requires YYYY-MM-DDTHH:MM format)
      const today = new Date().toISOString().split('T')[0];
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-end-datetime"]', `${today}T17:30`);

      // Verify the datetime was set
      const timeValue = await endTimeInput.getValue();
      expect(timeValue).toBe(`${today}T17:30`);
    });

    it('should save and display entered end time', async () => {
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click new record button
      const newRecordBtn = await $('button.new-record');
      await newRecordBtn.waitForClickable({ timeout: 5000 });
      await newRecordBtn.click();
      await browser.pause(300);

      const today = new Date().toISOString().split('T')[0];

      // Fill required fields using atomic method
      // Note: trip-start-datetime is datetime-local type, requires YYYY-MM-DDTHH:MM format
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-start-datetime"]', `${today}T08:00`);

      // Set end datetime (also datetime-local format)
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, '[data-testid="trip-end-datetime"]', `${today}T09:30`);

      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-origin"]', 'TestOrigin');

      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-destination"]', 'TestDestination');

      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-distance"]', '30');

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

      // Verify the trip appears with the end time
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('09:30');
    });
  });

  describe('Driver Name (§4b)', () => {
    it('should display driver name column from vehicle settings', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
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

      // Verify driver column exists
      const driverHeader = await $('.col-driver');
      expect(await driverHeader.isExisting()).toBe(true);

      // Verify the trip row shows driver name
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      expect(rowText).toContain('Ján Novák');
    });
  });

  describe('Odometer Start (§4f)', () => {
    it('should display odometer start derived from previous trip', async () => {
      // First trip: odo ends at 50025
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-10T08:00`,
        origin: 'Home',
        destination: 'Office',
        distanceKm: 25,
        odometer: 50025,
        purpose: 'First trip',
      });

      // Second trip: odo starts at 50025 (previous trip's end)
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
        origin: 'Office',
        destination: 'Home',
        distanceKm: 25,
        odometer: 50050,
        purpose: 'Second trip',
      });

      await browser.refresh();
      await waitForAppReady();

      // Get grid data to verify odometer start
      const gridData = await getTripGridData(vehicleId, year);

      expect(gridData.trips.length).toBe(2);

      // First trip should have initial odometer as start (50000)
      const trip1 = gridData.trips.find(t => t.purpose === 'First trip');
      expect(gridData.odometerStart[trip1!.id!]).toBe(50000);

      // Second trip should have first trip's end odo as start (50025)
      const trip2 = gridData.trips.find(t => t.purpose === 'Second trip');
      expect(gridData.odometerStart[trip2!.id!]).toBe(50025);
    });

    it('should display odo start column in UI', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
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

      // Verify odo start column exists
      const odoStartHeader = await $('.col-odo-start');
      expect(await odoStartHeader.isExisting()).toBe(true);

      // Verify the trip row shows odo start value
      const tripRows = await $$('.trip-grid tbody tr:not(.first-record):not(.editing)');
      expect(tripRows.length).toBeGreaterThan(0);

      const rowText = await tripRows[0].getText();
      // First trip should show initial odometer (50000)
      expect(rowText).toContain('50000');
    });
  });

  describe('Column Visibility', () => {
    it('should hide trip number column when toggled off', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
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

      // Verify trip number column is initially visible
      let tripNumHeader = await $('.col-trip-number');
      expect(await tripNumHeader.isExisting()).toBe(true);

      // Hide the column via IPC
      await setHiddenColumnsViaIpc(['tripNumber']);

      // Refresh to pick up changes
      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Verify trip number column is hidden
      tripNumHeader = await $('.col-trip-number');
      expect(await tripNumHeader.isExisting()).toBe(false);

      // Verify via IPC
      const hiddenColumns = await getHiddenColumns();
      expect(hiddenColumns).toContain('tripNumber');
    });

    it('should hide driver column when toggled off', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
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

      // Verify driver column is initially visible
      let driverHeader = await $('.col-driver');
      expect(await driverHeader.isExisting()).toBe(true);

      // Hide the column via IPC
      await setHiddenColumnsViaIpc(['driver']);

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Verify driver column is hidden
      driverHeader = await $('.col-driver');
      expect(await driverHeader.isExisting()).toBe(false);
    });

    it('should hide odo start column when toggled off', async () => {
      await seedTrip({
        vehicleId,
        startDatetime: `${year}-01-15T08:00`,
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

      // Verify odo start column is initially visible
      let odoStartHeader = await $('.col-odo-start');
      expect(await odoStartHeader.isExisting()).toBe(true);

      // Hide the column via IPC
      await setHiddenColumnsViaIpc(['odoStart']);

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Verify odo start column is hidden
      odoStartHeader = await $('.col-odo-start');
      expect(await odoStartHeader.isExisting()).toBe(false);
    });

    it('should persist multiple hidden legal compliance columns', async () => {
      // Hide multiple legal compliance columns
      await setHiddenColumnsViaIpc(['tripNumber', 'driver', 'odoStart']);

      // Refresh to pick up hidden columns changes
      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Verify all three are hidden
      const tripNumHeader = await $('.col-trip-number');
      const driverHeader = await $('.col-driver');
      const odoStartHeader = await $('.col-odo-start');

      expect(await tripNumHeader.isExisting()).toBe(false);
      expect(await driverHeader.isExisting()).toBe(false);
      expect(await odoStartHeader.isExisting()).toBe(false);

      // Reload and verify persistence
      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const hiddenColumns = await getHiddenColumns();
      expect(hiddenColumns).toContain('tripNumber');
      expect(hiddenColumns).toContain('driver');
      expect(hiddenColumns).toContain('odoStart');
    });
  });
});
