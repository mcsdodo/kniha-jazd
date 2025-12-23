<script lang="ts">
	import type { Trip, Route } from '$lib/types';
	import { createTrip, updateTrip, deleteTrip, getRoutes } from '$lib/api';
	import TripRow from './TripRow.svelte';
	import { onMount } from 'svelte';

	export let vehicleId: string;
	export let trips: Trip[] = [];
	export let onTripsChanged: () => void;

	let routes: Route[] = [];
	let showNewRow = false;

	onMount(async () => {
		await loadRoutes();
	});

	async function loadRoutes() {
		try {
			routes = await getRoutes(vehicleId);
		} catch (error) {
			console.error('Failed to load routes:', error);
		}
	}

	function handleNewRecord() {
		showNewRow = true;
	}

	async function handleSaveNew(tripData: Partial<Trip>) {
		try {
			await createTrip(
				vehicleId,
				tripData.date!,
				tripData.origin!,
				tripData.destination!,
				tripData.distance_km!,
				tripData.odometer!,
				tripData.purpose!,
				tripData.fuel_liters,
				tripData.fuel_cost_eur,
				tripData.other_costs_eur,
				null
			);
			showNewRow = false;
			onTripsChanged();
			await loadRoutes(); // Refresh routes after adding trip
		} catch (error) {
			console.error('Failed to create trip:', error);
			alert('Nepodarilo sa vytvoriť záznam');
		}
	}

	async function handleUpdate(trip: Trip, tripData: Partial<Trip>) {
		try {
			await updateTrip(
				trip.id,
				tripData.date!,
				tripData.origin!,
				tripData.destination!,
				tripData.distance_km!,
				tripData.odometer!,
				tripData.purpose!,
				tripData.fuel_liters,
				tripData.fuel_cost_eur,
				tripData.other_costs_eur,
				null
			);
			onTripsChanged();
			await loadRoutes(); // Refresh routes after updating trip
		} catch (error) {
			console.error('Failed to update trip:', error);
			alert('Nepodarilo sa aktualizovať záznam');
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteTrip(id);
			onTripsChanged();
		} catch (error) {
			console.error('Failed to delete trip:', error);
			alert('Nepodarilo sa odstrániť záznam');
		}
	}

	function handleCancelNew() {
		showNewRow = false;
	}

	// Sort trips by date descending (newest first)
	$: sortedTrips = [...trips].sort(
		(a, b) => new Date(b.date).getTime() - new Date(a.date).getTime()
	);

	// Get the last ODO value (from the most recent trip)
	$: lastOdometer = sortedTrips.length > 0 ? sortedTrips[0].odometer : 0;
</script>

<div class="trip-grid">
	<div class="header">
		<h2>Jazdy ({trips.length})</h2>
		<button class="new-record" on:click={handleNewRecord} disabled={showNewRow}>
			Nový záznam
		</button>
	</div>

	<div class="table-container">
		<table>
			<thead>
				<tr>
					<th>Dátum</th>
					<th>Odkiaľ</th>
					<th>Kam</th>
					<th>Km</th>
					<th>ODO</th>
					<th>Účel</th>
					<th>PHM (L)</th>
					<th>Cena €</th>
					<th>Iné €</th>
					<th>Akcie</th>
				</tr>
			</thead>
			<tbody>
				{#if showNewRow}
					<TripRow
						trip={null}
						{routes}
						isNew={true}
						previousOdometer={lastOdometer}
						onSave={handleSaveNew}
						onCancel={handleCancelNew}
						onDelete={() => {}}
					/>
				{/if}
				{#each sortedTrips as trip, index (trip.id)}
					<TripRow
						{trip}
						{routes}
						isNew={false}
						previousOdometer={index < sortedTrips.length - 1 ? sortedTrips[index + 1].odometer : 0}
						onSave={(data) => handleUpdate(trip, data)}
						onCancel={() => {}}
						onDelete={handleDelete}
					/>
				{/each}
				{#if trips.length === 0 && !showNewRow}
					<tr class="empty">
						<td colspan="10">Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.</td>
					</tr>
				{/if}
			</tbody>
		</table>
	</div>
</div>

<style>
	.trip-grid {
		background: white;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		overflow: hidden;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e0e0e0;
	}

	.header h2 {
		margin: 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.new-record {
		padding: 0.625rem 1.25rem;
		background-color: #3498db;
		color: white;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.new-record:hover:not(:disabled) {
		background-color: #2980b9;
	}

	.new-record:disabled {
		background-color: #bdc3c7;
		cursor: not-allowed;
	}

	.table-container {
		overflow-x: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.875rem;
	}

	thead {
		background-color: #f8f9fa;
		position: sticky;
		top: 0;
	}

	th {
		padding: 0.75rem 0.5rem;
		text-align: left;
		font-weight: 600;
		color: #2c3e50;
		border-bottom: 2px solid #e0e0e0;
	}

	tbody tr.empty td {
		padding: 2rem;
		text-align: center;
		color: #7f8c8d;
		font-style: italic;
	}
</style>
