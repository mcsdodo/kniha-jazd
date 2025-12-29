<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { goto } from '$app/navigation';

	let unassignedCount = $state(0);
	let loading = $state(true);

	onMount(() => {
		loadCount();
		// Refresh count every 30 seconds
		const interval = setInterval(loadCount, 30000);
		return () => clearInterval(interval);
	});

	async function loadCount() {
		try {
			const receipts = await api.getUnassignedReceipts();
			unassignedCount = receipts.length;
		} catch (error) {
			console.error('Failed to load unassigned receipts:', error);
		} finally {
			loading = false;
		}
	}

	function handleClick() {
		goto('/doklady?filter=unassigned');
	}
</script>

{#if !loading && unassignedCount > 0}
	<button class="indicator" onclick={handleClick} title="Nepridelene doklady">
		{unassignedCount}
	</button>
{/if}

<style>
	.indicator {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.5rem 0.75rem;
		background: #e74c3c;
		color: white;
		border: none;
		border-radius: 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}

	.indicator:hover {
		background: #c0392b;
	}
</style>
