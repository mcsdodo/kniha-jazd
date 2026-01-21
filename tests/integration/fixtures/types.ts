/**
 * TypeScript interfaces matching Rust models in src-tauri/src/models.rs
 *
 * These types are used by test fixtures to ensure type safety
 * and consistency with the backend data structures.
 */

/**
 * Vehicle powertrain type - determines which fields are required/displayed
 */
export type VehicleType = 'Ice' | 'Bev' | 'Phev';

/**
 * Vehicle data structure matching Rust Vehicle struct
 */
export interface Vehicle {
  id?: string; // UUID, generated if not provided
  name: string;
  licensePlate: string;
  vehicleType: VehicleType;
  // Fuel system (ICE + PHEV) - null for BEV
  tankSizeLiters?: number;
  tpConsumption?: number; // l/100km from technical passport
  // Energy system (BEV + PHEV) - null for ICE
  batteryCapacityKwh?: number;
  baselineConsumptionKwh?: number; // kWh/100km, user-defined
  initialBatteryPercent?: number; // Initial SoC % (default: 100%)
  // Common fields
  initialOdometer: number;
  isActive?: boolean;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Trip data structure matching Rust Trip struct
 */
export interface Trip {
  id?: string; // UUID, generated if not provided
  vehicleId?: string;
  date: string; // YYYY-MM-DD format
  origin: string;
  destination: string;
  distanceKm: number;
  odometer: number;
  purpose: string;
  // Fuel system (ICE + PHEV)
  fuelLiters?: number;
  fuelCostEur?: number;
  fullTank?: boolean;
  // Energy system (BEV + PHEV)
  energyKwh?: number;
  energyCostEur?: number;
  fullCharge?: boolean;
  socOverridePercent?: number;
  // Other costs
  otherCostsEur?: number;
  otherCostsNote?: string;
  sortOrder?: number;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Route (learned route for autocomplete) matching Rust Route struct
 */
export interface Route {
  id?: string;
  vehicleId: string;
  origin: string;
  destination: string;
  distanceKm: number;
  usageCount?: number;
  lastUsed?: string;
}

/**
 * App settings matching Rust Settings struct
 */
export interface Settings {
  id?: string;
  companyName: string;
  companyIco: string;
  bufferTripPurpose?: string;
  updatedAt?: string;
}

/**
 * Receipt status enum matching Rust ReceiptStatus
 */
export type ReceiptStatus = 'Pending' | 'Parsed' | 'NeedsReview' | 'Assigned';

/**
 * Confidence level for parsed receipt fields
 */
export type ConfidenceLevel = 'Unknown' | 'High' | 'Medium' | 'Low';

/**
 * Field confidence structure
 */
export interface FieldConfidence {
  liters: ConfidenceLevel;
  totalPrice: ConfidenceLevel;
  date: ConfidenceLevel;
}

/**
 * Currency codes for multi-currency receipts
 */
export type ReceiptCurrency = 'EUR' | 'CZK' | 'HUF' | 'PLN';

/**
 * Receipt data structure matching Rust Receipt struct
 */
export interface Receipt {
  id?: string;
  vehicleId?: string;
  tripId?: string;
  filePath: string;
  fileName: string;
  scannedAt?: string;
  // Parsed fields
  liters?: number | null;
  totalPriceEur?: number | null;
  receiptDate?: string | null; // YYYY-MM-DD format
  stationName?: string | null;
  stationAddress?: string | null;
  vendorName?: string | null;
  costDescription?: string | null;
  // Multi-currency support
  originalAmount?: number | null;
  originalCurrency?: ReceiptCurrency | null;
  sourceYear?: number | null;
  // Status tracking
  status?: ReceiptStatus;
  confidence?: FieldConfidence;
  rawOcrText?: string | null;
  errorMessage?: string | null;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Trip statistics returned by get_trip_grid_data
 */
export interface TripStats {
  fuelRemainingLiters: number;
  avgConsumptionRate: number;
  lastConsumptionRate: number;
  marginPercent?: number;
  isOverLimit: boolean;
  totalKm: number;
  totalFuelLiters: number;
  totalFuelCostEur: number;
  bufferKm: number;
}

/**
 * Pre-calculated data for trip grid display
 */
export interface TripGridData {
  trips: Trip[];
  // Fuel data (ICE + PHEV)
  rates: Record<string, number>;
  estimatedRates: string[];
  fuelRemaining: Record<string, number>;
  consumptionWarnings: string[];
  // Energy data (BEV + PHEV)
  energyRates: Record<string, number>;
  estimatedEnergyRates: string[];
  batteryRemainingKwh: Record<string, number>;
  batteryRemainingPercent: Record<string, number>;
  socOverrideTrips: string[];
  // Shared warnings
  dateWarnings: string[];
  missingReceipts: string[];
}

/**
 * Preview result for live calculation feedback
 */
export interface PreviewResult {
  fuelRemaining: number;
  consumptionRate: number;
  marginPercent: number;
  isOverLimit: boolean;
  isEstimatedRate: boolean;
}

/**
 * Receipt verification status
 */
export interface ReceiptVerification {
  receiptId: string;
  matched: boolean;
  matchedTripId?: string;
  matchedTripDate?: string;
  matchedTripRoute?: string;
}

/**
 * Result of verifying all receipts
 */
export interface VerificationResult {
  total: number;
  matched: number;
  unmatched: number;
  receipts: ReceiptVerification[];
}
