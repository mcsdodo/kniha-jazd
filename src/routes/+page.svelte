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

	// For compensation suggestion
	let bufferKm = 0.0;
	let currentLocation = '';

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
			if (stats.is_over_limit && stats.margin_percent !== null) {
				// Calculate buffer km needed to get to 18% target
				const targetMargin = 0.18;
				const actualRate = stats.last_consumption_rate;
				const tpRate = $activeVehicleStore.tp_consumption;

				// Find last trip location
				if (trips.length > 0) {
					const lastTrip = trips[trips.length - 1];
					currentLocation = lastTrip.destination;
				}

				// Simple buffer calculation - we need to dilute the consumption
				// This is a simplified version, the real calculation would need last fill-up data
				bufferKm = 100; // Placeholder - should be calculated properly
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
				col_date: $LL.export.colDate(),
				col_origin: $LL.export.colOrigin(),
				col_destination: $LL.export.colDestination(),
				col_purpose: $LL.export.colPurpose(),
				col_km: $LL.export.colKm(),
				col_odo: $LL.export.colOdo(),
				col_fuel_liters: $LL.export.colFuelLiters(),
				col_fuel_cost: $LL.export.colFuelCost(),
				col_other_costs: $LL.export.colOtherCosts(),
				col_note: $LL.export.colNote(),
				col_remaining: $LL.export.colRemaining(),
				col_consumption: $LL.export.colConsumption(),
				footer_total_km: $LL.export.footerTotalKm(),
				footer_total_fuel: $LL.export.footerTotalFuel(),
				footer_other_costs: $LL.export.footerOtherCosts(),
				footer_avg_consumption: $LL.export.footerAvgConsumption(),
				footer_deviation: $LL.export.footerDeviation(),
				footer_tp_norm: $LL.export.footerTpNorm(),
				print_hint: $LL.export.printHint()
			};

			await openExportPreview(
				$activeVehicleStore.id,
				$selectedYearStore,
				$activeVehicleStore.license_plate,
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
								<span class="stat-value">{stats.total_km.toLocaleString('sk-SK')} km</span>
							</span>
							<span class="stat">
								<span class="stat-label">{$LL.stats.fuel()}:</span>
								<span class="stat-value">{stats.total_fuel_liters.toFixed(1)} L / {stats.total_fuel_cost_eur.toFixed(2)} €</span>
							</span>
						</div>
						<div class="stats-row">
							<span class="stat">
								<span class="stat-label">{$LL.stats.consumption()}:</span>
								<span class="stat-value">{stats.avg_consumption_rate.toFixed(2)} L/100km</span>
							</span>
							{#if stats.margin_percent !== null}
								<span class="stat" class:warning={stats.is_over_limit}>
									<span class="stat-label">{$LL.stats.deviation()}:</span>
									<span class="stat-value">{stats.margin_percent.toFixed(1)}%</span>
								</span>
							{/if}
							<span class="stat">
								<span class="stat-label">{$LL.stats.remaining()}:</span>
								<span class="stat-value">{stats.fuel_remaining_liters.toFixed(1)} L</span>
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
					<span class="value">{$activeVehicleStore.license_plate}</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.tankSize()}:</span>
					<span class="value">{$activeVehicleStore.tank_size_liters} L</span>
				</div>
				<div class="info-item">
					<span class="label">{$LL.vehicle.tpConsumption()}:</span>
					<span class="value">{$activeVehicleStore.tp_consumption} L/100km</span>
				</div>
			</div>
		</div>

		{#if stats?.is_over_limit && stats.margin_percent !== null}
			<CompensationBanner
				vehicleId={$activeVehicleStore.id}
				marginPercent={stats.margin_percent}
				{bufferKm}
				{currentLocation}
				onTripAdded={handleTripsChanged}
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
					tankSize={$activeVehicleStore.tank_size_liters}
					tpConsumption={$activeVehicleStore.tp_consumption}
					initialOdometer={$activeVehicleStore.initial_odometer}
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
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
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
		background-color: #27ae60;
		color: white;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.export-btn:hover:not(:disabled) {
		background-color: #219653;
	}

	.export-btn:disabled {
		background-color: #bdc3c7;
		cursor: not-allowed;
	}

	.vehicle-info h2 {
		margin: 0;
		font-size: 1.25rem;
		color: #2c3e50;
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
		color: #7f8c8d;
	}

	.stat-value {
		font-weight: 600;
		color: #2c3e50;
	}

	.stat.warning .stat-value {
		color: #d39e00;
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
		color: #7f8c8d;
		font-weight: 500;
	}

	.value {
		font-size: 1rem;
		color: #2c3e50;
		font-weight: 600;
	}

	.trip-section {
		/* TripGrid has its own styling */
	}

	.loading {
		text-align: center;
		padding: 2rem;
		color: #7f8c8d;
		font-style: italic;
	}

	.no-vehicle {
		background: white;
		padding: 2rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		text-align: center;
	}

	.no-vehicle h2 {
		margin: 0 0 1rem 0;
		color: #2c3e50;
	}

	.no-vehicle p {
		color: #7f8c8d;
		margin-bottom: 1.5rem;
	}

	.button {
		display: inline-block;
		padding: 0.75rem 1.5rem;
		background-color: #3498db;
		color: white;
		text-decoration: none;
		border-radius: 4px;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.button:hover {
		background-color: #2980b9;
	}
</style>
