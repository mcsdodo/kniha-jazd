/**
 * Tier 1: PHEV (Plug-in Hybrid Electric Vehicle) Trip Integration Tests
 *
 * Tests the trip management for plug-in hybrid vehicles.
 * PHEV vehicles can use both fuel AND energy on the same trip.
 * The system tracks both consumption systems independently.
 *
 * Key features tested:
 * - Mixed fuel and energy on same trip
 * - Both consumption rates in stats
 * - Fuel-only trips (energy_kwh = null)
 * - Energy-only trips (fuel_liters = null)
 * - Correct margin calculation for PHEV
 *
 * Business Rule: PHEV fuel consumption must be <= 120% of TP rate
 * (same as ICE vehicles, but only for the fuel portion)
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  getTripGridData,
} from '../../utils/db';
import { createSkodaOctaviaPhev, createVwPassatGte } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';

describe('Tier 1: PHEV Trips', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('PHEV Mixed Fuel and Energy', () => {
    it('should record both fuel and energy on same trip', async () => {
      // Create PHEV vehicle: Skoda Octavia iV
      // Tank: 40L, TP: 1.5 l/100km, Battery: 13 kWh, Baseline: 15 kWh/100km
      const vehicleData = createSkodaOctaviaPhev({
        name: 'PHEV Mixed Trip Test',
        licensePlate: 'PHEV-001',
        initialOdometer: 22000,
        tankSizeLiters: 40,
        tpConsumption: 1.5,
        batteryCapacityKwh: 13,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 100,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip with both fuel and energy refills
      // Long trip: 200km, refuel AND recharge at the same stop
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-15`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 200,
        odometer: 22200,
        purpose: TripPurposes.conference,
        // Fuel refill
        fuelLiters: 8,
        fuelCostEur: 12.00,
        fullTank: true,
        // Energy charging
        energyKwh: 10,
        energyCostEur: 3.50,
        fullCharge: true,
      });

      // Refresh to see the trip
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 1 trip
      expect(gridData.trips.length).toBe(1);

      // Verify both fuel and energy fields are present
      const savedTrip = gridData.trips[0];

      // Fuel fields
      expect(savedTrip.fuelLiters).toBe(8);
      expect(savedTrip.fuelCostEur).toBe(12.00);
      expect(savedTrip.fullTank).toBe(true);

      // Energy fields
      expect(savedTrip.energyKwh).toBe(10);
      expect(savedTrip.energyCostEur).toBe(3.50);
      expect(savedTrip.fullCharge).toBe(true);
    });
  });

  describe('PHEV Both Consumption Rates', () => {
    it('should show both consumption rates in stats', async () => {
      // Create PHEV vehicle: VW Passat GTE
      // Tank: 50L, TP: 1.6 l/100km, Battery: 14.1 kWh, Baseline: 16 kWh/100km
      const vehicleData = createVwPassatGte({
        name: 'PHEV Dual Rates Test',
        licensePlate: 'PHEV-002',
        initialOdometer: 35000,
        tankSizeLiters: 50,
        tpConsumption: 1.6,
        batteryCapacityKwh: 14.1,
        baselineConsumptionKwh: 16,
        initialBatteryPercent: 100,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Drive to deplete battery and use some fuel (establishes baseline)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.zilina,
        distanceKm: 200,
        odometer: 35200,
        purpose: TripPurposes.business,
      });

      // Trip 2: Refuel and recharge with full tank/charge
      // This allows calculation of both consumption rates
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-15`,
        origin: SlovakCities.zilina,
        destination: SlovakCities.kosice,
        distanceKm: 250,
        odometer: 35450,
        purpose: TripPurposes.business,
        fuelLiters: 6,
        fuelCostEur: 9.00,
        fullTank: true,
        energyKwh: 12,
        energyCostEur: 4.20,
        fullCharge: true,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the trip with both refills
      const refillTrip = gridData.trips.find((t) =>
        t.fuelLiters !== undefined && t.fuelLiters !== null &&
        t.energyKwh !== undefined && t.energyKwh !== null
      );
      expect(refillTrip).toBeDefined();

      const tripId = refillTrip?.id as string;

      // Both fuel and energy rates should be calculated
      // Fuel rate (l/100km)
      const fuelRate = gridData.rates[tripId];
      expect(fuelRate).toBeDefined();
      expect(fuelRate).toBeGreaterThan(0);

      // Energy rate (kWh/100km)
      const energyRate = gridData.energyRates[tripId];
      expect(energyRate).toBeDefined();
      expect(energyRate).toBeGreaterThan(0);

      // Both fuel and battery remaining should be tracked
      expect(gridData.fuelRemaining[tripId]).toBeDefined();
      expect(gridData.batteryRemainingKwh[tripId]).toBeDefined();
      expect(gridData.batteryRemainingPercent[tripId]).toBeDefined();
    });
  });

  describe('PHEV Fuel-Only Trip', () => {
    // This test only uses fuel fields, which ARE persisted, so it should work
    it('should handle fuel-only trip on PHEV (energy_kwh = null)', async () => {
      // Create PHEV vehicle
      const vehicleData = createSkodaOctaviaPhev({
        name: 'PHEV Fuel Only Test',
        licensePlate: 'PHEV-003',
        initialOdometer: 45000,
        tankSizeLiters: 40,
        tpConsumption: 1.5,
        batteryCapacityKwh: 13,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 0, // Battery depleted
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Establish baseline distance
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-03-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 65,
        odometer: 45065,
        purpose: TripPurposes.business,
      });

      // Fuel-only trip (battery depleted, no charging available)
      // This simulates running the PHEV in pure ICE mode
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-03-15`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 70,
        odometer: 45135,
        purpose: TripPurposes.business,
        fuelLiters: 5,
        fuelCostEur: 7.50,
        fullTank: true,
        // No energy fields - fuel only
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the fuel-only trip
      const fuelOnlyTrip = gridData.trips.find((t) =>
        t.fuelLiters !== undefined && t.fuelLiters !== null &&
        (t.energyKwh === undefined || t.energyKwh === null)
      );
      expect(fuelOnlyTrip).toBeDefined();

      // Verify fuel fields are present
      expect(fuelOnlyTrip?.fuelLiters).toBe(5);
      expect(fuelOnlyTrip?.fuelCostEur).toBe(7.50);
      expect(fuelOnlyTrip?.fullTank).toBe(true);

      // Energy fields should be null/undefined (Rust returns null for Option::None)
      expect(fuelOnlyTrip?.energyKwh).toBeNull();
      expect(fuelOnlyTrip?.energyCostEur).toBeNull();

      const tripId = fuelOnlyTrip?.id as string;

      // Fuel rate should be calculated
      const fuelRate = gridData.rates[tripId];
      expect(fuelRate).toBeDefined();
      expect(fuelRate).toBeGreaterThan(0);

      // Fuel remaining should be tracked
      expect(gridData.fuelRemaining[tripId]).toBeDefined();
    });
  });

  describe('PHEV Energy-Only Trip', () => {
    it('should handle energy-only trip on PHEV (fuel_liters = null)', async () => {
      // Create PHEV vehicle with full battery
      const vehicleData = createSkodaOctaviaPhev({
        name: 'PHEV Energy Only Test',
        licensePlate: 'PHEV-004',
        initialOdometer: 50000,
        tankSizeLiters: 40,
        tpConsumption: 1.5,
        batteryCapacityKwh: 13,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 100, // Full battery
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Establish baseline distance
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-04-10`,
        origin: SlovakCities.bratislava,
        destination: 'Bratislava - Petrzalka',
        distanceKm: 15,
        odometer: 50015,
        purpose: TripPurposes.clientMeeting,
      });

      // Energy-only trip (short trip, fully electric)
      // This simulates using only battery for short urban trips
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-04-15`,
        origin: 'Bratislava - Petrzalka',
        destination: SlovakCities.bratislava,
        distanceKm: 15,
        odometer: 50030,
        purpose: TripPurposes.commute,
        // No fuel fields - energy only
        energyKwh: 5,
        energyCostEur: 1.75,
        fullCharge: true,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the energy-only trip
      const energyOnlyTrip = gridData.trips.find((t) =>
        (t.fuelLiters === undefined || t.fuelLiters === null) &&
        t.energyKwh !== undefined && t.energyKwh !== null
      );
      expect(energyOnlyTrip).toBeDefined();

      // Verify energy fields are present
      expect(energyOnlyTrip?.energyKwh).toBe(5);
      expect(energyOnlyTrip?.energyCostEur).toBe(1.75);
      expect(energyOnlyTrip?.fullCharge).toBe(true);

      // Fuel fields should be null/undefined
      expect(energyOnlyTrip?.fuelLiters).toBeUndefined();
      expect(energyOnlyTrip?.fuelCostEur).toBeUndefined();

      const tripId = energyOnlyTrip?.id as string;

      // Energy rate should be calculated
      const energyRate = gridData.energyRates[tripId];
      expect(energyRate).toBeDefined();
      expect(energyRate).toBeGreaterThan(0);

      // Battery remaining should be tracked
      expect(gridData.batteryRemainingKwh[tripId]).toBeDefined();
      expect(gridData.batteryRemainingPercent[tripId]).toBeDefined();
    });
  });

  describe('PHEV Margin Calculation', () => {
    it('should calculate correct margin for PHEV with mixed usage', async () => {
      // Create PHEV vehicle: VW Passat GTE
      // TP consumption: 1.6 l/100km (only fuel portion is regulated)
      // Legal limit: 1.6 * 1.20 = 1.92 l/100km
      const vehicleData = createVwPassatGte({
        name: 'PHEV Margin Test',
        licensePlate: 'PHEV-005',
        initialOdometer: 60000,
        tankSizeLiters: 50,
        tpConsumption: 1.6,
        batteryCapacityKwh: 14.1,
        baselineConsumptionKwh: 16,
        initialBatteryPercent: 50, // Half battery
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
        batteryCapacityKwh: vehicleData.batteryCapacityKwh,
        baselineConsumptionKwh: vehicleData.baselineConsumptionKwh,
        initialBatteryPercent: vehicleData.initialBatteryPercent,
      });

      const year = new Date().getFullYear();

      // Trip 1: Drive to establish baseline distance
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-05-10`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.zilina,
        distanceKm: 200,
        odometer: 60200,
        purpose: TripPurposes.business,
      });

      // Trip 2: Refuel with consumption over 20% margin
      // TP rate: 1.6, Legal limit: 1.92, We'll create ~2.0 l/100km (25% over)
      // For 200km at 2.0 l/100km = 4 liters
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-05-15`,
        origin: SlovakCities.zilina,
        destination: SlovakCities.kosice,
        distanceKm: 200,
        odometer: 60400,
        purpose: TripPurposes.conference,
        fuelLiters: 4,
        fuelCostEur: 6.00,
        fullTank: true,
        energyKwh: 8,
        energyCostEur: 2.80,
        fullCharge: true,
      });

      // Refresh to see the trips
      await browser.refresh();
      await waitForAppReady();

      // Get grid data via IPC to verify
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Should have 2 trips
      expect(gridData.trips.length).toBe(2);

      // Find the trip with refueling
      const refuelTrip = gridData.trips.find((t) =>
        t.fuelLiters !== undefined && t.fuelLiters !== null
      );
      expect(refuelTrip).toBeDefined();

      const tripId = refuelTrip?.id as string;

      // Get fuel consumption rate
      const fuelRate = gridData.rates[tripId];
      expect(fuelRate).toBeDefined();

      // Fuel rate should be around 2.0 l/100km (4L / 200km * 100)
      // This exceeds the 20% margin (1.92 l/100km) so warning should be triggered
      expect(fuelRate).toBeGreaterThan(1.92);

      // Since consumption exceeds 20% margin, there should be a consumption warning
      expect(gridData.consumptionWarnings).toContain(tripId);

      // Energy rate should also be calculated independently
      const energyRate = gridData.energyRates[tripId];
      expect(energyRate).toBeDefined();
      expect(energyRate).toBeGreaterThan(0);

      // Both remaining values should be tracked
      expect(gridData.fuelRemaining[tripId]).toBeDefined();
      expect(gridData.batteryRemainingKwh[tripId]).toBeDefined();
    });
  });
});
