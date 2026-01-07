/**
 * Scenario factory functions for integration tests
 *
 * Provides complete test scenarios combining vehicles, trips, and settings
 * for common test cases like consumption limits, year transitions, etc.
 */

import type { Vehicle, Trip, Settings, Receipt } from './types';
import {
  createSkodaOctavia,
  createSkodaEnyaq,
  createSkodaOctaviaPhev,
  createTestIceVehicle,
  createTestBevVehicle,
} from './vehicles';
import {
  createTrip,
  createTripWithFuel,
  createBevTripWithCharge,
  createMonthlyTrips,
  createTripsWithPeriodicRefueling,
  TripPurposes,
  SlovakCities,
} from './trips';
import { createReceipt, createReceiptsMatchingTrips } from './receipts';

// =============================================================================
// Test Company Settings
// =============================================================================

/**
 * Default test company settings (Slovak fake data)
 */
export const testCompanySettings: Settings = {
  companyName: 'Test Company s.r.o.',
  companyIco: '12345678',
  bufferTripPurpose: 'Sluzobna cesta',
};

/**
 * Alternative test company
 */
export const altCompanySettings: Settings = {
  companyName: 'ABC Logistika s.r.o.',
  companyIco: '87654321',
  bufferTripPurpose: 'Dovoz tovaru',
};

// =============================================================================
// Scenario Interface
// =============================================================================

export interface TestScenario {
  name: string;
  description: string;
  vehicle: Vehicle;
  trips: Trip[];
  receipts?: Receipt[];
  settings?: Settings;
  expectedMarginPercent?: number;
  expectedIsOverLimit?: boolean;
}

// =============================================================================
// Under-Limit Scenarios (consumption <= 120% of TP rate)
// =============================================================================

/**
 * Scenario: Vehicle with consumption exactly at TP rate (0% over)
 *
 * - TP rate: 7.0 l/100km
 * - Actual consumption: 7.0 l/100km
 * - Margin: 0%
 */
export function createUnderLimitScenario(year: number = new Date().getFullYear()): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // Drive 100km, refuel 7L = exactly 7.0 l/100km
  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 1,
      origin: SlovakCities.bratislava,
      destination: SlovakCities.trnava,
      distanceKm: 50,
      odometer: 50050,
      purpose: TripPurposes.business,
    }),
    createTripWithFuel(7.0, 10.5, {
      year,
      month: 1,
      day: 5,
      origin: SlovakCities.trnava,
      destination: SlovakCities.bratislava,
      distanceKm: 50,
      odometer: 50100,
      purpose: TripPurposes.business,
    }),
  ];

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Under Limit - Exact TP Rate',
    description: 'Consumption exactly at TP rate (0% over)',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
    expectedMarginPercent: 0,
    expectedIsOverLimit: false,
  };
}

/**
 * Scenario: Vehicle with consumption at 115% of TP rate (safe margin)
 *
 * - TP rate: 7.0 l/100km
 * - Actual consumption: 8.05 l/100km
 * - Margin: 15%
 */
export function createSafeMarginScenario(year: number = new Date().getFullYear()): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // Drive 100km, refuel 8.05L = 8.05 l/100km (15% over)
  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 1,
      distanceKm: 50,
      odometer: 50050,
    }),
    createTripWithFuel(8.05, 12.08, {
      year,
      month: 1,
      day: 5,
      distanceKm: 50,
      odometer: 50100,
    }),
  ];

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Under Limit - Safe Margin',
    description: 'Consumption at 115% of TP rate (safe 15% margin)',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
    expectedMarginPercent: 15,
    expectedIsOverLimit: false,
  };
}

/**
 * Scenario: Vehicle at exactly 120% margin (boundary case)
 *
 * - TP rate: 7.0 l/100km
 * - Actual consumption: 8.4 l/100km
 * - Margin: 20% (exactly at legal limit)
 */
export function createBoundaryScenario(year: number = new Date().getFullYear()): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // Drive 100km, refuel 8.4L = 8.4 l/100km (20% over)
  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 1,
      distanceKm: 50,
      odometer: 50050,
    }),
    createTripWithFuel(8.4, 12.6, {
      year,
      month: 1,
      day: 5,
      distanceKm: 50,
      odometer: 50100,
    }),
  ];

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Boundary - Exactly 20%',
    description: 'Consumption exactly at 120% of TP rate (legal boundary)',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
    expectedMarginPercent: 20,
    expectedIsOverLimit: false, // 20% is still within limit
  };
}

