<script lang="ts">
	import type { Trip, Route } from '$lib/types';
	import Autocomplete from './Autocomplete.svelte';

	export let trip: Trip | null = null;
	export let routes: Route[] = [];
	export let isNew: boolean = false;
	export let previousOdometer: number = 0;
	export let consumptionRate: number = 0;
	export let zostatok: number = 0;
	export let onSave: (tripData: Partial<Trip>) => void;
	export let onCancel: () => void;
	export let onDelete: (id: string) => void;

	let isEditing = isNew;
	let manualOdoEdit = false; // Track if user manually edited ODO

	// Form state
	let formData = {
		date: trip?.date || new Date().toISOString().split('T')[0],
		origin: trip?.origin || '',
		destination: trip?.destination || '',
		distance_km: trip?.distance_km || 0,
		odometer: trip?.odometer || 0,
		purpose: trip?.purpose || '',
		fuel_liters: trip?.fuel_liters || null,
		fuel_cost_eur: trip?.fuel_cost_eur || null,
		other_costs_eur: trip?.other_costs_eur || null
	};

	// Get unique locations from routes
	$: locationSuggestions = Array.from(
		new Set([...routes.map((r) => r.origin), ...routes.map((r) => r.destination)])
	).sort();

	// Auto-update ODO when km changes (unless user manually edited ODO)
	function handleKmChange(event: Event) {
		const km = parseFloat((event.target as HTMLInputElement).value) || 0;
		formData.distance_km = km;
		// Always auto-calculate ODO if not manually edited (previousOdometer can be 0)
		if (!manualOdoEdit) {
			formData.odometer = previousOdometer + km;
		}
	}

	function handleOdoChange(event: Event) {
		manualOdoEdit = true;
		formData.odometer = parseFloat((event.target as HTMLInputElement).value) || 0;
	}

	function handleEdit() {
		isEditing = true;
	}

	function handleSave() {
		onSave(formData);
		isEditing = false;
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
				other_costs_eur: trip?.other_costs_eur || null
			};
			isEditing = false;
		}
	}

	function handleDeleteClick() {
		if (trip?.id && confirm('Naozaj chcete odstrániť tento záznam?')) {
			onDelete(trip.id);
		}
	}
</script>

{#if isEditing}
	<tr class="editing">
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
			<input type="number" value={formData.distance_km} on:input={handleKmChange} step="0.1" min="0" />
		</td>
		<td>
			<input type="number" value={formData.odometer} on:input={handleOdoChange} step="0.1" min="0" />
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
		<td class="actions">
			<button class="save" on:click={handleSave}>Uložiť</button>
			<button class="cancel" on:click={handleCancel}>Zrušiť</button>
		</td>
	</tr>
{:else if trip}
	<tr on:dblclick={handleEdit}>
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
		<td class="actions">
			<button class="delete" on:click|stopPropagation={handleDeleteClick}>Odstrániť</button>
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
</style>
