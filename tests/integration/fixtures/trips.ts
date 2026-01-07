/**
 * Trip factory functions for integration tests
 *
 * Provides trip presets with Slovak diacritics and realistic data.
 * All factories support a year parameter to avoid hardcoded years.
 */

import type { Trip } from './types';

/**
 * Format a date as YYYY-MM-DD string
 */
function formatDate(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

/**
 * Common Slovak cities with proper diacritics
 */
export const SlovakCities = {
  bratislava: 'Bratislava',
  kosice: 'Kosice',
  presov: 'Presov',
  zilina: 'Zilina',
  banskaBystrcia: 'Banska Bystrica',
  nitra: 'Nitra',
  trnava: 'Trnava',
  trencin: 'Trencin',
  poprad: 'Poprad',
  martin: 'Martin',
  zvolen: 'Zvolen',
  michalovce: 'Michalovce',
  lucenec: 'Lucenec',
  ruzomberok: 'Ruzomberok',
  prievidza: 'Prievidza',
} as const;

/**
 * Common trip purposes in Slovak
 */
export const TripPurposes = {
  business: 'Sluzobna cesta',
  clientMeeting: 'Stretnutie s klientom',
  delivery: 'Dovoz tovaru',
  training: 'Skolenie',
  conference: 'Konferencia',
  inspection: 'Kontrola',
  repair: 'Oprava vozidla',
  personal: 'Sukromna jazda',
  commute: 'Dochadza do prace',
} as const;

/**
 * Common Slovak routes with distances (approximate)
 */
export const CommonRoutes = {
  bratislavaKosice: { origin: 'Bratislava', destination: 'Kosice', distance: 400 },
  bratislavaZilina: { origin: 'Bratislava', destination: 'Zilina', distance: 200 },
  bratislavaBanskaBystrcia: { origin: 'Bratislava', destination: 'Banska Bystrica', distance: 215 },
  kosicePresov: { origin: 'Kosice', destination: 'Presov', distance: 36 },
  zilinaMartin: { origin: 'Zilina', destination: 'Martin', distance: 30 },
  nitraTrnava: { origin: 'Nitra', destination: 'Trnava', distance: 65 },
  bratislavaTrencin: { origin: 'Bratislava', destination: 'Trencin', distance: 130 },
  popradZvolen: { origin: 'Poprad', destination: 'Zvolen', distance: 120 },
} as const;

// =============================================================================
// Base Trip Factory
// =============================================================================

export interface TripFactoryOptions extends Partial<Trip> {
  year?: number;
  month?: number;
  day?: number;
}

/**
 * Create a basic trip with required fields
 * Year defaults to current year if not specified
 */
export function createTrip(options: TripFactoryOptions = {}): Trip {
  const {
    year = new Date().getFullYear(),
    month = 1,
    day = 15,
    ...rest
  } = options;

  return {
    date: formatDate(year, month, day),
    origin: SlovakCities.bratislava,
    destination: SlovakCities.kosice,
    distanceKm: 400,
    odometer: 10400,
    purpose: TripPurposes.business,
    fullTank: false,
    fullCharge: false,
    ...rest,
  };
}

// =============================================================================
// ICE Trip Presets
// =============================================================================

/**
 * Create a trip without refueling (regular business trip)
 */
export function createBusinessTrip(options: TripFactoryOptions = {}): Trip {
  return createTrip({
    purpose: TripPurposes.business,
    ...options,
  });
}

/**
 * Create a trip with fuel refill
 */
export function createTripWithFuel(
  fuelLiters: number,
  fuelCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    fuelLiters,
    fuelCostEur,
    fullTank: true,
    ...options,
  });
}

/**
 * Create a trip with partial fuel refill (not full tank)
 */
export function createTripWithPartialFuel(
  fuelLiters: number,
  fuelCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    fuelLiters,
    fuelCostEur,
    fullTank: false,
    ...options,
  });
}

// =============================================================================
// BEV Trip Presets
// =============================================================================

/**
 * Create a BEV trip without charging
 */
export function createBevTrip(options: TripFactoryOptions = {}): Trip {
  return createTrip({
    fuelLiters: undefined,
    fuelCostEur: undefined,
    fullTank: false,
    ...options,
  });
}

/**
 * Create a BEV trip with full charge
 */
export function createBevTripWithCharge(
  energyKwh: number,
  energyCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    fuelLiters: undefined,
    fuelCostEur: undefined,
    fullTank: false,
    energyKwh,
    energyCostEur,
    fullCharge: true,
    ...options,
  });
}

/**
 * Create a BEV trip with partial charge
 */
