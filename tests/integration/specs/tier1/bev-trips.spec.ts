/**
 * Tier 1: BEV (Battery Electric Vehicle) Trip Integration Tests
 *
 * Tests the trip management for pure electric vehicles.
 * BEV vehicles use energy_kwh, energy_cost_eur fields instead of fuel.
 * No fuel-related fields should be populated for BEV trips.
 *
 * Key features tested:
 * - Energy charging sessions (kWh, cost)
 * - Energy consumption rate calculation (kWh/100km)
 * - Battery SoC tracking
 * - Null fuel fields on BEV trips
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  getTripGridData,
} from '../../utils/db';
import { createTeslaModel3, createSkodaEnyaq } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';

describe('Tier 1: BEV Trips', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('BEV Trip with Charging', () => {
    it('should create BEV trip with charging session (kWh, cost)', async () => {
      // Create BEV vehicle: Tesla Model 3
      // Battery: 75 kWh, Baseline consumption: 15 kWh/100km, Initial SoC: 90%
      const vehicleData = createTeslaModel3({
        name: 'BEV Charging Test',
        licensePlate: 'BEV-001',
        initialOdometer: 15000,
        batteryCapacityKwh: 75,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 90,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Drive 100km (uses ~15 kWh at baseline rate)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 15100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Drive 50km with charging session (full charge)
      // Charging: 30 kWh at 0.35 EUR/kWh = 10.50 EUR
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-15`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 50,
        odometer: 15150,
        purpose: TripPurposes.business,
        energyKwh: 30,
        energyCostEur: 10.50,
        fullCharge: true,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the trip with charging
      const chargeTrip = gridData.trips.find((t) => t.energyKwh !== undefined && t.energyKwh !== null);
      expect(chargeTrip).toBeDefined();
      expect(chargeTrip?.energyKwh).toBe(30);
      expect(chargeTrip?.energyCostEur).toBe(10.50);
      expect(chargeTrip?.fullCharge).toBe(true);

      // Fuel fields should be null/undefined for BEV trips (Rust returns null for Option::None)
      expect(chargeTrip?.fuelLiters).toBeNull();
      expect(chargeTrip?.fuelCostEur).toBeNull();
    });
  });

  describe('BEV Energy Consumption Rate', () => {
    it('should calculate energy consumption rate (kWh/100km)', async () => {
      // Create BEV vehicle: Skoda Enyaq
      // Battery: 77 kWh, Baseline consumption: 17 kWh/100km, Initial SoC: 100%
      const vehicleData = createSkodaEnyaq({
        name: 'BEV Rate Calc Test',
        licensePlate: 'BEV-002',
        initialOdometer: 8000,
        batteryCapacityKwh: 77,
        baselineConsumptionKwh: 17,
        initialBatteryPercent: 100,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Drive 100km (establishes distance since last charge)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.zilina,
        distanceKm: 100,
        odometer: 8100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Drive 50km with full charge
      // Total distance since last full charge: 100km + 50km = 150km
      // Energy charged: 18 kWh (full charge)
      // Consumption rate: 18 / 150 * 100 = 12 kWh/100km
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-15`,
        origin: SlovakCities.zilina,
        destination: SlovakCities.martin,
        distanceKm: 50,
        odometer: 8150,
        purpose: TripPurposes.business,
        energyKwh: 18,
        energyCostEur: 6.30,
        fullCharge: true,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the trip with charging
      const chargeTrip = gridData.trips.find((t) => t.energyKwh !== undefined && t.energyKwh !== null);
      expect(chargeTrip).toBeDefined();

      // Get energy consumption rate for this trip
      const tripId = chargeTrip?.id;
      if (tripId) {
        const rate = gridData.energyRates[tripId];
        // Rate should be around 12 kWh/100km (18 kWh / 150 km * 100)
        expect(rate).toBeDefined();
        expect(rate).toBeGreaterThan(0);
        expect(rate).toBeCloseTo(12, 1);
      }
    });
  });

  describe('BEV Battery SoC Tracking', () => {
    it('should track battery SoC remaining after trips', async () => {
      // Create BEV vehicle: Tesla Model 3
      // Battery: 75 kWh, Baseline consumption: 15 kWh/100km, Initial SoC: 90%
      // Initial battery: 75 * 0.90 = 67.5 kWh
      const vehicleData = createTeslaModel3({
        name: 'BEV SoC Tracking Test',
        licensePlate: 'BEV-003',
        initialOdometer: 20000,
        batteryCapacityKwh: 75,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 90,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Drive 100km (uses ~15 kWh at baseline rate)
      // After trip: 67.5 - 15 = 52.5 kWh (70% SoC)
      const trip1 = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-03-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 20100,
        purpose: TripPurposes.business,
      });

      // Trip 2: Drive another 100km without charging
      // After trip: 52.5 - 15 = 37.5 kWh (50% SoC)
      const trip2 = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-03-15`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 20200,
        purpose: TripPurposes.business,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Check battery remaining after each trip
      // Trip 1: 67.5 - 15 = 52.5 kWh
      const trip1Id = trip1.id as string;
      const battery1Kwh = gridData.batteryRemainingKwh[trip1Id];
      const battery1Percent = gridData.batteryRemainingPercent[trip1Id];

      expect(battery1Kwh).toBeDefined();
      expect(battery1Kwh).toBeCloseTo(52.5, 1);
      expect(battery1Percent).toBeDefined();
      expect(battery1Percent).toBeCloseTo(70, 1);

      // Trip 2: 52.5 - 15 = 37.5 kWh
      const trip2Id = trip2.id as string;
      const battery2Kwh = gridData.batteryRemainingKwh[trip2Id];
      const battery2Percent = gridData.batteryRemainingPercent[trip2Id];

      expect(battery2Kwh).toBeDefined();
      expect(battery2Kwh).toBeCloseTo(37.5, 1);
      expect(battery2Percent).toBeDefined();
      expect(battery2Percent).toBeCloseTo(50, 1);
    });
  });

  describe('BEV Trips Without Fuel Fields', () => {
    it('should create BEV trip without fuel fields (fuel_liters = null)', async () => {
      // Create BEV vehicle
      const vehicleData = createTeslaModel3({
        name: 'BEV No Fuel Test',
        licensePlate: 'BEV-004',
        initialOdometer: 25000,
        batteryCapacityKwh: 75,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 80,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Create a BEV trip - no fuel fields should be present
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-04-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 400,
        odometer: 25400,
        purpose: TripPurposes.conference,
        // BEV trip with energy fields only
        energyKwh: 70,
        energyCostEur: 24.50,
        fullCharge: true,
        // Explicitly NOT setting fuel fields
      });

      // Refresh to see the trip
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 1 trip
      expect(gridData.trips.length).toBe(1);

      // Verify the trip
      const savedTrip = gridData.trips[0];

      // Energy fields should be populated
      expect(savedTrip.energyKwh).toBe(70);
      expect(savedTrip.energyCostEur).toBe(24.50);
      expect(savedTrip.fullCharge).toBe(true);

      // Fuel fields should be null for BEV (Rust returns null for Option::None)
      // Note: fuelLiters and fuelCostEur are the important fields - fullTank is just a boolean
      // that may have a default value, which is irrelevant for BEV vehicles
      expect(savedTrip.fuelLiters).toBeNull();
      expect(savedTrip.fuelCostEur).toBeNull();

      // No fuel-related data in grid (BEV has no fuel system)
      const tripId = savedTrip.id;
      // For BEV, fuel rates may be undefined (not in the HashMap) rather than null
      expect(gridData.rates[tripId]).toBeFalsy(); // No fuel rate
      expect(gridData.fuelRemaining[tripId]).toBeFalsy(); // No fuel remaining

      // Energy data should exist
      expect(gridData.energyRates[tripId]).toBeDefined();
      expect(gridData.batteryRemainingKwh[tripId]).toBeDefined();
      expect(gridData.batteryRemainingPercent[tripId]).toBeDefined();

      // No consumption warnings for BEV (no legal limit for electricity)
      expect(gridData.consumptionWarnings.length).toBe(0);
    });
  });
});
