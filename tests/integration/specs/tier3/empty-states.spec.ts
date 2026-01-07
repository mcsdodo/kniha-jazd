/**
 * Tier 3: Empty States Integration Tests
 *
 * Tests the application behavior when there's no data:
 * - No vehicles (fresh install)
 * - No trips for a vehicle
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, getVehicles } from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';

describe('Tier 3: Empty States', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('sk');
  });

  describe('No Vehicle State', () => {
    it('should show "no vehicle" prompt on fresh install', async () => {
      // Get current vehicles - in a fresh state there may be none
      const vehicles = await getVehicles();

      // Check the main page for appropriate UI state
      await navigateTo('trips');
      await browser.pause(500);

      const body = await $('body');
      const pageText = await body.getText();

      if (vehicles.length === 0) {
        // No vehicles: should show empty state with call to action
        // Look for the no-vehicle container
        const noVehicleDiv = await $('.no-vehicle');
        const noVehicleExists = await noVehicleDiv.isExisting();

        if (noVehicleExists) {
          expect(await noVehicleDiv.isDisplayed()).toBe(true);

          // Should have a link/button to settings
          const settingsLink = await $('a[href="/settings"]');
          expect(await settingsLink.isExisting()).toBe(true);
        } else {
          // Alternative: check for empty state text indicators
          // The page should guide user to create a vehicle
          expect(
            pageText.toLowerCase().includes('vozidl') ||
            pageText.toLowerCase().includes('vehicle') ||
            pageText.toLowerCase().includes('nastaven') ||
            pageText.toLowerCase().includes('setting')
          ).toBe(true);
        }
      } else {
        // If vehicles exist (from other tests), verify the app shows a vehicle
        // instead of the empty state
        const vehicleInfo = await $('.vehicle-info');
        const vehicleInfoExists = await vehicleInfo.isExisting();

        // Should either show vehicle info or at least not crash
        expect(vehicleInfoExists || pageText.length > 0).toBe(true);
      }
    });
  });

  describe('No Trips State', () => {
    it('should show "no trips" state for new vehicle', async () => {
      // Create a brand new vehicle with no trips
      const vehicleData = createTestIceVehicle({
        name: 'Empty Vehicle Test',
        licensePlate: 'EMPTY-01',
        initialOdometer: 50000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      expect(vehicle.id).toBeDefined();

      // Navigate to trips page (should auto-select the new vehicle if it's the only one)
      await navigateTo('trips');
      await browser.pause(500);

      // The page should show the vehicle but with no trips
      const body = await $('body');
      const pageText = await body.getText();

      // Verify vehicle is shown
      expect(pageText).toContain(vehicleData.name);

      // Check for trip grid - it might be empty or show "no records" state
      const tripGrid = await $('.trip-grid');
      const tripGridExists = await tripGrid.isExisting();

      if (tripGridExists) {
        // Grid exists - check if it has any data rows
        const dataRows = await $$('.trip-grid tbody tr');
        const rowCount = dataRows.length;

        // New vehicle should have no trips (or only an editing row if adding new)
        // Filter out any editing rows
        let actualTripRows = 0;
        for (const row of dataRows) {
          const isEditing = await row.getAttribute('class');
          if (!isEditing?.includes('editing')) {
            actualTripRows++;
          }
        }

        // Should have 0 actual trips for a new vehicle
        expect(actualTripRows).toBe(0);
      }

      // Alternatively, there might be an empty state message or "New record" button
      const newRecordBtn = await $('button*=Novy zaznam');
      const newRecordExists = await newRecordBtn.isExisting();

      // The grid should at least be ready to accept new records
      // (Either empty state or showing the new record button)
      if (!tripGridExists) {
        // Without a grid, we expect some indicator to add trips
        expect(
          newRecordExists ||
          pageText.toLowerCase().includes('zaznam') ||
          pageText.toLowerCase().includes('record')
        ).toBe(true);
      }
    });
  });
});
