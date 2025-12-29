<script lang="ts">
	import type { Receipt } from '$lib/types';
	import { getUnassignedReceipts } from '$lib/api';
	import { onMount } from 'svelte';

	interface Props {
		tripDate: string;
		onSelect: (receipt: Receipt) => void;
		onClose: () => void;
	}

	let { tripDate, onSelect, onClose }: Props = $props();

	let receipts = $state<Receipt[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	onMount(async () => {
		await loadReceipts();
	});

	async function loadReceipts() {
		loading = true;
		try {
			const all = await getUnassignedReceipts();
			// Sort by date proximity to trip
			receipts = all.sort((a, b) => {
				const aDiff = a.receipt_date
					? Math.abs(new Date(a.receipt_date).getTime() - new Date(tripDate).getTime())
					: Infinity;
				const bDiff = b.receipt_date
					? Math.abs(new Date(b.receipt_date).getTime() - new Date(tripDate).getTime())
					: Infinity;
				return aDiff - bDiff;
			});
		} catch (e) {
			console.error('Failed to load receipts:', e);
			error = 'Nepodarilo sa nacitat doklady';
		} finally {
			loading = false;
		}
	}

	function handleSelect(receipt: Receipt) {
		onSelect(receipt);
	}

	function formatDate(d: string | null): string {
		return d ? new Date(d).toLocaleDateString('sk-SK') : '--';
	}

	function isWithin3Days(receiptDate: string | null): boolean {
		if (!receiptDate) return false;
		const diff = Math.abs(new Date(receiptDate).getTime() - new Date(tripDate).getTime());
		return diff <= 3 * 24 * 60 * 60 * 1000;
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
		<h2>Vyber doklad pre jazdu</h2>
		<div class="trip-info">
			<span>Datum jazdy: {formatDate(tripDate)}</span>
		</div>

		{#if loading}
			<p class="placeholder">Nacitavam doklady...</p>
		{:else if error}
			<p class="error">{error}</p>
		{:else if receipts.length === 0}
			<p class="placeholder">Ziadne nepriradene doklady.</p>
		{:else}
			<div class="receipt-list">
				{#each receipts as receipt}
					{@const highlighted = isWithin3Days(receipt.receipt_date)}
					<button
						class="receipt-item"
						class:highlight={highlighted}
						onclick={() => handleSelect(receipt)}
					>
						<span class="filename">{receipt.file_name}</span>
						<span class="date">{formatDate(receipt.receipt_date)}</span>
						<span class="liters">{receipt.liters ?? '??'} L</span>
						<span class="price">{receipt.total_price_eur ?? '??'} EUR</span>
					</button>
				{/each}
			</div>
		{/if}

		<div class="modal-actions">
			<button class="button-secondary" onclick={onClose}>Zrusit</button>
			<button class="button-secondary" onclick={onClose}>Zadat manualne</button>
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

	.trip-info {
		background: #f5f5f5;
		padding: 0.75rem;
		border-radius: 4px;
		margin-bottom: 1rem;
	}

	.receipt-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 300px;
		overflow-y: auto;
	}

	.receipt-item {
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

	.receipt-item:hover {
		background: #f5f5f5;
	}

	.receipt-item.highlight {
		border-color: #3498db;
		background: #ebf5fb;
	}

	.receipt-item.highlight:hover {
		background: #d4e6f1;
	}

	.filename {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
		color: #2c3e50;
	}

	.date {
		color: #7f8c8d;
		min-width: 80px;
	}

	.liters {
		font-weight: 500;
		color: #27ae60;
		min-width: 60px;
	}

	.price {
		color: #34495e;
		min-width: 80px;
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
		gap: 0.5rem;
	}

	.button-secondary {
		padding: 0.5rem 1rem;
		background-color: #ecf0f1;
		color: #2c3e50;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button-secondary:hover {
		background-color: #d5dbdb;
	}
</style>