// =============================================================================
// Over-Limit Scenarios (consumption > 120% of TP rate)
// =============================================================================

/**
 * Scenario: Vehicle with consumption at 125% of TP rate (over limit)
 *
 * - TP rate: 7.0 l/100km
 * - Actual consumption: 8.75 l/100km
 * - Margin: 25% (exceeds 20% legal limit)
 */
export function createOverLimitScenario(year: number = new Date().getFullYear()): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // Drive 100km, refuel 8.75L = 8.75 l/100km (25% over)
  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 1,
      distanceKm: 50,
      odometer: 50050,
    }),
    createTripWithFuel(8.75, 13.13, {
      year,
      month: 1,
      day: 5,
      distanceKm: 50,
      odometer: 50100,
    }),
  ];

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Over Limit - 25%',
    description: 'Consumption at 125% of TP rate (5% over legal limit)',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
    expectedMarginPercent: 25,
    expectedIsOverLimit: true,
  };
}

/**
 * Scenario: Vehicle significantly over limit (needs compensation trips)
 *
 * - TP rate: 7.0 l/100km
 * - Actual consumption: 9.1 l/100km
 * - Margin: 30% (significantly over)
 */
export function createSignificantlyOverLimitScenario(
  year: number = new Date().getFullYear()
): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // Drive 100km, refuel 9.1L = 9.1 l/100km (30% over)
  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 1,
      distanceKm: 50,
      odometer: 50050,
    }),
    createTripWithFuel(9.1, 13.65, {
      year,
      month: 1,
      day: 5,
      distanceKm: 50,
      odometer: 50100,
    }),
  ];

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Over Limit - 30%',
    description: 'Consumption at 130% of TP rate (needs compensation trips)',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
    expectedMarginPercent: 30,
    expectedIsOverLimit: true,
  };
}

// =============================================================================
// Year Transition Scenarios
// =============================================================================

/**
 * Scenario: Year transition with fuel carryover
 *
 * Tests that fuel remaining carries over correctly from one year to the next.
 */
export function createYearTransitionScenario(
  fromYear: number = new Date().getFullYear() - 1
): TestScenario {
  const toYear = fromYear + 1;

  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // End of previous year: fill tank
  const prevYearTrips: Trip[] = [
    createTrip({
      year: fromYear,
      month: 12,
      day: 15,
      distanceKm: 100,
      odometer: 50100,
    }),
    createTripWithFuel(45.0, 67.5, {
      year: fromYear,
      month: 12,
      day: 20,
      distanceKm: 100,
      odometer: 50200,
    }),
    createTrip({
      year: fromYear,
      month: 12,
      day: 28,
      distanceKm: 50,
      odometer: 50250,
    }),
  ];

  // Start of new year: trips without refuel (using carryover)
  const newYearTrips: Trip[] = [
    createTrip({
      year: toYear,
      month: 1,
      day: 3,
      distanceKm: 100,
      odometer: 50350,
    }),
    createTrip({
      year: toYear,
      month: 1,
      day: 8,
      distanceKm: 100,
      odometer: 50450,
    }),
    createTripWithFuel(35.0, 52.5, {
      year: toYear,
      month: 1,
      day: 15,
      distanceKm: 100,
      odometer: 50550,
    }),
  ];

  const trips = [...prevYearTrips, ...newYearTrips];
  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Year Transition',
    description: 'Tests fuel carryover from one year to the next',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
  };
}

/**
 * Scenario: Multi-year data with year filtering
 *
 * Tests that year picker correctly filters trips.
 */
export function createMultiYearScenario(
  startYear: number = new Date().getFullYear() - 2
): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 30000,
  });

  const trips: Trip[] = [];
  let odometer = 30000;

  // Generate trips for 3 years
  for (let yearOffset = 0; yearOffset < 3; yearOffset++) {
    const year = startYear + yearOffset;
    const yearTrips = createTripsWithPeriodicRefueling(year, 12, 4, 35, odometer);
    trips.push(...yearTrips);
    odometer += yearTrips.reduce((sum, t) => sum + t.distanceKm, 0);
  }

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Multi-Year Data',
    description: 'Tests year picker filtering across multiple years',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
  };
}

