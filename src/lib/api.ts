// API wrapper for backend commands

import { apiCall } from './api-adapter';
import type { Vehicle, Trip, Route, Settings, TripStats, BackupInfo, BackupType, CleanupPreview, CleanupResult, BackupRetention, TripGridData, ExportLabels, PreviewResult, VehicleType, TripForAssignment, DatePrefillMode, InferredTripTime, CopiedTripDefaults, HaSettings, SecretField, PaperlessSettings, PaperlessCustomFieldInfo, PaperlessInvoiceRow, GeneratedRoute, RouteMap, Place, GeocodeCandidate, PlaceSource, Waypoint, RouteStart, InsertPoint, LegInsertPoint, RoundTripRoutes, CascadePlan, CascadeResult, DistanceWriteback } from './types';

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

// The plain `create_trip`, `update_trip` and `delete_trip` commands have no
// wrapper here on purpose (task 81, M3). They write exactly what they are
// given, with no cascade and no confirmation, so a wrapper in this module
// invites a component to save a trip the way the cascade was built to stop.
// The commands themselves stay on the RPC dispatcher: the task 79 correction
// procedure depends on `update_trip` writing a row verbatim.

/**
 * Save a trip and move the odometer of every later row of the same year.
 *
 * With `dryRun: true` nothing is written and the returned plan is what fills
 * the confirmation modal. Call it again with `dryRun: false` to apply. The
 * apply recomputes from the stored book, so the two calls can disagree if the
 * book moved in between -- which is the point.
 */
export async function updateTripCascade(
	id: string,
	startDatetime: string, // Full ISO datetime "YYYY-MM-DDTHH:MM"
	endDatetime: string,   // Full ISO datetime "YYYY-MM-DDTHH:MM"
	origin: string,
	destination: string,
	distanceKm: number,
	odometer: number,
	purpose: string,
	// Fuel fields (ICE + PHEV)
	fuelLiters: number | null | undefined,
	fuelCostEur: number | null | undefined,
	fullTank: boolean | null | undefined,
	// Energy fields (BEV + PHEV)
	energyKwh: number | null | undefined,
	energyCostEur: number | null | undefined,
	fullCharge: boolean | null | undefined,
	socOverridePercent: number | null | undefined,
	// Other
	otherCostsEur: number | null | undefined,
	otherCostsNote: string | null | undefined,
	dryRun: boolean
): Promise<CascadeResult> {
	return await apiCall('update_trip_cascade', {
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
		otherCostsNote,
		dryRun
	});
}

/**
 * Write a route's road distance onto the trip it illustrates.
 *
 * Call it twice: `dryRun: true` fills the confirmation modal and writes
 * nothing, then `dryRun: false` writes. The apply call plans again from the
 * stored book rather than replaying the dry run's numbers, so a book that
 * moved in between is corrected against as it is now.
 *
 * Only the distance crosses the wire. The trip's other fields are not
 * resubmitted, so this command cannot change them even by mistake.
 */
export async function applyRouteDistance(
	tripId: string,
	roadKm: number,
	dryRun: boolean
): Promise<DistanceWriteback> {
	return await apiCall('apply_route_distance', { tripId, roadKm, dryRun });
}

/**
 * Create a trip and move the odometer of every later row of the same year.
 * The backend derives the new row's odometer from the chain, so this takes
 * no `odometer` parameter -- passing one would let the browser overrule the
 * book.
 *
 * With `dryRun: true` nothing is written and the returned plan is what fills
 * the confirmation modal.
 */
export async function createTripCascade(
	vehicleId: string,
	startDatetime: string, // Full ISO datetime "YYYY-MM-DDTHH:MM"
	endDatetime: string,   // Full ISO datetime "YYYY-MM-DDTHH:MM"
	origin: string,
	destination: string,
	distanceKm: number,
	purpose: string,
	// Fuel fields (ICE + PHEV)
	fuelLiters: number | null | undefined,
	fuelCost: number | null | undefined,
	fullTank: boolean | null | undefined,
	// Energy fields (BEV + PHEV)
	energyKwh: number | null | undefined,
	energyCostEur: number | null | undefined,
	fullCharge: boolean | null | undefined,
	socOverridePercent: number | null | undefined,
	// Other
	otherCosts: number | null | undefined,
	otherCostsNote: string | null | undefined,
	dryRun: boolean
): Promise<CascadeResult> {
	return await apiCall('create_trip_cascade', {
		vehicleId,
		startDatetime,
		endDatetime,
		origin,
		destination,
		distanceKm,
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
		dryRun
	});
}

/**
 * Remove a trip and close the gap it leaves in the odometer chain.
 * With `dryRun: true` nothing is deleted and the returned plan is what fills
 * the confirmation modal.
 */
