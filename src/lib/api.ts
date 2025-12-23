// API wrapper for Tauri commands

import { invoke } from '@tauri-apps/api/core';
import type { Vehicle, Trip, Route, CompensationSuggestion, Settings, TripStats } from './types';

// Vehicle commands
export async function getVehicles(): Promise<Vehicle[]> {
	return await invoke('get_vehicles');
}

export async function getActiveVehicle(): Promise<Vehicle | null> {
	return await invoke('get_active_vehicle');
}

export async function createVehicle(
	name: string,
	license_plate: string,
	tank_size_liters: number,
	tp_consumption: number
): Promise<Vehicle> {
	return await invoke('create_vehicle', {
		name,
		licensePlate: license_plate,
		tankSizeLiters: tank_size_liters,
		tpConsumption: tp_consumption
	});
}

export async function updateVehicle(vehicle: Vehicle): Promise<void> {
	return await invoke('update_vehicle', { vehicle });
}

export async function deleteVehicle(id: string): Promise<void> {
	return await invoke('delete_vehicle', { id });
}

export async function setActiveVehicle(id: string): Promise<void> {
	return await invoke('set_active_vehicle', { id });
}

// Trip commands
export async function getTrips(vehicleId: string): Promise<Trip[]> {
	return await invoke('get_trips', { vehicleId });
}

export async function getTripsForYear(vehicleId: string, year: number): Promise<Trip[]> {
	return await invoke('get_trips_for_year', { vehicleId, year });
}

export async function createTrip(
	vehicleId: string,
	date: string,
	origin: string,
	destination: string,
	distanceKm: number,
	odometer: number,
	purpose: string,
	fuelLiters?: number | null,
	fuelCostEur?: number | null,
	otherCostsEur?: number | null,
	otherCostsNote?: string | null
): Promise<Trip> {
	return await invoke('create_trip', {
		vehicleId,
		date,
		origin,
		destination,
		distanceKm,
		odometer,
		purpose,
		fuelLiters,
		fuelCostEur,
		otherCostsEur,
		otherCostsNote
	});
}

export async function updateTrip(
	id: string,
	date: string,
	origin: string,
	destination: string,
	distanceKm: number,
	odometer: number,
	purpose: string,
	fuelLiters?: number | null,
	fuelCostEur?: number | null,
	otherCostsEur?: number | null,
	otherCostsNote?: string | null
): Promise<Trip> {
	return await invoke('update_trip', {
		id,
		date,
		origin,
		destination,
		distanceKm,
		odometer,
		purpose,
		fuelLiters,
		fuelCostEur,
		otherCostsEur,
		otherCostsNote
	});
}

export async function deleteTrip(id: string): Promise<void> {
	return await invoke('delete_trip', { id });
}

// Route commands
export async function getRoutes(vehicleId: string): Promise<Route[]> {
	return await invoke('get_routes', { vehicleId });
}

// Compensation suggestion
export async function getCompensationSuggestion(
	vehicleId: string,
	bufferKm: number,
	currentLocation: string
): Promise<CompensationSuggestion> {
	return await invoke('get_compensation_suggestion', { vehicleId, bufferKm, currentLocation });
}

// Settings commands
export async function getSettings(): Promise<Settings | null> {
	return await invoke('get_settings');
}

export async function saveSettings(
	companyName: string,
	companyIco: string,
	bufferTripPurpose: string
): Promise<Settings> {
	return await invoke('save_settings', {
		companyName,
		companyIco,
		bufferTripPurpose
	});
}

// Trip statistics
export async function calculateTripStats(vehicleId: string): Promise<TripStats> {
	return await invoke('calculate_trip_stats', { vehicleId });
}
