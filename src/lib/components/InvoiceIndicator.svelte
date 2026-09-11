<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import LL from '$lib/i18n/i18n-svelte';

	let count = $state(0);
	let loading = $state(true);

	onMount(() => {
		loadCount();
		const interval = setInterval(loadCount, 300000); // 5 min: hits the Paperless API
		return () => clearInterval(interval);
	});

	$effect(() => {
		const _vehicle = $activeVehicleStore;
		const _year = $selectedYearStore;
		loadCount();
	});

	async function loadCount() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) {
			count = 0;
			loading = false;
			return;
		}
		try {
			count = await api.countUnlinkedPaperlessFuelInvoices(vehicle.id, $selectedYearStore);
		} catch {
			count = 0;
		} finally {
			loading = false;
		}
	}
</script>

{#if !loading && count > 0}
	<span class="badge" title={$LL.doklady.paperless.unlinkedBadge()}>{count}</span>
{/if}

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.25rem;
		height: 1.25rem;
		padding: 0 0.375rem;
		background: var(--badge-danger-bg);
		color: var(--badge-danger-color);
		border-radius: 10px;
		font-size: 0.75rem;
		font-weight: 600;
		margin-left: 0.25rem;
	}
</style>
