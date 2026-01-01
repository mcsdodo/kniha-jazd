<script lang="ts">
	import type { Trip, Route, PreviewResult } from '$lib/types';
	import Autocomplete from './Autocomplete.svelte';
	import { confirmStore } from '$lib/stores/confirm';
	import LL from '$lib/i18n/i18n-svelte';

	export let trip: Trip | null = null;
	export let routes: Route[] = [];
	export let purposeSuggestions: string[] = [];
	export let isNew: boolean = false;
	export let previousOdometer: number = 0;
	export let consumptionRate: number = 0;
	export let zostatok: number = 0;
	export let defaultDate: string = new Date().toISOString().split('T')[0]; // For new rows
	export let onSave: (tripData: Partial<Trip>) => void;
	export let onCancel: () => void;
	export let onDelete: (id: string) => void;
	export let onInsertAbove: () => void = () => {};
	export let onEditStart: () => void = () => {};
	export let onEditEnd: () => void = () => {};
	export let onMoveUp: () => void = () => {};
	export let onMoveDown: () => void = () => {};
	export let canMoveUp: boolean = false;
	export let canMoveDown: boolean = false;
	export let hasDateWarning: boolean = false;
	export let hasConsumptionWarning: boolean = false;
	export let isEstimatedRate: boolean = false;
	export let hasMatchingReceipt: boolean = true;
	// Live preview props
	export let previewData: PreviewResult | null = null;
	export let onPreviewRequest: (km: number, fuel: number | null, fullTank: boolean) => void = () => {};

	let isEditing = isNew;
	let manualOdoEdit = false; // Track if user manually edited ODO

	// Form state - use null for new rows to show placeholder
	let formData = {
		date: trip?.date || defaultDate,
		origin: trip?.origin || '',
		destination: trip?.destination || '',
		distance_km: trip?.distance_km ?? (isNew ? null : 0),
		odometer: trip?.odometer ?? (isNew ? null : 0),
		purpose: trip?.purpose || '',
		fuel_liters: trip?.fuel_liters || null,
		fuel_cost_eur: trip?.fuel_cost_eur || null,
		other_costs_eur: trip?.other_costs_eur || null,
		other_costs_note: trip?.other_costs_note || '',
		full_tank: trip?.full_tank ?? true // Default to full tank
	};

	// Get unique locations from routes
	$: locationSuggestions = Array.from(
		new Set([...routes.map((r) => r.origin), ...routes.map((r) => r.destination)])
	).sort();

	// Find matching route and auto-fill distance
	function tryAutoFillDistance() {
		if (!formData.origin || !formData.destination) return;

		const matchingRoute = routes.find(
			(r) => r.origin === formData.origin && r.destination === formData.destination
		);

		if (matchingRoute && formData.distance_km === null) {
			formData.distance_km = matchingRoute.distance_km;
			// Also update ODO if not manually edited
			if (!manualOdoEdit) {
				formData.odometer = previousOdometer + matchingRoute.distance_km;
			}
		}
	}

	function handleOriginSelect(value: string) {
		formData.origin = value;
		tryAutoFillDistance();
	}

	function handleDestinationSelect(value: string) {
		formData.destination = value;
		tryAutoFillDistance();
	}

	// Auto-update ODO when km changes (unless user manually edited ODO)
	function handleKmChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		const km = inputValue === '' ? null : (parseFloat(inputValue) || 0);
		formData.distance_km = km;
		// Always auto-calculate ODO if not manually edited (previousOdometer can be 0)
		if (!manualOdoEdit && km !== null) {
			formData.odometer = previousOdometer + km;
		}
		// Request live preview calculation
		onPreviewRequest(km ?? 0, formData.fuel_liters, formData.full_tank);
	}

	// Request preview when fuel changes
	function handleFuelChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		formData.fuel_liters = inputValue === '' ? null : (parseFloat(inputValue) || null);
		onPreviewRequest(formData.distance_km ?? 0, formData.fuel_liters, formData.full_tank);
	}

	// Request preview when full_tank changes
	function handleFullTankChange() {
		onPreviewRequest(formData.distance_km ?? 0, formData.fuel_liters, formData.full_tank);
	}

	function handleOdoChange(event: Event) {
		manualOdoEdit = true;
		const inputValue = (event.target as HTMLInputElement).value;
		formData.odometer = inputValue === '' ? null : (parseFloat(inputValue) || 0);
	}

	function handleEdit() {
		isEditing = true;
		onEditStart();
		// Trigger preview immediately with current values
		onPreviewRequest(formData.distance_km ?? 0, formData.fuel_liters, formData.full_tank);
	}

	function handleSave() {
		// Ensure numeric fields have proper values (convert null to 0)
		const dataToSave = {
			...formData,
			distance_km: formData.distance_km ?? 0,
			odometer: formData.odometer ?? 0
		};
		onSave(dataToSave);
		isEditing = false;
		if (!isNew) {
			onEditEnd();
		}
	}

	function handleCancel() {
		if (isNew) {
			onCancel();
		} else {
			// Reset form data
			formData = {
				date: trip?.date || new Date().toISOString().split('T')[0],
				origin: trip?.origin || '',
				destination: trip?.destination || '',
				distance_km: trip?.distance_km || 0,
				odometer: trip?.odometer || 0,
				purpose: trip?.purpose || '',
				fuel_liters: trip?.fuel_liters || null,
				fuel_cost_eur: trip?.fuel_cost_eur || null,
				other_costs_eur: trip?.other_costs_eur || null,
				other_costs_note: trip?.other_costs_note || '',
				full_tank: trip?.full_tank ?? true
			};
			isEditing = false;
			onEditEnd();
		}
	}

	function handleDeleteClick() {
		if (trip?.id) {
			confirmStore.show({
				title: $LL.confirm.deleteRecordTitle(),
				message: $LL.confirm.deleteRecordMessage(),
				confirmText: $LL.common.delete(),
				danger: true,
				onConfirm: () => onDelete(trip!.id)
			});
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			handleSave();
		} else if (event.key === 'Escape') {
			handleCancel();
		}
	}

