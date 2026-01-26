<script lang="ts">
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import TripGrid from '$lib/components/TripGrid.svelte';
	import CompensationBanner from '$lib/components/CompensationBanner.svelte';
	import { getTripsForYear, calculateTripStats, openExportPreview } from '$lib/api';
	import type { Trip, TripStats, ExportLabels } from '$lib/types';
	import { onMount } from 'svelte';
	import { toast } from '$lib/stores/toast';
	import LL, { locale } from '$lib/i18n/i18n-svelte';

	let exporting = false;

	let trips: Trip[] = [];
	let initialLoading = true; // Only true for first load, keeps TripGrid mounted during refreshes
	let stats: TripStats | null = null;

	// Sort state from TripGrid (for export)
	let sortColumn: 'manual' | 'date' = 'manual';
	let sortDirection: 'asc' | 'desc' = 'asc';

	// For compensation warning
	let bufferKm = 0.0;

	onMount(async () => {
		await loadTrips(true);
	});

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
				col_date: $LL.export.colDate(),
				col_time: $LL.export.colTime(),
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
				labels
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
				{#if stats}
					<div class="stats-container">
						<div class="stats-row">
							<span class="stat">
								<span class="stat-label">{$LL.stats.totalDriven()}:</span>
								<span class="stat-value">{stats.totalKm.toLocaleString('sk-SK')} km</span>
							</span>
							<span class="stat">
								<span class="stat-label">{$LL.stats.fuel()}:</span>
								<span class="stat-value">{stats.totalFuelLiters.toFixed(1)} L / {stats.totalFuelCostEur.toFixed(2)} €</span>
							</span>
						</div>
						<div class="stats-row">
							<span class="stat">
								<span class="stat-label">{$LL.stats.consumption()}:</span>
								<span class="stat-value">{stats.avgConsumptionRate.toFixed(2)} L/100km</span>
							</span>
							{#if stats.marginPercent !== null}
								<span class="stat" class:warning={stats.isOverLimit}>
									<span class="stat-label">{$LL.stats.deviation()}:</span>
									<span class="stat-value">{stats.marginPercent.toFixed(1)}%</span>
								</span>
							{/if}
							<span class="stat">
								<span class="stat-label">{$LL.stats.remaining()}:</span>
								<span class="stat-value">{stats.fuelRemainingLiters.toFixed(1)} L</span>
							</span>
						</div>
					</div>
				{/if}
			</div>
			<div class="info-grid">
				<div class="info-item">
					<span class="label">{$LL.vehicle.name()}:</span>
					<span class="value">{$activeVehicleStore.name}</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.licensePlate()}:</span>
					<span class="value">{$activeVehicleStore.licensePlate}</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.tankSize()}:</span>
					<span class="value">{$activeVehicleStore.tankSizeLiters} L</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.tpConsumption()}:</span>
					<span class="value">{$activeVehicleStore.tpConsumption} L/100km</span>
				</div>
				{#if $activeVehicleStore.vin}
				<div class="info-item">
					<span class="label">VIN:</span>
					<span class="value">{$activeVehicleStore.vin}</span>
				</div>
				{/if}
				{#if $activeVehicleStore.driverName}
				<div class="info-item">
					<span class="label">{$LL.vehicleModal.driverLabel()}:</span>
					<span class="value">{$activeVehicleStore.driverName}</span>
				</div>
				{/if}
			</div>
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

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}

	.info-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
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
