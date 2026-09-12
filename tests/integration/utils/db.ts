/**
 * Database seeding utilities for integration tests
 *
 * These utilities seed test data via the backend's HTTP RPC endpoint, which is
 * the same interface the frontend uses. This ensures data is properly validated
 * and stored exactly as the production app would handle it.
 *
 * Note: Direct SQLite access is avoided because:
 * 1. The backend validates data the same way production does
 * 2. Database schema migrations are handled by the backend
 * 3. Tests verify the real data flow path
 */

import type {
  Vehicle,
  VehicleType,
  Trip,
  Settings,
  TripGridData,
} from '../fixtures/types';
import type { TestScenario } from '../fixtures/scenarios';
import { waitForAppReady } from './app';

/**
 * Generate a UUID (simple implementation for testing)
 */
function generateUuid(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

// =============================================================================
// Backend Configuration
// =============================================================================

const SERVER_URL = process.env.WDIO_SERVER_URL || 'http://localhost:3457';

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Wait for the app to be ready. DOM ready means the app loaded from the
 * server -- the backend is inherently available via HTTP RPC.
 */
async function ensureAppReady(): Promise<boolean> {
  try {
    await waitForAppReady();
    return true;
  } catch {
    return false;
  }
}

/**
 * Execute a backend command over the HTTP RPC endpoint.
 *
 * This is the single point of backend communication for all test utilities.
 * All seed/query functions go through this helper, and spec files should use it
 * too rather than reaching for the backend any other way.
 */
export async function rpc<T>(
  cmd: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const resp = await fetch(`${SERVER_URL}/api/rpc`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-KJ-Client': '1',
    },
    body: JSON.stringify({ command: cmd, args }),
  });
  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`RPC '${cmd}' failed (${resp.status}): ${text}`);
  }
  return await resp.json() as T;
}

// =============================================================================
// Vehicle Seeding
// =============================================================================

/**
 * Vehicle data structure for seeding (matches create_vehicle command args)
 */
export interface SeedVehicleData {
  name: string;
  licensePlate: string;
  initialOdometer: number;
  vehicleType?: VehicleType;
  // Fuel fields (ICE + PHEV)
  tankSizeLiters?: number;
  tpConsumption?: number;
  // Battery fields (BEV + PHEV)
  batteryCapacityKwh?: number;
  baselineConsumptionKwh?: number;
  initialBatteryPercent?: number;
  // Legal compliance (2026)
  driverName?: string;
}

/**
 * Seed a vehicle via the backend RPC endpoint
 * @returns The created vehicle object with ID
 */
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  // Use camelCase - the RPC layer converts to snake_case for Rust
  const args = {
    name: data.name,
    licensePlate: data.licensePlate,
    initialOdometer: data.initialOdometer,
    vehicleType: data.vehicleType || 'Ice',
    tankSizeLiters: data.tankSizeLiters,
    tpConsumption: data.tpConsumption,
    batteryCapacityKwh: data.batteryCapacityKwh,
    baselineConsumptionKwh: data.baselineConsumptionKwh,
    initialBatteryPercent: data.initialBatteryPercent,
    driverName: data.driverName,
  };

  const vehicle = await rpc<Vehicle>('create_vehicle', args);

  // Refresh the page to ensure UI reflects the new data
  await browser.refresh();
  await waitForAppReady();

  return vehicle;
}

/**
 * Seed a vehicle from a Vehicle fixture object
 */
export async function seedVehicleFromFixture(vehicle: Vehicle): Promise<Vehicle> {
  return seedVehicle({
    name: vehicle.name,
    licensePlate: vehicle.licensePlate,
    initialOdometer: vehicle.initialOdometer,
    vehicleType: vehicle.vehicleType,
    tankSizeLiters: vehicle.tankSizeLiters,
    tpConsumption: vehicle.tpConsumption,
    batteryCapacityKwh: vehicle.batteryCapacityKwh,
    baselineConsumptionKwh: vehicle.baselineConsumptionKwh,
    initialBatteryPercent: vehicle.initialBatteryPercent,
    driverName: vehicle.driverName,
  });
}

// =============================================================================
// Trip Seeding
// =============================================================================

/**
 * Trip data structure for seeding (matches create_trip command args)
 */
