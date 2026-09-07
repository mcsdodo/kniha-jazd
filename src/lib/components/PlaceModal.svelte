<script lang="ts">
	import 'leaflet/dist/leaflet.css';
	import { onMount, onDestroy, untrack } from 'svelte';
	import type { DivIcon, Map as LeafletMap, Marker } from 'leaflet';
	import { geocodePlace } from '$lib/api';
	import type { GeocodeCandidate, Place, PlaceSource } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	let {
		place,
		onSave,
		onClear,
		onClose
	}: {
		place: Place;
		/** Hands the confirmed coordinate to the page, which owns the write. This
		 *  dialog imports no write command at all, so a geocoder answer physically
		 *  cannot reach the database before a human presses Save (ADR-032). */
		onSave: (coords: { lat: number; lon: number; source: PlaceSource }) => void;
		onClear: () => void;
		onClose: () => void;
	} = $props();

	/** Roughly Slovakia — the view for a place with no coordinate yet. */
	const UNPLACED_CENTER: [number, number] = [48.7, 19.7];
	const UNPLACED_ZOOM = 7;
	/** Close enough to read a street once the place has been located. */
	const PLACED_ZOOM = 15;

	// The pending pin. Every field here is local until Save: it is seeded from
	// the prop and then free to diverge, since a dialog that re-mirrored `place`
	// would throw the user's edits away.
	//
	// What makes it a snapshot is that a $state(...) initialiser runs exactly
	// once — untrack() adds no guarantee of its own, it only marks that
	// once-only read as deliberate and silences `state_referenced_locally`.
	// That once-only seeding is also why the settings page wraps this dialog in
	// {#key}: a `place` swapped in under a live instance would leave the
	// previous place's coordinate sitting here, under the new place's name.
	let lat = $state<number | null>(untrack(() => place.lat));
	let lon = $state<number | null>(untrack(() => place.lon));
	let source = $state<PlaceSource | null>(untrack(() => place.source));

	// Prefilled with the name the trips use so walking the list is one click per
	// row. It is only ever a query — the search still waits for a submit.
	let query = $state(untrack(() => place.displayName));
	let candidates = $state<GeocodeCandidate[]>([]);
	let searching = $state(false);
	let searchFailed = $state(false);
	let hasSearched = $state(false);

	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletReady = $state(false);
	let mapReady = $state(false);

	// Plain (non-reactive) handles: Leaflet objects are mutable and must never
	// become effect dependencies.
	let leaflet: typeof import('leaflet') | null = null;
	let map: LeafletMap | null = null;
	let marker: Marker | null = null;
	let pinIcon: DivIcon | null = null;

	let canSave = $derived(lat !== null && lon !== null && source !== null);
	// Clearing only means something for a coordinate that is already stored.
	let canClear = $derived(place.lat !== null && place.lon !== null);

	onMount(async () => {
		// Leaflet touches `window` at import time — keep it out of the module graph.
		leaflet = (await import('leaflet')).default;
		leafletReady = true;
	});

	onDestroy(() => {
		marker = null;
		map?.remove();
		map = null;
	});

	// Create the map once Leaflet and the container element exist. Not done in
	// onMount: Leaflet is loaded lazily, so the container and the library only
	// both exist some time after mount.
	//
	// No invalidateSize() here on purpose. That call is the fix for a map built
	// inside a container that had no dimensions yet — a dialog that lives in the
	// DOM and is merely hidden. This one is rendered under `{#if editingPlace}`,
	// so it mounts already open, and the container's height comes from CSS that
	// shipped with the page. By the time the lazy import resolves the container
	// is laid out at its real size.
	$effect(() => {
		if (!leafletReady || !mapEl || map) return;
		const L = leaflet;
		if (!L) return;

		// A div icon carries no image asset, so nothing has to survive the
		// bundler's URL rewriting — the pin is drawn by the CSS below. Supplying
		// `className` also replaces Leaflet's default `leaflet-div-icon` styling.
		pinIcon = L.divIcon({ className: 'place-pin', iconSize: [18, 18], iconAnchor: [9, 9] });

		const placed = lat !== null && lon !== null;
		map = L.map(mapEl).setView(
			placed ? [lat as number, lon as number] : UNPLACED_CENTER,
			placed ? PLACED_ZOOM : UNPLACED_ZOOM
		);
		L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
			maxZoom: 19,
			// Required by the OpenStreetMap tile usage policy.
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
		}).addTo(map);
		// Clicking the map is how a place the geocoder cannot find gets its pin.
		map.on('click', (event) => {
			lat = event.latlng.lat;
			lon = event.latlng.lng;
			source = 'manual';
		});
		mapReady = true;
	});

	// Keep the pin wherever the pending coordinate is.
	$effect(() => {
		const pinLat = lat;
		const pinLon = lon;
		if (!mapReady || !map || !leaflet || !pinIcon) return;

		if (pinLat === null || pinLon === null) {
			if (marker) {
				map.removeLayer(marker);
				marker = null;
			}
			return;
		}

		if (!marker) {
			marker = leaflet.marker([pinLat, pinLon], { icon: pinIcon, draggable: true }).addTo(map);
			marker.on('dragend', () => {
				const moved = marker?.getLatLng();
				if (!moved) return;
				lat = moved.lat;
				lon = moved.lng;
				// A hand-placed pin is no longer the geocoder's answer.
				source = 'manual';
			});
			return;
		}
		// Deliberately no setView: recentring on every drag would fight the user.
		marker.setLatLng([pinLat, pinLon]);
	});

	/** Submit-only. Nominatim allows one request per second and this dialog is
	 *  where that budget is spent, so no keystroke may start a request. */
	function handleSearchSubmit(event: SubmitEvent) {
		event.preventDefault();
		void runSearch();
	}

	async function runSearch() {
		const trimmed = query.trim();
		if (!trimmed || searching) return;
		searching = true;
		searchFailed = false;
		hasSearched = true;
		try {
			candidates = await geocodePlace(trimmed);
		} catch (error) {
			console.error('Failed to geocode place:', error);
			// Drop the previous matches: leaving them on screen under an error
			// banner invites saving a coordinate from a stale query.
			candidates = [];
			searchFailed = true;
		} finally {
			searching = false;
		}
	}

	/** The label is the geocoder's own rendering of the address. It is shown
	 *  while choosing and then discarded — only the coordinate is kept, because
	 *  the stored name has to stay the spelling the trips use (ADR-034). */
	function selectCandidate(candidate: GeocodeCandidate) {
		lat = candidate.lat;
		lon = candidate.lon;
		source = 'geocoder';
		map?.setView([candidate.lat, candidate.lon], PLACED_ZOOM);
	}

	function isSelected(candidate: GeocodeCandidate): boolean {
		return source === 'geocoder' && candidate.lat === lat && candidate.lon === lon;
	}

	function handleSave() {
		if (lat === null || lon === null || source === null) return;
		onSave({ lat, lon, source });
	}

	function handleBackgroundClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			onClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}

	/** Three decimals is ~100 m — the same precision the list shows. */
	function formatPending(): string {
		if (lat === null || lon === null) return '—';
		return `${lat.toFixed(3)}, ${lon.toFixed(3)}`;
	}
