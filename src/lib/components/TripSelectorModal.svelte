<script lang="ts">
	import type { Trip, Receipt, TripForAssignment, MismatchReason } from '$lib/types';
	import { getTripsForReceiptAssignment } from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { onMount } from 'svelte';
	import LL from '$lib/i18n/i18n-svelte';

	interface Props {
		receipt: Receipt;
		onSelect: (trip: Trip) => void;
		onClose: () => void;
	}

	let { receipt, onSelect, onClose }: Props = $props();

	let tripItems = $state<TripForAssignment[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		await loadTrips();
	});

	async function loadTrips() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			error = $LL.tripSelector.noVehicleSelected();
			loading = false;
			return;
		}

		loading = true;
		try {
			// Use the new API that returns trips with attachment eligibility
			const items = await getTripsForReceiptAssignment(
				receipt.id,
				vehicle.id,
				$selectedYearStore
			);
			// Sort by date proximity to receipt date
			tripItems = items.sort((a, b) => {
				const aDiff = dateProximity(a.trip.date, receipt.receiptDate);
				const bDiff = dateProximity(b.trip.date, receipt.receiptDate);
				return aDiff - bDiff;
			});
		} catch (e) {
			console.error('Failed to load trips:', e);
			error = $LL.tripSelector.loadError();
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

	function handleTripClick(item: TripForAssignment) {
		if (item.canAttach) {
			onSelect(item.trip);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClose();
		}
	}

	function getMismatchReasonText(reason: MismatchReason | null): string {
		if (!reason) return '';
		switch (reason) {
			case 'date': return $LL.tripSelector.mismatchDate();
			case 'liters': return $LL.tripSelector.mismatchLiters();
			case 'price': return $LL.tripSelector.mismatchPrice();
			case 'liters_and_price': return $LL.tripSelector.mismatchLitersAndPrice();
			case 'date_and_liters': return $LL.tripSelector.mismatchDateAndLiters();
			case 'date_and_price': return $LL.tripSelector.mismatchDateAndPrice();
			case 'all': return $LL.tripSelector.mismatchAll();
			default: return '';
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
		<h2>{$LL.tripSelector.title()}</h2>
		<div class="receipt-info">
			<span class="file-name">{receipt.fileName}</span>
			<span class="separator">|</span>
			<span>{receipt.liters?.toFixed(2) ?? '??'} L</span>
			<span class="separator">|</span>
			<span>{receipt.totalPriceEur?.toFixed(2) ?? '??'} EUR</span>
			{#if receipt.receiptDate}
				<span class="separator">|</span>
				<span>{formatDate(receipt.receiptDate)}</span>
			{/if}
		</div>

		{#if loading}
			<p class="placeholder">{$LL.tripSelector.loadingTrips()}</p>
		{:else if error}
			<p class="error">{error}</p>
		{:else if tripItems.length === 0}
			<p class="placeholder">{$LL.tripSelector.noTrips()}</p>
		{:else}
			<div class="trip-list">
				{#each tripItems as item}
					{@const disabled = !item.canAttach}
					{@const highlighted = isWithin3Days(item.trip.date, receipt.receiptDate)}
					<button
						class="trip-item"
						class:highlight={highlighted}
						class:disabled
						class:matches={item.attachmentStatus === 'matches'}
						onclick={() => handleTripClick(item)}
						{disabled}
					>
						<span class="date">{formatDate(item.trip.date)}</span>
						<span class="route">{item.trip.origin} → {item.trip.destination}</span>
						{#if item.attachmentStatus === 'matches'}
							<span class="match-indicator">✓ {$LL.tripSelector.matchesReceipt()}</span>
						{:else if item.attachmentStatus === 'differs'}
							<span class="existing">
								{item.trip.fuelLiters?.toFixed(2)} L — {getMismatchReasonText(item.mismatchReason)}
							</span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}

		<div class="modal-actions">
			<button class="button-small" onclick={onClose}>{$LL.common.cancel()}</button>
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
		background: var(--overlay-bg);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: var(--bg-surface);
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
		color: var(--text-primary);
	}

	.receipt-info {
		background: var(--bg-surface-alt);
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
		color: var(--text-muted);
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
		border: 1px solid var(--border-input);
		border-radius: 4px;
		background: var(--bg-surface);
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: background-color 0.2s;
		color: var(--text-primary);
	}

	.trip-item:hover:not(:disabled) {
		background: var(--bg-surface-alt);
	}

	.trip-item.highlight {
		border-color: var(--accent-primary);
		background: var(--accent-primary-light-bg);
	}

	.trip-item.highlight:hover:not(:disabled) {
		background: var(--accent-primary-light-hover);
	}

	.trip-item.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.trip-item.matches {
		border-color: var(--accent-success, #22c55e);
		background: var(--accent-success-light-bg, #f0fdf4);
	}

	.trip-item.matches:hover:not(:disabled) {
		background: var(--accent-success-light-hover, #dcfce7);
	}

	.date {
		font-weight: 500;
		min-width: 80px;
		color: var(--text-primary);
	}

	.route {
		flex: 1;
		color: var(--text-primary);
	}

	.existing {
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.match-indicator {
		color: var(--accent-success, #22c55e);
		font-size: 0.875rem;
		font-weight: 500;
	}

	.placeholder {
		color: var(--text-secondary);
		font-style: italic;
		text-align: center;
		padding: 1rem;
	}

	.error {
		color: var(--accent-danger);
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
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}
</style>
