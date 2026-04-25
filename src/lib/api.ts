// API wrapper for Tauri commands

import { apiCall, IS_TAURI } from './api-adapter';
import type { Vehicle, Trip, Route, Settings, TripStats, BackupInfo, BackupType, CleanupPreview, CleanupResult, BackupRetention, TripGridData, Receipt, ReceiptSettings, ScanResult, SyncResult, VerificationResult, ExportLabels, PreviewResult, VehicleType, TripForAssignment, DatePrefillMode, InferredTripTime } from './types';

// Vehicle commands
export async function getVehicles(): Promise<Vehicle[]> {
	return await apiCall('get_vehicles');
}

export async function getActiveVehicle(): Promise<Vehicle | null> {
	return await apiCall('get_active_vehicle');
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
	return await apiCall('create_vehicle', {
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
	return await apiCall('update_vehicle', { vehicle });
}

export async function deleteVehicle(id: string): Promise<void> {
	return await apiCall('delete_vehicle', { id });
}

export async function setActiveVehicle(id: string): Promise<void> {
	return await apiCall('set_active_vehicle', { id });
}

// Trip commands
export async function getTrips(vehicleId: string): Promise<Trip[]> {
	return await apiCall('get_trips', { vehicleId });
}

export async function getTripsForYear(vehicleId: string, year: number): Promise<Trip[]> {
	return await apiCall('get_trips_for_year', { vehicleId, year });
}

export async function getYearsWithTrips(vehicleId: string): Promise<number[]> {
	return await apiCall('get_years_with_trips', { vehicleId });
}

export async function getTripGridData(vehicleId: string, year: number): Promise<TripGridData> {
	return await apiCall('get_trip_grid_data', { vehicleId, year });
}

export async function calculateMagicFillLiters(
	vehicleId: string,
	year: number,
	currentTripKm: number,
	editingTripId?: string | null
): Promise<number> {
	return await apiCall('calculate_magic_fill_liters', { vehicleId, year, currentTripKm, editingTripId });
}

export async function createTrip(
	vehicleId: string,
	startDatetime: string, // Full ISO datetime "YYYY-MM-DDTHH:MM"
	endDatetime: string,   // Full ISO datetime "YYYY-MM-DDTHH:MM"
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
	return await apiCall('create_trip', {
		vehicleId,
		startDatetime,
		endDatetime,
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
	startDatetime: string, // Full ISO datetime "YYYY-MM-DDTHH:MM"
	endDatetime: string,   // Full ISO datetime "YYYY-MM-DDTHH:MM"
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
	return await apiCall('update_trip', {
		id,
		startDatetime,
		endDatetime,
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
	return await apiCall('delete_trip', { id });
}

export async function reorderTrip(
	tripId: string,
	newSortOrder: number
): Promise<Trip[]> {
	return await apiCall('reorder_trip', {
		tripId,
		newSortOrder
	});
}

// Route commands
export async function getRoutes(vehicleId: string): Promise<Route[]> {
	return await apiCall('get_routes', { vehicleId });
}

// Purpose suggestions (across all years)
export async function getPurposes(vehicleId: string): Promise<string[]> {
	return await apiCall('get_purposes', { vehicleId });
}

// Settings commands
export async function getSettings(): Promise<Settings | null> {
	return await apiCall('get_settings');
}

export async function saveSettings(
	companyName: string,
	companyIco: string,
	bufferTripPurpose: string
): Promise<Settings> {
	return await apiCall('save_settings', {
		companyName,
		companyIco,
		bufferTripPurpose
	});
}

// Trip statistics
export async function calculateTripStats(vehicleId: string, year: number): Promise<TripStats> {
	return await apiCall('calculate_trip_stats', { vehicleId, year });
}

// Backup commands
export async function createBackup(): Promise<BackupInfo> {
	return await apiCall('create_backup');
}

export async function listBackups(): Promise<BackupInfo[]> {
	return await apiCall('list_backups');
}

export async function getBackupInfo(filename: string): Promise<BackupInfo> {
	return await apiCall('get_backup_info', { filename });
}

export async function restoreBackup(filename: string): Promise<void> {
	return await apiCall('restore_backup', { filename });
}

export async function deleteBackup(filename: string): Promise<void> {
	return await apiCall('delete_backup', { filename });
}

export async function revealBackup(filename: string): Promise<void> {
	const path = await apiCall<string>('get_backup_path', { filename });
	if (IS_TAURI) {
		const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
		await revealItemInDir(path);
	}
}

export async function createBackupWithType(
	backupType: BackupType,
	updateVersion: string | null
): Promise<BackupInfo> {
	return await apiCall('create_backup_with_type', { backupType, updateVersion });
}

export async function getCleanupPreview(keepCount: number): Promise<CleanupPreview> {
	return await apiCall('get_cleanup_preview', { keepCount });
}

export async function cleanupPreUpdateBackups(keepCount: number): Promise<CleanupResult> {
	return await apiCall('cleanup_pre_update_backups', { keepCount });
}

export async function getBackupRetention(): Promise<BackupRetention | null> {
	return await apiCall('get_backup_retention');
}

export async function setBackupRetention(retention: BackupRetention): Promise<void> {
	return await apiCall('set_backup_retention', { retention });
}

// Export - returns HTML string (used in server/browser mode)
export async function exportHtml(
	vehicleId: string,
	year: number,
	labels: ExportLabels,
): Promise<string> {
	return await apiCall('export_html', { vehicleId, year, labels });
}

// Export - opens HTML in default browser for printing (desktop only)
export async function openExportPreview(
	vehicleId: string,
	year: number,
	licensePlate: string,
	sortColumn: string,
	sortDirection: string,
	labels: ExportLabels,
	hiddenColumns: string[]
): Promise<void> {
	await apiCall('export_to_browser', { vehicleId, year, licensePlate, sortColumn, sortDirection, labels, hiddenColumns });
}

// Receipt commands
export async function getReceiptSettings(): Promise<ReceiptSettings> {
	return await apiCall('get_receipt_settings');
}

export async function getReceipts(year?: number): Promise<Receipt[]> {
	return await apiCall('get_receipts', { year: year ?? null });
}

export async function getReceiptsForVehicle(vehicleId: string, year?: number): Promise<Receipt[]> {
	return await apiCall('get_receipts_for_vehicle', { vehicleId, year: year ?? null });
}

export async function getUnassignedReceipts(): Promise<Receipt[]> {
	return await apiCall('get_unassigned_receipts');
}

export async function scanReceipts(): Promise<ScanResult> {
	return await apiCall('scan_receipts');
}

export async function syncReceipts(): Promise<SyncResult> {
	return await apiCall('sync_receipts');
}

export async function processPendingReceipts(): Promise<SyncResult> {
	return await apiCall('process_pending_receipts');
}

export async function updateReceipt(receipt: Receipt): Promise<void> {
	return await apiCall('update_receipt', { receipt });
}

export async function deleteReceipt(id: string): Promise<void> {
	return await apiCall('delete_receipt', { id });
}

export async function unassignReceipt(id: string): Promise<void> {
	return await apiCall('unassign_receipt', { id });
}

export async function revertReceiptOverride(id: string): Promise<void> {
	return await apiCall('revert_receipt_override', { id });
}

export async function reprocessReceipt(id: string): Promise<Receipt> {
	return await apiCall('reprocess_receipt', { id });
}

export async function assignReceiptToTrip(
	receiptId: string,
	tripId: string,
	vehicleId: string,
	assignmentType: 'Fuel' | 'Other',
	mismatchOverride: boolean = false
): Promise<Receipt> {
	return await apiCall('assign_receipt_to_trip', {
		receiptId,
		tripId,
		vehicleId,
		assignmentType,
		mismatchOverride
	});
}

export async function getTripsForReceiptAssignment(
	receiptId: string,
	vehicleId: string,
	year: number
): Promise<TripForAssignment[]> {
	return await apiCall('get_trips_for_receipt_assignment', { receiptId, vehicleId, year });
}

export async function verifyReceipts(vehicleId: string, year: number): Promise<VerificationResult> {
	return await apiCall('verify_receipts', { vehicleId, year });
}

// Window
export interface WindowSize {
	width: number;
	height: number;
}

export async function getOptimalWindowSize(): Promise<WindowSize> {
	return await apiCall('get_optimal_window_size');
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
	return await apiCall('preview_trip_calculation', {
		vehicleId,
		year,
		distanceKm,
		fuelLiters,
		fullTank,
		insertAtSortOrder,
		editingTripId
	});
}

// Theme (type is defined in constants.ts)
import type { ThemeMode } from '$lib/constants';
export type { ThemeMode };

export async function getThemePreference(): Promise<ThemeMode> {
	return apiCall<string>('get_theme_preference') as Promise<ThemeMode>;
}

export async function setThemePreference(theme: ThemeMode): Promise<void> {
	return apiCall('set_theme_preference', { theme });
}

// Auto-update settings
export async function getAutoCheckUpdates(): Promise<boolean> {
	return apiCall<boolean>('get_auto_check_updates');
}

export async function setAutoCheckUpdates(enabled: boolean): Promise<void> {
	return apiCall('set_auto_check_updates', { enabled });
}

// Date prefill mode settings
export async function getDatePrefillMode(): Promise<DatePrefillMode> {
	return apiCall<DatePrefillMode>('get_date_prefill_mode');
}

export async function setDatePrefillMode(mode: DatePrefillMode): Promise<void> {
	return apiCall('set_date_prefill_mode', { mode });
}

// Receipt settings
export async function setGeminiApiKey(apiKey: string): Promise<void> {
	return apiCall('set_gemini_api_key', { apiKey });
}

export async function setReceiptsFolderPath(path: string): Promise<void> {
	return apiCall('set_receipts_folder_path', { path });
}

// Home Assistant settings
export interface HaSettingsResponse {
	url: string | null;
	hasToken: boolean;
}

export async function getHaSettings(): Promise<HaSettingsResponse> {
	return apiCall<HaSettingsResponse>('get_ha_settings');
}

export async function saveHaSettings(url: string | null, token: string | null): Promise<void> {
	return apiCall('save_ha_settings', { url, token });
}

// For frontend HA API calls (includes token)
export interface HaLocalSettings {
	haUrl: string | null;
	haApiToken: string | null;
}

export async function getLocalSettingsForHa(): Promise<HaLocalSettings> {
	// Backend uses #[serde(rename_all = "camelCase")] so fields are already camelCase
	return apiCall<HaLocalSettings>('get_local_settings_for_ha');
}

// Test HA connection from backend (avoids CORS issues)
export async function testHaConnection(): Promise<boolean> {
	return apiCall<boolean>('test_ha_connection');
}

// Fetch ODO value from HA for a specific sensor
export async function fetchHaOdo(sensorId: string): Promise<number | null> {
	return apiCall<number | null>('fetch_ha_odo', { sensorId });
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
	return apiCall<DbLocationInfo>('get_db_location');
}

export async function getAppMode(): Promise<AppModeInfo> {
	return apiCall<AppModeInfo>('get_app_mode');
}

export async function checkTargetHasDb(targetFolder: string): Promise<boolean> {
	return apiCall<boolean>('check_target_has_db', { targetPath: targetFolder });
}

export interface MoveDbResult {
	success: boolean;
	newPath: string;
	filesMoved: number;
}

export async function moveDatabase(targetFolder: string): Promise<MoveDbResult> {
	return apiCall<MoveDbResult>('move_database', { targetFolder });
}

export async function resetDatabaseLocation(): Promise<MoveDbResult> {
	return apiCall<MoveDbResult>('reset_database_location');
}

// Time inference
export async function getInferredTripTimeForRoute(
	vehicleId: string, origin: string, destination: string, rowDate: string
): Promise<InferredTripTime | null> {
	return await apiCall('get_inferred_trip_time_for_route', {
		vehicleId, origin, destination, rowDate,
	});
}

// Hidden columns
export async function getHiddenColumns(): Promise<string[]> {
	return apiCall<string[]>('get_hidden_columns');
}

export async function setHiddenColumns(columns: string[]): Promise<void> {
	return apiCall('set_hidden_columns', { columns });
}

// Server Mode
export interface ServerStatus {
	running: boolean;
	port: number | null;
	url: string | null;
}

export async function getServerStatus(): Promise<ServerStatus> {
	return await apiCall('get_server_status');
}

export async function startServer(port: number): Promise<ServerStatus> {
	return await apiCall('start_server', { port });
}

export async function stopServer(): Promise<void> {
	return await apiCall('stop_server');
}
