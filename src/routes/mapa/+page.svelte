<script lang="ts">
	import 'leaflet/dist/leaflet.css';
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import type { Map as LeafletMap, Polyline } from 'leaflet';
	import { generateRoute, getTripRoute, saveTripRoute, deleteTripRoute, getTrips } from '$lib/api';
	import type { GeneratedRoute, RouteMap, Trip } from '$lib/types';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { toast } from '$lib/stores/toast';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import LL from '$lib/i18n/i18n-svelte';

	// Map view before the first route arrives. Every render calls fitBounds, so
	// this is only ever visible for the moment between mount and first draw.
	const INITIAL_CENTER: [number, number] = [48.7, 19.7];
	const INITIAL_ZOOM = 7;

	let tripId = $derived($page.url.searchParams.get('trip') ?? '');

	let trip = $state<Trip | null>(null);
	/** Route persisted against the trip. Only save/remove ever change this. */
	let savedRoute = $state<RouteMap | null>(null);
	/** Freshly generated, not persisted. Regenerating writes nothing. */
	let generated = $state<GeneratedRoute | null>(null);

	let loading = $state(true);
	let generating = $state(false);
	let saving = $state(false);
	let removing = $state(false);
	let error = $state<string | null>(null);
	/** False for errors a retry cannot fix, so the button is not offered. */
	let retryable = $state(true);
	let confirmingRemove = $state(false);
	let savedNotice = $state(false);

	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletReady = $state(false);
	let mapReady = $state(false);

	// Plain (non-reactive) handles: Leaflet objects are mutable and must never
	// become effect dependencies.
	let leaflet: typeof import('leaflet') | null = null;
	let map: LeafletMap | null = null;
	let routeLayer: Polyline | null = null;
	let dataLoadStarted = false;

	let displayRoute = $derived<GeneratedRoute | RouteMap | null>(generated ?? savedRoute);
	let stopNames = $derived(
		displayRoute
			? displayRoute.waypoints.map((w) => w.name).filter((name): name is string => !!name)
			: []
	);
	// Both come from the backend. The tolerance is a business rule and has one
	// home in Rust (ADR-008); deriving it here would measure road distance
	// against a threshold the algorithm applies to a different quantity, so the
	// page could flag a route the backend considers perfectly in tolerance.
	let deviationPercent = $derived(displayRoute?.deviationPercent ?? null);
	let deviationOffTarget = $derived(displayRoute?.offTarget ?? false);
	let busy = $derived(loading || generating || saving || removing);

	onMount(async () => {
		// Leaflet touches `window` at import time — keep it out of the module graph.
		leaflet = (await import('leaflet')).default;
		leafletReady = true;
	});

	onDestroy(() => {
		routeLayer = null;
		map?.remove();
		map = null;
	});

	// Create the map once Leaflet and the container element exist. Not done in
	// onMount: Leaflet is loaded lazily, so the container element and the library
	// only both exist some time after mount.
	$effect(() => {
		if (!leafletReady || !mapEl || map) return;
		const L = leaflet;
		if (!L) return;

		map = L.map(mapEl).setView(INITIAL_CENTER, INITIAL_ZOOM);
		L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
			maxZoom: 19,
			// Required by the OpenStreetMap tile usage policy.
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
		}).addTo(map);
		mapReady = true;
	});

	// Draw whatever route is currently on display.
	$effect(() => {
		const route = displayRoute;
		if (!mapReady || !map || !leaflet) return;

		if (routeLayer) {
			map.removeLayer(routeLayer);
			routeLayer = null;
		}
		if (!route || route.coordinates.length === 0) return;

		routeLayer = leaflet
			.polyline(route.coordinates, { color: '#0066cc', weight: 5, opacity: 0.85 })
			.addTo(map);
		map.fitBounds(routeLayer.getBounds(), { padding: [30, 30] });
	});

	// The trip comes from the vehicle the layout activates, which is populated
	// asynchronously — so react to it rather than reading it on mount.
	$effect(() => {
		const vehicle = $activeVehicleStore;
		if (!vehicle || dataLoadStarted) return;
		dataLoadStarted = true;
		void loadTripAndRoute(vehicle.id);
	});

	async function loadTripAndRoute(vehicleId: string) {
		loading = true;
		error = null;
		retryable = true;
		try {
			const trips = await getTrips(vehicleId);
			trip = trips.find((t) => t.id === tripId) ?? null;
			if (!trip) {
				// Not a generation failure — nothing was generated. Retrying would
				// fail identically forever, so this branch offers no retry.
				error = $LL.routeMap.tripNotFound();
				retryable = false;
				return;
			}

			savedRoute = await getTripRoute(tripId);
			if (!savedRoute) {
				await runGenerate(trip.distanceKm);
			}
		} catch (e) {
			console.error('Failed to load route map:', e);
			error = $LL.routeMap.error();
		} finally {
			loading = false;
		}
	}

	/** Generates and displays a route. Persists nothing — only handleSave does. */
	async function runGenerate(targetKm: number) {
		generating = true;
		error = null;
		savedNotice = false;
		try {
			generated = await generateRoute(targetKm);
		} catch (e) {
			console.error('Failed to generate route:', e);
			// Drop the previous proposal: leaving it would let the user save a
			// stale route while an error banner is on screen.
			generated = null;
			error = $LL.routeMap.error();
		} finally {
			generating = false;
		}
	}

	function handleRegenerate() {
		if (!trip) return;
		void runGenerate(trip.distanceKm);
	}

	function handleRetry() {
		error = null;
		if (trip) {
			void runGenerate(trip.distanceKm);
			return;
		}
		const vehicle = $activeVehicleStore;
		if (vehicle) void loadTripAndRoute(vehicle.id);
	}

	async function handleSave() {
		if (!generated || !tripId) return;
		saving = true;
		try {
			await saveTripRoute(tripId, generated);
			// Re-read so the displayed route is the persisted one, not a local copy.
			savedRoute = await getTripRoute(tripId);
			generated = null;
			announce('route-map-saved');
			savedNotice = true;
			toast.success($LL.routeMap.saved());
		} catch (e) {
			console.error('Failed to save route map:', e);
			toast.error($LL.routeMap.error());
		} finally {
			saving = false;
		}
	}

	async function handleRemoveConfirmed() {
		confirmingRemove = false;
		if (!savedRoute || !tripId) return;
		removing = true;
		try {
			await deleteTripRoute(tripId);
			savedRoute = null;
			savedNotice = false;
			announce('route-map-removed');
			toast.success($LL.routeMap.removed());
		} catch (e) {
			console.error('Failed to remove route map:', e);
			toast.error($LL.routeMap.error());
		} finally {
			removing = false;
		}
	}

	/** Tells an open logbook tab that this trip's map appeared or disappeared,
	 *  so its row icon does not go stale while both tabs are open. */
	function announce(type: 'route-map-saved' | 'route-map-removed') {
		if (typeof BroadcastChannel === 'undefined') return;
		const channel = new BroadcastChannel('kniha-jazd');
		channel.postMessage({ type, tripId });
		channel.close();
	}

	function formatDeviation(percent: number): string {
		return `${percent >= 0 ? '+' : ''}${percent.toFixed(1)} %`;
	}
