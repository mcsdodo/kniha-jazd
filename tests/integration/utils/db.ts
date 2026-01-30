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
  TripGridData,
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
 * Tauri v2 global interface (requires withGlobalTauri: true in tauri.conf.json)
 */
interface TauriGlobal {
  core: {
    invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  };
}

declare global {
  interface Window {
    __TAURI__?: TauriGlobal;
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
    // Verify Tauri v2 core.invoke is available
    const tauriAvailable = await browser.execute(() => {
      return typeof window.__TAURI__ !== 'undefined' &&
             typeof window.__TAURI__.core !== 'undefined' &&
             typeof window.__TAURI__.core.invoke === 'function';
    });
    return tauriAvailable;
  } catch {
    return false;
  }
}

/**
 * Execute a Tauri command via browser context (Tauri v2 API)
 */
async function invokeTauri<T>(
  cmd: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const result = await browser.execute(
    async (command: string, commandArgs: Record<string, unknown>) => {
      if (!window.__TAURI__ || !window.__TAURI__.core) {
        throw new Error('Tauri not available in browser context');
      }
      try {
        return await window.__TAURI__.core.invoke(command, commandArgs);
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
 * Seed a vehicle via Tauri IPC
 * @returns The created vehicle object with ID
 */
export async function seedVehicle(data: SeedVehicleData): Promise<Vehicle> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for seeding');
  }

  // Use camelCase - Tauri v2 invoke automatically converts to snake_case for Rust
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

  // Default endDatetime to startDatetime if not provided
  const endDatetime = data.endDatetime ?? data.startDatetime;

  // Use camelCase - Tauri v2 invoke automatically converts to snake_case for Rust
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
    insertAtPosition: data.insertAtPosition,
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

  // Use camelCase - Tauri v2 invoke automatically converts to snake_case for Rust
  const args = {
    companyName: data.companyName,
    companyIco: data.companyIco,
    bufferTripPurpose: data.bufferTripPurpose || 'Sluzobna cesta',
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
 * Set a vehicle as the active vehicle
 * @param vehicleId The ID of the vehicle to set as active
 */
export async function setActiveVehicle(vehicleId: string): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  await invokeTauri<void>('set_active_vehicle', { id: vehicleId });
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
  return invokeTauri<TripGridData>('get_trip_grid_data', {
    vehicleId,
    year,
  });
}

// =============================================================================
// Receipt Seeding and Processing
// =============================================================================

/**
 * Scan receipts folder for new files (creates Pending receipts in database)
 *
 * Note: Receipts can only be created via folder scanning - there's no direct
 * create_receipt command. Place files in the receipts folder and call this.
 */
export async function triggerReceiptScan(): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for receipt scan');
  }
  await invokeTauri<void>('scan_receipts');
}

/**
 * Process all pending receipts with Gemini (or mock in test mode)
 *
 * When KNIHA_JAZD_MOCK_GEMINI_DIR is set, this loads mock JSON files
 * instead of calling the real Gemini API.
 */
export async function syncReceipts(): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for sync');
  }
  await invokeTauri<void>('sync_receipts');
}

/**
 * Reprocess a single receipt by ID
 *
 * @param receiptId The receipt ID to reprocess
 */
export async function reprocessReceipt(receiptId: string): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready for reprocess');
  }
  await invokeTauri<void>('reprocess_receipt', { id: receiptId });
}

/**
 * Get all receipts for a given year
 */
export async function getReceipts(year: number): Promise<Receipt[]> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return invokeTauri<Receipt[]>('get_receipts', { year });
}

/**
 * Get receipts for a specific vehicle
 */
export async function getReceiptsForVehicle(
  vehicleId: string,
  year: number
): Promise<Receipt[]> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return invokeTauri<Receipt[]>('get_receipts_for_vehicle', {
    vehicleId,
    year,
  });
}

/**
 * Result from get_trips_for_receipt_assignment
 */
export interface TripForAssignment {
  trip: Trip;
  canAttach: boolean;
  attachmentStatus: string; // "matches" | "differs" | "empty"
  mismatchReason: string | null; // "date" | "liters" | "price" | "date_and_*" | "all"
}

/**
 * Get trips available for receipt assignment with compatibility info
 *
 * This is the key function for testing mismatch detection. It returns
 * each trip with information about whether the receipt data matches.
 */
export async function getTripsForReceiptAssignment(
  receiptId: string,
  vehicleId: string,
  year: number
): Promise<TripForAssignment[]> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  return invokeTauri<TripForAssignment[]>('get_trips_for_receipt_assignment', {
    receiptId,
    vehicleId,
    year,
  });
}

/**
 * Set the receipts folder path via settings
 *
 * Note: KNIHA_JAZD_RECEIPTS_FOLDER env var is NOT implemented in Rust.
 * Must set the folder via this settings command.
 */
export async function setReceiptsFolderPath(folderPath: string): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  await invokeTauri<void>('set_receipts_folder_path', { path: folderPath });
}

/**
 * Update a receipt (for editing currency conversions, etc.)
 */
export async function updateReceipt(receipt: Receipt): Promise<void> {
  const ready = await ensureAppReady();
  if (!ready) {
    throw new Error('App not ready');
  }
  await invokeTauri<void>('update_receipt', { receipt });
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
