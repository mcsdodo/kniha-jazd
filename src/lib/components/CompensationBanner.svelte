<script lang="ts">
	import LL from '$lib/i18n/i18n-svelte';

	export let marginPercent: number;
	export let bufferKm: number;
</script>

<div class="compensation-banner">
	<div class="warning-header">
		<span class="warning-icon">⚠️</span>
		<h3>{$LL.compensation.title()}</h3>
	</div>
	<div class="warning-content">
		<p class="margin-info">
			{@html $LL.compensation.currentDeviation({ percent: marginPercent.toFixed(1) })}
		</p>
		<p class="buffer-info">{@html $LL.compensation.additionalKmNeeded({ km: bufferKm.toFixed(0) })}</p>
	</div>
</div>

<style>
	.compensation-banner {
		background: var(--warning-bg);
		border: 2px solid var(--warning-border);
		border-radius: 8px;
		padding: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.warning-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.warning-icon {
		font-size: 1.5rem;
	}

	.warning-header h3 {
		margin: 0;
		font-size: 1.125rem;
		color: var(--warning-color);
	}

	.warning-content {
		padding-left: 2.25rem;
	}

	.margin-info,
	.buffer-info {
		margin: 0.5rem 0;
		color: var(--warning-color);
		font-size: 0.9375rem;
	}

	:global(.margin-info strong),
	:global(.buffer-info strong) {
		color: var(--warning-highlight);
		font-weight: 600;
	}
</style>