export async function deleteTripCascade(id: string, dryRun: boolean): Promise<CascadePlan> {
	return await apiCall('delete_trip_cascade', { id, dryRun });
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

export async function getPaperlessInvoices(vehicleId: string, year: number): Promise<PaperlessInvoiceRow[]> {
	return apiCall<PaperlessInvoiceRow[]>('get_paperless_invoices', { vehicleId, year });
}

export async function countUnlinkedPaperlessFuelInvoices(
	vehicleId: string,
	year: number,
): Promise<number> {
	return apiCall<number>('count_unlinked_paperless_fuel_invoices', { vehicleId, year });
}

export async function getTripsForPaperlessAssignment(
	docId: number,
	vehicleId: string,
	year: number,
): Promise<TripForAssignment[]> {
	return await apiCall('get_trips_for_paperless_assignment', { docId, vehicleId, year });
}

export async function assignPaperlessInvoice(
	docId: number,
	tripId: string,
	vehicleId: string,
	assignmentType: 'Fuel' | 'Other',
	mismatchOverride: boolean = false,
): Promise<void> {
	return await apiCall('assign_paperless_invoice', {
		docId, tripId, vehicleId, assignmentType, mismatchOverride,
	});
}

export async function unassignPaperlessInvoice(docId: number): Promise<void> {
	return await apiCall('unassign_paperless_invoice', { docId });
}

export async function revertPaperlessOverride(docId: number): Promise<void> {
	return await apiCall('revert_paperless_override', { docId });
}

// Route map commands (Task 70)
export async function generateRoute(targetKm: number): Promise<GeneratedRoute> {
	return await apiCall('generate_route', { targetKm });
}

/**
 * Open a trip's route map: the backend decides loop vs direct and resolves
 * both endpoints against the place book in ONE round trip. Call this instead
 * of comparing origin to destination here (ADR-008).
 */
export async function startRouteForTrip(tripId: string): Promise<RouteStart> {
	return await apiCall('start_route_for_trip', { tripId });
}

/**
 * Route an ordered waypoint list, offering alternatives where the routing
 * service can produce them. Pass `insert` to have the backend slot a
 * dragged-in point into the list first -- the returned routes' `waypoints`
 * are authoritative and should be adopted as-is.
 *
 * `roundTrip` asks the backend to append a return leg back to the route's own
 * first waypoint (Task 19) -- marshalling only, the append itself happens in
 * Rust (`route_direct_internal`).
 *
 * Returns alternatives in the routing service's own order (fastest first).
 * Never re-sort them -- that ordering is the product decision.
 */
export async function routeDirect(
	waypoints: Waypoint[],
	targetKm: number,
	insert?: InsertPoint,
	roundTrip?: boolean
): Promise<GeneratedRoute[]> {
	return await apiCall('route_direct', {
		waypoints,
		targetKm,
		insert: insert ?? null,
		roundTrip: roundTrip ?? false
	});
}

export async function getTripRoute(tripId: string): Promise<RouteMap | null> {
	return await apiCall('get_trip_route', { tripId });
}

/**
 * Route a round trip as TWO requests, one per leg, so each leg gets its own
 * alternatives and the way home can differ from the way out.
 *
 * Send `inbound: []` on the first call after the checkbox is ticked -- the
 * backend derives the return leg from the outbound one. On every later call,
 * send the lists the previous response returned: they are authoritative, and
 * the backend re-joins their ends anyway.
 *
 * Alternatives come back in the routing service's own order (fastest first).
 * Never re-sort them (ADR-038).
 */
export async function routeRoundTrip(
	outbound: Waypoint[],
	inbound: Waypoint[],
	targetKm: number,
	insert?: LegInsertPoint
): Promise<RoundTripRoutes> {
	return await apiCall('route_round_trip', {
		outbound,
		inbound,
		targetKm,
		insert: insert ?? null
	});
}

// coordinates, datasetVersion and durationS are intentionally not sent -- the
// backend re-derives coordinates and datasetVersion (polyline decode +
// bundled dataset version) and never persists durationS at all. Adding them
// here would be silently ignored: serde drops unknown fields by default.
//
// roundTrip is a separate parameter, not a field on GeneratedRoute: the
// backend's GeneratedRoute carries no such field (it describes geometry, not
// the request that produced it), so the checkbox state the caller is
// currently showing is the only source of truth for what to persist.
export async function saveTripRoute(
	tripId: string,
	route: GeneratedRoute,
	roundTrip: boolean
): Promise<void> {
	return await apiCall('save_trip_route', {
		tripId,
		waypoints: route.waypoints,
		polyline: route.polyline,
		targetKm: route.targetKm,
		roadKm: route.roadKm,
		mode: route.mode,
		roundTrip,
	});
}

/**
 * Persist the chosen pair of legs as one route.
 *
 * No waypoint list, no polyline, no combined distance: the backend joins the
 * legs, concatenates the geometry and sums the distances itself (ADR-008).
 * Every value sent here is one the routing response produced.
 */
export async function saveTripRoundTripRoute(
	tripId: string,
	outboundWaypoints: Waypoint[],
	inboundWaypoints: Waypoint[],
	outboundPolyline: string,
	inboundPolyline: string,
	outboundRoadKm: number,
	inboundRoadKm: number,
	targetKm: number
): Promise<void> {
	return await apiCall('save_trip_round_trip_route', {
		tripId,
		outboundWaypoints,
		inboundWaypoints,
		outboundPolyline,
		inboundPolyline,
		outboundRoadKm,
		inboundRoadKm,
		targetKm
	});
}

export async function deleteTripRoute(tripId: string): Promise<void> {
	return await apiCall('delete_trip_route', { tripId });
}

// Place book commands (Task 75)
export async function listPlaces(): Promise<Place[]> {
	return await apiCall('list_places');
}

export async function geocodePlace(query: string): Promise<GeocodeCandidate[]> {
	return await apiCall('geocode_place', { query });
}

export async function savePlace(
	displayName: string,
	lat: number,
	lon: number,
	source: PlaceSource
): Promise<void> {
	return await apiCall('save_place', { displayName, lat, lon, source });
}

export async function clearPlace(displayName: string): Promise<void> {
	return await apiCall('clear_place', { displayName });
}
