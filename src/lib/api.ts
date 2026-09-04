// API wrapper for backend commands

import { apiCall } from './api-adapter';
import type { Vehicle, Trip, Route, Settings, TripStats, BackupInfo, BackupType, CleanupPreview, CleanupResult, BackupRetention, TripGridData, Receipt, ReceiptSettings, ScanResult, SyncResult, VerificationResult, ExportLabels, PreviewResult, VehicleType, TripForAssignment, DatePrefillMode, InferredTripTime, CopiedTripDefaults, HaSettings, SecretField, PaperlessSettings, PaperlessCustomFieldInfo, InvoiceSourceMode, PaperlessInvoiceRow, InvoiceRef, InvoiceData, GeneratedRoute, RouteMap } from './types';

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
	otherCostsNote?: string | null
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
		otherCostsNote
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
	hiddenColumns: string[],
	sortDirection: string
): Promise<string> {
	return await apiCall('export_html', { vehicleId, year, labels, hiddenColumns, sortDirection });
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

export async function revertReceiptOverride(id: string): Promise<void> {
	return await apiCall('revert_receipt_override', { id });
}

export async function reprocessReceipt(id: string): Promise<Receipt> {
	return await apiCall('reprocess_receipt', { id });
}

export async function verifyReceipts(vehicleId: string, year: number): Promise<VerificationResult> {
	return await apiCall('verify_receipts', { vehicleId, year });
}

// Live Preview
export async function previewTripCalculation(
	vehicleId: string,
	year: number,
	distanceKm: number,
	fuelLiters: number | null,
	fullTank: boolean,
	insertAtTripId: string | null,
	editingTripId: string | null
): Promise<PreviewResult> {
	return await apiCall('preview_trip_calculation', {
		vehicleId,
		year,
		distanceKm,
		fuelLiters,
		fullTank,
		insertAtTripId,
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

// App version (works in desktop and web/server mode)
export async function getAppVersion(): Promise<string> {
	return apiCall<string>('get_app_version');
}

// Date prefill mode settings
export async function getDatePrefillMode(): Promise<DatePrefillMode> {
	return apiCall<DatePrefillMode>('get_date_prefill_mode');
}

export async function setDatePrefillMode(mode: DatePrefillMode): Promise<void> {
	return apiCall('set_date_prefill_mode', { mode });
}

// Time inference settings
export async function getInferTripTimes(): Promise<boolean> {
	return apiCall<boolean>('get_infer_trip_times');
}

export async function setInferTripTimes(enabled: boolean): Promise<void> {
	return apiCall('set_infer_trip_times', { enabled });
}

// Receipt settings
export async function setGeminiApiKey(apiKey: string): Promise<void> {
	return apiCall('set_gemini_api_key', { apiKey });
}

export async function setReceiptsFolderPath(path: string): Promise<void> {
	return apiCall('set_receipts_folder_path', { path });
}

// Home Assistant settings — shape lives in types.ts (HaSettings) so the page and
// the API wrapper can't drift apart.
export async function getHaSettings(): Promise<HaSettings> {
	return apiCall<HaSettings>('get_ha_settings');
}

export async function saveHaSettings(url: string | null, token: string | null): Promise<void> {
	return apiCall('save_ha_settings', { url, token });
}

/**
 * Reveal a configured secret for display.
 *
 * The backend demands the PIN from KNIHA_JAZD_REVEAL_PIN on every reveal — there
 * is no trusted local path any more. Throws with the backend's message on a
 * wrong/absent PIN or while locked out.
 */
export async function revealSecret(field: SecretField, pin?: string): Promise<string> {
	return apiCall<string>('reveal_secret', { field, pin: pin ?? '' });
}

// Test HA connection from backend (avoids CORS issues)
export async function testHaConnection(): Promise<boolean> {
	return apiCall<boolean>('test_ha_connection');
}

// Fetch ODO value from HA for a specific sensor
export async function fetchHaOdo(sensorId: string): Promise<number | null> {
	return apiCall<number | null>('fetch_ha_odo', { sensorId });
}

export interface AppModeInfo {
	mode: string;
	isReadOnly: boolean;
	readOnlyReason: string | null;
}

export async function getAppMode(): Promise<AppModeInfo> {
	return apiCall<AppModeInfo>('get_app_mode');
}

// Time inference
export async function getInferredTripTimeForRoute(
	vehicleId: string, origin: string, destination: string, rowDate: string
): Promise<InferredTripTime | null> {
	return await apiCall('get_inferred_trip_time_for_route', {
		vehicleId, origin, destination, rowDate,
	});
}

export async function getCopiedTripDefaults(
	tripId: string, year: number
): Promise<CopiedTripDefaults> {
	return await apiCall('get_copied_trip_defaults', { tripId, year });
}

// Hidden columns
export async function getHiddenColumns(): Promise<string[]> {
	return apiCall<string[]>('get_hidden_columns');
}

export async function setHiddenColumns(columns: string[]): Promise<void> {
	return apiCall('set_hidden_columns', { columns });
}

// Paperless-ngx integration
export async function getPaperlessSettings(): Promise<PaperlessSettings> {
	return apiCall<PaperlessSettings>('get_paperless_settings');
}

// null = keep existing value, '' (empty string) = clear the value
export async function savePaperlessSettings(
	url: string | null,
	token: string | null,
	enabled: boolean | null = null,
	fieldNameDatetime: string | null = null,
	fieldNameLiters: string | null = null,
	fieldNameTotal: string | null = null,
): Promise<void> {
	return apiCall('save_paperless_settings', {
		url,
		token,
		enabled,
		fieldNameDatetime,
		fieldNameLiters,
		fieldNameTotal,
	});
}

export async function testPaperlessConnection(): Promise<boolean> {
	return apiCall<boolean>('test_paperless_connection');
}

/**
 * Fetch the list of all custom fields from the configured Paperless server.
 * Used by Settings UI to populate the field-name dropdowns.
 *
 * Throws if Paperless is unreachable or unauthenticated. The Settings UI
 * treats `Result.Err("not configured")` as "hide the section" rather than
 * surfacing an error toast.
 */
export async function listPaperlessCustomFields(): Promise<PaperlessCustomFieldInfo[]> {
	return apiCall<PaperlessCustomFieldInfo[]>('list_paperless_custom_fields');
}

export async function getInvoiceSourceMode(): Promise<InvoiceSourceMode> {
	return apiCall<InvoiceSourceMode>('get_invoice_source_mode');
}

export async function getPaperlessInvoices(vehicleId: string, year: number): Promise<PaperlessInvoiceRow[]> {
	return apiCall<PaperlessInvoiceRow[]>('get_paperless_invoices', { vehicleId, year });
}

// Unified invoice commands (Task 64)
export async function getTripsForInvoiceAssignment(
	invoiceRef: InvoiceRef,
	invoiceData: InvoiceData | null,
	vehicleId: string,
	year: number,
): Promise<TripForAssignment[]> {
	return await apiCall('get_trips_for_invoice_assignment', {
		invoiceRef, invoiceData, vehicleId, year,
	});
}

export async function assignInvoiceToTrip(
	invoiceRef: InvoiceRef,
	invoiceData: InvoiceData | null,
	tripId: string,
	vehicleId: string,
	assignmentType: 'Fuel' | 'Other',
	mismatchOverride: boolean = false,
): Promise<void> {
	return await apiCall('assign_invoice_to_trip', {
		invoiceRef, invoiceData, tripId, vehicleId, assignmentType, mismatchOverride,
	});
}

export async function unassignInvoice(invoiceRef: InvoiceRef): Promise<void> {
	return await apiCall('unassign_invoice', { invoiceRef });
}

// Route map commands (Task 70)
export async function generateRoute(targetKm: number): Promise<GeneratedRoute> {
	return await apiCall('generate_route', { targetKm });
}

export async function getTripRoute(tripId: string): Promise<RouteMap | null> {
	return await apiCall('get_trip_route', { tripId });
}

// coordinates and datasetVersion are intentionally not sent — the backend
// re-derives both (polyline decode + bundled dataset version). Adding them
// here would be silently ignored: serde drops unknown fields by default.
export async function saveTripRoute(tripId: string, route: GeneratedRoute): Promise<void> {
	return await apiCall('save_trip_route', {
		tripId,
		waypoints: route.waypoints,
		polyline: route.polyline,
		targetKm: route.targetKm,
		roadKm: route.roadKm,
	});
}

export async function deleteTripRoute(tripId: string): Promise<void> {
	return await apiCall('delete_trip_route', { tripId });
}
