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
