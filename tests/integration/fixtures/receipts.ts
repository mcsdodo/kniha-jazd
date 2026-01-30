/**
 * Receipt factory functions for integration tests
 *
 * Provides receipt presets with complete fields for fuel receipts.
 */

import type { Receipt, ReceiptStatus, ConfidenceLevel, FieldConfidence } from './types';

/**
 * Generate a unique receipt filename
 */
function uniqueReceiptFilename(): string {
  const timestamp = Date.now();
  const random = Math.random().toString(36).substring(2, 7);
  return `receipt_${timestamp}_${random}.jpg`;
}

/**
 * Format a date as YYYY-MM-DD string
 */
function formatDate(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

/**
 * Common Slovak fuel stations
 */
export const FuelStations = {
  slovnaftBratislava: {
    name: 'Slovnaft',
    address: 'Pristavna 10, 821 09 Bratislava',
  },
  slovnaftKosice: {
    name: 'Slovnaft',
    address: 'Juzna trieda 76, 040 01 Kosice',
  },
  omvBratislava: {
    name: 'OMV',
    address: 'Bajkalska 25, 821 01 Bratislava',
  },
  omvZilina: {
    name: 'OMV',
    address: 'Vysokoskolakov 52, 010 08 Zilina',
  },
  shellBratislava: {
    name: 'Shell',
    address: 'Einsteinova 25, 851 01 Bratislava',
  },
  shellTrencin: {
    name: 'Shell',
    address: 'Legionarska 2, 911 01 Trencin',
  },
  mollBanskaBystrcia: {
    name: 'MOL',
    address: 'Namestie SNP 12, 974 01 Banska Bystrica',
  },
  mollNitra: {
    name: 'MOL',
    address: 'Stefanikova trieda 31, 949 01 Nitra',
  },
} as const;

// =============================================================================
// Base Receipt Factory
// =============================================================================

export interface ReceiptFactoryOptions extends Partial<Receipt> {
  year?: number;
  month?: number;
  day?: number;
}

/**
 * Create a high-confidence receipt with all fields populated
 */
function createHighConfidence(): FieldConfidence {
  return {
    liters: 'High',
    totalPrice: 'High',
    date: 'High',
  };
}

/**
 * Create a medium-confidence receipt
 */
function createMediumConfidence(): FieldConfidence {
  return {
    liters: 'High',
    totalPrice: 'Medium',
    date: 'High',
  };
}

/**
 * Create a low-confidence receipt
 */
function createLowConfidence(): FieldConfidence {
  return {
    liters: 'Low',
    totalPrice: 'Low',
    date: 'Medium',
  };
}

/**
 * Create a base receipt with all fields
 */
export function createReceipt(options: ReceiptFactoryOptions = {}): Receipt {
  const {
    year = new Date().getFullYear(),
    month = 1,
    day = 15,
    ...rest
  } = options;

  const fileName = uniqueReceiptFilename();

  return {
    filePath: `C:\\Receipts\\${year}\\${fileName}`,
    fileName,
    liters: 45.0,
    totalPriceEur: 67.5, // ~1.50 EUR/L
    receiptDate: formatDate(year, month, day),
    stationName: FuelStations.slovnaftBratislava.name,
    stationAddress: FuelStations.slovnaftBratislava.address,
    sourceYear: year,
    status: 'Parsed',
    confidence: createHighConfidence(),
    ...rest,
  };
}

// =============================================================================
// Receipt Status Presets
// =============================================================================

/**
 * Create a pending receipt (file detected, not yet parsed)
 */
export function createPendingReceipt(options: ReceiptFactoryOptions = {}): Receipt {
  return createReceipt({
    status: 'Pending',
    liters: undefined,
    totalPriceEur: undefined,
    receiptDate: undefined,
    stationName: undefined,
    stationAddress: undefined,
    confidence: {
      liters: 'Unknown',
      totalPrice: 'Unknown',
      date: 'Unknown',
    },
    ...options,
  });
}

/**
 * Create a parsed receipt with high confidence
 */
export function createParsedReceipt(
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    status: 'Parsed',
    liters,
    totalPriceEur,
    confidence: createHighConfidence(),
    ...options,
  });
}

/**
 * Create a receipt that needs review (parsed but uncertain)
 */
export function createNeedsReviewReceipt(
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    status: 'NeedsReview',
    liters,
    totalPriceEur,
    confidence: createLowConfidence(),
    ...options,
  });
}

