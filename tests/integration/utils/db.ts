/**
 * Direct SQLite database utilities for test seeding
 *
 * These utilities allow tests to set up complex scenarios quickly
 * by directly inserting data into the database, bypassing the UI.
 */

// Direct database utilities - placeholder for future optimization
// For now, tests use UI interactions to create data

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
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

// Note: Direct DB seeding requires the app to have created the database first.
// For now, we'll rely on UI-based setup in tests.
// This file is a placeholder for future optimization if needed.

export interface SeedVehicleOptions {
  name?: string;
  licensePlate?: string;
  tankSize?: number;
  tpConsumption?: number;
  initialOdometer?: number;
}

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
 * Note: Direct seeding will be implemented once we verify
 * the basic test flow works through the UI.
 *
 * For now, tests should use UI interactions to create data,
 * which also validates that the UI works correctly.
 */
export const seedUtils = {
  /**
   * Placeholder for direct vehicle seeding
   * TODO: Implement once schema is verified in tests
   */
  createVehicle: async (_options: SeedVehicleOptions): Promise<string> => {
    console.warn('Direct DB seeding not yet implemented - use UI');
    return generateUuid();
  },

  /**
   * Placeholder for direct trip seeding
   * TODO: Implement once schema is verified in tests
   */
  createTrip: async (_options: SeedTripOptions): Promise<string> => {
    console.warn('Direct DB seeding not yet implemented - use UI');
    return generateUuid();
  }
};
