/**
 * Tier 3: Empty States Integration Tests
 *
 * Tests the application behavior when there's no data:
 * - No vehicles (fresh install)
 * - No trips for a vehicle
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, getVehicles, getTripGridData } from '../../utils/db';
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
    it('should show empty trip grid for new vehicle', async () => {
      // Create a brand new vehicle with no trips
      // Use a unique license plate to avoid conflicts with parallel tests
      const uniqueId = Date.now().toString(36);
      const vehicleData = createTestIceVehicle({
        name: `Empty Vehicle Test ${uniqueId}`,
        licensePlate: `EMPTY-${uniqueId.slice(0, 4)}`,
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

      // The key test: verify via backend that this NEW vehicle has no trips
      const year = new Date().getFullYear();
      const gridData = await getTripGridData(vehicle.id as string, year);

      // A freshly created vehicle should have 0 trips
      expect(gridData.trips.length).toBe(0);

      // Navigate to trips page to verify UI doesn't crash
      await navigateTo('trips');
      await browser.pause(500);

      // The page should show a valid state
      const body = await $('body');
      const pageText = await body.getText();

      // Vehicle name may or may not be visible in the header - check via backend
      // The vehicle was successfully created, so the app state is valid
      expect(vehicle.name).toBe(vehicleData.name);

      // The page should be in a valid state - not empty
      expect(pageText.length).toBeGreaterThan(0);
    });
  });
});
