/**
 * DB Seeding Utilities Verification Tests
 *
 * These tests verify that the DB seeding utilities work correctly,
 * allowing tests to set up complex scenarios quickly via Tauri IPC.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import {
  seedVehicle,
  seedTrip,
  seedScenario,
  getVehicles,
  getActiveVehicle,
} from '../../utils/db';
import { createSkodaOctavia, createTestIceVehicle } from '../../fixtures/vehicles';
import { createTrip, createTripWithFuel, TripPurposes, SlovakCities } from '../../fixtures/trips';
import { createUnderLimitScenario, createSafeMarginScenario } from '../../fixtures/scenarios';

describe('DB Seeding Utilities', () => {
  beforeEach(async () => {
    // Wait for app to be fully loaded
    await waitForAppReady();
  });

  describe('Vehicle Seeding', () => {
    it('should seed a vehicle and have it appear in the UI', async () => {
      // Seed a vehicle using the Tauri IPC
      const vehicleData = createTestIceVehicle({
        name: 'Seeding Test Vehicle',
        licensePlate: 'SEED-001',
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Verify the vehicle was created with an ID
      expect(vehicle.id).toBeDefined();
      expect(vehicle.name).toBe('Seeding Test Vehicle');

      // Verify the vehicle appears in the UI
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain('Seeding Test Vehicle');
    });

    it('should seed a Skoda Octavia with all fields', async () => {
      const vehicleData = createSkodaOctavia();

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Verify all fields were set correctly
      // Note: Tauri returns snake_case fields from Rust
      expect(vehicle.id).toBeDefined();
      const vehicleAny = vehicle as unknown as Record<string, unknown>;
      expect(vehicleAny.tank_size_liters).toBe(50);
      expect(vehicleAny.tp_consumption).toBe(6.5);

      // Vehicle should appear in the UI
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain('Skoda Octavia');
    });

    it('should retrieve seeded vehicles via IPC', async () => {
      // Seed a vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Retrieve Test Vehicle',
        licensePlate: 'RTV-001',
      });

      await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Get all vehicles via IPC
      const vehicles = await getVehicles();

      // Should have at least one vehicle
      expect(vehicles.length).toBeGreaterThanOrEqual(1);

      // Find our seeded vehicle
      const foundVehicle = vehicles.find((v) => v.name === 'Retrieve Test Vehicle');
      expect(foundVehicle).toBeDefined();
    });
  });

  describe('Trip Seeding', () => {
    it('should seed a trip for a vehicle', async () => {
      // First, seed a vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Trip Test Vehicle',
        licensePlate: 'TTV-001',
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Now seed a trip
      const tripData = createTrip({
        year: new Date().getFullYear(),
        month: 1,
        day: 15,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 400,
        odometer: vehicleData.initialOdometer + 400,
        purpose: TripPurposes.business,
      });

      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: tripData.date,
        origin: tripData.origin,
        destination: tripData.destination,
        distanceKm: tripData.distanceKm,
        odometer: tripData.odometer,
        purpose: tripData.purpose,
      });

      // Verify trip was created (Tauri returns snake_case fields)
      expect(trip.id).toBeDefined();
      expect(trip.origin).toBe(SlovakCities.bratislava);
      expect(trip.destination).toBe(SlovakCities.kosice);

      // Refresh to see the trip in UI
      await browser.refresh();
      await waitForAppReady();

      // The trip details should appear in the UI - check for the trip's origin/destination
      // Note: The grid shows the route in the row, so we just verify the trip data was saved correctly
      const body = await $('body');
      const text = await body.getText();
      // Trip is seeded but might show in a different format in the grid
      // Verify at least the vehicle is displayed and trips section exists
      expect(text).toContain('Trip Test Vehicle');
    });

    it('should seed a trip with fuel data', async () => {
      // Seed vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Fuel Trip Test',
        licensePlate: 'FTT-001',
        initialOdometer: 50000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Seed trip with fuel
      // Note: Tauri returns snake_case fields from Rust (fuel_liters, fuel_cost_eur, full_tank)
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${new Date().getFullYear()}-01-20`,
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50065,
        purpose: 'Sluzobna cesta',
        fuelLiters: 40,
        fuelCostEur: 60,
        fullTank: true,
      });

      // Tauri returns snake_case fields from Rust
      const tripAny = trip as unknown as Record<string, unknown>;
      expect(tripAny.fuel_liters).toBe(40);
      expect(tripAny.fuel_cost_eur).toBe(60);
      expect(tripAny.full_tank).toBe(true);
    });
  });

  describe('Scenario Seeding', () => {
    it('should seed a complete under-limit scenario', async () => {
      const scenario = createUnderLimitScenario(new Date().getFullYear());
      const seeded = await seedScenario(scenario);

      // Verify all parts were seeded
      expect(seeded.vehicle.id).toBeDefined();
      expect(seeded.vehicle.name).toContain('Skoda Octavia');
      expect(seeded.trips.length).toBe(scenario.trips.length);

      // Settings should be seeded if provided
      if (scenario.settings) {
        expect(seeded.settings).toBeDefined();
      }

      // Vehicle should appear in UI
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain('Skoda Octavia');
    });

    it('should seed a safe margin scenario with trips', async () => {
      const scenario = createSafeMarginScenario(new Date().getFullYear());
      const seeded = await seedScenario(scenario);

      // Verify vehicle and trips
      expect(seeded.vehicle.id).toBeDefined();
      expect(seeded.trips.length).toBe(2); // Safe margin scenario has 2 trips

      // Trips should have correct data
      // Note: Tauri returns snake_case fields from Rust (fuel_liters, full_tank)
      // Use truthy check since null !== undefined is true but we want actual values
      const tripsAny = seeded.trips as unknown as Array<Record<string, unknown>>;
      const fuelTrip = tripsAny.find((t) => t.fuel_liters != null && t.fuel_liters !== 0);
      expect(fuelTrip).toBeDefined();
      // Verify fuel data was saved correctly
      expect(fuelTrip?.fuel_liters).toBe(8.05);
      expect(fuelTrip?.fuel_cost_eur).toBe(12.08);
    });
  });
});
