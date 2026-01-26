<script lang="ts">
	import type { Trip, Route, TripGridData, PreviewResult, VehicleType, SuggestedFillup } from '$lib/types';
	import { DatePrefillMode } from '$lib/types';
	import { createTrip, updateTrip, deleteTrip, getRoutes, getPurposes, reorderTrip, getTripGridData, previewTripCalculation, calculateMagicFillLiters, getDatePrefillMode, setDatePrefillMode } from '$lib/api';
	import TripRow from './TripRow.svelte';
	import SegmentedToggle from './SegmentedToggle.svelte';
	import { onMount, tick } from 'svelte';
	import { toast } from '$lib/stores/toast';
	import { triggerReceiptRefresh } from '$lib/stores/receipts';
	import LL from '$lib/i18n/i18n-svelte';

	export let vehicleId: string;
	export let trips: Trip[] = [];
	export let year: number = new Date().getFullYear();
	export let onTripsChanged: () => void | Promise<void>;
	export let tpConsumption: number = 5.1; // Vehicle's TP consumption rate
	export let tankSize: number = 66;
	export let initialOdometer: number = 0;
	// EV support
	export let vehicleType: VehicleType = 'Ice';
	export let batteryCapacityKwh: number = 0;
	export let baselineConsumptionKwh: number = 0;

	// Derived: show columns based on vehicle type
	$: showFuelColumns = vehicleType === 'Ice' || vehicleType === 'Phev';
	$: showEnergyColumns = vehicleType === 'Bev' || vehicleType === 'Phev';

	// Use year-specific starting odometer from backend (carryover from previous year)
	// Falls back to vehicle's initial odometer if not available
	$: effectiveInitialOdometer = gridData?.yearStartOdometer ?? initialOdometer;

	// Pre-calculated grid data from backend
	let gridData: TripGridData | null = null;
	// Fuel data
	let consumptionRates: Map<string, number> = new Map();
	let estimatedRates: Set<string> = new Set();
	let fuelConsumed: Map<string, number> = new Map();
	let fuelRemaining: Map<string, number> = new Map();
	let consumptionWarnings: Set<string> = new Set();
	// Energy data (BEV/PHEV)
	let energyRates: Map<string, number> = new Map();
	let estimatedEnergyRates: Set<string> = new Set();
	let batteryRemainingKwh: Map<string, number> = new Map();
	let batteryRemainingPercent: Map<string, number> = new Map();
	let socOverrideTrips: Set<string> = new Set();
	// Shared
	let dateWarnings: Set<string> = new Set();
	// Suggested fillup (for trips in open period)
	let suggestedFillup: Map<string, SuggestedFillup> = new Map();

	// Fetch grid data from backend whenever trips change
	async function loadGridData() {
		try {
			gridData = await getTripGridData(vehicleId, year);
			// Convert backend data to Maps/Sets for efficient lookup
			// Fuel
			consumptionRates = new Map(Object.entries(gridData.rates));
			estimatedRates = new Set(gridData.estimatedRates);
			fuelConsumed = new Map(Object.entries(gridData.fuelConsumed));
			fuelRemaining = new Map(Object.entries(gridData.fuelRemaining));
			consumptionWarnings = new Set(gridData.consumptionWarnings);
			// Energy
			energyRates = new Map(Object.entries(gridData.energyRates));
			estimatedEnergyRates = new Set(gridData.estimatedEnergyRates);
			batteryRemainingKwh = new Map(Object.entries(gridData.batteryRemainingKwh));
			batteryRemainingPercent = new Map(Object.entries(gridData.batteryRemainingPercent));
			socOverrideTrips = new Set(gridData.socOverrideTrips);
			// Shared
			dateWarnings = new Set(gridData.dateWarnings);
			// Suggested fillup
			suggestedFillup = new Map(Object.entries(gridData.suggestedFillup));
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

	// Live preview state
	let previewData: PreviewResult | null = null;
	let previewingTripId: string | null = null; // Which row is previewing (null = new row)

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

	// Purpose suggestions loaded from backend (across all years)
	let purposeSuggestions: string[] = [];

	// Date prefill mode for new entries
	let datePrefillMode: typeof DatePrefillMode[keyof typeof DatePrefillMode] = DatePrefillMode.Previous;

	onMount(async () => {
		await loadRoutes();
		await loadPurposes();
		// Load date prefill preference
		try {
			datePrefillMode = await getDatePrefillMode();
		} catch (error) {
			console.error('Failed to load date prefill mode:', error);
		}
	});

	async function loadRoutes() {
		try {
			routes = await getRoutes(vehicleId);
		} catch (error) {
			console.error('Failed to load routes:', error);
		}
	}

	async function loadPurposes() {
		try {
			purposeSuggestions = await getPurposes(vehicleId);
		} catch (error) {
			console.error('Failed to load purposes:', error);
		}
	}

	async function handleDatePrefillChange(event: CustomEvent<string>) {
		const mode = event.detail as typeof DatePrefillMode[keyof typeof DatePrefillMode];
		datePrefillMode = mode;
		try {
			await setDatePrefillMode(mode);
		} catch (error) {
			console.error('Failed to save date prefill mode:', error);
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
				tripData.distanceKm!,
				tripData.odometer!,
				tripData.purpose!,
				// Fuel fields
				tripData.fuelLiters,
				tripData.fuelCostEur,
				tripData.fullTank,
				// Energy fields
				tripData.energyKwh,
				tripData.energyCostEur,
				tripData.fullCharge,
				null, // socOverridePercent - rarely used on new trips
				// Other
				tripData.otherCostsEur,
				tripData.otherCostsNote,
				insertAtSortOrder
			);

			showNewRow = false;
			insertAtSortOrder = null;
			insertDate = null;
			// Clear preview
			previewData = null;
			previewingTripId = null;
			// First refresh trips from DB, then recalculate ODO on updated list
			// (Fix: recalculateAllOdo was running on stale trips prop before)
			await onTripsChanged();
			await tick(); // Wait for Svelte prop update
			await recalculateAllOdo();
			await loadRoutes();
			await loadPurposes();
		} catch (error) {
			console.error('Failed to create trip:', error);
			toast.error($LL.toast.errorCreateTrip());
		}
	}

	async function handleUpdate(trip: Trip, tripData: Partial<Trip>) {
		try {
			await updateTrip(
				trip.id,
				tripData.date!,
				tripData.origin!,
				tripData.destination!,
				tripData.distanceKm!,
				tripData.odometer!,
				tripData.purpose!,
				// Fuel fields
				tripData.fuelLiters,
				tripData.fuelCostEur,
				tripData.fullTank,
				// Energy fields
				tripData.energyKwh,
				tripData.energyCostEur,
				tripData.fullCharge,
				trip.socOverridePercent, // Preserve existing SoC override
				// Other
				tripData.otherCostsEur,
				tripData.otherCostsNote
			);

			await recalculateNewerTripsOdo(trip.id, tripData.odometer!);
			onTripsChanged();
			await loadRoutes();
			await loadPurposes();
			triggerReceiptRefresh(); // Update nav badge after trip change
		} catch (error) {
			console.error('Failed to update trip:', error);
			toast.error($LL.toast.errorUpdateTrip());
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
			runningOdo = runningOdo + t.distanceKm;
			if (Math.abs(t.odometer - runningOdo) > 0.01) {
				await updateTrip(
					t.id, t.date, t.origin, t.destination, t.distanceKm, runningOdo,
					t.purpose,
					t.fuelLiters, t.fuelCostEur, t.fullTank,
					t.energyKwh, t.energyCostEur, t.fullCharge, t.socOverridePercent,
					t.otherCostsEur, t.otherCostsNote
				);
			}
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteTrip(id);
			onTripsChanged();
			triggerReceiptRefresh(); // Update nav badge after trip deletion
		} catch (error) {
			console.error('Failed to delete trip:', error);
			toast.error($LL.toast.errorDeleteTrip());
		}
	}

	function handleCancelNew() {
		showNewRow = false;
		insertAtSortOrder = null;
		insertDate = null;
		// Clear preview
		previewData = null;
		previewingTripId = null;
	}

	function handleEditStart(tripId: string) {
		editingTripId = tripId;
	}

	function handleEditEnd() {
		editingTripId = null;
		// Clear preview
		previewData = null;
		previewingTripId = null;
	}

	function handleInsertAbove(targetTrip: Trip) {
		insertAtSortOrder = targetTrip.sortOrder;
		insertDate = targetTrip.date;
		showNewRow = true;
	}

	// Live preview calculation handler
	async function handlePreviewRequest(
		tripId: string | null,
		sortOrder: number | null,
		km: number,
		fuel: number | null,
		fullTank: boolean
	) {
		try {
			previewData = await previewTripCalculation(
				vehicleId,
				year,
				km,
				fuel,
				fullTank,
				sortOrder,
				tripId
			);
			previewingTripId = tripId;
		} catch (error) {
			console.error('Preview calculation failed:', error);
			// Don't show error toast - preview is non-critical
			previewData = null;
		}
	}

	// Magic fill - calculate suggested liters for 105-120% of TP consumption
	async function handleMagicFill(km: number, tripId: string | null): Promise<number> {
		try {
			return await calculateMagicFillLiters(vehicleId, year, km, tripId);
		} catch (error) {
			console.error('Magic fill calculation failed:', error);
			return 0;
		}
	}

	// Move trip up (swap with previous row - lower sortOrder)
	async function handleMoveUp(tripId: string, currentIndex: number) {
		if (reorderDisabled || currentIndex === 0) return;

		try {
			// Get the sortOrder of the trip above us
			const targetSortOrder = sortedTrips[currentIndex - 1].sortOrder;
			await reorderTrip(tripId, targetSortOrder);
			// First refresh trips to get updated sortOrder, then recalculate ODO
			await onTripsChanged();
			await tick();
			await recalculateAllOdo();
		} catch (error) {
			console.error('Failed to move trip:', error);
			toast.error($LL.toast.errorMoveTrip());
		}
	}

	// Move trip down (swap with next row - higher sortOrder)
	async function handleMoveDown(tripId: string, currentIndex: number) {
		if (reorderDisabled || currentIndex >= sortedTrips.length - 1) return;

		try {
			// Get the sortOrder of the trip below us
			const targetSortOrder = sortedTrips[currentIndex + 1].sortOrder;
			await reorderTrip(tripId, targetSortOrder);
			// First refresh trips to get updated sortOrder, then recalculate ODO
			await onTripsChanged();
			await tick();
			await recalculateAllOdo();
		} catch (error) {
			console.error('Failed to move trip:', error);
			toast.error($LL.toast.errorMoveTrip());
		}
	}

	async function recalculateAllOdo() {
		const chronological = [...trips]
			.sort((a, b) => a.sortOrder - b.sortOrder)
			.reverse();

		let runningOdo = effectiveInitialOdometer;
		for (const trip of chronological) {
			runningOdo += trip.distanceKm;
			if (Math.abs(trip.odometer - runningOdo) > 0.01) {
				await updateTrip(
					trip.id, trip.date, trip.origin, trip.destination, trip.distanceKm, runningOdo,
					trip.purpose,
					trip.fuelLiters, trip.fuelCostEur, trip.fullTank,
					trip.energyKwh, trip.energyCostEur, trip.fullCharge, trip.socOverridePercent,
					trip.otherCostsEur, trip.otherCostsNote
				);
			}
		}
	}

	// Synthetic "Prvý záznam" trip (starting point)
	const FIRST_RECORD_ID = '__first_record__';
	$: firstRecordTrip = {
		id: FIRST_RECORD_ID,
		vehicleId: vehicleId,
		date: `${year}-01-01`,
		origin: '-',
		destination: '-',
		distanceKm: 0,
		odometer: effectiveInitialOdometer,
		purpose: $LL.trips.firstRecord(),
		fuelLiters: null,
		fuelCostEur: null,
		fullTank: true,
		// Energy fields
		energyKwh: null,
		energyCostEur: null,
		fullCharge: false,
		socOverridePercent: null,
		// Other
		otherCostsEur: null,
		otherCostsNote: null,
		sortOrder: 999999, // Always last in manual sort
		createdAt: '',
		updatedAt: ''
	} as Trip;

	// Display order (based on current sort settings)
	$: sortedTrips = [...trips, firstRecordTrip].sort((a, b) => {
		let diff: number;
		if (sortColumn === 'manual') {
			diff = a.sortOrder - b.sortOrder;
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

	$: lastOdometer = sortedTrips.length > 0 ? sortedTrips[0].odometer : effectiveInitialOdometer;

	// Legend counts
	$: partialCount = trips.filter(t => t.fuelLiters && !t.fullTank).length;
	$: missingReceiptCount = gridData?.missingReceipts.length ?? 0;
	$: consumptionWarningCount = consumptionWarnings.size;
	// Legend suggested fillup - provided directly by backend (no frontend logic needed)
	$: legendSuggestedFillup = gridData?.legendSuggestedFillup ?? null;

	$: defaultNewDate = (() => {
		// "Today" mode or no trips: use today's date
		if (datePrefillMode === DatePrefillMode.Today || sortedTrips.length === 0) {
			return new Date().toISOString().split('T')[0];
		}
		// "Previous" mode: last trip date + 1 day
		const maxDate = new Date(sortedTrips[0].date);
		maxDate.setDate(maxDate.getDate() + 1);
		return maxDate.toISOString().split('T')[0];
	})();
</script>

<div class="trip-grid">
	<div class="header">
		<h2>{$LL.trips.title()} ({trips.length})</h2>
		<div class="header-actions">
			<button class="new-record" on:click={handleNewRecord} disabled={showNewRow}>
				{$LL.trips.newRecord()}
			</button>
			<SegmentedToggle
				options={[
					{ value: DatePrefillMode.Previous, label: $LL.trips.datePrefillPrevious() },
					{ value: DatePrefillMode.Today, label: $LL.trips.datePrefillToday() }
				]}
				value={datePrefillMode}
				on:change={handleDatePrefillChange}
				size="small"
				title={$LL.trips.datePrefillTooltip()}
			/>
		</div>
	</div>

	<div class="table-container">
		{#if partialCount > 0 || missingReceiptCount > 0 || consumptionWarningCount > 0 || legendSuggestedFillup}
			<div class="table-legend">
				{#if legendSuggestedFillup}
					<span class="legend-item suggested-fillup">
						<span class="suggested-indicator">💡</span>
						{$LL.trips.legend.suggestedFillup({ liters: legendSuggestedFillup.liters.toFixed(2), rate: legendSuggestedFillup.consumptionRate.toFixed(2) })}
					</span>
				{/if}
				{#if partialCount > 0}
					<span class="legend-item"><span class="partial-indicator">*</span> {$LL.trips.legend.partialFillup()} ({partialCount})</span>
				{/if}
				{#if missingReceiptCount > 0}
					<span class="legend-item"><span class="no-receipt-indicator">⚠</span> {$LL.trips.legend.noReceipt()} ({missingReceiptCount})</span>
				{/if}
				{#if consumptionWarningCount > 0}
					<span class="legend-item"><span class="consumption-warning-sample"></span> {$LL.trips.legend.highConsumption()} ({consumptionWarningCount})</span>
				{/if}
			</div>
		{/if}
		<table>
			<thead>
				<tr>
					<th class="sortable" on:click={() => toggleSort('date')}>
						{$LL.trips.columns.date()}
						{#if sortColumn === 'date'}
							<span class="sort-indicator">{sortDirection === 'asc' ? '▲' : '▼'}</span>
						{/if}
					</th>
					<th>{$LL.trips.columns.origin()}</th>
					<th>{$LL.trips.columns.destination()}</th>
					<th>{$LL.trips.columns.km()}</th>
					<th>{$LL.trips.columns.odo()}</th>
					<th>{$LL.trips.columns.purpose()}</th>
					{#if showFuelColumns}
						<th>{$LL.trips.columns.fuelLiters()}</th>
						<th>{$LL.trips.columns.fuelCost()}</th>
						<th>{$LL.trips.columns.fuelConsumed()}</th>
						<th>{$LL.trips.columns.consumptionRate()}</th>
						<th>{$LL.trips.columns.remaining()}</th>
					{/if}
					{#if showEnergyColumns}
						<th>{$LL.trips.columns.energyKwh()}</th>
						<th>{$LL.trips.columns.energyCost()}</th>
						<th>{$LL.trips.columns.energyRate()}</th>
						<th>{$LL.trips.columns.batteryRemaining()}</th>
					{/if}
					<th>{$LL.trips.columns.otherCosts()}</th>
					<th>{$LL.trips.columns.otherCostsNote()}</th>
					<th>
						{$LL.trips.columns.actions()}
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
						fuelConsumed={0}
						fuelRemaining={sortedTrips.length > 0 ? fuelRemaining.get(sortedTrips[0].id) || tankSize : tankSize}
						{vehicleType}
						energyRate={sortedTrips.length > 0 ? energyRates.get(sortedTrips[0].id) || baselineConsumptionKwh : baselineConsumptionKwh}
						batteryRemainingKwh={sortedTrips.length > 0 ? batteryRemainingKwh.get(sortedTrips[0].id) || batteryCapacityKwh : batteryCapacityKwh}
						batteryRemainingPercent={sortedTrips.length > 0 ? batteryRemainingPercent.get(sortedTrips[0].id) || 100 : 100}
						onSave={handleSaveNew}
						onCancel={handleCancelNew}
						onDelete={() => {}}
						previewData={previewingTripId === null ? previewData : null}
						onPreviewRequest={(km, fuel, fullTank) => handlePreviewRequest(null, null, km, fuel, fullTank)}
						onMagicFill={handleMagicFill}
					/>
				{/if}
				<!-- Trip rows -->
				{#each sortedTrips as trip, index (trip.id)}
					<!-- New row inserted above this trip (not for first record) -->
					{#if showNewRow && insertAtSortOrder === trip.sortOrder && !isFirstRecord(trip)}
						<TripRow
							trip={null}
							{routes}
							{purposeSuggestions}
							isNew={true}
							previousOdometer={index < sortedTrips.length - 1 ? sortedTrips[index + 1].odometer : effectiveInitialOdometer}
							defaultDate={insertDate || trip.date}
							consumptionRate={consumptionRates.get(trip.id) || tpConsumption}
							fuelConsumed={0}
							fuelRemaining={fuelRemaining.get(trip.id) || tankSize}
							{vehicleType}
							energyRate={energyRates.get(trip.id) || baselineConsumptionKwh}
							batteryRemainingKwh={batteryRemainingKwh.get(trip.id) || batteryCapacityKwh}
							batteryRemainingPercent={batteryRemainingPercent.get(trip.id) || 100}
							onSave={handleSaveNew}
							onCancel={handleCancelNew}
							onDelete={() => {}}
							previewData={previewingTripId === null ? previewData : null}
							onPreviewRequest={(km, fuel, fullTank) => handlePreviewRequest(null, insertAtSortOrder, km, fuel, fullTank)}
							onMagicFill={handleMagicFill}
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
							{#if showFuelColumns}
								<td>-</td>
								<td>-</td>
								<td class="number calculated">0.00</td>
								<td class="number">{tpConsumption.toFixed(2)}</td>
								<td class="number">{tankSize.toFixed(1)}</td>
							{/if}
							{#if showEnergyColumns}
								<td>-</td>
								<td>-</td>
								<td class="number">{baselineConsumptionKwh.toFixed(2)}</td>
								<td class="number">{batteryCapacityKwh.toFixed(1)} kWh</td>
							{/if}
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
							previousOdometer={index < sortedTrips.length - 1 ? sortedTrips[index + 1].odometer : effectiveInitialOdometer}
							consumptionRate={consumptionRates.get(trip.id) || tpConsumption}
							fuelConsumed={fuelConsumed.get(trip.id) || 0}
							fuelRemaining={fuelRemaining.get(trip.id) || 0}
							{vehicleType}
							energyRate={energyRates.get(trip.id) || baselineConsumptionKwh}
							batteryRemainingKwh={batteryRemainingKwh.get(trip.id) || 0}
							batteryRemainingPercent={batteryRemainingPercent.get(trip.id) || 0}
							isEstimatedEnergyRate={estimatedEnergyRates.has(trip.id)}
							hasSocOverride={socOverrideTrips.has(trip.id)}
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
							hasMatchingReceipt={!gridData?.missingReceipts.includes(trip.id)}
							previewData={previewingTripId === trip.id ? previewData : null}
							onPreviewRequest={(km, fuel, fullTank) => handlePreviewRequest(trip.id, trip.sortOrder, km, fuel, fullTank)}
							suggestedFillup={suggestedFillup.get(trip.id) ?? null}
							onMagicFill={handleMagicFill}
						/>
					{/if}
				{/each}
				<!-- Empty state (only if no trips, first record is always there) -->
				{#if trips.length === 0 && !showNewRow}
					<tr class="empty">
						<td colspan={9 + (showFuelColumns ? 5 : 0) + (showEnergyColumns ? 4 : 0)}>{$LL.trips.emptyState()}</td>
					</tr>
				{/if}
			</tbody>
		</table>
	</div>
</div>

<style>
	.trip-grid {
		background: var(--bg-surface);
		border-radius: 8px;
		box-shadow: 0 1px 3px var(--shadow-default);
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-default);
	}

	.header h2 {
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.header-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.new-record {
		padding: 0.625rem 1.25rem;
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.new-record:hover:not(:disabled) {
		background-color: var(--btn-active-primary-hover);
	}

	.new-record:disabled {
		background-color: var(--text-muted);
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
		color: var(--text-primary);
	}

	thead {
		background-color: var(--bg-surface-alt);
		position: sticky;
		top: 0;
	}

	th {
		padding: 0.75rem 0.25rem;
		text-align: left;
		font-weight: 600;
		color: var(--text-primary);
		border-bottom: 2px solid var(--border-default);
		overflow: hidden;
		text-overflow: ellipsis;
	}

	th.sortable {
		cursor: pointer;
		user-select: none;
		transition: background-color 0.2s;
	}

	th.sortable:hover {
		background-color: var(--btn-secondary-hover);
	}

	.sort-indicator {
		margin-left: 0.25rem;
		font-size: 0.75rem;
		color: var(--accent-primary);
	}

	/* Column widths - total should be 100% */
	th:nth-child(1) { width: 5%; }   /* Dátum */
	th:nth-child(2) { width: 16%; }  /* Odkiaľ */
	th:nth-child(3) { width: 16%; }  /* Kam */
	th:nth-child(4) { width: 4%; text-align: right; }   /* Km */
	th:nth-child(5) { width: 5%; text-align: right; }   /* ODO */
	th:nth-child(6) { width: 12%; }  /* Účel */
	th:nth-child(7) { width: 4%; text-align: right; }   /* PHM (L) */
	th:nth-child(8) { width: 4%; text-align: right; }   /* Cena € */
	th:nth-child(9) { width: 4%; text-align: right; }   /* Spotr. (L) - NEW */
	th:nth-child(10) { width: 4%; text-align: right; }  /* l/100km */
	th:nth-child(11) { width: 4%; text-align: right; }  /* Zostatok */
	th:nth-child(12) { width: 4%; text-align: right; }  /* Iné € */
	th:nth-child(13) { width: 10%; }  /* Iné pozn. */
	th:nth-child(14) { width: 8%; text-align: center; } /* Akcie */
	tbody tr.empty td {
		padding: 2rem;
		text-align: center;
		color: var(--text-secondary);
		font-style: italic;
	}

	tbody tr.first-record {
		background-color: var(--bg-body);
		color: var(--text-secondary);
		font-style: italic;
	}

	tbody tr.first-record td {
		padding: 0.5rem 0.25rem;
		border-bottom: 1px solid var(--border-default);
		overflow: hidden;
		text-overflow: ellipsis;
	}

	tbody tr.first-record td.purpose {
		font-weight: 500;
		color: var(--text-primary);
	}

	tbody tr.first-record td.number {
		text-align: right;
		font-style: normal;
		color: var(--text-primary);
	}

	.table-legend {
		display: flex;
		gap: 1.5rem;
		padding: 0.75rem 1rem;
		background: var(--warning-bg);
		border: 1px solid var(--warning-border);
		border-radius: 4px;
		margin-bottom: 0.75rem;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.partial-indicator {
		color: var(--accent-warning);
		font-weight: bold;
	}

	.no-receipt-indicator {
		color: var(--accent-warning-dark);
	}

	.consumption-warning-sample {
		display: inline-block;
		width: 12px;
		height: 12px;
		background: var(--warning-bg);
		border: 1px solid var(--warning-border);
		border-radius: 2px;
	}

	.suggested-fillup {
		color: var(--accent-success);
	}

	.suggested-indicator {
		font-size: 1rem;
	}
</style>
