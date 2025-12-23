<script lang="ts">
	import type { Trip, Route } from '$lib/types';
	import { createTrip, updateTrip, deleteTrip, getRoutes } from '$lib/api';
	import TripRow from './TripRow.svelte';
	import { onMount } from 'svelte';

	export let vehicleId: string;
	export let trips: Trip[] = [];
	export let onTripsChanged: () => void;
	export let tpConsumption: number = 5.1; // Vehicle's TP consumption rate

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
			// Update the edited trip
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

			// Cascade ODO updates to newer trips
			await recalculateNewerTripsOdo(trip.id, tripData.odometer!);

			onTripsChanged();
			await loadRoutes();
		} catch (error) {
			console.error('Failed to update trip:', error);
			alert('Nepodarilo sa aktualizovať záznam');
		}
	}

	// Recalculate ODO for all trips newer than the edited one
	async function recalculateNewerTripsOdo(editedTripId: string, newOdo: number) {
		// Sort by date ascending (oldest first) for correct ODO calculation
		const chronological = [...trips].sort(
			(a, b) => new Date(a.date).getTime() - new Date(b.date).getTime()
		);

		// Find the index of the edited trip
		const editedIndex = chronological.findIndex((t) => t.id === editedTripId);
		if (editedIndex === -1 || editedIndex === chronological.length - 1) return;

		// Update ODO for all newer trips
		let runningOdo = newOdo;
		for (let i = editedIndex + 1; i < chronological.length; i++) {
			const t = chronological[i];
			runningOdo = runningOdo + t.distance_km;

			// Only update if ODO actually changed
			if (Math.abs(t.odometer - runningOdo) > 0.01) {
				await updateTrip(
					t.id,
					t.date,
					t.origin,
					t.destination,
					t.distance_km,
					runningOdo,
					t.purpose,
					t.fuel_liters,
					t.fuel_cost_eur,
					t.other_costs_eur,
					t.other_costs_note
				);
			}
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

	// Calculate "Použitá spotreba" for each trip
	// This is the consumption rate from the last fill-up, carried forward
	$: consumptionRates = calculateConsumptionRates(trips);

	// Calculate "Zostatok" (remaining fuel) for each trip
	$: fuelRemaining = calculateFuelRemaining(trips, consumptionRates);

	export let tankSize: number = 66; // Default tank size, should be passed from vehicle

	function calculateConsumptionRates(tripList: Trip[]): Map<string, number> {
		const rates = new Map<string, number>();

		// Sort chronologically (oldest first)
		const chronological = [...tripList].sort(
			(a, b) => new Date(a.date).getTime() - new Date(b.date).getTime()
		);

		// Start with TP consumption rate (from "Prvý záznam")
		let currentRate = tpConsumption;
		let kmSinceLastFillup = 0;

		for (const trip of chronological) {
			// If this trip has a fill-up, calculate new rate
			if (trip.fuel_liters && trip.fuel_liters > 0 && kmSinceLastFillup > 0) {
				currentRate = (trip.fuel_liters / kmSinceLastFillup) * 100;
				kmSinceLastFillup = 0; // Reset for next period
			}

			// Store the rate for this trip
			rates.set(trip.id, currentRate);

			// Accumulate km for next fill-up calculation
			kmSinceLastFillup += trip.distance_km;
		}

		return rates;
	}

	function calculateFuelRemaining(tripList: Trip[], rates: Map<string, number>): Map<string, number> {
		const remaining = new Map<string, number>();

		// Sort chronologically (oldest first)
		const chronological = [...tripList].sort(
			(a, b) => new Date(a.date).getTime() - new Date(b.date).getTime()
		);

		// Start with full tank (from "Prvý záznam")
		let zostatok = tankSize;

		for (const trip of chronological) {
			const rate = rates.get(trip.id) || 0;

			// Calculate fuel used for this trip: spotreba = km * rate / 100
			const spotreba = rate > 0 ? (trip.distance_km * rate) / 100 : 0;

			// Subtract fuel used
			zostatok = zostatok - spotreba;

			// Add fuel if this was a fill-up
			if (trip.fuel_liters && trip.fuel_liters > 0) {
				zostatok = zostatok + trip.fuel_liters;
				// Cap at tank size
				if (zostatok > tankSize) {
					zostatok = tankSize;
				}
			}

			// Don't go negative
			if (zostatok < 0) zostatok = 0;

			remaining.set(trip.id, zostatok);
		}

		return remaining;
	}
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
					<th>Zostatok</th>
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
						zostatok={sortedTrips.length > 0 ? fuelRemaining.get(sortedTrips[0].id) || 0 : 0}
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
						zostatok={fuelRemaining.get(trip.id) || 0}
						onSave={(data) => handleUpdate(trip, data)}
						onCancel={() => {}}
						onDelete={handleDelete}
					/>
				{/each}
				{#if trips.length === 0 && !showNewRow}
					<tr class="empty">
						<td colspan="11">Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.</td>
					</tr>
				{/if}
				<!-- Synthetic "Prvý záznam" row - starting values -->
				<tr class="first-record">
					<td>-</td>
					<td>-</td>
					<td>-</td>
					<td>-</td>
					<td>-</td>
					<td class="purpose">Prvý záznam</td>
					<td>-</td>
					<td>-</td>
					<td class="number">{tankSize.toFixed(1)}</td>
					<td>-</td>
					<td></td>
				</tr>
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

	tbody tr.first-record {
		background-color: #f5f5f5;
		color: #7f8c8d;
		font-style: italic;
	}

	tbody tr.first-record td {
		padding: 0.5rem;
		border-bottom: 1px solid #e0e0e0;
	}

	tbody tr.first-record td.purpose {
		font-weight: 500;
		color: #2c3e50;
	}

	tbody tr.first-record td.number {
		text-align: right;
		font-style: normal;
		color: #2c3e50;
	}
</style>