</script>

<div class="map-page" data-test="route-map-page">
	<div class="header">
		<h1>{$LL.routeMap.title()}</h1>
		{#if trip}
			<span class="trip-summary" data-test="trip-summary">
				{trip.origin} → {trip.destination} · {trip.distanceKm} km
			</span>
		{/if}
	</div>

	<div class="toolbar">
		<button
			class="button"
			data-test="regenerate-btn"
			onclick={handleRegenerate}
			disabled={busy || !trip}
		>
			{generating ? $LL.routeMap.generating() : $LL.routeMap.regenerate()}
		</button>
		<button
			class="button secondary"
			data-test="save-btn"
			onclick={handleSave}
			disabled={busy || !generated}
		>
			{$LL.routeMap.save()}
		</button>
		{#if savedRoute}
			<button
				class="button danger"
				data-test="remove-btn"
				onclick={() => (confirmingRemove = true)}
				disabled={busy}
			>
				{$LL.routeMap.remove()}
			</button>
		{/if}
	</div>

	{#if savedNotice}
		<div class="saved-notice" data-test="saved-notice">
			<span>{$LL.routeMap.saved()}</span>
			<button class="button-small" onclick={() => window.close()}>{$LL.common.close()}</button>
		</div>
	{/if}

	{#if error}
		<div class="error-box" data-test="route-map-error">
			<span>{error}</span>
			{#if retryable}
				<button class="button-small" data-test="retry-btn" onclick={handleRetry}>
					{$LL.routeMap.retry()}
				</button>
			{/if}
		</div>
	{/if}

	{#if loading || generating}
		<p class="status" data-test="route-map-status">
			{generating ? $LL.routeMap.generating() : $LL.common.loading()}
		</p>
	{:else if displayRoute}
		<div class="route-info">
			<span class="info-item">
				<span class="label">{$LL.routeMap.targetKm()}</span>
				<span class="value" data-test="target-km">{displayRoute.targetKm.toFixed(1)} km</span>
			</span>
			<span class="info-item">
				<span class="label">{$LL.routeMap.actualKm()}</span>
				<span class="value" data-test="actual-km">{displayRoute.roadKm.toFixed(1)} km</span>
			</span>
			{#if deviationPercent !== null}
				<span class="info-item">
					<span class="label">{$LL.routeMap.deviation()}</span>
					<span class="value" class:off-target={deviationOffTarget} data-test="deviation">
						{formatDeviation(deviationPercent)}
					</span>
				</span>
			{/if}
		</div>
		{#if stopNames.length > 0}
			<p class="stops" data-test="stops">
				<span class="label">{$LL.routeMap.stops()} ({stopNames.length})</span>
				{stopNames.join(' → ')}
			</p>
		{/if}
	{/if}

	<div class="map-canvas" bind:this={mapEl} data-test="route-map-canvas"></div>
</div>

{#if confirmingRemove}
	<ConfirmModal
		title={$LL.routeMap.remove()}
		message={$LL.routeMap.confirmRemove()}
		confirmText={$LL.common.delete()}
		danger={true}
		onConfirm={handleRemoveConfirmed}
		onCancel={() => (confirmingRemove = false)}
	/>
{/if}

<style>
	.map-page {
		max-width: 1000px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		align-items: baseline;
		gap: 1rem;
		flex-wrap: wrap;
		margin-bottom: 1rem;
	}

	.header h1 {
		margin: 0;
		color: var(--text-primary);
	}

	.trip-summary {
		color: var(--text-secondary);
	}

	.toolbar {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.saved-notice {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
		border-radius: 4px;
		background: var(--toast-success-bg);
		color: var(--toast-success-color);
	}

	.error-box {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1rem;
		margin-bottom: 1rem;
		border-radius: 4px;
		background: var(--toast-error-bg);
		color: var(--toast-error-color);
	}

	.status {
		color: var(--text-secondary);
		font-style: italic;
		margin: 0 0 1rem 0;
	}

	.route-info {
		display: flex;
		gap: 1.5rem;
		flex-wrap: wrap;
		margin-bottom: 0.5rem;
	}

	.info-item {
		display: flex;
		align-items: baseline;
		gap: 0.375rem;
	}

	.label {
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.value {
		font-weight: 500;
		color: var(--text-primary);
	}

	.value.off-target {
		color: var(--accent-warning-dark);
		font-weight: 600;
	}

	.stops {
		margin: 0 0 1rem 0;
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.map-canvas {
		height: 60vh;
		min-height: 360px;
		width: 100%;
		border-radius: 6px;
		border: 1px solid var(--border-default);
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button:hover:not(:disabled) {
		background-color: var(--btn-active-primary-hover);
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button.secondary {
		background-color: var(--btn-active-success-bg);
		color: var(--btn-active-success-color);
	}

	.button.secondary:hover:not(:disabled) {
		background-color: var(--btn-active-success-hover);
	}

	.button.danger {
		background-color: var(--accent-danger-bg);
		color: var(--accent-danger);
	}

	.button.danger:hover:not(:disabled) {
		background-color: var(--accent-danger-hover-bg);
	}

	.button-small {
		padding: 0.375rem 0.875rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}
</style>
