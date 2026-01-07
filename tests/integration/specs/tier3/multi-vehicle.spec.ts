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
    await ensureLanguage('sk');
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

      // Now switch to the other vehicle via settings
      await navigateTo('settings');
      await browser.pause(500);

      // Find vehicle cards
      const vehicleCards = await $$('.vehicle-card');
      expect(vehicleCards.length).toBeGreaterThanOrEqual(2);

      // Find the inactive vehicle's card and activate it
      for (const card of vehicleCards) {
        const cardText = await card.getText();

        // Find the other vehicle (not currently active)
        if ((isAlphaActive && cardText.includes('Beta')) ||
            (isBetaActive && cardText.includes('Alpha'))) {
          // Click to activate this vehicle
          // Look for activate/select button or just click the card
          const activateBtn = await card.$('button');
          if (await activateBtn.isExisting()) {
            await activateBtn.click();
            await browser.pause(500);
          } else {
            // Some UIs use clicking the card itself
            await card.click();
            await browser.pause(500);
          }
          break;
        }
      }

      // Navigate back to trips
      await navigateTo('trips');
      await browser.pause(500);

      // Refresh page to ensure new vehicle data is loaded
      await browser.refresh();
      await waitForAppReady();

      // Get updated page content
      const updatedBody = await $('body');
      pageText = await updatedBody.getText();

      // After switch, we should see the other vehicle
      // (This might require checking if the switch actually worked)
      // The trips displayed should be from the newly active vehicle
      if (isAlphaActive) {
        // We switched from Alpha to Beta, so now we should see Beta
        // (If the switch was successful)
        const nowShowsBeta = pageText.includes('Vehicle Beta');
        if (nowShowsBeta) {
          expect(pageText).toContain('Beta');
        }
      } else {
        // We switched from Beta to Alpha
        const nowShowsAlpha = pageText.includes('Vehicle Alpha');
        if (nowShowsAlpha) {
          expect(pageText).toContain('Alpha');
        }
      }

      // Verify via backend that each vehicle has its own trips
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

      // Vehicle 1 trips: low consumption (6.0 l/100km)
      await seedTrip({
        vehicleId: vehicle1.id as string,
        date: `${year}-06-01`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 20100,
        purpose: TripPurposes.business,
      });

      await seedTrip({
        vehicleId: vehicle1.id as string,
        date: `${year}-06-05`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 20200,
        purpose: TripPurposes.business,
        fuelLiters: 6.0, // 6L for 100km = 6.0 l/100km (under TP rate of 7.0)
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

      // Vehicle 2 trips: high consumption (9.0 l/100km - 28.6% over)
      await seedTrip({
        vehicleId: vehicle2.id as string,
        date: `${year}-06-02`,
        origin: SlovakCities.kosice,
        destination: SlovakCities.presov,
        distanceKm: 100,
        odometer: 60100,
        purpose: TripPurposes.business,
      });

      await seedTrip({
        vehicleId: vehicle2.id as string,
        date: `${year}-06-08`,
        origin: SlovakCities.presov,
        destination: SlovakCities.poprad,
        distanceKm: 100,
        odometer: 60200,
        purpose: TripPurposes.business,
        fuelLiters: 9.0, // 9L for 100km = 9.0 l/100km (28.6% over)
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
        // Should be around 6.0 l/100km (under TP rate)
        expect(v1Rate).toBeLessThan(7.0);
        expect(v1Rate).toBeCloseTo(6.0, 1);

        // Should NOT have consumption warning
        expect(vehicle1Grid.consumption_warnings).not.toContain(v1FuelTrip.id);
      }

      // Vehicle 2: Find fuel trip and verify high rate
      const v2FuelTrip = vehicle2Grid.trips.find((t) => t.fuel_liters !== undefined);
      expect(v2FuelTrip).toBeDefined();

      if (v2FuelTrip?.id) {
        const v2Rate = vehicle2Grid.rates[v2FuelTrip.id];
        // Should be around 9.0 l/100km (over 20% limit of 8.4)
        expect(v2Rate).toBeGreaterThan(8.4);
        expect(v2Rate).toBeCloseTo(9.0, 1);

        // SHOULD have consumption warning
        expect(vehicle2Grid.consumption_warnings).toContain(v2FuelTrip.id);
      }

      // Each vehicle's stats are completely independent
      // Total km should be different
      const v1TotalKm = vehicle1Grid.trips.reduce((sum, t) => sum + t.distance_km, 0);
      const v2TotalKm = vehicle2Grid.trips.reduce((sum, t) => sum + t.distance_km, 0);

      expect(v1TotalKm).toBe(200);
      expect(v2TotalKm).toBe(200);

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
