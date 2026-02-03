<script lang="ts">
	import type { Trip, Receipt, TripForAssignment, MismatchReason, AssignmentType } from '$lib/types';
	import { getTripsForReceiptAssignment } from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { onMount } from 'svelte';
	import LL from '$lib/i18n/i18n-svelte';

	interface AssignmentResult {
		trip: Trip;
		assignmentType: AssignmentType;
		mismatchOverride: boolean;
	}

	interface Props {
		receipt: Receipt;
		onSelect: (result: AssignmentResult) => void;
		onClose: () => void;
	}

	let { receipt, onSelect, onClose }: Props = $props();

	// Step 1: Trip list, Step 2: Assignment type selection
	let step = $state<'tripList' | 'assignmentType'>('tripList');
	let selectedTrip = $state<TripForAssignment | null>(null);
	let assignmentType = $state<AssignmentType>('Fuel');

	let tripItems = $state<TripForAssignment[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Determine if this receipt looks like fuel (has liters)
	let looksLikeFuel = $derived(receipt.liters !== null && receipt.liters > 0);

	// Detect if there's a mismatch when assigning as FUEL
	let hasMismatch = $derived(() => {
		if (!selectedTrip || assignmentType !== 'Fuel') return false;
		// If trip has fuel and status is 'differs', there's a mismatch
		return selectedTrip.attachmentStatus === 'differs';
	});

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
			const items = await getTripsForReceiptAssignment(
				receipt.id,
				vehicle.id,
				$selectedYearStore
			);
			// Sort by date proximity to receipt date
			tripItems = items.sort((a, b) => {
				const aDiff = dateProximity(getTripDate(a.trip), receipt.receiptDatetime);
				const bDiff = dateProximity(getTripDate(b.trip), receipt.receiptDatetime);
				return aDiff - bDiff;
			});
		} catch (e) {
			console.error('Failed to load trips:', e);
			error = $LL.tripSelector.loadError();
		} finally {
			loading = false;
		}
	}

	function getTripDate(trip: Trip): string {
		return trip.startDatetime.slice(0, 10);
	}

	function dateProximity(tripDate: string, receiptDatetime: string | null): number {
		if (!receiptDatetime) return Infinity;
		const t = new Date(tripDate).getTime();
		const r = new Date(receiptDatetime).getTime();
		return Math.abs(t - r);
	}

	function isWithin3Days(tripDate: string, receiptDatetime: string | null): boolean {
		if (!receiptDatetime) return false;
		const diff = Math.abs(new Date(tripDate).getTime() - new Date(receiptDatetime).getTime());
		return diff <= 3 * 24 * 60 * 60 * 1000;
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('sk-SK');
	}

	function handleTripClick(item: TripForAssignment) {
		if (!item.canAttach) return;
		selectedTrip = item;
		// Pre-select based on receipt type
		assignmentType = looksLikeFuel ? 'Fuel' : 'Other';
		step = 'assignmentType';
	}

	function handleBack() {
		step = 'tripList';
		selectedTrip = null;
	}

	function handleAssign(override: boolean = false) {
		if (!selectedTrip) return;
		onSelect({
			trip: selectedTrip.trip,
			assignmentType,
			mismatchOverride: override
		});
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			if (step === 'assignmentType') {
				handleBack();
			} else {
				onClose();
			}
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
		{#if step === 'tripList'}
			<!-- Step 1: Select trip -->
			<h2>{$LL.tripSelector.title()}</h2>
			<div class="receipt-info">
				<span class="file-name">{receipt.fileName}</span>
				<span class="separator">|</span>
				<span>{receipt.liters?.toFixed(2) ?? '??'} L</span>
				<span class="separator">|</span>
				<span>{receipt.totalPriceEur?.toFixed(2) ?? '??'} EUR</span>
				{#if receipt.receiptDatetime}
					<span class="separator">|</span>
					<span>{formatDate(receipt.receiptDatetime)}</span>
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
						{@const highlighted = isWithin3Days(getTripDate(item.trip), receipt.receiptDatetime)}
						<button
							class="trip-item"
							class:highlight={highlighted}
							class:disabled
							class:matches={item.attachmentStatus === 'matches'}
							onclick={() => handleTripClick(item)}
							{disabled}
						>
							<span class="date">{formatDate(getTripDate(item.trip))}</span>
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

		{:else if step === 'assignmentType' && selectedTrip}
			<!-- Step 2: Select assignment type -->
			<h2>{$LL.tripSelector.selectType()}</h2>

			<div class="selected-trip-info">
				<span class="date">{formatDate(getTripDate(selectedTrip.trip))}</span>
				<span class="route">{selectedTrip.trip.origin} → {selectedTrip.trip.destination}</span>
			</div>

			<div class="assignment-type-selector">
				<label class="type-option" class:selected={assignmentType === 'Fuel'}>
					<input
						type="radio"
						name="assignmentType"
						value="Fuel"
						bind:group={assignmentType}
					/>
					<span class="type-icon">⛽</span>
					<span class="type-label">{$LL.tripSelector.assignAsFuel()}</span>
				</label>
				<label class="type-option" class:selected={assignmentType === 'Other'}>
					<input
						type="radio"
						name="assignmentType"
						value="Other"
						bind:group={assignmentType}
					/>
					<span class="type-icon">📄</span>
					<span class="type-label">{$LL.tripSelector.assignAsOther()}</span>
				</label>
			</div>

			{#if assignmentType === 'Fuel' && selectedTrip.attachmentStatus === 'differs'}
				<!-- Mismatch warning for FUEL assignment -->
				<div class="mismatch-warning">
					<div class="warning-header">
						<span class="warning-icon">⚠</span>
						<span>{$LL.tripSelector.dataMismatch()}</span>
					</div>
					<div class="mismatch-details">
						{getMismatchReasonText(selectedTrip.mismatchReason)}
					</div>
					<div class="mismatch-actions">
						<button class="button-small" onclick={handleBack}>{$LL.common.cancel()}</button>
						<button class="button-small warning" onclick={() => handleAssign(false)}>
							{$LL.tripSelector.assignWithWarning()}
						</button>
						<button class="button-small primary" onclick={() => handleAssign(true)}>
							{$LL.tripSelector.assignAndConfirm()}
						</button>
					</div>
				</div>
			{:else}
				<!-- Normal assignment -->
				<div class="modal-actions">
					<button class="button-small" onclick={handleBack}>{$LL.common.cancel()}</button>
					<button class="button-small primary" onclick={() => handleAssign(false)}>
						{$LL.tripSelector.confirmAssignment()}
					</button>
				</div>
			{/if}
		{/if}
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
		gap: 0.5rem;
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

	.button-small.primary {
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}

	.button-small.primary:hover {
		background-color: var(--btn-active-primary-hover);
	}

	.button-small.warning {
		background-color: var(--warning-bg);
		color: var(--warning-color);
		border: 1px solid var(--warning-border);
	}

	.button-small.warning:hover {
		background-color: var(--warning-border);
	}

	/* Step 2: Assignment type selector */
	.selected-trip-info {
		background: var(--bg-surface-alt);
		padding: 0.75rem;
		border-radius: 4px;
		margin-bottom: 1rem;
		display: flex;
		gap: 1rem;
		align-items: center;
	}

	.assignment-type-selector {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.type-option {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		border: 2px solid var(--border-input);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.type-option:hover {
		border-color: var(--accent-primary);
		background: var(--bg-surface-alt);
	}

	.type-option.selected {
		border-color: var(--accent-primary);
		background: var(--accent-primary-light-bg);
	}

	.type-option input[type="radio"] {
		margin: 0;
	}

	.type-icon {
		font-size: 1.25rem;
	}

	.type-label {
		font-weight: 500;
		color: var(--text-primary);
	}

	/* Mismatch warning */
	.mismatch-warning {
		background: var(--warning-bg);
		border: 1px solid var(--warning-border);
		border-radius: 6px;
		padding: 1rem;
		margin-top: 1rem;
	}

	.warning-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
		color: var(--warning-color);
		margin-bottom: 0.5rem;
	}

	.warning-icon {
		font-size: 1.25rem;
	}

	.mismatch-details {
		color: var(--text-primary);
		font-size: 0.875rem;
		margin-bottom: 1rem;
	}

	.mismatch-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		justify-content: flex-end;
	}
</style>
