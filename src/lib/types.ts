// TypeScript interfaces matching Rust models

export type VehicleType = 'Ice' | 'Bev' | 'Phev';

export interface Vehicle {
	id: string;
	name: string;
	licensePlate: string;
	vehicleType: VehicleType;
	// Fuel fields (ICE + PHEV)
	tankSizeLiters: number | null;
	tpConsumption: number | null;
	// Battery fields (BEV + PHEV)
	batteryCapacityKwh: number | null;
	baselineConsumptionKwh: number | null;
	initialBatteryPercent: number | null;
	// Common
	initialOdometer: number;
	isActive: boolean;
	vin?: string | null;
	driverName?: string | null;
	createdAt: string;
	updatedAt: string;
}

export interface Trip {
	id: string;
	vehicleId: string;
	date: string; // NaiveDate serialized as string
	origin: string;
	destination: string;
	distanceKm: number;
	odometer: number;
	purpose: string;
	// Fuel fields (ICE + PHEV)
	fuelLiters?: number | null;
	fuelCostEur?: number | null;
	fullTank: boolean; // true = full tank fillup, false = partial
	// Energy fields (BEV + PHEV)
	energyKwh?: number | null;
	energyCostEur?: number | null;
	fullCharge: boolean;
	socOverridePercent?: number | null;
	// Other
	otherCostsEur?: number | null;
	otherCostsNote?: string | null;
	sortOrder: number;
	createdAt: string;
	updatedAt: string;
}

export type AttachmentStatus = 'empty' | 'matches' | 'differs';
export type MismatchReason = 'date' | 'liters' | 'price' | 'liters_and_price' | 'date_and_liters' | 'date_and_price' | 'all';

export interface TripForAssignment {
	trip: Trip;
	canAttach: boolean;
	attachmentStatus: AttachmentStatus;
	mismatchReason: MismatchReason | null;
}

export interface Route {
	id: string;
	vehicleId: string;
	origin: string;
	destination: string;
	distanceKm: number;
	usageCount: number;
	lastUsed: string;
}

export interface Settings {
	id: string;
	companyName: string;
	companyIco: string;
	bufferTripPurpose: string;
	updatedAt: string;
}

export interface TripStats {
	fuelRemainingLiters: number;
	avgConsumptionRate: number; // Average: total_fuel / total_km * 100
	lastConsumptionRate: number; // From last fill-up period (for margin)
	marginPercent: number | null; // null if no fill-up yet
	isOverLimit: boolean;
	totalKm: number;
	totalFuelLiters: number;
	totalFuelCostEur: number;
	bufferKm: number; // Additional km needed to reach 18% margin (0.0 if under target)
}

export interface BackupInfo {
	filename: string;
	createdAt: string;
	sizeBytes: number;
	vehicleCount: number;
	tripCount: number;
}

export interface TripGridData {
	trips: Trip[];
	// Fuel data (ICE + PHEV)
	rates: Record<string, number>; // tripId -> l/100km
	estimatedRates: string[]; // tripIds using TP rate (estimated)
	fuelRemaining: Record<string, number>; // tripId -> fuel remaining
	consumptionWarnings: string[]; // tripIds over 120% TP
	// Energy data (BEV + PHEV)
	energyRates: Record<string, number>; // tripId -> kWh/100km
	estimatedEnergyRates: string[]; // tripIds using baseline rate
	batteryRemainingKwh: Record<string, number>; // tripId -> kWh
	batteryRemainingPercent: Record<string, number>; // tripId -> %
	socOverrideTrips: string[]; // tripIds with manual SoC override
	// Warnings
	dateWarnings: string[]; // tripIds with date ordering issues
	missingReceipts: string[]; // tripIds missing receipts
	// Year boundary data
	yearStartOdometer: number; // Starting ODO for this year (carryover from previous year)
}

export type ReceiptStatus = 'Pending' | 'Parsed' | 'NeedsReview' | 'Assigned';
export type ConfidenceLevel = 'Unknown' | 'High' | 'Medium' | 'Low';