// =============================================================================
// BEV Scenarios
// =============================================================================

/**
 * Scenario: BEV with normal energy consumption
 */
export function createBevUnderLimitScenario(
  year: number = new Date().getFullYear()
): TestScenario {
  const vehicle = createSkodaEnyaq({
    batteryCapacityKwh: 77,
    baselineConsumptionKwh: 17,
    initialBatteryPercent: 100,
    initialOdometer: 10000,
  });

  const trips: Trip[] = [
    createBevTripWithCharge(15.0, 4.5, {
      year,
      month: 1,
      day: 5,
      distanceKm: 100,
      odometer: 10100,
    }),
    createBevTripWithCharge(17.0, 5.1, {
      year,
      month: 1,
      day: 10,
      distanceKm: 100,
      odometer: 10200,
    }),
  ];

  return {
    name: 'BEV Under Limit',
    description: 'BEV with energy consumption below baseline',
    vehicle,
    trips,
    settings: testCompanySettings,
  };
}

// =============================================================================
// Empty/Edge Case Scenarios
// =============================================================================

/**
 * Scenario: Empty vehicle (no trips)
 */
export function createEmptyVehicleScenario(): TestScenario {
  const vehicle = createTestIceVehicle();

  return {
    name: 'Empty Vehicle',
    description: 'Vehicle with no trips (fresh setup)',
    vehicle,
    trips: [],
    settings: testCompanySettings,
  };
}

/**
 * Scenario: Single trip without refuel
 */
export function createSingleTripScenario(year: number = new Date().getFullYear()): TestScenario {
  const vehicle = createTestIceVehicle({ initialOdometer: 10000 });

  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 15,
      distanceKm: 100,
      odometer: 10100,
    }),
  ];

  return {
    name: 'Single Trip',
    description: 'Vehicle with single trip, no refuel (uses estimated rate)',
    vehicle,
    trips,
    settings: testCompanySettings,
  };
}

/**
 * Scenario: Multiple trips without any refuel
 */
export function createNoRefuelScenario(year: number = new Date().getFullYear()): TestScenario {
  const vehicle = createTestIceVehicle({ initialOdometer: 10000, tankSizeLiters: 60 });

  const trips = createMonthlyTrips(year, 1, 5, 10000, {
    purpose: TripPurposes.business,
  });

  return {
    name: 'No Refuel',
    description: 'Multiple trips without refueling (all estimated rates)',
    vehicle,
    trips,
    settings: testCompanySettings,
  };
}

// =============================================================================
// Compensation Trip Scenario
// =============================================================================

/**
 * Scenario: Over limit, needs compensation trips to bring margin down
 *
 * This scenario can be used to test the compensation trip suggestion feature.
 */
export function createNeedsCompensationScenario(
  year: number = new Date().getFullYear()
): TestScenario {
  const vehicle = createSkodaOctavia({
    tpConsumption: 7.0,
    tankSizeLiters: 50,
    initialOdometer: 50000,
  });

  // Create a situation where margin is ~28% over (needs ~200km of compensation)
  // Drive 100km, refuel 8.96L = 8.96 l/100km (28% over)
  const trips: Trip[] = [
    createTrip({
      year,
      month: 1,
      day: 1,
      distanceKm: 50,
      odometer: 50050,
    }),
    createTripWithFuel(8.96, 13.44, {
      year,
      month: 1,
      day: 5,
      distanceKm: 50,
      odometer: 50100,
    }),
  ];

  const receipts = createReceiptsMatchingTrips(trips);

  return {
    name: 'Needs Compensation',
    description: 'Over limit (28%), needs compensation trips to bring margin down to 16-19%',
    vehicle,
    trips,
    receipts,
    settings: testCompanySettings,
    expectedMarginPercent: 28,
    expectedIsOverLimit: true,
  };
}

// =============================================================================
// Re-export types and factories for convenience
// =============================================================================

export type { TestScenario };
export {
  createSkodaOctavia,
  createSkodaEnyaq,
  createSkodaOctaviaPhev,
  createTestIceVehicle,
  createTestBevVehicle,
} from './vehicles';
export { createTrip, createTripWithFuel, createBevTripWithCharge } from './trips';
export { createReceipt } from './receipts';
