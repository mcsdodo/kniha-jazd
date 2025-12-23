<script lang="ts">
	import type { Trip, Route } from '$lib/types';
	import { createTrip, updateTrip, deleteTrip, getRoutes, reorderTrip } from '$lib/api';
	import TripRow from './TripRow.svelte';
	import { onMount } from 'svelte';

	export let vehicleId: string;
	export let trips: Trip[] = [];
	export let onTripsChanged: () => void;
	export let tpConsumption: number = 5.1; // Vehicle's TP consumption rate

	let routes: Route[] = [];
	let showNewRow = false;
	let editingTripId: string | null = null;
	let insertAtSortOrder: number | null = null;
	let insertDate: string | null = null;

	// Drag disabled when editing or adding new row
	$: dragDisabled = showNewRow || editingTripId !== null;

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
				null,
				insertAtSortOrder // Pass insert position if set
			);
			showNewRow = false;
			insertAtSortOrder = null;
			insertDate = null;
			// Recalculate ODO for all trips
			await recalculateAllOdo();
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
		// Sort by date ascending (oldest first), using odometer as tiebreaker
		const chronological = [...trips].sort((a, b) => {
			const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
			if (dateDiff !== 0) return dateDiff;
			return a.odometer - b.odometer;
		});

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
		insertAtSortOrder = null;
		insertDate = null;
	}

	function handleEditStart(tripId: string) {
		editingTripId = tripId;
	}

	function handleEditEnd() {
		editingTripId = null;
	}

	function handleInsertAbove(targetTrip: Trip) {
		insertAtSortOrder = targetTrip.sort_order;
		insertDate = targetTrip.date;
		showNewRow = true;
	}

	// Native HTML5 drag-drop state
	let draggedTripId: string | null = null;
	let dropTargetIndex: number | null = null;

	function handleDragStart(tripId: string) {
		draggedTripId = tripId;
	}

	function handleDragEnd() {
		draggedTripId = null;
		dropTargetIndex = null;
	}

	function handleDragOver(event: DragEvent, index: number) {
		event.preventDefault();
		dropTargetIndex = index;
	}

	function handleDragLeave() {
		dropTargetIndex = null;
	}

	async function handleDrop(event: DragEvent, targetIndex: number) {
		event.preventDefault();
		if (!draggedTripId) return;

		const currentIndex = sortedTrips.findIndex(t => t.id === draggedTripId);
		if (currentIndex === -1 || currentIndex === targetIndex) {
			draggedTripId = null;
			dropTargetIndex = null;
			return;
		}

		// Get new date from trip at target position
		const targetTrip = sortedTrips[targetIndex];
		const newDate = targetTrip ? targetTrip.date : sortedTrips[0]?.date || new Date().toISOString().split('T')[0];

		try {
			await reorderTrip(draggedTripId, targetIndex, newDate);
			await recalculateAllOdo();
			onTripsChanged();
		} catch (error) {
			console.error('Failed to reorder trip:', error);
			alert('Nepodarilo sa zmeniť poradie');
			onTripsChanged();
		} finally {
			draggedTripId = null;
			dropTargetIndex = null;
		}
	}

	// Recalculate ODO for all trips after reordering
	async function recalculateAllOdo() {
		// Sort by sort_order ascending, then reverse for chronological (oldest first)
		const chronological = [...trips]
			.sort((a, b) => a.sort_order - b.sort_order)
			.reverse();

		let runningOdo = initialOdometer;
		for (const trip of chronological) {
			runningOdo += trip.distance_km;
			if (Math.abs(trip.odometer - runningOdo) > 0.01) {
				await updateTrip(
					trip.id,
					trip.date,
					trip.origin,
					trip.destination,
					trip.distance_km,
					runningOdo,
					trip.purpose,
					trip.fuel_liters,
					trip.fuel_cost_eur,
					trip.other_costs_eur,
					trip.other_costs_note
				);
			}
		}
	}

	// Sort trips by sort_order ascending (0 = top/newest)
	$: sortedTrips = [...trips].sort((a, b) => a.sort_order - b.sort_order);

	// Get the last ODO value (from the most recent trip, or initial ODO if no trips)
	$: lastOdometer = sortedTrips.length > 0 ? sortedTrips[0].odometer : initialOdometer;

	// Default date for new entry: max date + 1 day
	$: defaultNewDate = (() => {
		if (sortedTrips.length === 0) {
			return new Date().toISOString().split('T')[0];
		}
		const maxDate = new Date(sortedTrips[0].date);
		maxDate.setDate(maxDate.getDate() + 1);
		return maxDate.toISOString().split('T')[0];
	})();

	// Calculate "Použitá spotreba" for each trip
	// This is the consumption rate from the last fill-up, carried forward
	$: consumptionRates = calculateConsumptionRates(trips);

	// Calculate "Zostatok" (remaining fuel) for each trip
	$: fuelRemaining = calculateFuelRemaining(trips, consumptionRates);

	export let tankSize: number = 66; // Default tank size, should be passed from vehicle
	export let initialOdometer: number = 0; // Starting ODO for "Prvý záznam"

	function calculateConsumptionRates(tripList: Trip[]): Map<string, number> {
		const rates = new Map<string, number>();

		// Sort chronologically (oldest first), using odometer as tiebreaker for same-day trips
		const chronological = [...tripList].sort((a, b) => {
			const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
			if (dateDiff !== 0) return dateDiff;
			return a.odometer - b.odometer;
		});

		// Two-pass algorithm:
		// 1. Find fill-ups and calculate rates for each period
		// 2. Apply rates RETROACTIVELY to all trips in that period

		// First pass: identify periods and calculate rates
		const periods: { tripIds: string[]; rate: number }[] = [];
		let currentPeriodTrips: string[] = [];
		let kmInPeriod = 0;

		for (const trip of chronological) {
			currentPeriodTrips.push(trip.id);
			kmInPeriod += trip.distance_km;

			// If this trip has a fill-up, calculate rate for this period
			if (trip.fuel_liters && trip.fuel_liters > 0 && kmInPeriod > 0) {
				const rate = (trip.fuel_liters / kmInPeriod) * 100;
				periods.push({ tripIds: [...currentPeriodTrips], rate });
				currentPeriodTrips = [];
				kmInPeriod = 0;
			}
		}

		// Handle remaining trips (no fill-up yet) - use TP rate
		if (currentPeriodTrips.length > 0) {
			periods.push({ tripIds: currentPeriodTrips, rate: tpConsumption });
		}

		// Second pass: apply rates to all trips in each period
		for (const period of periods) {
			for (const tripId of period.tripIds) {
				rates.set(tripId, period.rate);
			}
		}

		return rates;
	}

	function calculateFuelRemaining(tripList: Trip[], rates: Map<string, number>): Map<string, number> {
		const remaining = new Map<string, number>();

		// Sort chronologically (oldest first), using odometer as tiebreaker for same-day trips
		const chronological = [...tripList].sort((a, b) => {
			const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
			if (dateDiff !== 0) return dateDiff;
			return a.odometer - b.odometer;
		});

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
					<th>l/100km</th>
					<th>Zostatok</th>
					<th>Iné €</th>
					<th>Akcie</th>
				</tr>
			</thead>
			<tbody>
				<!-- New row (not part of drag zone) -->
				{#if showNewRow}
					<TripRow
						trip={null}
						{routes}
						isNew={true}
						previousOdometer={insertAtSortOrder !== null
							? (sortedTrips.find(t => t.sort_order === insertAtSortOrder)?.odometer || lastOdometer)
							: lastOdometer}
						defaultDate={insertDate || defaultNewDate}
						consumptionRate={sortedTrips.length > 0 ? consumptionRates.get(sortedTrips[0].id) || tpConsumption : tpConsumption}
						zostatok={sortedTrips.length > 0 ? fuelRemaining.get(sortedTrips[0].id) || tankSize : tankSize}
						onSave={handleSaveNew}
						onCancel={handleCancelNew}
						onDelete={() => {}}
						{dragDisabled}
					/>
				{/if}
					<!-- Trip rows -->
				{#each sortedTrips as trip, index (trip.id)}
					<TripRow
						{trip}
						{routes}
						isNew={false}
						previousOdometer={index < sortedTrips.length - 1 ? sortedTrips[index + 1].odometer : initialOdometer}
						consumptionRate={consumptionRates.get(trip.id) || tpConsumption}
						zostatok={fuelRemaining.get(trip.id) || 0}
						onSave={(data) => handleUpdate(trip, data)}
						onCancel={() => {}}
						onDelete={handleDelete}
						onInsertAbove={() => handleInsertAbove(trip)}
						onEditStart={() => handleEditStart(trip.id)}
						onEditEnd={handleEditEnd}
						{dragDisabled}
						onDragStart={() => handleDragStart(trip.id)}
						onDragEnd={handleDragEnd}
						onDragOver={(e) => handleDragOver(e, index)}
						onDragLeave={handleDragLeave}
						onDrop={(e) => handleDrop(e, index)}
						isDragTarget={dropTargetIndex === index && draggedTripId !== trip.id}
						isDragging={draggedTripId === trip.id}
					/>
				{/each}
				<!-- Empty state -->
				{#if trips.length === 0 && !showNewRow}
					<tr class="empty">
						<td colspan="12">Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.</td>
					</tr>
				{/if}
				<!-- Synthetic "Prvý záznam" row - starting values -->
				<tr class="first-record">
					<td>-</td>
					<td>-</td>
					<td>-</td>
					<td>-</td>
					<td class="number">{initialOdometer.toFixed(1)}</td>
					<td class="purpose">Prvý záznam</td>
					<td>-</td>
					<td>-</td>
					<td class="number">{tpConsumption.toFixed(2)}</td>
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
		width: 100%;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.875rem;
		table-layout: fixed;
	}

	thead {
		background-color: #f8f9fa;
		position: sticky;
		top: 0;
	}

	th {
		padding: 0.75rem 0.25rem;
		text-align: left;
		font-weight: 600;
		color: #2c3e50;
		border-bottom: 2px solid #e0e0e0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Column widths - total should be 100% */
	th:nth-child(1) { width: 9%; }   /* Dátum */
	th:nth-child(2) { width: 14%; }  /* Odkiaľ */
	th:nth-child(3) { width: 14%; }  /* Kam */
	th:nth-child(4) { width: 5%; }   /* Km */
	th:nth-child(5) { width: 6%; }   /* ODO */
	th:nth-child(6) { width: 10%; }  /* Účel */
	th:nth-child(7) { width: 6%; }   /* PHM (L) */
	th:nth-child(8) { width: 6%; }   /* Cena € */
	th:nth-child(9) { width: 6%; }   /* l/100km */
	th:nth-child(10) { width: 6%; }  /* Zostatok */
	th:nth-child(11) { width: 5%; }  /* Iné € */
	th:nth-child(12) { width: 13%; } /* Akcie */

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
		padding: 0.5rem 0.25rem;
		border-bottom: 1px solid #e0e0e0;
		overflow: hidden;
		text-overflow: ellipsis;
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