/**
 * Create an assigned receipt (linked to a trip)
 */
export function createAssignedReceipt(
  tripId: string,
  vehicleId: string,
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    status: 'Assigned',
    tripId,
    vehicleId,
    liters,
    totalPriceEur,
    confidence: createHighConfidence(),
    ...options,
  });
}

// =============================================================================
// Station-Specific Receipts
// =============================================================================

/**
 * Create a Slovnaft receipt
 */
export function createSlovnaftReceipt(
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    liters,
    totalPriceEur,
    stationName: FuelStations.slovnaftBratislava.name,
    stationAddress: FuelStations.slovnaftBratislava.address,
    ...options,
  });
}

/**
 * Create an OMV receipt
 */
export function createOmvReceipt(
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    liters,
    totalPriceEur,
    stationName: FuelStations.omvBratislava.name,
    stationAddress: FuelStations.omvBratislava.address,
    ...options,
  });
}

/**
 * Create a Shell receipt
 */
export function createShellReceipt(
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    liters,
    totalPriceEur,
    stationName: FuelStations.shellBratislava.name,
    stationAddress: FuelStations.shellBratislava.address,
    ...options,
  });
}

/**
 * Create a MOL receipt
 */
export function createMolReceipt(
  liters: number,
  totalPriceEur: number,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    liters,
    totalPriceEur,
    stationName: FuelStations.mollBanskaBystrcia.name,
    stationAddress: FuelStations.mollBanskaBystrcia.address,
    ...options,
  });
}

// =============================================================================
// Error/Edge Case Receipts
// =============================================================================

/**
 * Create a receipt with parsing error
 */
export function createErrorReceipt(
  errorMessage: string,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    status: 'NeedsReview',
    liters: undefined,
    totalPriceEur: undefined,
    receiptDate: undefined,
    stationName: undefined,
    stationAddress: undefined,
    errorMessage,
    confidence: {
      liters: 'Unknown',
      totalPrice: 'Unknown',
      date: 'Unknown',
    },
    ...options,
  });
}

/**
 * Create a receipt with partial data (e.g., blurry image)
 */
export function createPartialReceipt(
  liters: number | undefined,
  totalPriceEur: number | undefined,
  options: ReceiptFactoryOptions = {}
): Receipt {
  return createReceipt({
    status: 'NeedsReview',
    liters,
    totalPriceEur,
    confidence: {
      liters: liters !== undefined ? 'Medium' : 'Unknown',
      totalPrice: totalPriceEur !== undefined ? 'Medium' : 'Unknown',
      date: 'Low',
    },
    ...options,
  });
}

// =============================================================================
// Receipt Sequence Generators
// =============================================================================

/**
 * Generate a sequence of receipts for a month
 */
export function createMonthlyReceipts(
  year: number,
  month: number,
  count: number,
  options: Partial<Receipt> = {}
): Receipt[] {
  const receipts: Receipt[] = [];

  for (let i = 0; i < count; i++) {
    const day = Math.min(1 + i * Math.floor(28 / count), 28);
    const liters = 35 + Math.floor(Math.random() * 20); // 35-55 liters
    const pricePerLiter = 1.45 + Math.random() * 0.15; // 1.45-1.60 EUR/L

    receipts.push(
      createReceipt({
        year,
        month,
        day,
        liters,
        totalPriceEur: Math.round(liters * pricePerLiter * 100) / 100,
        ...options,
      })
    );
  }

  return receipts;
}

/**
 * Generate receipts matching a trip sequence (for verification testing)
 */
export function createReceiptsMatchingTrips(
  trips: Array<{ startDatetime: string; fuelLiters?: number; fuelCostEur?: number }>
): Receipt[] {
  return trips
    .filter((trip) => trip.fuelLiters !== undefined)
    .map((trip) => {
      // Extract date portion from startDatetime (e.g., "2024-01-15T08:00" -> "2024-01-15")
      const dateStr = trip.startDatetime.slice(0, 10);
      const [year, month, day] = dateStr.split('-').map(Number);
      return createReceipt({
        year,
        month,
        day,
        liters: trip.fuelLiters,
        totalPriceEur: trip.fuelCostEur,
      });
    });
}

// =============================================================================
// Re-export types for convenience
// =============================================================================

export type { Receipt, ReceiptStatus, ConfidenceLevel, FieldConfidence } from './types';
