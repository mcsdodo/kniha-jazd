<script lang="ts">
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import TripGrid from '$lib/components/TripGrid.svelte';
	import CompensationBanner from '$lib/components/CompensationBanner.svelte';
	import { getTripsForYear, calculateTripStats, exportPdf } from '$lib/api';
	import type { Trip, TripStats } from '$lib/types';
	import { onMount } from 'svelte';

	let exporting = false;

	let trips: Trip[] = [];
	let initialLoading = true; // Only true for first load, keeps TripGrid mounted during refreshes
	let stats: TripStats | null = null;

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
			const saved = await exportPdf(
				$activeVehicleStore.id,
				$selectedYearStore,
				$activeVehicleStore.license_plate
			);
			if (saved) {
				// PDF saved successfully
			}
		} catch (error) {
			console.error('Export failed:', error);
			alert('Export zlyhal: ' + error);
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
					<h2>Aktívne vozidlo</h2>
					<button class="export-btn" onclick={handleExport} disabled={exporting || trips.length === 0}>
						{exporting ? 'Exportujem...' : 'Export PDF'}
					</button>
				</div>
				{#if stats}
					<div class="stats-container">
						<div class="stats-row">
							<span class="stat">
								<span class="stat-label">Celkovo najazdené:</span>
								<span class="stat-value">{stats.total_km.toLocaleString('sk-SK')} km</span>
							</span>
							<span class="stat">
								<span class="stat-label">PHM:</span>
								<span class="stat-value">{stats.total_fuel_liters.toFixed(1)} L / {stats.total_fuel_cost_eur.toFixed(2)} €</span>
							</span>
						</div>
						<div class="stats-row">
							<span class="stat">
								<span class="stat-label">Spotreba:</span>
								<span class="stat-value">{stats.avg_consumption_rate.toFixed(2)} L/100km</span>
							</span>
							{#if stats.margin_percent !== null}
								<span class="stat" class:warning={stats.is_over_limit}>
									<span class="stat-label">Odchýlka:</span>
									<span class="stat-value">{stats.margin_percent.toFixed(1)}%</span>
								</span>
							{/if}
							<span class="stat">
								<span class="stat-label">Zostatok:</span>
								<span class="stat-value">{stats.zostatok_liters.toFixed(1)} L</span>
							</span>
						</div>
					</div>
				{/if}
			</div>
			<div class="info-grid">
				<div class="info-item">
					<span class="label">Názov:</span>
					<span class="value">{$activeVehicleStore.name}</span>
				</div>
				<div class="info-item">
					<span class="label">ŠPZ:</span>
					<span class="value">{$activeVehicleStore.license_plate}</span>
				</div>
				<div class="info-item">
					<span class="label">Objem nádrže:</span>
					<span class="value">{$activeVehicleStore.tank_size_liters} L</span>
				</div>
				<div class="info-item">
					<span class="label">Spotreba (TP):</span>
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
				<p class="loading">Načítavam...</p>
			{:else}
				<TripGrid
					vehicleId={$activeVehicleStore.id}
					{trips}
					year={$selectedYearStore}
					tankSize={$activeVehicleStore.tank_size_liters}
					tpConsumption={$activeVehicleStore.tp_consumption}
					initialOdometer={$activeVehicleStore.initial_odometer}
					onTripsChanged={handleTripsChanged}
				/>
			{/if}
		</div>
	{:else}
		<div class="no-vehicle">
			<h2>Žiadne vozidlo</h2>
			<p>Prosím, vyberte vozidlo z hlavného menu alebo ho vytvorte v nastaveniach.</p>
			<a href="/settings" class="button">Prejsť do nastavení</a>
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
