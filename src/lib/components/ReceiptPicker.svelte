<script lang="ts">
	import type { Receipt } from '$lib/types';
	import { getUnassignedReceipts } from '$lib/api';

	export let tripDate: string;
	export let onSelect: (receipt: Receipt) => void;

	let receipts: Receipt[] = [];
	let open = false;
	let loading = false;

	async function handleOpen() {
		open = true;
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
		} catch (error) {
			console.error('Failed to load receipts:', error);
		} finally {
			loading = false;
		}
	}

	function handleSelect(receipt: Receipt) {
		onSelect(receipt);
		open = false;
	}

	function handleClose() {
		open = false;
	}

	function formatDate(d: string | null): string {
		return d ? new Date(d).toLocaleDateString('sk-SK') : '--';
	}

	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.receipt-picker')) {
			open = false;
		}
	}
</script>

<svelte:window on:click={handleClickOutside} />

<div class="receipt-picker">
	<button type="button" class="picker-button" on:click|stopPropagation={handleOpen}>
		Doklad
	</button>

	{#if open}
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="dropdown" on:click|stopPropagation>
			{#if loading}
				<div class="dropdown-item loading">Nacitavam...</div>
			{:else if receipts.length === 0}
				<div class="dropdown-item empty">Ziadne nepriradene doklady</div>
			{:else}
				{#each receipts.slice(0, 5) as receipt}
					<button class="dropdown-item" on:click={() => handleSelect(receipt)}>
						<span class="filename">{receipt.file_name}</span>
						<span class="date">{formatDate(receipt.receipt_date)}</span>
						<span class="liters">{receipt.liters?.toFixed(1) ?? '??'} L</span>
					</button>
				{/each}
			{/if}
			<button class="dropdown-item manual" on:click={handleClose}> Zadat manualne </button>
		</div>
	{/if}
</div>

<style>
	.receipt-picker {
		position: relative;
		display: inline-block;
	}

	.picker-button {
		padding: 0.25rem 0.5rem;
		font-size: 0.7rem;
		background: #ecf0f1;
		border: 1px solid #ddd;
		border-radius: 4px;
		cursor: pointer;
		white-space: nowrap;
	}

	.picker-button:hover {
		background: #d5dbdb;
	}

	.dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		background: white;
		border: 1px solid #ddd;
		border-radius: 4px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
		z-index: 100;
		min-width: 220px;
		max-height: 200px;
		overflow-y: auto;
	}

	.dropdown-item {
		display: flex;
		gap: 0.5rem;
		padding: 0.5rem;
		width: 100%;
		text-align: left;
		border: none;
		background: none;
		cursor: pointer;
		border-bottom: 1px solid #eee;
		font-size: 0.75rem;
	}

	.dropdown-item:last-child {
		border-bottom: none;
	}

	.dropdown-item:hover {
		background: #f5f5f5;
	}

	.dropdown-item.loading,
	.dropdown-item.empty {
		color: #7f8c8d;
		font-style: italic;
		cursor: default;
	}

	.dropdown-item.manual {
		color: #7f8c8d;
		font-style: italic;
		justify-content: center;
	}

	.filename {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.date {
		color: #7f8c8d;
	}

	.liters {
		font-weight: 500;
	}
</style>