export interface FieldConfidence {
	liters: ConfidenceLevel;
	totalPrice: ConfidenceLevel;
	date: ConfidenceLevel;
}

export interface Receipt {
	id: string;
	vehicleId: string | null;
	tripId: string | null;
	filePath: string;
	fileName: string;
	scannedAt: string;
	liters: number | null;
	totalPriceEur: number | null;
	receiptDate: string | null;
	stationName: string | null;
	stationAddress: string | null;
	sourceYear: number | null; // Year from folder structure (e.g., 2024 from "2024/" folder)
	vendorName: string | null; // Vendor/store name for non-fuel receipts
	costDescription: string | null; // Description of cost for non-fuel receipts
	status: ReceiptStatus;
	confidence: FieldConfidence;
	rawOcrText: string | null;
	errorMessage: string | null;
	createdAt: string;
	updatedAt: string;
}

export interface ReceiptSettings {
	geminiApiKey: string | null;
	receiptsFolderPath: string | null;
	geminiApiKeyFromOverride: boolean;
	receiptsFolderFromOverride: boolean;
}

export interface SyncError {
	fileName: string;
	error: string;
}

export interface SyncResult {
	processed: Receipt[];
	errors: SyncError[];
	warning: string | null; // Warning message for invalid folder structure
}

export interface ScanResult {
	newCount: number;
	warning: string | null;
}

// Reason why a receipt could not be matched to a trip (verification)
export type ReceiptMismatchReason =
	| { type: 'none' }
	| { type: 'missingReceiptData' }
	| { type: 'noFuelTripFound' }
	| { type: 'dateMismatch'; receiptDate: string; closestTripDate: string }
	| { type: 'litersMismatch'; receiptLiters: number; tripLiters: number }
	| { type: 'priceMismatch'; receiptPrice: number; tripPrice: number }
	| { type: 'noOtherCostMatch' };

export interface ReceiptVerification {
	receiptId: string;
	matched: boolean;
	matchedTripId: string | null;
	matchedTripDate: string | null;
	matchedTripRoute: string | null;
	mismatchReason: ReceiptMismatchReason;
}

export interface VerificationResult {
	total: number;
	matched: number;
	unmatched: number;
	receipts: ReceiptVerification[];
}

// Live preview result for trip editing
export interface PreviewResult {
	fuelRemaining: number;
	consumptionRate: number;
	marginPercent: number;
	isOverLimit: boolean;
	isEstimatedRate: boolean;
}

// Export labels passed to Rust for HTML export
// NOTE: Keep snake_case - this is passed TO Rust for HTML template rendering
export interface ExportLabels {
	// Language code for HTML lang attribute
	lang: string;
	// Page title
	page_title: string;
	// Header labels
	header_company: string;
	header_ico: string;
	header_vehicle: string;
	header_license_plate: string;
	header_tank_size: string;
	header_tp_consumption: string;
	header_year: string;
	// Header labels for BEV
	header_battery_capacity: string;
	header_baseline_consumption: string;
	// VIN and Driver
	header_vin: string;
	header_driver: string;
	// Column headers
	col_date: string;
	col_origin: string;
	col_destination: string;
	col_purpose: string;
	col_km: string;
	col_odo: string;
	col_fuel_liters: string;
	col_fuel_cost: string;
	col_other_costs: string;
	col_note: string;
	col_remaining: string;
	col_consumption: string;
	// Column headers for BEV
	col_energy_kwh: string;
	col_energy_cost: string;
	col_battery_remaining: string;
	col_energy_rate: string;
	// Footer labels
	footer_total_km: string;
	footer_total_fuel: string;
	footer_other_costs: string;
	footer_avg_consumption: string;
	footer_deviation: string;
	footer_tp_norm: string;
	// Footer labels for BEV
	footer_total_energy: string;
	footer_avg_energy_rate: string;
	footer_baseline_norm: string;
	// Print hint
	print_hint: string;
}
