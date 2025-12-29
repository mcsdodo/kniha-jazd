<script lang="ts">
	import type { Trip, Receipt, TripGridData } from '$lib/types';
	import { getTripGridData } from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { onMount } from 'svelte';

	interface Props {
		receipt: Receipt;
		onSelect: (trip: Trip) => void;
		onClose: () => void;
	}

	let { receipt, onSelect, onClose }: Props = $props();

	let trips = $state<Trip[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		await loadTrips();
	});

	async function loadTrips() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			error = 'Nie je vybraté vozidlo';
			loading = false;
			return;
		}

		loading = true;
		try {
			const gridData: TripGridData = await getTripGridData(vehicle.id, $selectedYearStore);
			// Sort by date proximity to receipt date
			trips = gridData.trips.sort((a, b) => {
				const aDiff = dateProximity(a.date, receipt.receipt_date);
				const bDiff = dateProximity(b.date, receipt.receipt_date);
				return aDiff - bDiff;
			});
		} catch (e) {
			console.error('Failed to load trips:', e);
			error = 'Nepodarilo sa načítať jazdy';
		} finally {
			loading = false;
		}
	}

	function dateProximity(tripDate: string, receiptDate: string | null): number {
		if (!receiptDate) return Infinity;
		const t = new Date(tripDate).getTime();
		const r = new Date(receiptDate).getTime();
		return Math.abs(t - r);
	}

	function isWithin3Days(tripDate: string, receiptDate: string | null): boolean {
		if (!receiptDate) return false;
		const diff = Math.abs(new Date(tripDate).getTime() - new Date(receiptDate).getTime());
		return diff <= 3 * 24 * 60 * 60 * 1000; // 3 days in ms
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('sk-SK');
	}

	function hasFuel(trip: Trip): boolean {
		return trip.fuel_liters != null && trip.fuel_liters > 0;
	}

	function handleTripClick(trip: Trip) {
		if (!hasFuel(trip)) {
			onSelect(trip);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClose();
		}
	}
</script>

<div
	class="modal-overlay"
	onclick={onClose}
	onkeydown={handleKeydown}
	role="button"
	tabindex="0"
>
	<div
		class="modal"
		onclick={(e) => e.stopPropagation()}
		onkeydown={() => {}}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<h2>Prideliť doklad k jazde</h2>
		<div class="receipt-info">
			<span class="file-name">{receipt.file_name}</span>
			<span class="separator">|</span>
			<span>{receipt.liters?.toFixed(2) ?? '??'} L</span>
			<span class="separator">|</span>
			<span>{receipt.total_price_eur?.toFixed(2) ?? '??'} EUR</span>
			{#if receipt.receipt_date}
				<span class="separator">|</span>
				<span>{formatDate(receipt.receipt_date)}</span>
			{/if}
		</div>

		{#if loading}
			<p class="placeholder">Načítavam jazdy...</p>
		{:else if error}
			<p class="error">{error}</p>
		{:else if trips.length === 0}
			<p class="placeholder">Žiadne jazdy na pridelenie.</p>
		{:else}
			<div class="trip-list">
				{#each trips as trip}
					{@const disabled = hasFuel(trip)}
					{@const highlighted = isWithin3Days(trip.date, receipt.receipt_date)}
					<button
						class="trip-item"
						class:highlight={highlighted}
						class:disabled
						onclick={() => handleTripClick(trip)}
						{disabled}
					>
						<span class="date">{formatDate(trip.date)}</span>
						<span class="route">{trip.origin} → {trip.destination}</span>
						{#if disabled}
							<span class="existing">už má: {trip.fuel_liters?.toFixed(2)} L</span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}

		<div class="modal-actions">
			<button class="button-small" onclick={onClose}>Zrušiť</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		max-width: 500px;
		width: 90%;
		max-height: 80vh;
		overflow-y: auto;
	}

	.modal h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.receipt-info {
		background: #f5f5f5;
		padding: 0.75rem;
		border-radius: 4px;
		margin-bottom: 1rem;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: center;
	}

	.file-name {
		font-weight: 500;
	}

	.separator {
		color: #bdc3c7;
	}

	.trip-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 300px;
		overflow-y: auto;
	}

	.trip-item {
		display: flex;
		gap: 1rem;
		padding: 0.75rem;
		border: 1px solid #ddd;
		border-radius: 4px;
		background: white;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: background-color 0.2s;
	}

	.trip-item:hover:not(:disabled) {
		background: #f5f5f5;
	}

	.trip-item.highlight {
		border-color: #3498db;
		background: #ebf5fb;
	}

	.trip-item.highlight:hover:not(:disabled) {
		background: #d4e6f1;
	}

	.trip-item.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.date {
		font-weight: 500;
		min-width: 80px;
		color: #2c3e50;
	}

	.route {
		flex: 1;
		color: #34495e;
	}

	.existing {
		color: #7f8c8d;
		font-size: 0.875rem;
	}

	.placeholder {
		color: #7f8c8d;
		font-style: italic;
		text-align: center;
		padding: 1rem;
	}

	.error {
		color: #c0392b;
		text-align: center;
		padding: 1rem;
	}

	.modal-actions {
		margin-top: 1rem;
		display: flex;
		justify-content: flex-end;
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: #ecf0f1;
		color: #2c3e50;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button-small:hover {
		background-color: #d5dbdb;
	}
</style>
