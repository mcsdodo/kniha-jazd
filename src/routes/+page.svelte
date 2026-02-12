<script lang="ts">
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import TripGrid from '$lib/components/TripGrid.svelte';
	import CompensationBanner from '$lib/components/CompensationBanner.svelte';
	import { getTripsForYear, calculateTripStats, openExportPreview, testHaConnection, getHiddenColumns } from '$lib/api';
	import type { Trip, TripStats, ExportLabels } from '$lib/types';
	import { onMount, onDestroy } from 'svelte';
	import { toast } from '$lib/stores/toast';
	import LL, { locale } from '$lib/i18n/i18n-svelte';
	import { haStore } from '$lib/stores/homeAssistant';

	let exporting = false;

	// Home Assistant state
	let haConnected = false;

	let trips: Trip[] = [];
	let initialLoading = true; // Only true for first load, keeps TripGrid mounted during refreshes
	let stats: TripStats | null = null;

	// Sort state from TripGrid (for export)
	let sortColumn: 'manual' | 'tripNumber' = 'tripNumber';
	let sortDirection: 'asc' | 'desc' = 'desc'; // desc = newest first

	// For compensation warning
	let bufferKm = 0.0;

	onMount(async () => {
		await loadTrips(true);

		// Check HA connection (Rust backend handles credentials)
		try {
			haConnected = await testHaConnection();
		} catch (e) {
			console.warn('Failed to test HA connection:', e);
			haConnected = false;
		}
	});

	onDestroy(() => {
		haStore.stopPeriodicRefresh();
	});

	// Start/stop HA refresh when vehicle changes (Rust backend handles credentials)
	$: if ($activeVehicleStore?.haOdoSensor && haConnected) {
		haStore.startPeriodicRefresh($activeVehicleStore.id, $activeVehicleStore.haOdoSensor, $activeVehicleStore.haFuelLevelSensor ?? undefined);
	} else {
		haStore.stopPeriodicRefresh();
	}

	// Calculate delta from real ODO vs highest logged ODO
	$: haOdoCache = $activeVehicleStore ? $haStore.cache.get($activeVehicleStore.id) : null;
	$: maxLoggedOdo = trips.length > 0 ? Math.max(...trips.map(t => t.odometer)) : null;
	$: haOdoDelta = haOdoCache && maxLoggedOdo !== null ? haOdoCache.value - maxLoggedOdo : null;
	$: haOdoWarning = haOdoDelta !== null && haOdoDelta >= 50;

	// HA fuel level: convert percentage to liters (ADR-013: display formatting in frontend)
	$: haFuelPercent = haOdoCache?.fuelLevelPercent ?? null;
	$: haFuelLiters = haFuelPercent !== null && $activeVehicleStore?.tankSizeLiters
		? (haFuelPercent * $activeVehicleStore.tankSizeLiters / 100)
		: null;

	// Format staleness (time since fetch)
	function formatStaleness(fetchedAt: number): string {
		const minutes = Math.floor((Date.now() - fetchedAt) / 60000);
		if (minutes < 60) {
			return `${minutes}m`;
		}
		const hours = Math.floor(minutes / 60);
		if (hours < 24) {
			return `${hours}h`;
		}
		return '1d+';
	}

	async function loadTrips(isInitial = false) {
		if (!$activeVehicleStore) {
			initialLoading = false;
			return;
		}

		try {
			// Only show loading placeholder on initial load, not refreshes
			// This prevents TripGrid from unmounting and losing scroll position
			if (isInitial) {
				initialLoading = true;
			}
			trips = await getTripsForYear($activeVehicleStore.id, $selectedYearStore);
			stats = await calculateTripStats($activeVehicleStore.id, $selectedYearStore);

			// Calculate buffer km if over limit
			if (stats.isOverLimit && stats.marginPercent !== null) {
				bufferKm = stats.bufferKm;
			}
		} catch (error) {
			console.error('Failed to load trips:', error);
		} finally {
			initialLoading = false;
		}
	}

	async function handleTripsChanged() {
		// Don't pass isInitial=true to keep TripGrid mounted (preserves scroll position)
		await loadTrips(false);
	}

	// Reload trips when active vehicle or selected year changes
	$: if ($activeVehicleStore && $selectedYearStore) {
		loadTrips(true);
	}

	async function handleExport() {
		if (!$activeVehicleStore || exporting) return;

		try {
			exporting = true;

			// Fetch latest hidden columns (may have changed since page load)
			const currentHiddenColumns = await getHiddenColumns();

			// Build export labels from translations
			const labels: ExportLabels = {
				lang: $locale,
				page_title: $LL.export.pageTitle(),
				header_company: $LL.export.headerCompany(),
				header_ico: $LL.export.headerIco(),
				header_vehicle: $LL.export.headerVehicle(),
				header_license_plate: $LL.export.headerLicensePlate(),
				header_tank_size: $LL.export.headerTankSize(),
				header_tp_consumption: $LL.export.headerTpConsumption(),
				header_year: $LL.export.headerYear(),
				// BEV header labels
				header_battery_capacity: $LL.export.headerBatteryCapacity(),
				header_baseline_consumption: $LL.export.headerBaselineConsumption(),
				// VIN and Driver
				header_vin: $LL.export.headerVin(),
				header_driver: $LL.export.headerDriver(),
				// Legal compliance columns (2026)
				col_trip_number: $LL.export.colTripNumber(),
				col_start_datetime: $LL.export.colStartDatetime(),
				col_end_datetime: $LL.export.colEndDatetime(),
				col_time: $LL.export.colTime(),
				col_driver: $LL.export.colDriver(),
				col_odo_start: $LL.export.colOdoStart(),
				col_origin: $LL.export.colOrigin(),
				col_destination: $LL.export.colDestination(),
				col_purpose: $LL.export.colPurpose(),
				col_km: $LL.export.colKm(),
				col_odo: $LL.export.colOdo(),
				col_fuel_liters: $LL.export.colFuelLiters(),
				col_fuel_cost: $LL.export.colFuelCost(),
				col_fuel_consumed: $LL.export.colFuelConsumed(),
				col_other_costs: $LL.export.colOtherCosts(),
				col_note: $LL.export.colNote(),
				col_remaining: $LL.export.colRemaining(),
				col_consumption: $LL.export.colConsumption(),
				// BEV column labels
				col_energy_kwh: $LL.export.colEnergyKwh(),
				col_energy_cost: $LL.export.colEnergyCost(),
				col_battery_remaining: $LL.export.colBatteryRemaining(),
				col_energy_rate: $LL.export.colEnergyRate(),
				footer_total_km: $LL.export.footerTotalKm(),
				footer_total_fuel: $LL.export.footerTotalFuel(),
				footer_other_costs: $LL.export.footerOtherCosts(),
				footer_avg_consumption: $LL.export.footerAvgConsumption(),
				footer_deviation: $LL.export.footerDeviation(),
				footer_tp_norm: $LL.export.footerTpNorm(),
				// BEV footer labels
				footer_total_energy: $LL.export.footerTotalEnergy(),
				footer_avg_energy_rate: $LL.export.footerAvgEnergyRate(),
				footer_baseline_norm: $LL.export.footerBaselineNorm(),
				print_hint: $LL.export.printHint()
			};

			await openExportPreview(
				$activeVehicleStore.id,
				$selectedYearStore,
				$activeVehicleStore.licensePlate,
				sortColumn,
				sortDirection,
				labels,
				currentHiddenColumns
			);
		} catch (error) {
			console.error('Export failed:', error);
			toast.error($LL.toast.errorExport({ error: String(error) }));
		} finally {
			exporting = false;
		}
	}
