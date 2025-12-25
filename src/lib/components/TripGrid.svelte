<script lang="ts">
	import type { Trip, Route } from '$lib/types';
	import { createTrip, updateTrip, deleteTrip, getRoutes, reorderTrip } from '$lib/api';
	import TripRow from './TripRow.svelte';
	import { onMount } from 'svelte';

	export let vehicleId: string;
	export let trips: Trip[] = [];
	export let onTripsChanged: () => void | Promise<void>;
	export let tpConsumption: number = 5.1; // Vehicle's TP consumption rate
	export let tankSize: number = 66;
	export let initialOdometer: number = 0;

	let routes: Route[] = [];
	let showNewRow = false;
	let editingTripId: string | null = null;
	let insertAtSortOrder: number | null = null;
	let insertDate: string | null = null;

	// Disable reorder buttons when editing or adding new row
	$: reorderDisabled = showNewRow || editingTripId !== null;

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
				tripData.other_costs_note,
				insertAtSortOrder
			);
			showNewRow = false;
			insertAtSortOrder = null;
			insertDate = null;
			await recalculateAllOdo();
			onTripsChanged();
			await loadRoutes();
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
				tripData.other_costs_note
			);
			await recalculateNewerTripsOdo(trip.id, tripData.odometer!);
			onTripsChanged();
			await loadRoutes();
		} catch (error) {
			console.error('Failed to update trip:', error);
			alert('Nepodarilo sa aktualizovať záznam');
		}
	}

	async function recalculateNewerTripsOdo(editedTripId: string, newOdo: number) {
		const chronological = [...trips].sort((a, b) => {
			const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
			if (dateDiff !== 0) return dateDiff;
			return a.odometer - b.odometer;
		});

		const editedIndex = chronological.findIndex((t) => t.id === editedTripId);
		if (editedIndex === -1 || editedIndex === chronological.length - 1) return;

		let runningOdo = newOdo;
		for (let i = editedIndex + 1; i < chronological.length; i++) {
			const t = chronological[i];
			runningOdo = runningOdo + t.distance_km;
			if (Math.abs(t.odometer - runningOdo) > 0.01) {
				await updateTrip(
					t.id, t.date, t.origin, t.destination, t.distance_km, runningOdo,
					t.purpose, t.fuel_liters, t.fuel_cost_eur, t.other_costs_eur, t.other_costs_note
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

	// Move trip up (swap with previous row)
	async function handleMoveUp(tripId: string, currentIndex: number) {
		if (reorderDisabled || currentIndex === 0) return;

		try {
			await reorderTrip(tripId, currentIndex - 1);
			await recalculateAllOdo();
			await onTripsChanged();
		} catch (error) {
			console.error('Failed to move trip:', error);
			alert('Nepodarilo sa presunúť záznam');
		}
	}

	// Move trip down (swap with next row)
	async function handleMoveDown(tripId: string, currentIndex: number) {
		if (reorderDisabled || currentIndex >= sortedTrips.length - 1) return;

		try {
			await reorderTrip(tripId, currentIndex + 1);
			await recalculateAllOdo();
			await onTripsChanged();
		} catch (error) {
			console.error('Failed to move trip:', error);
			alert('Nepodarilo sa presunúť záznam');
		}
	}

	async function recalculateAllOdo() {
		const chronological = [...trips]
			.sort((a, b) => a.sort_order - b.sort_order)
			.reverse();

		let runningOdo = initialOdometer;
		for (const trip of chronological) {
			runningOdo += trip.distance_km;
			if (Math.abs(trip.odometer - runningOdo) > 0.01) {
				await updateTrip(
					trip.id, trip.date, trip.origin, trip.destination, trip.distance_km, runningOdo,
					trip.purpose, trip.fuel_liters, trip.fuel_cost_eur, trip.other_costs_eur, trip.other_costs_note
				);
			}
		}
	}

	// Sort trips by sort_order ascending (0 = top/newest)
	$: sortedTrips = [...trips].sort((a, b) => a.sort_order - b.sort_order);

	$: lastOdometer = sortedTrips.length > 0 ? sortedTrips[0].odometer : initialOdometer;

	$: defaultNewDate = (() => {
		if (sortedTrips.length === 0) {
			return new Date().toISOString().split('T')[0];
		}
		const maxDate = new Date(sortedTrips[0].date);
		maxDate.setDate(maxDate.getDate() + 1);
		return maxDate.toISOString().split('T')[0];
	})();

	// Calculate consumption rates for each trip
	$: consumptionRates = calculateConsumptionRates(trips);

	// Calculate remaining fuel for each trip
	$: fuelRemaining = calculateFuelRemaining(trips, consumptionRates);

	// Check if date is out of order (light red highlight)
	$: dateWarnings = calculateDateWarnings(sortedTrips);

	// Check if consumption is over limit (light orange highlight)
	$: consumptionWarnings = calculateConsumptionWarnings(sortedTrips, consumptionRates);

	function calculateConsumptionRates(tripList: Trip[]): Map<string, number> {
		const rates = new Map<string, number>();
		const chronological = [...tripList].sort((a, b) => {
			const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
			if (dateDiff !== 0) return dateDiff;
			return a.odometer - b.odometer;
		});

		const periods: { tripIds: string[]; rate: number }[] = [];
		let currentPeriodTrips: string[] = [];
		let kmInPeriod = 0;

		for (const trip of chronological) {
			currentPeriodTrips.push(trip.id);
			kmInPeriod += trip.distance_km;

			if (trip.fuel_liters && trip.fuel_liters > 0 && kmInPeriod > 0) {
				const rate = (trip.fuel_liters / kmInPeriod) * 100;
				periods.push({ tripIds: [...currentPeriodTrips], rate });
				currentPeriodTrips = [];
				kmInPeriod = 0;
			}
		}

		if (currentPeriodTrips.length > 0) {
			periods.push({ tripIds: currentPeriodTrips, rate: tpConsumption });
		}

		for (const period of periods) {
			for (const tripId of period.tripIds) {
				rates.set(tripId, period.rate);
			}
		}

		return rates;
	}

	function calculateFuelRemaining(tripList: Trip[], rates: Map<string, number>): Map<string, number> {
		const remaining = new Map<string, number>();
		const chronological = [...tripList].sort((a, b) => {
			const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
			if (dateDiff !== 0) return dateDiff;
			return a.odometer - b.odometer;
		});

		let zostatok = tankSize;
		for (const trip of chronological) {
			const rate = rates.get(trip.id) || 0;
			const spotreba = rate > 0 ? (trip.distance_km * rate) / 100 : 0;
			zostatok = zostatok - spotreba;

			if (trip.fuel_liters && trip.fuel_liters > 0) {
				zostatok = zostatok + trip.fuel_liters;
				if (zostatok > tankSize) zostatok = tankSize;
			}

			if (zostatok < 0) zostatok = 0;
			remaining.set(trip.id, zostatok);
		}

		return remaining;
	}

	// Check if each row's date fits between neighbors (by sort_order)
	function calculateDateWarnings(sorted: Trip[]): Set<string> {
		const warnings = new Set<string>();

		for (let i = 0; i < sorted.length; i++) {
			const trip = sorted[i];
			const prevTrip = i > 0 ? sorted[i - 1] : null;
			const nextTrip = i < sorted.length - 1 ? sorted[i + 1] : null;

			// sort_order 0 = newest (should have highest date)
			// Check: prevTrip.date >= trip.date >= nextTrip.date
			if (prevTrip && trip.date > prevTrip.date) {
				warnings.add(trip.id);
			}
			if (nextTrip && trip.date < nextTrip.date) {
				warnings.add(trip.id);
			}
		}

		return warnings;
	}

	// Check if consumption rate exceeds 120% of TP rate (legal limit)
	function calculateConsumptionWarnings(sorted: Trip[], rates: Map<string, number>): Set<string> {
		const warnings = new Set<string>();
		const limit = tpConsumption * 1.2; // 120% of TP rate

		for (const trip of sorted) {
			const rate = rates.get(trip.id);
			if (rate && rate > limit) {
				warnings.add(trip.id);
			}
		}

		return warnings;
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
					<th>Iné pozn.</th>
					<th>Akcie</th>
				</tr>
			</thead>
			<tbody>
				<!-- New row at top (when adding via "Nový záznam" button) -->
				{#if showNewRow && insertAtSortOrder === null}
					<TripRow
						trip={null}
						{routes}
						isNew={true}
						previousOdometer={lastOdometer}
						defaultDate={defaultNewDate}
						consumptionRate={sortedTrips.length > 0 ? consumptionRates.get(sortedTrips[0].id) || tpConsumption : tpConsumption}
						zostatok={sortedTrips.length > 0 ? fuelRemaining.get(sortedTrips[0].id) || tankSize : tankSize}
						onSave={handleSaveNew}
						onCancel={handleCancelNew}
						onDelete={() => {}}
					/>
				{/if}
				<!-- Trip rows -->
				{#each sortedTrips as trip, index (trip.id)}
					<!-- New row inserted above this trip -->
					{#if showNewRow && insertAtSortOrder === trip.sort_order}
						<TripRow
							trip={null}
							{routes}
							isNew={true}
							previousOdometer={index < sortedTrips.length - 1 ? sortedTrips[index + 1].odometer : initialOdometer}
							defaultDate={insertDate || trip.date}
							consumptionRate={consumptionRates.get(trip.id) || tpConsumption}
							zostatok={fuelRemaining.get(trip.id) || tankSize}
							onSave={handleSaveNew}
							onCancel={handleCancelNew}
							onDelete={() => {}}
						/>
					{/if}
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
						onMoveUp={() => handleMoveUp(trip.id, index)}
						onMoveDown={() => handleMoveDown(trip.id, index)}
						canMoveUp={!reorderDisabled && index > 0}
						canMoveDown={!reorderDisabled && index < sortedTrips.length - 1}
						hasDateWarning={dateWarnings.has(trip.id)}
						hasConsumptionWarning={consumptionWarnings.has(trip.id)}
					/>
				{/each}
				<!-- Empty state -->
				{#if trips.length === 0 && !showNewRow}
					<tr class="empty">
						<td colspan="13">Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.</td>
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
	th:nth-child(1) { width: 6%; }   /* Dátum */
	th:nth-child(2) { width: 16%; }  /* Odkiaľ */
	th:nth-child(3) { width: 16%; }  /* Kam */
	th:nth-child(4) { width: 4%; text-align: right; }   /* Km */
	th:nth-child(5) { width: 5%; text-align: right; }   /* ODO */
	th:nth-child(6) { width: 12%; }  /* Účel */
	th:nth-child(7) { width: 4%; text-align: right; }   /* PHM (L) */
	th:nth-child(8) { width: 4%; text-align: right; }   /* Cena € */
	th:nth-child(9) { width: 5%; text-align: right; }   /* l/100km */
	th:nth-child(10) { width: 5%; text-align: right; }  /* Zostatok */
	th:nth-child(11) { width: 4%; text-align: right; }  /* Iné € */
	th:nth-child(12) { width: 10%; }  /* Iné pozn. */
	th:nth-child(13) { width: 9%; text-align: center; } /* Akcie */

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
