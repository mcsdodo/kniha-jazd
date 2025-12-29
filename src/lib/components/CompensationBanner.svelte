<script lang="ts">
	import { getCompensationSuggestion, createTrip } from '$lib/api';
	import type { CompensationSuggestion } from '$lib/types';
	import { onMount } from 'svelte';
	import { toast } from '$lib/stores/toast';
	import LL from '$lib/i18n/i18n-svelte';

	export let vehicleId: string;
	export let marginPercent: number;
	export let bufferKm: number;
	export let currentLocation: string;
	export let onTripAdded: () => void;

	let suggestion: CompensationSuggestion | null = null;
	let loading = true;
	let adding = false;

	onMount(async () => {
		await loadSuggestion();
	});

	async function loadSuggestion() {
		try {
			loading = true;
			suggestion = await getCompensationSuggestion(vehicleId, bufferKm, currentLocation);
		} catch (error) {
			console.error('Failed to load compensation suggestion:', error);
		} finally {
			loading = false;
		}
	}

	async function handleAddTrip() {
		if (!suggestion || adding) return;

		try {
			adding = true;
			const today = new Date().toISOString().split('T')[0];
			await createTrip(
				vehicleId,
				today,
				suggestion.origin,
				suggestion.destination,
				suggestion.distance_km,
				0, // odometer - will be filled manually
				suggestion.purpose,
				null, // no fuel
				null,
				null,
				null
			);
			onTripAdded();
		} catch (error) {
			console.error('Failed to add compensation trip:', error);
			toast.error($LL.toast.errorAddCompensationTrip());
		} finally {
			adding = false;
		}
	}
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

		{#if loading}
			<p class="loading-text">{$LL.compensation.searchingSuggestion()}</p>
		{:else if suggestion}
			<div class="suggestion">
				<h4>{$LL.compensation.suggestionTitle()}</h4>
				<div class="suggestion-details">
					<div class="detail-row">
						<span class="label">{$LL.compensation.origin()}</span>
						<span class="value">{suggestion.origin}</span>
					</div>
					<div class="detail-row">
						<span class="label">{$LL.compensation.destination()}</span>
						<span class="value">{suggestion.destination}</span>
					</div>
					<div class="detail-row">
						<span class="label">{$LL.compensation.distance()}</span>
						<span class="value">{suggestion.distance_km.toFixed(1)} km</span>
					</div>
					<div class="detail-row">
						<span class="label">{$LL.compensation.purpose()}</span>
						<span class="value">{suggestion.purpose}</span>
					</div>
					{#if suggestion.is_buffer}
						<p class="buffer-note">
							{$LL.compensation.bufferNote()}
						</p>
					{/if}
				</div>
				<button class="add-button" on:click={handleAddTrip} disabled={adding}>
					{adding ? $LL.compensation.adding() : $LL.compensation.addTrip()}
				</button>
			</div>
		{/if}
	</div>
</div>

<style>
	.compensation-banner {
		background: #fff3cd;
		border: 2px solid #ffc107;
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
		color: #856404;
	}

	.warning-content {
		padding-left: 2.25rem;
	}

	.margin-info,
	.buffer-info {
		margin: 0.5rem 0;
		color: #856404;
		font-size: 0.9375rem;
	}

	.margin-info strong,
	.buffer-info strong {
		color: #d39e00;
		font-weight: 600;
	}

	.loading-text {
		font-style: italic;
		color: #856404;
		margin: 1rem 0;
	}

	.suggestion {
		margin-top: 1rem;
		padding: 1rem;
		background: white;
		border-radius: 6px;
		border: 1px solid #ffc107;
	}

	.suggestion h4 {
		margin: 0 0 0.75rem 0;
		font-size: 1rem;
		color: #856404;
	}

	.suggestion-details {
		margin-bottom: 1rem;
	}

	.detail-row {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.detail-row .label {
		font-weight: 500;
		color: #856404;
		min-width: 100px;
	}

	.detail-row .value {
		color: #212529;
		font-weight: 600;
	}

	.buffer-note {
		margin-top: 0.75rem;
		padding: 0.5rem;
		background: #fff9e6;
		border-radius: 4px;
		font-size: 0.875rem;
		color: #856404;
		font-style: italic;
	}

	.add-button {
		width: 100%;
		padding: 0.75rem 1.5rem;
		background-color: #28a745;
		color: white;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		font-size: 0.9375rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.add-button:hover:not(:disabled) {
		background-color: #218838;
	}

	.add-button:disabled {
		background-color: #6c757d;
		cursor: not-allowed;
		opacity: 0.6;
	}
</style>
