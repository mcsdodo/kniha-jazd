/**
 * Database seeding utilities for integration tests
 *
 * These utilities seed test data via Tauri IPC commands, which is the same
 * interface the frontend uses. This ensures data is properly validated and
 * stored exactly as the production app would handle it.
 *
 * Note: Direct SQLite access is avoided because:
 * 1. Tauri IPC validates data the same way production does
 * 2. Database schema migrations are handled by Tauri
 * 3. Tests verify the real data flow path
 */

import type {
  Vehicle,
  VehicleType,
  Trip,
  Receipt,
  Settings,
} from '../fixtures/types';
import type { TestScenario } from '../fixtures/scenarios';
import { waitForAppReady } from './app';

/**
 * Get test data directory from environment
 */
export function getTestDataDir(): string {
  return process.env.KNIHA_JAZD_DATA_DIR || '';
}

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
// Type Definitions for Tauri IPC
// =============================================================================

/**
 * Tauri invoke interface (available in browser context)
 */
interface TauriInvoke {
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

declare global {
  interface Window {
    __TAURI__?: TauriInvoke;
  }
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Wait for app to be ready and ensure Tauri IPC is available
 * Returns true if ready, false otherwise
 */
async function ensureAppReady(): Promise<boolean> {
  try {
    await waitForAppReady();
    // Verify Tauri is available
    const tauriAvailable = await browser.execute(() => {
      return typeof window.__TAURI__ !== 'undefined';
    });
    return tauriAvailable;
  } catch {
    return false;
  }
}

/**
 * Execute a Tauri command via browser context
 */
async function invokeTauri<T>(
  cmd: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const result = await browser.execute(
    async (command: string, commandArgs: Record<string, unknown>) => {
      if (!window.__TAURI__) {
        throw new Error('Tauri not available in browser context');
      }
      try {
        return await window.__TAURI__.invoke(command, commandArgs);
      } catch (e) {
        // Return error as string so it can be caught on the Node side
        throw new Error(`Tauri command '${command}' failed: ${e}`);
      }
    },
    cmd,
    args
  );
  return result as T;
}

/**
 * Convert camelCase to snake_case for Tauri command parameters
 */
function toSnakeCase(str: string): string {
  return str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

/**
 * Convert an object's keys from camelCase to snake_case
 */
function keysToSnakeCase(obj: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj)) {
    result[toSnakeCase(key)] = value;
  }
  return result;
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
}

/**
 * Seed a vehicle via Tauri IPC
 * @returns The created vehicle object with ID
 */
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  // Convert to snake_case for Rust command
  const args = {
    name: data.name,
    license_plate: data.licensePlate,
    initial_odometer: data.initialOdometer,
    vehicle_type: data.vehicleType || 'Ice',
    tank_size_liters: data.tankSizeLiters,
    tp_consumption: data.tpConsumption,
    battery_capacity_kwh: data.batteryCapacityKwh,
    baseline_consumption_kwh: data.baselineConsumptionKwh,
    initial_battery_percent: data.initialBatteryPercent,
  };

  const vehicle = await invokeTauri<Vehicle>('create_vehicle', args);

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
  date: string;
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
  // Position control
  insertAtPosition?: number;
}

/**
 * Seed a trip via Tauri IPC
 * @returns The created trip object with ID
 */
export async function seedTrip(data: SeedTripData): Promise<Trip> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  // Convert to snake_case for Rust command
  const args = {
    vehicle_id: data.vehicleId,
    date: data.date,
    origin: data.origin,
    destination: data.destination,
    distance_km: data.distanceKm,
    odometer: data.odometer,
    purpose: data.purpose,
    fuel_liters: data.fuelLiters,
    fuel_cost: data.fuelCostEur,
    full_tank: data.fullTank,
    energy_kwh: data.energyKwh,
    energy_cost_eur: data.energyCostEur,
    full_charge: data.fullCharge,
    soc_override_percent: data.socOverridePercent,
    other_costs: data.otherCostsEur,
    other_costs_note: data.otherCostsNote,
    insert_at_position: data.insertAtPosition,
  };

  const trip = await invokeTauri<Trip>('create_trip', args);
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
    date: trip.date,
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
    insertAtPosition: trip.sortOrder,
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
 * Seed settings via Tauri IPC
 */
export async function seedSettings(data: SeedSettingsData): Promise<Settings> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  const args = {
    company_name: data.companyName,
    company_ico: data.companyIco,
    buffer_trip_purpose: data.bufferTripPurpose || 'Sluzobna cesta',
  };

  const settings = await invokeTauri<Settings>('save_settings', args);
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
  return invokeTauri<Vehicle[]>('get_vehicles');
}

/**
 * Get the active vehicle
 */
export async function getActiveVehicle(): Promise<Vehicle | null> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return invokeTauri<Vehicle | null>('get_active_vehicle');
}

/**
 * Get trip grid data for a vehicle and year
 */
export async function getTripGridData(
  vehicleId: string,
  year: number
): Promise<{ trips: Trip[] }> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return invokeTauri<{ trips: Trip[] }>('get_trip_grid_data', {
    vehicle_id: vehicleId,
    year,
  });
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
  date?: string;
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
      date: options.date || '2024-01-15',
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
