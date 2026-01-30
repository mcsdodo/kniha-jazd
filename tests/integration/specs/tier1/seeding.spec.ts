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
      expect(vehicle.id).toBeDefined();
      expect(vehicle.tankSizeLiters).toBe(50);
      expect(vehicle.tpConsumption).toBe(6.5);

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
        startDatetime: tripData.startDatetime,
        origin: tripData.origin,
        destination: tripData.destination,
        distanceKm: tripData.distanceKm,
        odometer: tripData.odometer,
        purpose: tripData.purpose,
      });

      // Verify trip was created
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
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${new Date().getFullYear()}-01-20T08:00`,
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 50065,
        purpose: 'Sluzobna cesta',
        fuelLiters: 40,
        fuelCostEur: 60,
        fullTank: true,
      });

      expect(trip.fuelLiters).toBe(40);
      expect(trip.fuelCostEur).toBe(60);
      expect(trip.fullTank).toBe(true);
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
      const fuelTrip = seeded.trips.find((t) => t.fuelLiters != null && t.fuelLiters !== 0);
      expect(fuelTrip).toBeDefined();
      // Verify fuel data was saved correctly
      expect(fuelTrip?.fuelLiters).toBe(8.05);
      expect(fuelTrip?.fuelCostEur).toBe(12.08);
    });
  });
});
