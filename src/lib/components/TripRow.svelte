<script lang="ts">
	import type { Trip, Route } from '$lib/types';
	import Autocomplete from './Autocomplete.svelte';

	export let trip: Trip | null = null;
	export let routes: Route[] = [];
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
	export let dragDisabled: boolean = false;
	export let tripId: string = '';
	export let onDragStart: (e: DragEvent) => void = () => {};
	export let onDragEnd: () => void = () => {};
	export let onDragOver: (e: DragEvent) => void = () => {};
	export let onDragLeave: () => void = () => {};
	export let onDrop: (e: DragEvent) => void = () => {};
	export let isDragTarget: boolean = false;
	export let isDragging: boolean = false;

	function handleDragStart(e: DragEvent) {
		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/plain', tripId);
			// Create a drag image from the parent row
			const row = (e.target as HTMLElement).closest('tr');
			if (row) {
				e.dataTransfer.setDragImage(row, 50, 20);
			}
		}
		onDragStart(e);
	}

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
		other_costs_note: trip?.other_costs_note || ''
	};

	// Get unique locations from routes
	$: locationSuggestions = Array.from(
		new Set([...routes.map((r) => r.origin), ...routes.map((r) => r.destination)])
	).sort();

	// Auto-update ODO when km changes (unless user manually edited ODO)
	function handleKmChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		const km = inputValue === '' ? null : (parseFloat(inputValue) || 0);
		formData.distance_km = km;
		// Always auto-calculate ODO if not manually edited (previousOdometer can be 0)
		if (!manualOdoEdit && km !== null) {
			formData.odometer = previousOdometer + km;
		}
	}

	function handleOdoChange(event: Event) {
		manualOdoEdit = true;
		const inputValue = (event.target as HTMLInputElement).value;
		formData.odometer = inputValue === '' ? null : (parseFloat(inputValue) || 0);
	}

	function handleEdit() {
		isEditing = true;
		onEditStart();
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
				other_costs_note: trip?.other_costs_note || ''
			};
			isEditing = false;
			onEditEnd();
		}
	}

	function handleDeleteClick() {
		if (trip?.id && confirm('Naozaj chcete odstrániť tento záznam?')) {
			onDelete(trip.id);
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
				placeholder="Odkiaľ"
				onSelect={(value) => (formData.origin = value)}
			/>
		</td>
		<td>
			<Autocomplete
				bind:value={formData.destination}
				suggestions={locationSuggestions}
				placeholder="Kam"
				onSelect={(value) => (formData.destination = value)}
			/>
		</td>
		<td>
			<input type="number" value={formData.distance_km} on:input={handleKmChange} step="0.1" min="0" placeholder="0.0" />
		</td>
		<td>
			<input type="number" value={formData.odometer} on:input={handleOdoChange} step="0.1" min="0" placeholder="0.0" />
		</td>
		<td>
			<input type="text" bind:value={formData.purpose} placeholder="Účel" />
		</td>
		<td>
			<input
				type="number"
				bind:value={formData.fuel_liters}
				step="0.01"
				min="0"
				placeholder="0.00"
			/>
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
		<td class="number calculated">
			{consumptionRate.toFixed(2)}
		</td>
		<td class="number calculated">
			{zostatok.toFixed(1)}
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
			<button class="save" on:click={handleSave}>Uložiť</button>
			<button class="cancel" on:click={handleCancel}>Zrušiť</button>
		</td>
	</tr>
{:else if trip}
	<tr
		draggable={!dragDisabled}
		on:dragstart={handleDragStart}
		on:dragend={onDragEnd}
		on:dblclick={handleEdit}
		on:dragover={onDragOver}
		on:dragleave={onDragLeave}
		on:drop={onDrop}
		class:drag-target={isDragTarget}
		class:dragging={isDragging}
	>
		<td>{new Date(trip.date).toLocaleDateString('sk-SK')}</td>
		<td>{trip.origin}</td>
		<td>{trip.destination}</td>
		<td class="number">{trip.distance_km.toFixed(1)}</td>
		<td class="number">{trip.odometer.toFixed(1)}</td>
		<td>{trip.purpose}</td>
		<td class="number">{trip.fuel_liters?.toFixed(2) || ''}</td>
		<td class="number">{trip.fuel_cost_eur?.toFixed(2) || ''}</td>
		<td class="number calculated">{consumptionRate.toFixed(2)}</td>
		<td class="number calculated">{zostatok.toFixed(1)}</td>
		<td class="number">{trip.other_costs_eur?.toFixed(2) || ''}</td>
		<td>{trip.other_costs_note || ''}</td>
		<td class="actions">
			<span class="icon-actions">
				<button
					class="icon-btn insert"
					on:click|stopPropagation={onInsertAbove}
					title="Vložiť záznam nad"
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<line x1="12" y1="5" x2="12" y2="19"></line>
						<line x1="5" y1="12" x2="19" y2="12"></line>
					</svg>
				</button>
				<button
					class="icon-btn delete"
					on:click|stopPropagation={handleDeleteClick}
					title="Odstrániť záznam"
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="3 6 5 6 21 6"></polyline>
						<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
					</svg>
				</button>
				{#if !dragDisabled}
					<span
						class="drag-handle"
						title="Presunúť záznam"
						draggable="true"
						role="button"
						tabindex="0"
						on:dragstart={handleDragStart}
						on:dragend={onDragEnd}
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<circle cx="9" cy="5" r="1"></circle>
							<circle cx="9" cy="12" r="1"></circle>
							<circle cx="9" cy="19" r="1"></circle>
							<circle cx="15" cy="5" r="1"></circle>
							<circle cx="15" cy="12" r="1"></circle>
							<circle cx="15" cy="19" r="1"></circle>
						</svg>
					</span>
				{/if}
			</span>
		</td>
	</tr>
{/if}

<style>
	tr {
		cursor: default;
		transition: background-color 0.2s;
	}

	tr:hover:not(.editing):not(.dragging) {
		background-color: #f8f9fa;
		cursor: pointer;
	}

	tr.drag-target {
		box-shadow: inset 0 3px 0 0 #3498db;
	}

	tr.dragging {
		opacity: 0.4;
		background-color: #e0e0e0;
	}

	tr.editing {
		background-color: #e3f2fd;
		cursor: default;
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

	td.actions {
		text-align: center;
		white-space: nowrap;
	}

	input {
		width: 100%;
		padding: 0.375rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.8rem;
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

	.drag-handle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.25rem;
		color: #9e9e9e;
		cursor: grab;
		border-radius: 4px;
		transition: color 0.2s, background-color 0.2s;
	}

	.drag-handle:hover {
		color: #616161;
		background-color: rgba(0, 0, 0, 0.05);
	}

	.drag-handle:active {
		cursor: grabbing;
	}
</style>
