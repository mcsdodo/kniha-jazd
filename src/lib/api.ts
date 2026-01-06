// API wrapper for Tauri commands

import { invoke } from '@tauri-apps/api/core';
import type { Vehicle, Trip, Route, CompensationSuggestion, Settings, TripStats, BackupInfo, TripGridData, Receipt, ReceiptSettings, ScanResult, SyncResult, VerificationResult, ExportLabels, PreviewResult, VehicleType } from './types';

// Vehicle commands
export async function getVehicles(): Promise<Vehicle[]> {
	return await invoke('get_vehicles');
}

export async function getActiveVehicle(): Promise<Vehicle | null> {
	return await invoke('get_active_vehicle');
}

export async function createVehicle(
	name: string,
	licensePlate: string,
	initialOdometer: number,
	vehicleType: VehicleType = 'Ice',
	tankSizeLiters?: number | null,
	tpConsumption?: number | null,
	batteryCapacityKwh?: number | null,
	baselineConsumptionKwh?: number | null,
	initialBatteryPercent?: number | null
): Promise<Vehicle> {
	return await invoke('create_vehicle', {
		name,
		licensePlate,
		initialOdometer,
		vehicleType,
		tankSizeLiters,
		tpConsumption,
		batteryCapacityKwh,
		baselineConsumptionKwh,
		initialBatteryPercent
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

export async function getYearsWithTrips(vehicleId: string): Promise<number[]> {
	return await invoke('get_years_with_trips', { vehicleId });
}

export async function getTripGridData(vehicleId: string, year: number): Promise<TripGridData> {
	return await invoke('get_trip_grid_data', { vehicleId, year });
}

export async function createTrip(
	vehicleId: string,
	date: string,
	origin: string,
	destination: string,
	distanceKm: number,
	odometer: number,
	purpose: string,
	// Fuel fields (ICE + PHEV)
	fuelLiters?: number | null,
	fuelCost?: number | null,
	fullTank?: boolean | null,
	// Energy fields (BEV + PHEV)
	energyKwh?: number | null,
	energyCostEur?: number | null,
	fullCharge?: boolean | null,
	socOverridePercent?: number | null,
	// Other
	otherCosts?: number | null,
	otherCostsNote?: string | null,
	insertAtPosition?: number | null
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
		fuelCost,
		fullTank,
		energyKwh,
		energyCostEur,
		fullCharge,
		socOverridePercent,
		otherCosts,
		otherCostsNote,
		insertAtPosition
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
	// Fuel fields (ICE + PHEV)
	fuelLiters?: number | null,
	fuelCostEur?: number | null,
	fullTank?: boolean | null,
	// Energy fields (BEV + PHEV)
	energyKwh?: number | null,
	energyCostEur?: number | null,
	fullCharge?: boolean | null,
	socOverridePercent?: number | null,
	// Other
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
		fullTank,
		energyKwh,
		energyCostEur,
		fullCharge,
		socOverridePercent,
		otherCostsEur,
		otherCostsNote
	});
}

export async function deleteTrip(id: string): Promise<void> {
	return await invoke('delete_trip', { id });
}

export async function reorderTrip(
	tripId: string,
	newSortOrder: number
): Promise<Trip[]> {
	return await invoke('reorder_trip', {
		tripId,
		newSortOrder
	});
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
export async function calculateTripStats(vehicleId: string, year: number): Promise<TripStats> {
	return await invoke('calculate_trip_stats', { vehicleId, year });
}

// Backup commands
export async function createBackup(): Promise<BackupInfo> {
	return await invoke('create_backup');
}

export async function listBackups(): Promise<BackupInfo[]> {
	return await invoke('list_backups');
}

export async function getBackupInfo(filename: string): Promise<BackupInfo> {
	return await invoke('get_backup_info', { filename });
}

export async function restoreBackup(filename: string): Promise<void> {
	return await invoke('restore_backup', { filename });
}

export async function deleteBackup(filename: string): Promise<void> {
	return await invoke('delete_backup', { filename });
}

// Export - opens HTML in default browser for printing
export async function openExportPreview(
	vehicleId: string,
	year: number,
	licensePlate: string,
	sortColumn: string,
	sortDirection: string,
	labels: ExportLabels
): Promise<void> {
	await invoke('export_to_browser', { vehicleId, year, licensePlate, sortColumn, sortDirection, labels });
}

// Receipt commands
export async function getReceiptSettings(): Promise<ReceiptSettings> {
	return await invoke('get_receipt_settings');
}

export async function getReceipts(year?: number): Promise<Receipt[]> {
	return await invoke('get_receipts', { year: year ?? null });
}

export async function getUnassignedReceipts(): Promise<Receipt[]> {
	return await invoke('get_unassigned_receipts');
}

export async function scanReceipts(): Promise<ScanResult> {
	return await invoke('scan_receipts');
}

export async function syncReceipts(): Promise<SyncResult> {
	return await invoke('sync_receipts');
}

export async function processPendingReceipts(): Promise<SyncResult> {
	return await invoke('process_pending_receipts');
}

export async function updateReceipt(receipt: Receipt): Promise<void> {
	return await invoke('update_receipt', { receipt });
}

export async function deleteReceipt(id: string): Promise<void> {
	return await invoke('delete_receipt', { id });
}

export async function reprocessReceipt(id: string): Promise<Receipt> {
	return await invoke('reprocess_receipt', { id });
}

export async function assignReceiptToTrip(
	receiptId: string,
	tripId: string,
	vehicleId: string
): Promise<Receipt> {
	return await invoke('assign_receipt_to_trip', { receiptId, tripId, vehicleId });
}

export async function verifyReceipts(vehicleId: string, year: number): Promise<VerificationResult> {
	return await invoke('verify_receipts', { vehicleId, year });
}

// Window
export interface WindowSize {
	width: number;
	height: number;
}

export async function getOptimalWindowSize(): Promise<WindowSize> {
	return await invoke('get_optimal_window_size');
}

// Live Preview
export async function previewTripCalculation(
	vehicleId: string,
	year: number,
	distanceKm: number,
	fuelLiters: number | null,
	fullTank: boolean,
	insertAtSortOrder: number | null,
	editingTripId: string | null
): Promise<PreviewResult> {
	return await invoke('preview_trip_calculation', {
		vehicleId,
		year,
		distanceKm,
		fuelLiters,
		fullTank,
		insertAtSortOrder,
		editingTripId
	});
}
