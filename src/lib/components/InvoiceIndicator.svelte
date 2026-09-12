<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { invoiceRefreshTrigger } from '$lib/stores/invoices';
	import LL from '$lib/i18n/i18n-svelte';

	let count = $state(0);
	let loading = $state(true);

	// Guards against out-of-order responses: a vehicle switch writes the vehicle
	// and the year separately, so two counts can be in flight and the slower one
	// must not overwrite the newer.
	let latestRequest = 0;

	onMount(() => {
		// No initial call here -- the $effect below runs on mount.
		const interval = setInterval(loadCount, 300000); // 5 min: hits the Paperless API
		return () => clearInterval(interval);
	});

	$effect(() => {
		const _vehicle = $activeVehicleStore;
		const _year = $selectedYearStore;
		const _trigger = $invoiceRefreshTrigger;
		loadCount();
	});

	async function loadCount() {
		const vehicle = $activeVehicleStore;
		const request = ++latestRequest;
		if (!vehicle) {
			count = 0;
			loading = false;
			return;
		}
		try {
			const next = await api.countUnlinkedPaperlessFuelInvoices(vehicle.id, $selectedYearStore);
			if (request !== latestRequest) return;
			count = next;
		} catch (error) {
			if (request !== latestRequest) return;
			// Paperless being unreachable is not "everything is linked", but the
			// nav is the wrong place to report it -- the Doklady page surfaces the
			// error. Hide the badge and leave a trace in the console.
			console.warn('Failed to count unlinked Paperless invoices:', error);
			count = 0;
		} finally {
			if (request === latestRequest) loading = false;
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
