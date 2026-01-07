/**
 * Tier 1: Year Handling Integration Tests
 *
 * Tests the year picker functionality including:
 * - Filtering trips by selected year
 * - Fuel remaining carryover from previous year
 *
 * These are critical features for multi-year vehicle logbook management.
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
import { selectYear, getSelectedYear } from '../../utils/forms';
import { Nav } from '../../utils/assertions';

describe('Tier 1: Year Handling', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('sk');
  });

  describe('Year Filtering', () => {
    it('should filter trips by selected year', async () => {
      // Create vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Year Filter Test Vehicle',
        licensePlate: 'YFT-001',
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

      // Create trips in 2024
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2024-06-15',
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 65,
        odometer: 30065,
        purpose: TripPurposes.business,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2024-09-20',
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 70,
        odometer: 30135,
        purpose: TripPurposes.clientMeeting,
        fuelLiters: 35,
        fuelCostEur: 52.5,
        fullTank: true,
      });

      // Create trips in 2025
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2025-02-10',
        origin: SlovakCities.nitra,
        destination: SlovakCities.bratislava,
        distanceKm: 90,
        odometer: 30225,
        purpose: TripPurposes.conference,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2025-05-25',
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 400,
        odometer: 30625,
        purpose: TripPurposes.business,
        fuelLiters: 40,
        fuelCostEur: 60,
        fullTank: true,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2025-08-15',
        origin: SlovakCities.kosice,
        destination: SlovakCities.presov,
        distanceKm: 36,
        odometer: 30661,
        purpose: TripPurposes.delivery,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Verify we can see the year picker
      const yearPicker = await $(Nav.yearPicker);
      const yearPickerExists = await yearPicker.isExisting();
      expect(yearPickerExists).toBe(true);

      // Select 2024 and verify trip count
      await selectYear(2024);
      await browser.pause(500);

      const gridData2024 = await getTripGridData(vehicle.id as string, 2024);
      expect(gridData2024.trips.length).toBe(2);

      // Verify the trips are from 2024
      const trips2024Dates = gridData2024.trips.map((t) => t.date);
      expect(trips2024Dates.every((d) => d.startsWith('2024'))).toBe(true);

      // Select 2025 and verify trip count
      await selectYear(2025);
      await browser.pause(500);

      const gridData2025 = await getTripGridData(vehicle.id as string, 2025);
      expect(gridData2025.trips.length).toBe(3);

      // Verify the trips are from 2025
      const trips2025Dates = gridData2025.trips.map((t) => t.date);
      expect(trips2025Dates.every((d) => d.startsWith('2025'))).toBe(true);

      // Verify UI shows correct trips for each year
      // When 2025 is selected, should see Kosice trip
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain(SlovakCities.kosice);
    });
  });

  describe('Year Carryover', () => {
    it('should carry over fuel remaining from previous year', async () => {
      // Create vehicle with known tank size
      const vehicleData = createTestIceVehicle({
        name: 'Carryover Test Vehicle',
        licensePlate: 'COT-001',
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

      // End of 2024: Create trips with a full tank refill
      // This establishes the fuel state at year end
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2024-12-01',
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 65,
        odometer: 40065,
        purpose: TripPurposes.business,
      });

      // Fill tank at end of year (45 liters)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2024-12-15',
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 70,
        odometer: 40135,
        purpose: TripPurposes.business,
        fuelLiters: 45,
        fuelCostEur: 67.5,
        fullTank: true,
      });

      // Last trip of year (uses some fuel but doesn't refill)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2024-12-28',
        origin: SlovakCities.nitra,
        destination: SlovakCities.bratislava,
        distanceKm: 90,
        odometer: 40225,
        purpose: TripPurposes.business,
      });

      // Start of 2025: Trip without refuel (should use carryover fuel)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2025-01-05',
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 65,
        odometer: 40290,
        purpose: TripPurposes.business,
      });

      // Refresh and check
      await browser.refresh();
      await waitForAppReady();

      // Get grid data for 2024 to see end-of-year fuel remaining
      const gridData2024 = await getTripGridData(vehicle.id as string, 2024);
      expect(gridData2024.trips.length).toBe(3);

      // Find the last trip of 2024 to get its fuel remaining
      const lastTrip2024 = gridData2024.trips.find((t) => t.date === '2024-12-28');
      expect(lastTrip2024).toBeDefined();

      const endOfYearFuel = lastTrip2024?.id
        ? gridData2024.fuel_remaining[lastTrip2024.id]
        : undefined;

      // Should have fuel remaining at end of year
      // After driving 65+70+90 = 225km with consumption ~7 l/100km = ~15.75L used
      // After refilling 45L on trip 2, minus consumption since then
      // The exact value depends on calculation, but should be > 0
      expect(endOfYearFuel).toBeDefined();
      expect(endOfYearFuel).toBeGreaterThan(0);

      // Get grid data for 2025 to see carryover
      const gridData2025 = await getTripGridData(vehicle.id as string, 2025);
      expect(gridData2025.trips.length).toBe(1);

      // The first trip of 2025 should have fuel remaining calculated from carryover
      const firstTrip2025 = gridData2025.trips[0];
      expect(firstTrip2025).toBeDefined();

      const startOfYearFuel = firstTrip2025?.id
        ? gridData2025.fuel_remaining[firstTrip2025.id]
        : undefined;

      // Should have fuel remaining carried over (minus the trip consumption)
      expect(startOfYearFuel).toBeDefined();
      expect(startOfYearFuel).toBeGreaterThan(0);

      // The first trip of 2025 should have less fuel than end of 2024
      // because 65km was driven (at ~7 l/100km = ~4.55L used)
      if (endOfYearFuel !== undefined && startOfYearFuel !== undefined) {
        expect(startOfYearFuel).toBeLessThan(endOfYearFuel);
        // The difference should be approximately the fuel used for 65km
        const fuelUsed = endOfYearFuel - startOfYearFuel;
        // Expected: 65 * 7 / 100 = 4.55 L (using TP rate as estimate)
        expect(fuelUsed).toBeGreaterThan(3);
        expect(fuelUsed).toBeLessThan(10);
      }
    });
  });
});
