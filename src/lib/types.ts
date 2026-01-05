// TypeScript interfaces matching Rust models

export interface Vehicle {
	id: string;
	name: string;
	license_plate: string;
	tank_size_liters: number;
	tp_consumption: number;
	initial_odometer: number;
	is_active: boolean;
	created_at: string;
	updated_at: string;
}

export interface Trip {
	id: string;
	vehicle_id: string;
	date: string; // NaiveDate serialized as string
	origin: string;
	destination: string;
	distance_km: number;
	odometer: number;
	purpose: string;
	fuel_liters?: number | null;
	fuel_cost_eur?: number | null;
	other_costs_eur?: number | null;
	other_costs_note?: string | null;
	full_tank: boolean; // true = full tank fillup, false = partial
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface Route {
	id: string;
	vehicle_id: string;
	origin: string;
	destination: string;
	distance_km: number;
	usage_count: number;
	last_used: string;
}

export interface CompensationSuggestion {
	origin: string;
	destination: string;
	distance_km: number;
	purpose: string;
	is_buffer: boolean;
}

export interface Settings {
	id: string;
	company_name: string;
	company_ico: string;
	buffer_trip_purpose: string;
	updated_at: string;
}

export interface TripStats {
	fuelRemainingLiters: number;
	avgConsumptionRate: number;  // Average: total_fuel / total_km * 100
	lastConsumptionRate: number; // From last fill-up period (for margin)
	marginPercent: number | null; // null if no fill-up yet
	isOverLimit: boolean;
	totalKm: number;
	totalFuelLiters: number;
	totalFuelCostEur: number;
}

export interface BackupInfo {
	filename: string;
	created_at: string;
	size_bytes: number;
	vehicle_count: number;
	trip_count: number;
}

export interface TripGridData {
	trips: Trip[];
	rates: Record<string, number>; // tripId -> l/100km
	estimated_rates: string[]; // tripIds using TP rate (estimated)
	fuel_remaining: Record<string, number>; // tripId -> fuel remaining
	date_warnings: string[]; // tripIds with date ordering issues
	consumption_warnings: string[]; // tripIds over 120% TP
	missing_receipts: string[]; // tripIds missing receipts
}

export type ReceiptStatus = 'Pending' | 'Parsed' | 'NeedsReview' | 'Assigned';
export type ConfidenceLevel = 'Unknown' | 'High' | 'Medium' | 'Low';

export interface FieldConfidence {
	liters: ConfidenceLevel;
	total_price: ConfidenceLevel;
	date: ConfidenceLevel;
}

export interface Receipt {
	id: string;
	vehicle_id: string | null;
	trip_id: string | null;
	file_path: string;
	file_name: string;
	scanned_at: string;
	liters: number | null;
	total_price_eur: number | null;
	receipt_date: string | null;
	station_name: string | null;
	station_address: string | null;
	source_year: number | null; // Year from folder structure (e.g., 2024 from "2024/" folder)
	status: ReceiptStatus;
	confidence: FieldConfidence;
	raw_ocr_text: string | null;
	error_message: string | null;
	created_at: string;
	updated_at: string;
}

export interface ReceiptSettings {
	gemini_api_key: string | null;
	receipts_folder_path: string | null;
	gemini_api_key_from_override: boolean;
	receipts_folder_from_override: boolean;
}

export interface SyncError {
	file_name: string;
	error: string;
}

export interface SyncResult {
	processed: Receipt[];
	errors: SyncError[];
	warning: string | null; // Warning message for invalid folder structure
}

export interface ScanResult {
	new_count: number;
	warning: string | null;
}

export interface ReceiptVerification {
	receipt_id: string;
	matched: boolean;
	matched_trip_id: string | null;
	matched_trip_date: string | null;
	matched_trip_route: string | null;
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
	// Footer labels
	footer_total_km: string;
	footer_total_fuel: string;
	footer_other_costs: string;
	footer_avg_consumption: string;
	footer_deviation: string;
	footer_tp_norm: string;
	// Print hint
	print_hint: string;
}