export interface SeedTripData {
  vehicleId: string;
  startDatetime: string; // Full ISO datetime "YYYY-MM-DDTHH:MM" or "YYYY-MM-DDTHH:MM:SS"
  endDatetime?: string; // Full ISO datetime (optional, defaults to startDatetime + 1 hour)
  origin: string;
  destination: string;
  distanceKm: number;
  odometer: number;
  purpose: string;
  // Fuel fields (ICE + PHEV)
  fuelLiters?: number;
  fuelCostEur?: number;
  fullTank?: boolean;
  // Energy fields (BEV + PHEV)
  energyKwh?: number;
  energyCostEur?: number;
  fullCharge?: boolean;
  socOverridePercent?: number;
  // Other costs
  otherCostsEur?: number;
  otherCostsNote?: string;
}

/**
 * Seed a trip via the backend RPC endpoint
 * @returns The created trip object with ID
 */
export async function seedTrip(data: SeedTripData): Promise<Trip> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  // Default endDatetime to startDatetime if not provided
  const endDatetime = data.endDatetime ?? data.startDatetime;

  // Use camelCase - the RPC layer converts to snake_case for Rust
  const args = {
    vehicleId: data.vehicleId,
    startDatetime: data.startDatetime,
    endDatetime: endDatetime,
    origin: data.origin,
    destination: data.destination,
    distanceKm: data.distanceKm,
    odometer: data.odometer,
    purpose: data.purpose,
    fuelLiters: data.fuelLiters,
    fuelCost: data.fuelCostEur,
    fullTank: data.fullTank,
    energyKwh: data.energyKwh,
    energyCostEur: data.energyCostEur,
    fullCharge: data.fullCharge,
    socOverridePercent: data.socOverridePercent,
    otherCosts: data.otherCostsEur,
    otherCostsNote: data.otherCostsNote,
  };

  const trip = await rpc<Trip>('create_trip', args);
  return trip;
}

/**
 * Seed a trip from a Trip fixture object
 * @param trip The trip fixture
 * @param vehicleId The vehicle ID to associate the trip with
 */
export async function seedTripFromFixture(
  trip: Trip,
  vehicleId: string
): Promise<Trip> {
  return seedTrip({
    vehicleId,
    startDatetime: trip.startDatetime,
    endDatetime: trip.endDatetime,
    origin: trip.origin,
    destination: trip.destination,
    distanceKm: trip.distanceKm,
    odometer: trip.odometer,
    purpose: trip.purpose,
    fuelLiters: trip.fuelLiters,
    fuelCostEur: trip.fuelCostEur,
    fullTank: trip.fullTank,
    energyKwh: trip.energyKwh,
    energyCostEur: trip.energyCostEur,
    fullCharge: trip.fullCharge,
    socOverridePercent: trip.socOverridePercent,
    otherCostsEur: trip.otherCostsEur,
    otherCostsNote: trip.otherCostsNote,
  });
}

/**
 * Seed multiple trips for a vehicle
 * @param trips Array of trip fixtures
 * @param vehicleId The vehicle ID to associate trips with
 * @returns Array of created trips
 */
export async function seedTrips(
  trips: Trip[],
  vehicleId: string
): Promise<Trip[]> {
  const createdTrips: Trip[] = [];
  for (const trip of trips) {
    const created = await seedTripFromFixture(trip, vehicleId);
    createdTrips.push(created);
  }
  return createdTrips;
}

// =============================================================================
// Settings Seeding
// =============================================================================

/**
 * Settings data structure for seeding
 */
export interface SeedSettingsData {
  companyName: string;
  companyIco: string;
  bufferTripPurpose?: string;
}

/**
 * Seed settings via the backend RPC endpoint
 */
export async function seedSettings(data: SeedSettingsData): Promise<Settings> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  // Use camelCase - the RPC layer converts to snake_case for Rust
  const args = {
    companyName: data.companyName,
    companyIco: data.companyIco,
    bufferTripPurpose: data.bufferTripPurpose || 'Sluzobna cesta',
  };

  const settings = await rpc<Settings>('save_settings', args);
  return settings;
}

/**
 * Seed settings from a Settings fixture object
 */
export async function seedSettingsFromFixture(
  settings: Settings
): Promise<Settings> {
  return seedSettings({
    companyName: settings.companyName,
    companyIco: settings.companyIco,
    bufferTripPurpose: settings.bufferTripPurpose,
  });
}

