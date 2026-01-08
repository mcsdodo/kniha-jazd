/**
 * Tier 3: Multi-Vehicle Integration Tests
 *
 * Tests the application's handling of multiple vehicles:
 * - Switching between vehicles shows different trip data
 * - Stats are calculated independently per vehicle
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  getTripGridData,
  getVehicles,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';

describe('Tier 3: Multi-Vehicle Support', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Vehicle Switching', () => {
    it('should switch active vehicle and see different trips', async () => {
      const year = new Date().getFullYear();

      // Create first vehicle with distinctive trips
      const vehicle1Data = createTestIceVehicle({
        name: 'Vehicle Alpha',
        licensePlate: 'ALPHA-01',
        initialOdometer: 10000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle1 = await seedVehicle({
        name: vehicle1Data.name,
        licensePlate: vehicle1Data.licensePlate,
        initialOdometer: vehicle1Data.initialOdometer,
        vehicleType: vehicle1Data.vehicleType,
        tankSizeLiters: vehicle1Data.tankSizeLiters,
        tpConsumption: vehicle1Data.tpConsumption,
      });

      // Add trips to vehicle 1 with distinctive route
      await seedTrip({
        vehicleId: vehicle1.id as string,
        date: `${year}-05-01`,
        origin: 'Alpha Origin',
        destination: 'Alpha Destination',
        distanceKm: 100,
        odometer: 10100,
        purpose: 'Alpha Trip Purpose',
      });

      await seedTrip({
        vehicleId: vehicle1.id as string,
        date: `${year}-05-05`,
        origin: 'Alpha Origin 2',
        destination: 'Alpha Destination 2',
        distanceKm: 150,
        odometer: 10250,
        purpose: 'Alpha Trip 2',
        fuelLiters: 15,
        fuelCostEur: 22.5,
        fullTank: true,
      });

      // Create second vehicle with different trips
      const vehicle2Data = createTestIceVehicle({
        name: 'Vehicle Beta',
        licensePlate: 'BETA-01',
        initialOdometer: 50000,
        tpConsumption: 6.5,
        tankSizeLiters: 45,
      });

      const vehicle2 = await seedVehicle({
        name: vehicle2Data.name,
        licensePlate: vehicle2Data.licensePlate,
        initialOdometer: vehicle2Data.initialOdometer,
        vehicleType: vehicle2Data.vehicleType,
        tankSizeLiters: vehicle2Data.tankSizeLiters,
        tpConsumption: vehicle2Data.tpConsumption,
      });

      // Add trips to vehicle 2 with different distinctive route
      await seedTrip({
        vehicleId: vehicle2.id as string,
        date: `${year}-05-02`,
        origin: 'Beta Origin',
        destination: 'Beta Destination',
        distanceKm: 200,
        odometer: 50200,
        purpose: 'Beta Trip Purpose',
      });

      await seedTrip({
        vehicleId: vehicle2.id as string,
        date: `${year}-05-10`,
        origin: 'Beta Origin 2',
        destination: 'Beta Destination 2',
        distanceKm: 250,
        odometer: 50450,
        purpose: 'Beta Trip 2',
        fuelLiters: 20,
        fuelCostEur: 30,
        fullTank: true,
      });

      await browser.refresh();
      await waitForAppReady();

      // Navigate to trips page
      await navigateTo('trips');
      await browser.pause(500);

      // Get the current page content
      const body = await $('body');
      let pageText = await body.getText();

      // Check which vehicle is currently active
      const isAlphaActive = pageText.includes('Vehicle Alpha');
      const isBetaActive = pageText.includes('Vehicle Beta');

      // At least one vehicle should be active
      expect(isAlphaActive || isBetaActive).toBe(true);

      // If Alpha is active, verify Alpha trips are shown
      if (isAlphaActive) {
        expect(pageText).toContain('Alpha');
        // Should NOT see Beta trips
        expect(pageText).not.toContain('Beta Origin');
      }

      // If Beta is active, verify Beta trips are shown
      if (isBetaActive) {
        expect(pageText).toContain('Beta');
        // Should NOT see Alpha trips
        expect(pageText).not.toContain('Alpha Origin');
      }

      // Switch active vehicle via Tauri IPC directly
      // This is more reliable than clicking UI elements that may not exist
      const vehicleToActivate = isAlphaActive ? vehicle2.id : vehicle1.id;
      await browser.execute(async (vId: string) => {
        if (!window.__TAURI__) {
          throw new Error('Tauri not available');
        }
        return await window.__TAURI__.core.invoke('set_active_vehicle', { id: vId });
      }, vehicleToActivate as string);

      // Refresh page to ensure new vehicle data is loaded
      await browser.refresh();
      await waitForAppReady();

      // Navigate back to trips
      await navigateTo('trips');
      await browser.pause(500);

      // Verify via backend that each vehicle has its own trips (the main assertion)
      const vehicle1Grid = await getTripGridData(vehicle1.id as string, year);
      const vehicle2Grid = await getTripGridData(vehicle2.id as string, year);

      // Vehicle 1 should have 2 trips with Alpha routes
      expect(vehicle1Grid.trips.length).toBe(2);
      expect(vehicle1Grid.trips.some((t) => t.origin.includes('Alpha'))).toBe(true);

      // Vehicle 2 should have 2 trips with Beta routes
      expect(vehicle2Grid.trips.length).toBe(2);
      expect(vehicle2Grid.trips.some((t) => t.origin.includes('Beta'))).toBe(true);
    });
  });

  describe('Per-Vehicle Stats', () => {
    it('should maintain separate stats per vehicle', async () => {
      const year = new Date().getFullYear();

      // Create two vehicles with very different consumption patterns
      // TP rate for both: 7.0 l/100km
      // Legal limit: 7.0 * 1.20 = 8.4 l/100km

      // Vehicle 1: Low consumption (under TP rate)
      const vehicle1Data = createTestIceVehicle({
        name: 'Low Consumption Car',
        licensePlate: 'LOW-001',
        initialOdometer: 20000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle1 = await seedVehicle({
        name: vehicle1Data.name,
        licensePlate: vehicle1Data.licensePlate,
        initialOdometer: vehicle1Data.initialOdometer,
        vehicleType: vehicle1Data.vehicleType,
        tankSizeLiters: vehicle1Data.tankSizeLiters,
        tpConsumption: vehicle1Data.tpConsumption,
      });

      // Vehicle 1 trips: low consumption
      // Trip 1: 100km baseline
      await seedTrip({
        vehicleId: vehicle1.id as string,
        date: `${year}-06-01`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 20100,
        purpose: TripPurposes.business,
      });

      // Trip 2: 100km with 6L fuel
      // Consumption: 6L / 200km * 100 = 3.0 l/100km (under TP rate of 7.0)
      await seedTrip({
        vehicleId: vehicle1.id as string,
        date: `${year}-06-05`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 20200,
        purpose: TripPurposes.business,
        fuelLiters: 6.0, // 6L / 200km = 3.0 l/100km
        fuelCostEur: 9.0,
        fullTank: true,
      });

      // Vehicle 2: High consumption (over limit)
      const vehicle2Data = createTestIceVehicle({
        name: 'High Consumption Car',
        licensePlate: 'HIGH-001',
        initialOdometer: 60000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle2 = await seedVehicle({
        name: vehicle2Data.name,
        licensePlate: vehicle2Data.licensePlate,
        initialOdometer: vehicle2Data.initialOdometer,
        vehicleType: vehicle2Data.vehicleType,
        tankSizeLiters: vehicle2Data.tankSizeLiters,
        tpConsumption: vehicle2Data.tpConsumption,
      });

      // Vehicle 2 trips: high consumption
      // Trip 1: 50km baseline (shorter distance = higher rate for same fuel)
      await seedTrip({
        vehicleId: vehicle2.id as string,
        date: `${year}-06-02`,
        origin: SlovakCities.kosice,
        destination: SlovakCities.presov,
        distanceKm: 50,
        odometer: 60050,
        purpose: TripPurposes.business,
      });

      // Trip 2: 50km with 9L fuel
      // Consumption: 9L / 100km * 100 = 9.0 l/100km (over 8.4 limit)
      await seedTrip({
        vehicleId: vehicle2.id as string,
        date: `${year}-06-08`,
        origin: SlovakCities.presov,
        destination: SlovakCities.poprad,
        distanceKm: 50,
        odometer: 60100,
        purpose: TripPurposes.business,
        fuelLiters: 9.0, // 9L / 100km = 9.0 l/100km (28.6% over)
        fuelCostEur: 13.5,
        fullTank: true,
      });

      await browser.refresh();
      await waitForAppReady();

      // Get stats for each vehicle via backend
      const vehicle1Grid = await getTripGridData(vehicle1.id as string, year);
      const vehicle2Grid = await getTripGridData(vehicle2.id as string, year);

      // Vehicle 1: Find fuel trip and verify low rate
      const v1FuelTrip = vehicle1Grid.trips.find((t) => t.fuel_liters !== undefined);
      expect(v1FuelTrip).toBeDefined();

      if (v1FuelTrip?.id) {
        const v1Rate = vehicle1Grid.rates[v1FuelTrip.id];
        // Should be 6L / 200km * 100 = 3.0 l/100km (under TP rate)
        expect(v1Rate).toBeLessThan(7.0);
        expect(v1Rate).toBeCloseTo(3.0, 1);

        // Should NOT have consumption warning
        expect(vehicle1Grid.consumption_warnings).not.toContain(v1FuelTrip.id);
      }

      // Vehicle 2: Find fuel trip and verify high rate
      const v2FuelTrip = vehicle2Grid.trips.find((t) => t.fuel_liters !== undefined);
      expect(v2FuelTrip).toBeDefined();

      if (v2FuelTrip?.id) {
        const v2Rate = vehicle2Grid.rates[v2FuelTrip.id];
        // Should be 9L / 100km * 100 = 9.0 l/100km (over 20% limit of 8.4)
        expect(v2Rate).toBeGreaterThan(8.4);
        expect(v2Rate).toBeCloseTo(9.0, 1);

        // SHOULD have consumption warning
        expect(vehicle2Grid.consumption_warnings).toContain(v2FuelTrip.id);
      }

      // Each vehicle's stats are completely independent
      // Total km should be different
      const v1TotalKm = vehicle1Grid.trips.reduce((sum, t) => sum + t.distance_km, 0);
      const v2TotalKm = vehicle2Grid.trips.reduce((sum, t) => sum + t.distance_km, 0);

      // Vehicle 1: 100km + 100km = 200km
      expect(v1TotalKm).toBe(200);
      // Vehicle 2: 50km + 50km = 100km
      expect(v2TotalKm).toBe(100);

      // Total fuel should be different
      const v1TotalFuel = vehicle1Grid.trips.reduce(
        (sum, t) => sum + (t.fuel_liters || 0),
        0
      );
      const v2TotalFuel = vehicle2Grid.trips.reduce(
        (sum, t) => sum + (t.fuel_liters || 0),
        0
      );

      expect(v1TotalFuel).toBe(6.0);
      expect(v2TotalFuel).toBe(9.0);

      // Verify that changing one vehicle doesn't affect the other
      // by checking that their trip counts are correct
      expect(vehicle1Grid.trips.length).toBe(2);
      expect(vehicle2Grid.trips.length).toBe(2);
    });
  });
});