</script>

<div
	class="modal-backdrop"
	onclick={handleBackgroundClick}
	onkeydown={handleKeydown}
	role="button"
	tabindex="-1"
>
	<div
		class="modal-content"
		data-testid="place-modal"
		data-place-name={place.displayName}
		data-place-source={source ?? ''}
	>
		<div class="modal-header">
			<h2>
				{$LL.places.editTitle()}
				<span class="place-name" data-testid="place-modal-name">{place.displayName}</span>
			</h2>
			<button
				class="close-button"
				data-testid="place-modal-close"
				title={$LL.common.close()}
				onclick={onClose}>&times;</button
			>
		</div>

		<div class="modal-body">
			<form class="search-row" onsubmit={handleSearchSubmit}>
				<input
					type="text"
					data-testid="place-search-input"
					bind:value={query}
					placeholder={$LL.places.searchPlaceholder()}
					aria-label={$LL.places.searchPlaceholder()}
				/>
				<button
					type="submit"
					class="button button-secondary"
					data-testid="place-search-submit"
					disabled={searching}
				>
					{searching ? $LL.places.searching() : $LL.places.search()}
				</button>
			</form>

			{#if searching}
				<p class="status" data-testid="place-search-status">{$LL.places.searching()}</p>
			{:else if searchFailed}
				<p class="status error" data-testid="place-search-error">{$LL.places.searchError()}</p>
			{:else if candidates.length > 0}
				<ul class="candidates" data-testid="place-candidates">
					{#each candidates as candidate (`${candidate.lat},${candidate.lon},${candidate.label}`)}
						<li>
							<button
								type="button"
								class="candidate"
								class:selected={isSelected(candidate)}
								data-testid="place-candidate"
								onclick={() => selectCandidate(candidate)}
							>
								{candidate.label}
							</button>
						</li>
					{/each}
				</ul>
			{:else if hasSearched}
				<p class="status" data-testid="place-no-results">{$LL.places.noResults()}</p>
			{/if}

			<div class="map-canvas" bind:this={mapEl} data-testid="place-map"></div>

			<div class="pin-row">
				<span class="hint" data-testid="place-pin-hint">{$LL.places.pinHint()}</span>
				<span class="coords" data-testid="place-modal-coords">{formatPending()}</span>
			</div>
		</div>

		<div class="modal-footer">
			{#if canClear}
				<button
					class="button button-danger"
					data-testid="place-modal-clear"
					onclick={onClear}
				>
					{$LL.places.clear()}
				</button>
			{/if}
			<button class="button button-secondary" data-testid="place-modal-cancel" onclick={onClose}>
				{$LL.common.cancel()}
			</button>
			<button
				class="button button-primary"
				data-testid="place-modal-save"
				disabled={!canSave}
				onclick={handleSave}
			>
				{$LL.places.save()}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-content {
		background: var(--bg-surface);
		border-radius: 8px;
		width: 90%;
		max-width: 560px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 4px 12px var(--shadow-default);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-default);
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
		min-width: 0;
	}

	.place-name {
		display: block;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-secondary);
		word-break: break-word;
	}

	.close-button {
		background: none;
		border: none;
		font-size: 2rem;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
	}

	.close-button:hover {
		color: var(--text-primary);
	}

	.modal-body {
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.search-row {
		display: flex;
		gap: 0.5rem;
	}

	.search-row input {
		flex: 1;
		min-width: 0;
		padding: 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 1rem;
		font-family: inherit;
		background-color: var(--bg-surface);
		color: var(--text-primary);
	}

	.search-row input:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	.status {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.status.error {
		color: var(--accent-danger);
	}

	.candidates {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: 9rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.candidate {
		width: 100%;
		text-align: left;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 4px;
		background: var(--bg-surface-alt);
		color: var(--text-primary);
		font-size: 0.875rem;
		font-family: inherit;
		cursor: pointer;
	}

	.candidate:hover {
		border-color: var(--accent-primary);
	}

	.candidate.selected {
		border-color: var(--accent-primary);
		background: var(--btn-primary-light-bg);
		color: var(--btn-primary-light-color);
	}

	.map-canvas {
		height: 320px;
		width: 100%;
		border-radius: 6px;
		border: 1px solid var(--border-default);
	}

	.pin-row {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 1rem;
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-secondary);
		font-style: italic;
	}

	.coords {
		font-size: 0.8125rem;
		font-variant-numeric: tabular-nums;
		color: var(--text-secondary);
		white-space: nowrap;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid var(--border-default);
	}

	.button {
		padding: 0.75rem 1.5rem;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 1rem;
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button-secondary {
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
	}

	.button-secondary:hover:not(:disabled) {
		background-color: var(--btn-secondary-hover);
	}

	.button-primary {
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}

	.button-primary:hover:not(:disabled) {
		background-color: var(--btn-active-primary-hover);
	}

	.button-danger {
		background-color: var(--accent-danger-bg);
		color: var(--accent-danger);
		margin-right: auto;
	}

	.button-danger:hover:not(:disabled) {
		background-color: var(--accent-danger-bg-hover);
	}

	/* Leaflet builds the marker element itself, outside Svelte's compiler, so
	   the pin's rule has to be global to reach it. */
	:global(.place-pin) {
		box-sizing: border-box;
		border-radius: 50%;
		background: var(--accent-primary);
		border: 3px solid var(--bg-surface);
		box-shadow: 0 1px 4px var(--shadow-default);
		cursor: grab;
	}
</style>
