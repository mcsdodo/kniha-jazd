<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import LL from '$lib/i18n/i18n-svelte';

	let needsAttentionCount = $state(0);
	let loading = $state(true);

	onMount(() => {
		loadCount();
		// Refresh count every 30 seconds
		const interval = setInterval(loadCount, 30000);
		return () => clearInterval(interval);
	});

	// Reload when vehicle or year changes
	$effect(() => {
		if ($activeVehicleStore || $selectedYearStore) {
			loadCount();
		}
	});

	async function loadCount() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			needsAttentionCount = 0;
			loading = false;
			return;
		}

		try {
			// Get receipts and verification
			const [receipts, verification] = await Promise.all([
				api.getReceipts(),
				api.verifyReceipts(vehicle.id, $selectedYearStore)
			]);

			// Count: unverified + needs_review
			const needsReviewCount = receipts.filter(r => r.status === 'NeedsReview').length;
			const unverifiedCount = verification.unmatched;

			needsAttentionCount = needsReviewCount + unverifiedCount;
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
