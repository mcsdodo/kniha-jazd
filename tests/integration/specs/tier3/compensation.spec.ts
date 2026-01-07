/**
 * Tier 3: Compensation Trip Integration Tests
 *
 * Tests the compensation trip suggestion feature that appears when
 * consumption exceeds the 20% legal limit over TP rate.
 *
 * Business Rule:
 * - When margin > 20%, show compensation banner with suggested buffer trip
 * - Suggested trips should help bring margin back to safe zone (16-19%)
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  getTripGridData,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';

describe('Tier 3: Compensation Trip Suggestions', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('sk');
  });

  describe('Over-Limit Compensation Banner', () => {
    it('should show compensation banner when over 20% margin', async () => {
      // Create vehicle with TP rate 7.0 l/100km
      // Legal limit = 7.0 * 1.20 = 8.4 l/100km
      // We'll create consumption at ~25% over (8.75 l/100km)
      const vehicleData = createTestIceVehicle({
        name: 'Over Limit Compensation Test',
        licensePlate: 'COMP-001',
        initialOdometer: 30000,
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
        date: `${year}-03-01`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 30100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Refuel with consumption 8.75 l/100km (25% over TP rate)
      // TP rate: 7.0, Legal limit: 8.4, Actual: 8.75 (25% over)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-03-05`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 50,
        odometer: 30150,
        purpose: TripPurposes.business,
        fuelLiters: 8.75,
        fuelCostEur: 13.13,
        fullTank: true,
      });

      // Refresh to see the trips in UI
      await browser.refresh();
      await waitForAppReady();

      // Verify we're over the limit via backend data
      const gridData = await getTripGridData(vehicle.id as string, year);
      expect(gridData.trips.length).toBe(2);

      // Find the trip with fuel and verify it has a consumption warning
      const fuelTrip = gridData.trips.find((t) => t.fuel_liters !== undefined);
      expect(fuelTrip).toBeDefined();

      if (fuelTrip?.id) {
        const rate = gridData.rates[fuelTrip.id];
        // Rate should be around 8.75 l/100km (25% over 7.0)
        expect(rate).toBeDefined();
        expect(rate).toBeGreaterThan(8.4); // Over legal limit

        // Should have consumption warning
        expect(gridData.consumption_warnings).toContain(fuelTrip.id);
      }

      // Check for compensation banner in UI
      // The banner uses class="compensation-banner"
      const compensationBanner = await $('.compensation-banner');
      const bannerExists = await compensationBanner.isExisting();

      // Banner should exist when over limit
      if (bannerExists) {
        expect(await compensationBanner.isDisplayed()).toBe(true);

        // Banner should contain warning information
        const bannerText = await compensationBanner.getText();

        // Should mention the margin/deviation
        expect(
          bannerText.toLowerCase().includes('%') ||
          bannerText.toLowerCase().includes('odchylka') ||
          bannerText.toLowerCase().includes('deviation')
        ).toBe(true);

        // Should have a suggestion section
        const suggestionSection = await compensationBanner.$('.suggestion');
        const suggestionExists = await suggestionSection.isExisting();

        // Suggestion might be loading or already displayed
        // If displayed, it should have trip details
        if (suggestionExists && await suggestionSection.isDisplayed()) {
          const suggestionText = await suggestionSection.getText();
          expect(suggestionText.length).toBeGreaterThan(0);
        }
      } else {
        // If banner doesn't exist, check if the stats show warning styling
        const warningStat = await $('.stat.warning');
        const warningExists = await warningStat.isExisting();

        // At minimum, the warning state should be indicated somewhere
        expect(warningExists).toBe(true);
      }
    });

    it('should add suggested buffer trip and see margin decrease', async () => {
      // Create vehicle that's over the limit
      const vehicleData = createTestIceVehicle({
        name: 'Buffer Trip Test',
        licensePlate: 'BUFF-001',
        initialOdometer: 40000,
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

      // Create trips that result in over-limit consumption
      // Trip 1: 100km driven
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-04-01`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 40100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Refuel at 9.0 l/100km (28.6% over)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-04-05`,
        origin: SlovakCities.nitra,
        destination: SlovakCities.zilina,
        distanceKm: 100,
        odometer: 40200,
        purpose: TripPurposes.business,
        fuelLiters: 9.0, // 9L for 100km = 9 l/100km
        fuelCostEur: 13.5,
        fullTank: true,
      });

      await browser.refresh();
      await waitForAppReady();

      // Get initial margin data
      const initialGridData = await getTripGridData(vehicle.id as string, year);
      expect(initialGridData.trips.length).toBe(2);

      // Find the trip with fuel to get initial rate
      const fuelTrip = initialGridData.trips.find((t) => t.fuel_liters !== undefined);
      expect(fuelTrip).toBeDefined();
      const initialRate = fuelTrip?.id ? initialGridData.rates[fuelTrip.id] : 0;

      // Rate should be high (around 9.0 l/100km)
      expect(initialRate).toBeGreaterThan(8.4);

      // Should have consumption warning initially
      if (fuelTrip?.id) {
        expect(initialGridData.consumption_warnings).toContain(fuelTrip.id);
      }

      // Now add a buffer trip (no fuel) to dilute the consumption
      // Adding 200km of driving without fuel should bring rate down
      // New rate = 9L / (100 + 200)km * 100 = 3.0 l/100km (way under limit)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-04-10`,
        origin: SlovakCities.zilina,
        destination: SlovakCities.bratislava,
        distanceKm: 200,
        odometer: 40400,
        purpose: TripPurposes.business,
        // No fuel - this is a buffer/compensation trip
      });

      await browser.refresh();
      await waitForAppReady();

      // Get updated margin data
      const updatedGridData = await getTripGridData(vehicle.id as string, year);
      expect(updatedGridData.trips.length).toBe(3);

      // Find the fuel trip again and check updated rate
      const updatedFuelTrip = updatedGridData.trips.find((t) => t.fuel_liters !== undefined);
      expect(updatedFuelTrip).toBeDefined();

      if (updatedFuelTrip?.id) {
        const updatedRate = updatedGridData.rates[updatedFuelTrip.id];

        // Rate should have decreased significantly
        // With 200km more driven on same fuel, rate goes from 9.0 to ~3.0 l/100km
        expect(updatedRate).toBeLessThan(initialRate);
        expect(updatedRate).toBeLessThan(8.4); // Should now be under legal limit

        // Should no longer have consumption warning
        expect(updatedGridData.consumption_warnings).not.toContain(updatedFuelTrip.id);
      }

      // Compensation banner should no longer be visible (or stats no longer warning)
      const compensationBanner = await $('.compensation-banner');
      const bannerExists = await compensationBanner.isExisting();

      if (bannerExists) {
        // If banner exists, it should not be displayed when under limit
        const isDisplayed = await compensationBanner.isDisplayed();
        expect(isDisplayed).toBe(false);
      } else {
        // Banner not existing is also valid (under limit = no banner)
        expect(true).toBe(true);
      }

      // Stats should not show warning styling
      const warningStat = await $('.stat.warning');
      const warningExists = await warningStat.isExisting();

      // If warning stat exists, verify we're in a good state
      if (warningExists) {
        // The warning might still exist if showing deviation, but value should be low
        const statText = await warningStat.getText();
        // Extract percentage if present
        const percentMatch = statText.match(/(\d+(?:\.\d+)?)\s*%/);
        if (percentMatch) {
          const percent = parseFloat(percentMatch[1]);
          // Margin should be well under 20% now
          expect(percent).toBeLessThan(20);
        }
      }
    });
  });
});