// =============================================================================
// Scenario Seeding
// =============================================================================

/**
 * Result of seeding a complete scenario
 */
export interface SeededScenario {
  vehicle: Vehicle;
  trips: Trip[];
  settings?: Settings;
}

/**
 * Seed a complete test scenario (vehicle + trips + settings)
 * This is the primary method for setting up complex test data quickly.
 *
 * @param scenario The test scenario to seed
 * @returns The seeded data with actual IDs
 */
export async function seedScenario(scenario: TestScenario): Promise<SeededScenario> {
  // 1. Seed vehicle first
  const vehicle = await seedVehicleFromFixture(scenario.vehicle);

  // 2. Seed settings if provided
  let settings: Settings | undefined;
  if (scenario.settings) {
    settings = await seedSettingsFromFixture(scenario.settings);
  }

  // 3. Seed all trips for this vehicle
  const trips = await seedTrips(scenario.trips, vehicle.id as string);

  // 4. Refresh the page to show all data
  await browser.refresh();
  await waitForAppReady();

  return {
    vehicle,
    trips,
    settings,
  };
}

// =============================================================================
// Data Retrieval (for verification)
// =============================================================================

/**
 * Get all vehicles from the database
 */
export async function getVehicles(): Promise<Vehicle[]> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return rpc<Vehicle[]>('get_vehicles');
}

/**
 * Get the active vehicle
 */
export async function getActiveVehicle(): Promise<Vehicle | null> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return rpc<Vehicle | null>('get_active_vehicle');
}

/**
 * Set a vehicle as the active vehicle
 * @param vehicleId The ID of the vehicle to set as active
 */
export async function setActiveVehicle(vehicleId: string): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  await rpc<void>('set_active_vehicle', { id: vehicleId });
  await browser.refresh();
  await waitForAppReady();
}

/**
 * Get trip grid data for a vehicle and year
 */
export async function getTripGridData(
  vehicleId: string,
  year: number
): Promise<TripGridData> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return rpc<TripGridData>('get_trip_grid_data', {
    vehicleId,
    year,
  });
}

/**
 * Update a trip via backend command.
 */
export async function updateTrip(args: Record<string, unknown>): Promise<Trip> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return rpc<Trip>('update_trip', args);
}

/**
 * Delete a trip by ID via backend command.
 */
export async function deleteTrip(tripId: string): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  await rpc<void>('delete_trip', { id: tripId });
}

// =============================================================================
// Legacy Compatibility (placeholder interfaces for existing code)
// =============================================================================

/**
 * @deprecated Use SeedVehicleData instead
 */
export interface SeedVehicleOptions {
  name?: string;
  licensePlate?: string;
  tankSize?: number;
  tpConsumption?: number;
  initialOdometer?: number;
}

/**
 * @deprecated Use SeedTripData instead
 */
export interface SeedTripOptions {
  vehicleId: string;
  startDatetime?: string;
  origin?: string;
  destination?: string;
  distanceKm?: number;
  odometer?: number;
  purpose?: string;
  fuelLiters?: number | null;
  fullTank?: boolean;
}

/**
 * @deprecated Use the individual seed functions instead
 */
export const seedUtils = {
  createVehicle: async (options: SeedVehicleOptions): Promise<string> => {
    const vehicle = await seedVehicle({
      name: options.name || 'Test Vehicle',
      licensePlate: options.licensePlate || 'TEST-001',
      initialOdometer: options.initialOdometer || 10000,
      vehicleType: 'Ice',
      tankSizeLiters: options.tankSize || 50,
      tpConsumption: options.tpConsumption || 7.0,
    });
    return vehicle.id as string;
  },

  createTrip: async (options: SeedTripOptions): Promise<string> => {
    const trip = await seedTrip({
      vehicleId: options.vehicleId,
      startDatetime: options.startDatetime || '2024-01-15T08:00',
      origin: options.origin || 'Bratislava',
      destination: options.destination || 'Kosice',
      distanceKm: options.distanceKm || 400,
      odometer: options.odometer || 10400,
      purpose: options.purpose || 'Sluzobna cesta',
      fuelLiters: options.fuelLiters ?? undefined,
      fullTank: options.fullTank,
    });
    return trip.id as string;
  },
};
