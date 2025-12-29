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
	zostatok_liters: number;
	avg_consumption_rate: number;  // Average: total_fuel / total_km * 100
	last_consumption_rate: number; // From last fill-up period (for margin)
	margin_percent: number | null; // null if no fill-up yet
	is_over_limit: boolean;
	total_km: number;
	total_fuel_liters: number;
	total_fuel_cost_eur: number;
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
	fuel_remaining: Record<string, number>; // tripId -> zostatok
	date_warnings: string[]; // tripIds with date ordering issues
	consumption_warnings: string[]; // tripIds over 120% TP
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
}
