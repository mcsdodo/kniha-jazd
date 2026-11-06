<script lang="ts">
	// Task 96: the place list on the phone. Places belong to the whole
	// database, not to a vehicle, so there is no vehicle gate. The values
	// come from the backend (ADR-008).
	import { onMount } from 'svelte';
	import { listPlaces } from '$lib/api';
	import type { Place } from '$lib/types';
	import { appModeStore } from '$lib/stores/appMode';
	import LL from '$lib/i18n/i18n-svelte';
	import MobileBar from '$lib/components/mobile/MobileBar.svelte';
	import { coords5 } from '$lib/components/mobile/MobileFormat';

	// The backend orders the list (unplaced first, then most-used, then by
	// name). Render it in the order it arrives.
	let places = $state<Place[]>([]);
	let loaded = $state(false);
	let loadError = $state<string | null>(null);
	let placeFilter = $state('');

	let placeFilterNeedle = $derived(placeFilter.trim().toLowerCase());
	// The same rule as the PC page: a query with diacritics matches `name`,
	// an ASCII query matches the backend's `normalisedName`. Do not add a
	// folding table here (ADR-008).
	let visiblePlaces = $derived(
		placeFilterNeedle
			? places.filter(
					(place) =>
						place.name.toLowerCase().includes(placeFilterNeedle) ||
						place.normalisedName.includes(placeFilterNeedle)
				)
			: places
	);

	onMount(loadPlaces);

	async function loadPlaces() {
		try {
			places = await listPlaces();
			loadError = null;
		} catch (error) {
			console.error('Failed to load places:', error);
			loadError = String(error);
		} finally {
			loaded = true;
		}
	}
</script>

<MobileBar title={$LL.places.title()} backHref="/m" backLabel={$LL.mobile.places.back()} />

<div class="body" data-testid="m-miesta">
	<input
		type="search"
		class="filter"
		placeholder={$LL.places.filterPlaceholder()}
		bind:value={placeFilter}
		data-testid="m-miesta-filter"
	/>
	{#if !$appModeStore.isReadOnly}
		<button type="button" class="add" data-testid="m-miesta-add">{$LL.places.add()}</button>
	{/if}

	{#if loadError}
		<p class="error">{loadError}</p>
	{:else if loaded && places.length === 0}
		<p class="note" data-testid="m-miesta-empty">{$LL.places.empty()}</p>
	{:else if loaded && visiblePlaces.length === 0}
		<p class="note" data-testid="m-miesta-no-matches">{$LL.places.noMatches()}</p>
	{/if}

	{#each visiblePlaces as place (place.id)}
		<div class="card" data-testid="m-miesta-card" data-place-id={place.id}>
			<div class="name-row">
				<span class="name" data-testid="m-miesta-card-name">{place.name}</span>
				{#if place.isHome}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="18"
						height="18"
						viewBox="0 0 24 24"
						fill="currentColor"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						class="home"
						data-testid="m-miesta-card-home"
						aria-hidden="true"
					><path d="M3 10.5 12 3l9 7.5" /><path d="M5 9.5V21h14V9.5" /></svg>
				{/if}
			</div>
			<div class="meta">
				<span data-testid="m-miesta-card-uses">{$LL.places.uses({ count: place.uses })}</span>
				<span data-testid="m-miesta-card-coords">{coords5(place.lat, place.lon)}</span>
			</div>
		</div>
	{/each}
</div>

<style>
	.body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 640px;
		margin: 0 auto;
	}

	.filter {
		min-height: 44px;
		padding: 0 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 10px;
		background: var(--bg-surface);
		color: var(--text-primary);
		font-size: 1rem;
		box-sizing: border-box;
	}

	.add {
		min-height: 44px;
		border-radius: 10px;
		border: 2px solid var(--accent-primary);
		background: var(--accent-primary);
		color: #fff;
		font-size: 1rem;
		font-weight: 600;
	}

	.card {
		min-height: 44px;
		padding: 0.7rem 0.9rem;
		border: 1px solid var(--border-default);
		border-radius: 12px;
		background: var(--bg-surface);
		box-sizing: border-box;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.name {
		font-weight: 600;
		overflow-wrap: anywhere;
	}

	.home {
		flex: none;
		color: var(--accent-primary);
	}

	.meta {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		margin-top: 0.2rem;
		font-size: 0.85rem;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}

	.note {
		margin: 0;
		color: var(--text-secondary);
	}

	.error {
		margin: 0;
		color: var(--accent-danger);
	}
</style>
