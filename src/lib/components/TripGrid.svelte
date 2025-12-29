<script lang="ts">
	import type { Trip, Route, TripGridData, Receipt } from '$lib/types';
	import { createTrip, updateTrip, deleteTrip, getRoutes, reorderTrip, getTripGridData, assignReceiptToTrip } from '$lib/api';
	import TripRow from './TripRow.svelte';
	import { onMount } from 'svelte';
	import { toast } from '$lib/stores/toast';

	// Track pending receipt assignment (for when a receipt is selected in TripRow)
	let pendingReceiptAssignment: Receipt | null = null;

	export let vehicleId: string;
	export let trips: Trip[] = [];
	export let year: number = new Date().getFullYear();
	export let onTripsChanged: () => void | Promise<void>;
	export let tpConsumption: number = 5.1; // Vehicle's TP consumption rate
	export let tankSize: number = 66;
	export let initialOdometer: number = 0;

	// Pre-calculated grid data from backend
	let gridData: TripGridData | null = null;
	let consumptionRates: Map<string, number> = new Map();
	let estimatedRates: Set<string> = new Set();
	let fuelRemaining: Map<string, number> = new Map();
	let dateWarnings: Set<string> = new Set();
	let consumptionWarnings: Set<string> = new Set();

	// Fetch grid data from backend whenever trips change
	async function loadGridData() {
		try {
			gridData = await getTripGridData(vehicleId, year);
			// Convert backend data to Maps/Sets for efficient lookup
			consumptionRates = new Map(Object.entries(gridData.rates));
			estimatedRates = new Set(gridData.estimated_rates);
			fuelRemaining = new Map(Object.entries(gridData.fuel_remaining));
			dateWarnings = new Set(gridData.date_warnings);
			consumptionWarnings = new Set(gridData.consumption_warnings);
		} catch (error) {
			console.error('Failed to load grid data:', error);
		}
	}

	// Reload grid data when trips or year change
	$: if (trips || year) {
		loadGridData();
	}

	let routes: Route[] = [];
	let showNewRow = false;
	let editingTripId: string | null = null;
	let insertAtSortOrder: number | null = null;
	let insertDate: string | null = null;

	// Sorting state (exported for parent access)
	type SortColumn = 'manual' | 'date';
	type SortDirection = 'asc' | 'desc';
	export let sortColumn: SortColumn = 'manual';
	export let sortDirection: SortDirection = 'asc'; // asc = newest first (sort_order 0 = newest)

	function toggleSort(column: SortColumn) {
		if (sortColumn === column) {
			// Toggle direction
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			// Switch column, default to newest first
			// For manual: asc (sort_order 0 = newest)
			// For date: desc (highest date = newest)
			sortColumn = column;
			sortDirection = column === 'manual' ? 'asc' : 'desc';
		}
	}

	// Disable reorder buttons when editing, adding new row, or not in manual sort mode
	$: reorderDisabled = showNewRow || editingTripId !== null || sortColumn !== 'manual';

	// Get unique purposes from trips for autocomplete (trim to avoid duplicates with trailing spaces)
	$: purposeSuggestions = Array.from(
		new Set(trips.map((t) => t.purpose.trim()).filter((p) => p !== ''))
	).sort();

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
			const newTrip = await createTrip(
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
				tripData.full_tank,
				insertAtSortOrder
			);

			// If a receipt was selected, assign it to the new trip
			if (pendingReceiptAssignment && newTrip) {
				try {
					await assignReceiptToTrip(pendingReceiptAssignment.id, newTrip.id, vehicleId);
					toast.success('Doklad bol pridelený k jazde');
				} catch (e) {
					console.error('Failed to assign receipt:', e);
					toast.error('Jazda uložená, ale nepodarilo sa prideliť doklad');
				}
				pendingReceiptAssignment = null;
			}

			showNewRow = false;
			insertAtSortOrder = null;
			insertDate = null;
			await recalculateAllOdo();
			onTripsChanged();
			await loadRoutes();
		} catch (error) {
			console.error('Failed to create trip:', error);
			toast.error('Nepodarilo sa vytvoriť záznam');
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
				tripData.other_costs_note,
				tripData.full_tank
			);

			// If a receipt was selected, assign it to the trip
			if (pendingReceiptAssignment) {
				try {
					await assignReceiptToTrip(pendingReceiptAssignment.id, trip.id, vehicleId);
					toast.success('Doklad bol pridelený k jazde');
				} catch (e) {
					console.error('Failed to assign receipt:', e);
					toast.error('Jazda uložená, ale nepodarilo sa prideliť doklad');
				}
				pendingReceiptAssignment = null;
			}

			await recalculateNewerTripsOdo(trip.id, tripData.odometer!);
			onTripsChanged();
			await loadRoutes();
		} catch (error) {
			console.error('Failed to update trip:', error);
			toast.error('Nepodarilo sa aktualizovať záznam');
		}
	}

	function handleReceiptSelected(receipt: Receipt) {
		pendingReceiptAssignment = receipt;
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
					t.purpose, t.fuel_liters, t.fuel_cost_eur, t.other_costs_eur, t.other_costs_note,
					t.full_tank
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
			toast.error('Nepodarilo sa odstrániť záznam');
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

	// Move trip up (swap with previous row - lower sort_order)
	async function handleMoveUp(tripId: string, currentIndex: number) {
		if (reorderDisabled || currentIndex === 0) return;

		try {
			// Get the sort_order of the trip above us
			const targetSortOrder = sortedTrips[currentIndex - 1].sort_order;
			await reorderTrip(tripId, targetSortOrder);
			await recalculateAllOdo();
			await onTripsChanged();
		} catch (error) {
			console.error('Failed to move trip:', error);
			toast.error('Nepodarilo sa presunúť záznam');
		}
	}

	// Move trip down (swap with next row - higher sort_order)
	async function handleMoveDown(tripId: string, currentIndex: number) {
		if (reorderDisabled || currentIndex >= sortedTrips.length - 1) return;

		try {
			// Get the sort_order of the trip below us
			const targetSortOrder = sortedTrips[currentIndex + 1].sort_order;
			await reorderTrip(tripId, targetSortOrder);
			await recalculateAllOdo();
			await onTripsChanged();
		} catch (error) {
			console.error('Failed to move trip:', error);
			toast.error('Nepodarilo sa presunúť záznam');
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
					trip.purpose, trip.fuel_liters, trip.fuel_cost_eur, trip.other_costs_eur, trip.other_costs_note,
					trip.full_tank
				);
			}
		}
	}

	// Synthetic "Prvý záznam" trip (starting point)
	const FIRST_RECORD_ID = '__first_record__';
	$: firstRecordTrip = {
		id: FIRST_RECORD_ID,
		vehicle_id: vehicleId,
		date: `${year}-01-01`,
		origin: '-',
		destination: '-',
		distance_km: 0,
		odometer: initialOdometer,
		purpose: 'Prvý záznam',
		fuel_liters: null,
		fuel_cost_eur: null,
		other_costs_eur: null,
		other_costs_note: null,
		full_tank: true,
		sort_order: 999999, // Always last in manual sort
		created_at: '',
		updated_at: ''
	} as Trip;

	// Display order (based on current sort settings)
	$: sortedTrips = [...trips, firstRecordTrip].sort((a, b) => {
		let diff: number;
		if (sortColumn === 'manual') {
			diff = a.sort_order - b.sort_order;
		} else {
			const dateA = new Date(a.date).getTime();
			const dateB = new Date(b.date).getTime();
			diff = dateA - dateB;
		}
		return sortDirection === 'asc' ? diff : -diff;
	});

	// Helper to check if a trip is the synthetic first record
	function isFirstRecord(trip: Trip): boolean {
		return trip.id === FIRST_RECORD_ID;
	}

	$: lastOdometer = sortedTrips.length > 0 ? sortedTrips[0].odometer : initialOdometer;

	$: defaultNewDate = (() => {
		if (sortedTrips.length === 0) {
			return new Date().toISOString().split('T')[0];
		}
		const maxDate = new Date(sortedTrips[0].date);
		maxDate.setDate(maxDate.getDate() + 1);
		return maxDate.toISOString().split('T')[0];
	})();
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
					<th class="sortable" on:click={() => toggleSort('date')}>
						Dátum
						{#if sortColumn === 'date'}
							<span class="sort-indicator">{sortDirection === 'asc' ? '▲' : '▼'}</span>
						{/if}
					</th>
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
					<th class="sortable" on:click={() => toggleSort('manual')}>
						Akcie
						{#if sortColumn === 'manual'}
							<span class="sort-indicator">⋮</span>
						{/if}
					</th>
				</tr>
			</thead>
			<tbody>
				<!-- New row at top (when adding via "Nový záznam" button) -->
				{#if showNewRow && insertAtSortOrder === null}
					<TripRow
						trip={null}
						{routes}
						{purposeSuggestions}
						isNew={true}
						previousOdometer={lastOdometer}
						defaultDate={defaultNewDate}
						consumptionRate={sortedTrips.length > 0 ? consumptionRates.get(sortedTrips[0].id) || tpConsumption : tpConsumption}
						zostatok={sortedTrips.length > 0 ? fuelRemaining.get(sortedTrips[0].id) || tankSize : tankSize}
						onSave={handleSaveNew}
						onCancel={handleCancelNew}
						onDelete={() => {}}
						onReceiptSelected={handleReceiptSelected}
					/>
				{/if}
				<!-- Trip rows -->
				{#each sortedTrips as trip, index (trip.id)}
					<!-- New row inserted above this trip (not for first record) -->
					{#if showNewRow && insertAtSortOrder === trip.sort_order && !isFirstRecord(trip)}
						<TripRow
							trip={null}
							{routes}
							{purposeSuggestions}
							isNew={true}
							previousOdometer={index < sortedTrips.length - 1 ? sortedTrips[index + 1].odometer : initialOdometer}
							defaultDate={insertDate || trip.date}
							consumptionRate={consumptionRates.get(trip.id) || tpConsumption}
							zostatok={fuelRemaining.get(trip.id) || tankSize}
							onSave={handleSaveNew}
							onCancel={handleCancelNew}
							onDelete={() => {}}
							onReceiptSelected={handleReceiptSelected}
						/>
					{/if}
					{#if isFirstRecord(trip)}
						<!-- Synthetic "Prvý záznam" row -->
						<tr class="first-record">
							<td>{trip.date.split('-').reverse().join('.')}</td>
							<td>-</td>
							<td>-</td>
							<td class="number">0</td>
							<td class="number">{trip.odometer.toFixed(0)}</td>
							<td class="purpose">{trip.purpose}</td>
							<td>-</td>
							<td>-</td>
							<td class="number">{tpConsumption.toFixed(2)}</td>
							<td class="number">{tankSize.toFixed(1)}</td>
							<td>-</td>
							<td>-</td>
							<td></td>
						</tr>
					{:else}
						<TripRow
							{trip}
							{routes}
							{purposeSuggestions}
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
							canMoveUp={!reorderDisabled && index > 0 && !isFirstRecord(sortedTrips[index - 1])}
							canMoveDown={!reorderDisabled && index < sortedTrips.length - 1 && !isFirstRecord(sortedTrips[index + 1])}
							hasDateWarning={dateWarnings.has(trip.id)}
							hasConsumptionWarning={consumptionWarnings.has(trip.id)}
							isEstimatedRate={estimatedRates.has(trip.id)}
							onReceiptSelected={handleReceiptSelected}
						/>
					{/if}
				{/each}
				<!-- Empty state (only if no trips, first record is always there) -->
				{#if trips.length === 0 && !showNewRow}
					<tr class="empty">
						<td colspan="13">Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.</td>
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

	th.sortable {
		cursor: pointer;
		user-select: none;
		transition: background-color 0.2s;
	}

	th.sortable:hover {
		background-color: #e9ecef;
	}

	.sort-indicator {
		margin-left: 0.25rem;
		font-size: 0.75rem;
		color: #3498db;
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
