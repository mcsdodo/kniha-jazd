<script lang="ts">
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import TripGrid from '$lib/components/TripGrid.svelte';
	import CompensationBanner from '$lib/components/CompensationBanner.svelte';
	import { getTrips, calculateTripStats } from '$lib/api';
	import type { Trip, TripStats } from '$lib/types';
	import { onMount } from 'svelte';

	let trips: Trip[] = [];
	let loading = true;
	let stats: TripStats | null = null;

	// For compensation suggestion
	let bufferKm = 0.0;
	let currentLocation = '';

	onMount(async () => {
		await loadTrips();
	});

	async function loadTrips() {
		if (!$activeVehicleStore) {
			loading = false;
			return;
		}

		try {
			loading = true;
			trips = await getTrips($activeVehicleStore.id);
			stats = await calculateTripStats($activeVehicleStore.id);

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
			loading = false;
		}
	}

	async function handleTripsChanged() {
		await loadTrips();
	}

	// Reload trips when active vehicle changes
	$: if ($activeVehicleStore) {
		loadTrips();
	}
</script>

<div class="main-page">
	{#if $activeVehicleStore}
		<div class="vehicle-info">
			<div class="vehicle-header">
				<h2>Aktívne vozidlo</h2>
				{#if stats}
					<div class="stats">
						<span class="stat">Zostatok: {stats.zostatok_liters.toFixed(1)}L</span>
						<span class="stat-separator">|</span>
						<span class="stat">Spotreba: {stats.avg_consumption_rate.toFixed(2)} L/100km</span>
						{#if stats.margin_percent !== null}
							<span class="stat-separator">|</span>
							<span class="stat" class:warning={stats.is_over_limit}>
								Odchýlka: {stats.margin_percent.toFixed(1)}%
							</span>
						{/if}
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
			{#if loading}
				<p class="loading">Načítavam...</p>
			{:else}
				<TripGrid
					vehicleId={$activeVehicleStore.id}
					{trips}
					tankSize={$activeVehicleStore.tank_size_liters}
					tpConsumption={$activeVehicleStore.tp_consumption}
					initialOdometer={$activeVehicleStore.initial_odometer}
					onTripsChanged={handleTripsChanged}
				/>
			{/if}
		</div>

		<div class="actions">
			<a href="/settings" class="button">Nastavenia</a>
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
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.vehicle-info h2 {
		margin: 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.stats {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		font-size: 0.875rem;
		color: #2c3e50;
	}

	.stat {
		font-weight: 600;
	}

	.stat.warning {
		color: #d39e00;
		font-weight: 700;
	}

	.stat-separator {
		color: #bdc3c7;
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

	.actions {
		display: flex;
		gap: 1rem;
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