export function createBevTripWithPartialCharge(
  energyKwh: number,
  energyCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    fuelLiters: undefined,
    fuelCostEur: undefined,
    fullTank: false,
    energyKwh,
    energyCostEur,
    fullCharge: false,
    ...options,
  });
}

/**
 * Create a BEV trip with manual SoC override (for battery degradation)
 */
export function createBevTripWithSocOverride(
  socPercent: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    fuelLiters: undefined,
    fuelCostEur: undefined,
    fullTank: false,
    socOverridePercent: socPercent,
    ...options,
  });
}

// =============================================================================
// PHEV Trip Presets
// =============================================================================

/**
 * Create a PHEV trip with both fuel and energy
 */
export function createPhevTrip(
  fuelLiters: number,
  fuelCostEur: number,
  energyKwh: number,
  energyCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    fuelLiters,
    fuelCostEur,
    fullTank: true,
    energyKwh,
    energyCostEur,
    fullCharge: true,
    ...options,
  });
}

/**
 * Create a PHEV trip with fuel only (ICE mode)
 */
export function createPhevFuelOnlyTrip(
  fuelLiters: number,
  fuelCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createTripWithFuel(fuelLiters, fuelCostEur, options);
}

/**
 * Create a PHEV trip with energy only (EV mode)
 */
export function createPhevEvOnlyTrip(
  energyKwh: number,
  energyCostEur: number,
  options: TripFactoryOptions = {}
): Trip {
  return createBevTripWithCharge(energyKwh, energyCostEur, options);
}

// =============================================================================
// Special Trip Types
// =============================================================================

/**
 * Create a trip with other costs (parking, tolls, etc.)
 */
export function createTripWithOtherCosts(
  otherCostsEur: number,
  otherCostsNote: string,
  options: TripFactoryOptions = {}
): Trip {
  return createTrip({
    otherCostsEur,
    otherCostsNote,
    ...options,
  });
}

/**
 * Create a short city trip
 */
export function createCityTrip(options: TripFactoryOptions = {}): Trip {
  return createTrip({
    origin: SlovakCities.bratislava,
    destination: 'Bratislava - Petrzalka',
    distanceKm: 15,
    purpose: TripPurposes.clientMeeting,
    ...options,
  });
}

/**
 * Create a long-distance trip (Bratislava to Kosice round trip)
 */
export function createLongDistanceTrip(options: TripFactoryOptions = {}): Trip {
  return createTrip({
    origin: SlovakCities.bratislava,
    destination: SlovakCities.kosice,
    distanceKm: 400,
    purpose: TripPurposes.conference,
    ...options,
  });
}

// =============================================================================
// Trip Sequence Generators
// =============================================================================

/**
 * Generate a sequence of trips for a month
 * Useful for testing consumption calculations over time
 */
export function createMonthlyTrips(
  year: number,
  month: number,
  count: number,
  baseOdometer: number,
  options: Partial<Trip> = {}
): Trip[] {
  const trips: Trip[] = [];
  let currentOdometer = baseOdometer;

  for (let i = 0; i < count; i++) {
    const day = Math.min(1 + i * Math.floor(28 / count), 28);
    const distance = 50 + Math.floor(Math.random() * 150); // 50-200 km

    trips.push(
      createTrip({
        year,
        month,
        day,
        distanceKm: distance,
        odometer: currentOdometer + distance,
        sortOrder: i,
        ...options,
      })
    );

    currentOdometer += distance;
  }

  return trips;
}

/**
 * Generate trips with periodic refueling
 * Creates a realistic pattern of trips with refueling every N trips
 */
export function createTripsWithPeriodicRefueling(
  year: number,
  tripCount: number,
  refuelEveryN: number,
  fuelPerRefill: number,
  baseOdometer: number
): Trip[] {
  const trips: Trip[] = [];
  let currentOdometer = baseOdometer;
  let month = 1;
  let day = 1;

  for (let i = 0; i < tripCount; i++) {
    const distance = 100 + Math.floor(Math.random() * 100); // 100-200 km
    const isRefuelTrip = (i + 1) % refuelEveryN === 0;

    trips.push(
      createTrip({
        year,
        month,
        day,
        distanceKm: distance,
        odometer: currentOdometer + distance,
        sortOrder: i,
        ...(isRefuelTrip
          ? {
              fuelLiters: fuelPerRefill,
              fuelCostEur: fuelPerRefill * 1.5, // ~1.50 EUR/L
              fullTank: true,
            }
          : {}),
      })
    );

    currentOdometer += distance;
    day += 2;
    if (day > 28) {
      day = 1;
      month += 1;
      if (month > 12) {
        month = 1;
      }
    }
  }

  return trips;
}

// =============================================================================
// Re-export types for convenience
// =============================================================================

export type { Trip } from './types';