</script>

<div class="main-page">
	{#if $activeVehicleStore}
		<div class="vehicle-info">
			<div class="vehicle-header">
				<div class="header-row">
					<h2>{$LL.home.activeVehicle()}</h2>
					<button class="export-btn" onclick={handleExport} disabled={exporting || trips.length === 0}>
						{exporting ? $LL.home.exporting() : $LL.home.exportForPrint()}
					</button>
				</div>
			</div>
			<!-- Row 1: Static vehicle info -->
			<div class="info-grid">
				<div class="info-item">
					<span class="label">{$LL.vehicle.name()}</span>
					<span class="value">{$activeVehicleStore.name}</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.licensePlate()}</span>
					<span class="value">{$activeVehicleStore.licensePlate}</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.tankSize()}</span>
					<span class="value">{$activeVehicleStore.tankSizeLiters} L</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.tpConsumption()}</span>
					<span class="value">{$activeVehicleStore.tpConsumption} L/100km</span>
				</div>
				{#if $activeVehicleStore.vin}
				<div class="info-item">
					<span class="label">VIN</span>
					<span class="value">{$activeVehicleStore.vin}</span>
				</div>
				{/if}
				{#if $activeVehicleStore.driverName}
				<div class="info-item">
					<span class="label">{$LL.vehicleModal.driverLabel()}</span>
					<span class="value">{$activeVehicleStore.driverName}</span>
				</div>
				{/if}
			</div>
			<!-- Row 2: Dynamic stats -->
			{#if stats}
				<div class="info-grid stats-grid">
					<div class="info-item">
						<span class="label">{$LL.stats.totalDriven()}</span>
						<span class="value">{stats.totalKm.toLocaleString('sk-SK')} km</span>
					</div>
					<div class="info-item">
						<span class="label">{$LL.stats.fuel()}</span>
						<span class="value">{stats.totalFuelLiters.toFixed(1)} L / {stats.totalFuelCostEur.toFixed(2)} €</span>
					</div>
					<div class="info-item">
						<span class="label">{$LL.stats.remaining()}</span>
						<span class="value">
							{stats.fuelRemainingLiters.toFixed(1)} L
							{#if haFuelLiters !== null}
								<span class="ha-fuel" title={$LL.homeAssistant.realFuelTooltip()}>({haFuelLiters.toFixed(1)} L)</span>
							{:else if $haStore.fuelError && $activeVehicleStore?.haFuelLevelSensor}
								<span class="ha-fuel-error" title={$LL.homeAssistant.realFuelTooltip()}>({$LL.homeAssistant.fetchError()})</span>
							{/if}
						</span>
					</div>
					<div class="info-item">
						<span class="label">{$LL.stats.consumption()}</span>
						<span class="value">{stats.avgConsumptionRate.toFixed(2)} L/100km</span>
					</div>
					{#if stats.marginPercent !== null}
						<div class="info-item" class:warning={stats.isOverLimit}>
							<span class="label">{$LL.stats.deviation()}</span>
							<span class="value">{stats.marginPercent.toFixed(1)}%</span>
						</div>
					{/if}
					{#if haOdoCache}
						<div class="info-item ha-odo" class:warning={haOdoWarning} title={$LL.homeAssistant.realOdoTooltip()}>
							<span class="label">{$LL.homeAssistant.realOdo()}</span>
							<span class="value">
								{haOdoCache.value.toLocaleString('sk-SK')} km
								{#if haOdoDelta !== null}
									<span class="delta" class:warning={haOdoWarning}>
										(+{haOdoDelta.toFixed(0)} km)
									</span>
								{/if}
							</span>
						</div>
					{:else if $haStore.odoError && $activeVehicleStore?.haOdoSensor}
						<div class="info-item ha-error">
							<span class="label">{$LL.homeAssistant.realOdo()}</span>
							<span class="value error">{$LL.homeAssistant.fetchError()}</span>
						</div>
					{/if}
				</div>
			{/if}
		</div>

		{#if stats?.isOverLimit && stats.marginPercent !== null}
			<CompensationBanner
				marginPercent={stats.marginPercent}
				{bufferKm}
			/>
		{/if}

		<div class="trip-section">
			{#if initialLoading}
				<p class="loading">{$LL.common.loading()}</p>
			{:else}
				<TripGrid
					vehicleId={$activeVehicleStore.id}
					{trips}
					year={$selectedYearStore}
					tankSize={$activeVehicleStore.tankSizeLiters ?? 0}
					tpConsumption={$activeVehicleStore.tpConsumption ?? 0}
					initialOdometer={$activeVehicleStore.initialOdometer}
					vehicleType={$activeVehicleStore.vehicleType}
					batteryCapacityKwh={$activeVehicleStore.batteryCapacityKwh ?? 0}
					baselineConsumptionKwh={$activeVehicleStore.baselineConsumptionKwh ?? 0}
					driverName={$activeVehicleStore.driverName ?? ''}
					onTripsChanged={handleTripsChanged}
					bind:sortColumn
					bind:sortDirection
				/>
			{/if}
		</div>
	{:else}
		<div class="no-vehicle">
			<h2>{$LL.home.noVehicle()}</h2>
			<p>{$LL.home.noVehicleDescription()}</p>
			<a href="/settings" class="button">{$LL.home.goToSettings()}</a>
		</div>
	{/if}
</div>

<style>
	.main-page {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.vehicle-info {
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px var(--shadow-default);
	}

	.vehicle-header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.header-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.export-btn {
		padding: 0.5rem 1rem;
		background-color: var(--btn-active-success-bg);
		color: var(--btn-active-success-color);
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.export-btn:hover:not(:disabled) {
		background-color: var(--btn-active-success-hover);
	}

	.export-btn:disabled {
		background-color: var(--text-muted);
		cursor: not-allowed;
	}

	.vehicle-info h2 {
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.stats-container {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.stats-row {
		display: flex;
		gap: 1.5rem;
		align-items: center;
		font-size: 0.875rem;
	}

	.stat {
		display: flex;
		gap: 0.25rem;
	}

	.stat-label {
		color: var(--text-secondary);
	}

	.stat-value {
		font-weight: 600;
		color: var(--text-primary);
	}

	.stat.warning .stat-value {
		color: var(--accent-warning);
	}

	/* Home Assistant fuel level inline display */
	.ha-fuel {
		color: var(--accent-warning);
		font-size: 0.85em;
		margin-left: 0.25rem;
	}

	.ha-fuel-error {
		color: var(--color-error, #dc2626);
		font-size: 0.85em;
		font-style: italic;
		margin-left: 0.25rem;
	}

	/* Home Assistant ODO display */
	.ha-odo-row {
		margin-top: 0.25rem;
		padding-top: 0.25rem;
		border-top: 1px dashed var(--border-default);
	}

	.ha-odo .delta {
		font-size: 0.8em;
		color: var(--text-secondary);
		margin-left: 0.25rem;
	}

	.ha-odo .delta.warning {
		color: var(--accent-warning);
		font-weight: 600;
	}

	.ha-odo .staleness {
		font-size: 0.75em;
		color: var(--text-muted);
		margin-left: 0.5rem;
	}

	.ha-loading {
		font-size: 0.8em;
		color: var(--text-muted);
		font-style: italic;
	}

	.ha-error .error {
		color: var(--color-error, #dc2626);
		font-style: italic;
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		padding: 0.75rem 0;
		border-bottom: 1px solid var(--border-default);
	}

	.info-grid.stats-grid {
		border-bottom: none;
		padding-bottom: 0;
	}

	.info-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.info-item.warning .value {
		color: var(--accent-warning);
	}

	.info-item.ha-odo .delta {
		font-size: 0.85em;
		color: var(--text-secondary);
	}

	.info-item.ha-odo .delta.warning {
		color: var(--accent-warning);
	}

	.info-item.ha-error .value {
		color: var(--color-error, #dc2626);
	}

	.label {
		font-size: 0.875rem;
		color: var(--text-secondary);
		font-weight: 500;
	}

	.value {
		font-size: 1rem;
		color: var(--text-primary);
		font-weight: 600;
	}

	.loading {
		text-align: center;
		padding: 2rem;
		color: var(--text-secondary);
		font-style: italic;
	}

	.no-vehicle {
		background: var(--bg-surface);
		padding: 2rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px var(--shadow-default);
		text-align: center;
	}

	.no-vehicle h2 {
		margin: 0 0 1rem 0;
		color: var(--text-primary);
	}

	.no-vehicle p {
		color: var(--text-secondary);
		margin-bottom: 1.5rem;
	}

	.button {
		display: inline-block;
		padding: 0.75rem 1.5rem;
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		text-decoration: none;
		border-radius: 4px;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.button:hover {
		background-color: var(--btn-active-primary-hover);
	}
</style>
