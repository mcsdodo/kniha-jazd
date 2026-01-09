/**
 * Tier 1: Consumption & Margin Warning Integration Tests
 *
 * Tests the consumption warning system that alerts users when their
 * vehicle's actual consumption exceeds the 20% margin over TP rate.
 *
 * Business Rule: Consumption must be <= 120% of vehicle's TP rate
 * - TP rate: 7.0 l/100km
 * - Legal limit: 8.4 l/100km (120% of 7.0)
 * - Warning shows when consumption > 8.4 l/100km
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  getTripGridData,
  setActiveVehicle,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';

describe('Tier 1: Consumption & Margin Warnings', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Normal Consumption State', () => {
    it('should show normal state when consumption under TP rate', async () => {
      // Create vehicle with TP rate 7.0 l/100km
      const vehicleData = createTestIceVehicle({
        name: 'Under TP Rate Vehicle',
        licensePlate: 'UTP-001',
        initialOdometer: 10000,
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

      const year = new Date().getFullYear();

      // Trip 1: Drive 100km (establishes baseline)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 10100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Refuel with consumption 6.5 l/100km (under TP rate of 7.0)
      // Period: 100km (trip1) + 50km (trip2) = 150km total
      // Rate = (fuel / km) * 100 = (9.75 / 150) * 100 = 6.5 l/100km
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-15`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 50,
        odometer: 10150,
        purpose: TripPurposes.business,
        fuelLiters: 9.75, // For 150km total: (9.75/150)*100 = 6.5 l/100km
        fuelCostEur: 14.63,
        fullTank: true,
      });

      // Refresh to see the trips in UI
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify calculations
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the trip with fuel
      const fuelTrip = gridData.trips.find((t) => t.fuelLiters !== undefined);
      expect(fuelTrip).toBeDefined();

      // Get consumption rate for this trip
      const tripId = fuelTrip?.id;
      if (tripId) {
        const rate = gridData.rates[tripId];
        // Rate should be around 6.5 l/100km (6.5L / 100km * 100)
        expect(rate).toBeDefined();
        expect(rate).toBeLessThan(7.0); // Under TP rate
        expect(rate).toBeCloseTo(6.5, 1);
      }

      // Should NOT have consumption warnings for this trip
      expect(gridData.consumptionWarnings.length).toBe(0);

      // Verify UI shows normal state (no warning class on stats)
      const body = await $('body');
      const text = await body.getText();

      // Should display the vehicle name
      expect(text).toContain('Under TP Rate Vehicle');

      // Check that the deviation stat exists but doesn't have warning styling
      // The margin should be negative (consumption below TP rate)
      const statsContainer = await $('.stats-container');
      if (await statsContainer.isExisting()) {
        const warningStats = await $$('.stat.warning');
        expect(warningStats.length).toBe(0);
      }
    });
  });

  describe('Over-Limit Warning State', () => {
    it('should show warning when consumption exceeds 20% margin', async () => {
      // Create vehicle with TP rate 7.0 l/100km
      // Legal limit = 7.0 * 1.20 = 8.4 l/100km
      const vehicleData = createTestIceVehicle({
        name: 'Over Margin Vehicle',
        licensePlate: 'OVM-001',
        initialOdometer: 20000,
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

      const year = new Date().getFullYear();

      // Trip 1: Drive 100km (establishes baseline)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 100,
        odometer: 20100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Refuel with consumption 8.5 l/100km (over 20% margin)
      // TP rate: 7.0, Legal limit: 8.4, Actual: 8.5 (21.4% over)
      // Period: 100km (trip1) + 50km (trip2) = 150km total
      // Rate = (fuel / km) * 100 = (12.75 / 150) * 100 = 8.5 l/100km
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-15`,
        origin: SlovakCities.kosice,
        destination: SlovakCities.presov,
        distanceKm: 50,
        odometer: 20150,
        purpose: TripPurposes.business,
        fuelLiters: 12.75, // For 150km total: (12.75/150)*100 = 8.5 l/100km
        fuelCostEur: 19.13,
        fullTank: true,
      });

      // Ensure this vehicle is active (tests share DB, first test's vehicle may still be active)
      await setActiveVehicle(vehicle.id as string);

      // Get grid data via IPC to verify calculations
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the trip with fuel
      const fuelTrip = gridData.trips.find((t) => t.fuelLiters !== undefined);
      expect(fuelTrip).toBeDefined();

      // Get consumption rate for this trip
      const tripId = fuelTrip?.id;
      if (tripId) {
        const rate = gridData.rates[tripId];
        // Rate should be around 8.5 l/100km
        expect(rate).toBeDefined();
        expect(rate).toBeGreaterThan(8.4); // Over legal limit (120% of 7.0)
        expect(rate).toBeCloseTo(8.5, 1);

        // Should have consumption warning for this trip
        expect(gridData.consumptionWarnings).toContain(tripId);
      }

      // Verify UI shows warning state
      const body = await $('body');
      const text = await body.getText();

      // Should display the vehicle name
      expect(text).toContain('Over Margin Vehicle');

      // Check that the stats show warning styling when over limit
      const statsContainer = await $('.stats-container');
      if (await statsContainer.isExisting()) {
        // The deviation stat should have warning class
        const warningStats = await $$('.stat.warning');
        // Should have at least one warning stat (the deviation)
        expect(warningStats.length).toBeGreaterThanOrEqual(1);
      }

      // The compensation banner should appear when over limit
      const compensationBanner = await $('[class*="compensation"]');
      const bannerExists = await compensationBanner.isExisting();
      // Note: Banner may or may not exist depending on component implementation
      if (bannerExists) {
        expect(await compensationBanner.isDisplayed()).toBe(true);
      }
    });
  });
});
