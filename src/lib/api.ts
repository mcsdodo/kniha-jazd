// API wrapper for Tauri commands

import { invoke } from '@tauri-apps/api/core';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import type { Vehicle, Trip, Route, Settings, TripStats, BackupInfo, BackupType, CleanupPreview, CleanupResult, BackupRetention, TripGridData, Receipt, ReceiptSettings, ScanResult, SyncResult, VerificationResult, ExportLabels, PreviewResult, VehicleType, TripForAssignment } from './types';

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
	initialBatteryPercent?: number | null,
	vin?: string | null,
	driverName?: string | null
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
		initialBatteryPercent,
		vin,
		driverName
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

export async function calculateMagicFillLiters(
	vehicleId: string,
	year: number,
	currentTripKm: number,
	editingTripId?: string | null
): Promise<number> {
	return await invoke('calculate_magic_fill_liters', { vehicleId, year, currentTripKm, editingTripId });
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

// Purpose suggestions (across all years)
export async function getPurposes(vehicleId: string): Promise<string[]> {
	return await invoke('get_purposes', { vehicleId });
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

export async function revealBackup(filename: string): Promise<void> {
	const path: string = await invoke('get_backup_path', { filename });
	await revealItemInDir(path);
}

export async function createBackupWithType(
	backupType: BackupType,
	updateVersion: string | null
): Promise<BackupInfo> {
	return await invoke('create_backup_with_type', { backupType, updateVersion });
}

export async function getCleanupPreview(keepCount: number): Promise<CleanupPreview> {
	return await invoke('get_cleanup_preview', { keepCount });
}

export async function cleanupPreUpdateBackups(keepCount: number): Promise<CleanupResult> {
	return await invoke('cleanup_pre_update_backups', { keepCount });
}

export async function getBackupRetention(): Promise<BackupRetention | null> {
	return await invoke('get_backup_retention');
}

export async function setBackupRetention(retention: BackupRetention): Promise<void> {
	return await invoke('set_backup_retention', { retention });
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

export async function getReceiptsForVehicle(vehicleId: string, year?: number): Promise<Receipt[]> {
	return await invoke('get_receipts_for_vehicle', { vehicleId, year: year ?? null });
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

export async function getTripsForReceiptAssignment(
	receiptId: string,
	vehicleId: string,
	year: number
): Promise<TripForAssignment[]> {
	return await invoke('get_trips_for_receipt_assignment', { receiptId, vehicleId, year });
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

// Theme
export type ThemeMode = 'system' | 'light' | 'dark';

export async function getThemePreference(): Promise<ThemeMode> {
	return invoke<string>('get_theme_preference') as Promise<ThemeMode>;
}

export async function setThemePreference(theme: ThemeMode): Promise<void> {
	return invoke('set_theme_preference', { theme });
}

// Auto-update settings
export async function getAutoCheckUpdates(): Promise<boolean> {
	return invoke<boolean>('get_auto_check_updates');
}

export async function setAutoCheckUpdates(enabled: boolean): Promise<void> {
	return invoke('set_auto_check_updates', { enabled });
}

// Receipt settings
export async function setGeminiApiKey(apiKey: string): Promise<void> {
	return invoke('set_gemini_api_key', { apiKey });
}

export async function setReceiptsFolderPath(path: string): Promise<void> {
	return invoke('set_receipts_folder_path', { path });
}

// Database location
export interface DbLocationInfo {
	dbPath: string;
	isCustomPath: boolean;
	backupsPath: string;
}

export interface AppModeInfo {
	mode: string;
	isReadOnly: boolean;
	readOnlyReason: string | null;
}

export async function getDbLocation(): Promise<DbLocationInfo> {
	return invoke<DbLocationInfo>('get_db_location');
}

export async function getAppMode(): Promise<AppModeInfo> {
	return invoke<AppModeInfo>('get_app_mode');
}

export async function checkTargetHasDb(targetFolder: string): Promise<boolean> {
	return invoke<boolean>('check_target_has_db', { targetPath: targetFolder });
}

export interface MoveDbResult {
	success: boolean;
	newPath: string;
	filesMoved: number;
}

export async function moveDatabase(targetFolder: string): Promise<MoveDbResult> {
	return invoke<MoveDbResult>('move_database', { targetFolder });
}

export async function resetDatabaseLocation(): Promise<MoveDbResult> {
	return invoke<MoveDbResult>('reset_database_location');
}
