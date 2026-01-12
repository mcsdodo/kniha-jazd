<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { receiptRefreshTrigger } from '$lib/stores/receipts';
	import LL from '$lib/i18n/i18n-svelte';

	let needsAttentionCount = $state(0);
	let loading = $state(true);

	onMount(() => {
		loadCount();
		// Refresh count every 30 seconds
		const interval = setInterval(loadCount, 30000);
		return () => clearInterval(interval);
	});

	// Reload when vehicle, year, or refresh trigger changes
	$effect(() => {
		// Access all reactive dependencies
		const _vehicle = $activeVehicleStore;
		const _year = $selectedYearStore;
		const _trigger = $receiptRefreshTrigger;
		loadCount();
	});

	async function loadCount() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			needsAttentionCount = 0;
			loading = false;
			return;
		}

		try {
			// Get verification from backend (includes unmatched count)
			const verification = await api.verifyReceipts(vehicle.id, $selectedYearStore);

			// Use backend's unmatched count directly (ADR-008: no frontend calculations)
			needsAttentionCount = verification.unmatched;
		} catch (error) {
			console.error('Failed to load receipt count:', error);
			needsAttentionCount = 0;
		} finally {
			loading = false;
		}
	}
</script>

{#if !loading && needsAttentionCount > 0}
	<span class="badge" title={$LL.receipts.filterNeedsReview()}>{needsAttentionCount}</span>
{/if}

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.25rem;
		height: 1.25rem;
		padding: 0 0.375rem;
		background: #e74c3c;
		color: white;
		border-radius: 10px;
		font-size: 0.75rem;
		font-weight: 600;
		margin-left: 0.25rem;
	}
</style>