</script>

{#if isEditing}
	<tr class="editing" on:keydown={handleKeydown}>
		<td>
			<input type="date" bind:value={formData.date} />
		</td>
		<td>
			<Autocomplete
				bind:value={formData.origin}
				suggestions={locationSuggestions}
				placeholder={$LL.trips.originPlaceholder()}
				onSelect={handleOriginSelect}
			/>
		</td>
		<td>
			<Autocomplete
				bind:value={formData.destination}
				suggestions={locationSuggestions}
				placeholder={$LL.trips.destinationPlaceholder()}
				onSelect={handleDestinationSelect}
			/>
		</td>
		<td>
			<input type="number" value={formData.distance_km} on:input={handleKmChange} step="1" min="0" placeholder="0" />
		</td>
		<td>
			<input type="number" value={formData.odometer} on:input={handleOdoChange} step="1" min="0" placeholder="0" />
		</td>
		<td>
			<Autocomplete
				bind:value={formData.purpose}
				suggestions={purposeSuggestions}
				placeholder={$LL.trips.purposePlaceholder()}
				onSelect={(value) => (formData.purpose = value)}
			/>
		</td>
		<td class="fuel-cell">
			<input
				type="number"
				value={formData.fuel_liters}
				on:input={handleFuelChange}
				step="0.01"
				min="0"
				placeholder="0.00"
			/>
			{#if formData.fuel_liters}
				<label class="full-tank-label">
					<input type="checkbox" bind:checked={formData.full_tank} on:change={handleFullTankChange} />
					<span class="checkmark"></span>
					<span class="label-text">{$LL.trips.fullTank()}</span>
				</label>
			{/if}
		</td>
		<td>
			<input
				type="number"
				bind:value={formData.fuel_cost_eur}
				step="0.01"
				min="0"
				placeholder="0.00"
			/>
		</td>
		<td class="number calculated" class:preview={previewData} class:over-limit={previewData?.isOverLimit}>
			{#if previewData}
				~{previewData.consumptionRate.toFixed(2)}
				<span class="margin" class:over-limit={previewData.isOverLimit} class:within-limit={!previewData.isOverLimit}>
					({previewData.marginPercent >= 0 ? '+' : ''}{previewData.marginPercent.toFixed(0)}%)
				</span>
			{:else}
				{consumptionRate.toFixed(2)}
			{/if}
		</td>
		<td class="number calculated" class:preview={previewData}>
			{#if previewData}
				~{previewData.zostatok.toFixed(1)}
			{:else}
				{zostatok.toFixed(1)}
			{/if}
		</td>
		<td>
			<input
				type="number"
				bind:value={formData.other_costs_eur}
				step="0.01"
				min="0"
				placeholder="0.00"
			/>
		</td>
		<td>
			<input
				type="text"
				bind:value={formData.other_costs_note}
				placeholder=""
			/>
		</td>
		<td class="actions">
			<button class="save" on:click={handleSave}>{$LL.common.save()}</button>
			<button class="cancel" on:click={handleCancel}>{$LL.common.cancel()}</button>
		</td>
	</tr>
{:else if trip}
	<tr
		on:dblclick={handleEdit}
		class:date-warning={hasDateWarning}
		class:consumption-warning={hasConsumptionWarning}
	>
		<td>{new Date(trip.date).toLocaleDateString('sk-SK')}</td>
		<td>{trip.origin}</td>
		<td>{trip.destination}</td>
		<td class="number">{trip.distance_km.toFixed(0)}</td>
		<td class="number">{trip.odometer.toFixed(0)}</td>
		<td>{trip.purpose}</td>
		<td class="number">
			{#if trip.fuel_liters}
				{trip.fuel_liters.toFixed(2)}
				{#if !trip.full_tank}
					<span class="partial-indicator" title={$LL.trips.partialFillup()}>*</span>
				{/if}
				{#if !hasMatchingReceipt}
					<span class="no-receipt-indicator" title={$LL.trips.noReceipt()}>⚠</span>
				{/if}
			{/if}
		</td>
		<td class="number">{trip.fuel_cost_eur?.toFixed(2) || ''}</td>
		<td class="number calculated" class:estimated={isEstimatedRate}>
			{consumptionRate.toFixed(2)}
			{#if isEstimatedRate}
				<span class="estimated-indicator" title={$LL.trips.estimatedRate()}>~</span>
			{/if}
		</td>
		<td class="number calculated">{zostatok.toFixed(1)}</td>
		<td class="number">{trip.other_costs_eur?.toFixed(2) || ''}</td>
		<td>{trip.other_costs_note || ''}</td>
		<td class="actions">
			<span class="icon-actions">
				<button
					class="icon-btn move-up"
					on:click|stopPropagation={onMoveUp}
					title={$LL.trips.moveUp()}
					disabled={!canMoveUp}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="18 15 12 9 6 15"></polyline>
					</svg>
				</button>
				<button
					class="icon-btn move-down"
					on:click|stopPropagation={onMoveDown}
					title={$LL.trips.moveDown()}
					disabled={!canMoveDown}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="6 9 12 15 18 9"></polyline>
					</svg>
				</button>
				<button
					class="icon-btn insert"
					on:click|stopPropagation={onInsertAbove}
					title={$LL.trips.insertAbove()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<line x1="12" y1="5" x2="12" y2="19"></line>
						<line x1="5" y1="12" x2="19" y2="12"></line>
					</svg>
				</button>
				<button
					class="icon-btn delete"
					on:click|stopPropagation={handleDeleteClick}
					title={$LL.trips.deleteRecord()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="3 6 5 6 21 6"></polyline>
						<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
					</svg>
				</button>
			</span>
		</td>
	</tr>
{/if}

<style>
	tr {
		cursor: default;
		transition: background-color 0.2s;
	}

	tr:hover:not(.editing) {
		background-color: #f8f9fa;
		cursor: pointer;
	}

	tr.editing {
		background-color: #e3f2fd;
		cursor: default;
	}

	tr.editing td input,
	tr.editing td :global(.autocomplete) {
		margin: 0 0.125rem;
		width: calc(100% - 0.25rem);
	}

	tr.date-warning {
		background-color: #ffebee; /* light red */
	}

	tr.date-warning:hover:not(.editing) {
		background-color: #ffcdd2; /* slightly darker red on hover */
	}

	tr.consumption-warning {
		background-color: #fff3e0; /* light orange */
	}

	tr.consumption-warning:hover:not(.editing) {
		background-color: #ffe0b2; /* slightly darker orange on hover */
	}

	/* If both warnings apply, date warning takes priority */
	tr.date-warning.consumption-warning {
		background-color: #ffebee;
	}

	td {
		padding: 0.5rem;
		border-bottom: 1px solid #e0e0e0;
	}

	td.number {
		text-align: right;
	}

	td.calculated {
		color: #7f8c8d;
		font-style: italic;
	}

	/* Live preview styling */
	td.preview {
		opacity: 0.85;
	}

	td.over-limit {
		background-color: #fff3e0;
	}

	.margin {
		font-size: 0.75rem;
		margin-left: 0.25rem;
	}

	.margin.over-limit {
		color: #e74c3c;
		font-weight: 500;
	}

	.margin.within-limit {
		color: #27ae60;
	}

	td.actions {
		text-align: center;
		white-space: nowrap;
	}

	input {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.875rem;
		box-sizing: border-box;
	}

	input[type='number'] {
		text-align: right;
	}

	button {
		padding: 0.375rem 0.75rem;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
		margin: 0 0.25rem;
	}

	.save {
		background-color: #4caf50;
		color: white;
	}

	.save:hover {
		background-color: #45a049;
	}

	.cancel {
		background-color: #9e9e9e;
		color: white;
	}

	.cancel:hover {
		background-color: #757575;
	}

	.delete {
		background-color: #f44336;
		color: white;
	}

	.delete:hover {
		background-color: #da190b;
	}

	.icon-actions {
		display: flex;
		gap: 0.25rem;
		justify-content: center;
		align-items: center;
	}

	.icon-btn {
		background: none;
		border: none;
		padding: 0.25rem;
		cursor: pointer;
		color: #9e9e9e;
		border-radius: 4px;
		transition: color 0.2s, background-color 0.2s;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		margin: 0;
	}

	.icon-btn:hover {
		background-color: rgba(0, 0, 0, 0.05);
	}

	.icon-btn.insert:hover {
		color: #3498db;
	}

	.icon-btn.delete:hover {
		color: #f44336;
		background-color: rgba(244, 67, 54, 0.1);
	}

	.icon-btn.move-up:hover:not(:disabled),
	.icon-btn.move-down:hover:not(:disabled) {
		color: #2196f3;
	}

	.icon-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	/* Fuel cell with checkbox */
	.fuel-cell {
		position: relative;
	}

	.full-tank-label {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: #666;
		cursor: pointer;
	}

	.full-tank-label input[type='checkbox'] {
		width: auto;
		margin: 0;
		cursor: pointer;
	}

	.full-tank-label .label-text {
		white-space: nowrap;
	}

	/* Partial fillup indicator */
	.partial-indicator {
		color: #ff9800;
		font-weight: bold;
		margin-left: 0.25rem;
	}

	/* No receipt indicator */
	.no-receipt-indicator {
		color: #e67e22;
		margin-left: 0.25rem;
		cursor: help;
	}

	/* Estimated rate styling */
	td.estimated {
		color: #9e9e9e;
	}

	.estimated-indicator {
		color: #9e9e9e;
		margin-left: 0.125rem;
	}
</style>
