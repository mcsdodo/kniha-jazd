/**
 * Vehicle factory functions for integration tests
 *
 * Provides preset vehicles with Slovak diacritics and realistic data.
 * All factories return partial Vehicle objects that can be overridden.
 */

import type { Vehicle, VehicleType } from './types';

/**
 * Generate a unique test ID to prevent data collisions between test runs
 */
export function uniqueTestId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
}

/**
 * Base vehicle factory - creates a minimal vehicle with required fields
 */
export function createVehicle(
  overrides: Partial<Vehicle> & { name: string; licensePlate: string; initialOdometer: number }
): Vehicle {
  return {
    vehicleType: 'Ice',
    ...overrides,
  };
}

// =============================================================================
// ICE (Internal Combustion Engine) Vehicle Presets
// =============================================================================

/**
 * Skoda Octavia - common Slovak company car
 * TP consumption: 6.5 l/100km
 */
export function createSkodaOctavia(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Skoda Octavia Combi',
    licensePlate: `BA-${testId.substring(0, 5)}AA`,
    vehicleType: 'Ice',
    tankSizeLiters: 50,
    tpConsumption: 6.5,
    initialOdometer: 45000,
    ...overrides,
  };
}

/**
 * VW Passat - larger company car
 * TP consumption: 7.2 l/100km
 */
export function createVwPassat(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Volkswagen Passat',
    licensePlate: `KE-${testId.substring(0, 5)}BB`,
    vehicleType: 'Ice',
    tankSizeLiters: 66,
    tpConsumption: 7.2,
    initialOdometer: 78000,
    ...overrides,
  };
}

/**
 * Skoda Superb - executive car
 * TP consumption: 7.0 l/100km
 */
export function createSkodaSuperb(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Skoda Superb',
    licensePlate: `ZA-${testId.substring(0, 5)}CC`,
    vehicleType: 'Ice',
    tankSizeLiters: 66,
    tpConsumption: 7.0,
    initialOdometer: 32000,
    ...overrides,
  };
}

/**
 * Generic test ICE vehicle - simple preset for basic tests
 */
export function createTestIceVehicle(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: `Test ICE ${testId.substring(0, 5)}`,
    licensePlate: `TEST-${testId.substring(0, 5)}`,
    vehicleType: 'Ice',
    tankSizeLiters: 50,
    tpConsumption: 7.0,
    initialOdometer: 10000,
    ...overrides,
  };
}

// =============================================================================
// BEV (Battery Electric Vehicle) Presets
// =============================================================================

/**
 * Tesla Model 3 - popular EV
 * Battery: 75 kWh, Baseline consumption: 15 kWh/100km
 */
export function createTeslaModel3(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Tesla Model 3',
    licensePlate: `EL-${testId.substring(0, 5)}EV`,
    vehicleType: 'Bev',
    batteryCapacityKwh: 75,
    baselineConsumptionKwh: 15,
    initialBatteryPercent: 90,
    initialOdometer: 15000,
    ...overrides,
  };
}

/**
 * Skoda Enyaq - Slovak-relevant EV
 * Battery: 77 kWh, Baseline consumption: 17 kWh/100km
 */
export function createSkodaEnyaq(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Skoda Enyaq iV',
    licensePlate: `BA-${testId.substring(0, 5)}EQ`,
    vehicleType: 'Bev',
    batteryCapacityKwh: 77,
    baselineConsumptionKwh: 17,
    initialBatteryPercent: 100,
    initialOdometer: 8000,
    ...overrides,
  };
}

/**
 * VW ID.4 - compact electric SUV
 * Battery: 77 kWh, Baseline consumption: 18 kWh/100km
 */
export function createVwId4(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Volkswagen ID.4',
    licensePlate: `KE-${testId.substring(0, 5)}ID`,
    vehicleType: 'Bev',
    batteryCapacityKwh: 77,
    baselineConsumptionKwh: 18,
    initialBatteryPercent: 85,
    initialOdometer: 12000,
    ...overrides,
  };
}

/**
 * Generic test BEV vehicle - simple preset for basic tests
 */
export function createTestBevVehicle(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: `Test BEV ${testId.substring(0, 5)}`,
    licensePlate: `EV-${testId.substring(0, 5)}`,
    vehicleType: 'Bev',
    batteryCapacityKwh: 75,
    baselineConsumptionKwh: 18,
    initialBatteryPercent: 100,
    initialOdometer: 5000,
    ...overrides,
  };
}

// =============================================================================
// PHEV (Plug-in Hybrid Electric Vehicle) Presets
// =============================================================================

/**
 * Skoda Octavia iV - PHEV version
 * Tank: 40L, TP: 1.5 l/100km, Battery: 13 kWh, Baseline: 15 kWh/100km
 */
export function createSkodaOctaviaPhev(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Skoda Octavia iV',
    licensePlate: `BA-${testId.substring(0, 5)}PH`,
    vehicleType: 'Phev',
    tankSizeLiters: 40,
    tpConsumption: 1.5,
    batteryCapacityKwh: 13,
    baselineConsumptionKwh: 15,
    initialBatteryPercent: 100,
    initialOdometer: 22000,
    ...overrides,
  };
}

/**
 * VW Passat GTE - executive PHEV
 * Tank: 50L, TP: 1.6 l/100km, Battery: 14.1 kWh, Baseline: 16 kWh/100km
 */
export function createVwPassatGte(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: 'Volkswagen Passat GTE',
    licensePlate: `ZA-${testId.substring(0, 5)}GT`,
    vehicleType: 'Phev',
    tankSizeLiters: 50,
    tpConsumption: 1.6,
    batteryCapacityKwh: 14.1,
    baselineConsumptionKwh: 16,
    initialBatteryPercent: 80,
    initialOdometer: 35000,
    ...overrides,
  };
}

/**
 * Generic test PHEV vehicle - simple preset for basic tests
 */
export function createTestPhevVehicle(overrides: Partial<Vehicle> = {}): Vehicle {
  const testId = uniqueTestId();
  return {
    name: `Test PHEV ${testId.substring(0, 5)}`,
    licensePlate: `PH-${testId.substring(0, 5)}`,
    vehicleType: 'Phev',
    tankSizeLiters: 45,
    tpConsumption: 1.5,
    batteryCapacityKwh: 13,
    baselineConsumptionKwh: 15,
    initialBatteryPercent: 100,
    initialOdometer: 10000,
    ...overrides,
  };
}

// =============================================================================
// Factory by Vehicle Type
// =============================================================================

/**
 * Create a vehicle by type with default presets
 */
export function createVehicleByType(
  type: VehicleType,
  overrides: Partial<Vehicle> = {}
): Vehicle {
  switch (type) {
    case 'Ice':
      return createSkodaOctavia(overrides);
    case 'Bev':
      return createSkodaEnyaq(overrides);
    case 'Phev':
      return createSkodaOctaviaPhev(overrides);
    default:
      throw new Error(`Unknown vehicle type: ${type}`);
  }
}

// =============================================================================
// Re-export types for convenience
// =============================================================================

export type { Vehicle, VehicleType } from './types';
