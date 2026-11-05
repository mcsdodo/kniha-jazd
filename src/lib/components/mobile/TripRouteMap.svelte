<script lang="ts">
	// The stored route of one trip as a small map (Task 93). The backend
	// sends the route already decoded to [lat, lon] pairs.
	import 'leaflet/dist/leaflet.css';
	import { onDestroy, onMount, tick } from 'svelte';
	import type { Map as LeafletMap } from 'leaflet';
	import { getTripRoute } from '$lib/api';
	import LL from '$lib/i18n/i18n-svelte';

	let { tripId }: { tripId: string } = $props();

	const ROUTE_COLOR = '#0984e3';

	let mapEl = $state<HTMLDivElement | null>(null);
	let status = $state<'loading' | 'ready' | 'error' | 'empty'>('loading');
	let map: LeafletMap | null = null;
	let destroyed = false;

	onMount(async () => {
		try {
			const [leaflet, route] = await Promise.all([
				import('leaflet').then((m) => m.default),
				getTripRoute(tripId)
			]);
			if (destroyed) return;
			if (!route || route.coordinates.length < 2) {
				status = 'empty';
				return;
			}
			status = 'ready';
			// The map needs its container in the DOM with a size.
			await tick();
			if (destroyed || !mapEl) return;
			// Pinch zoom and drag help to see the route in detail. The scroll
			// wheel stays off, so the page still scrolls.
			map = leaflet.map(mapEl, { scrollWheelZoom: false, zoomControl: false });
			leaflet
				.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
					maxZoom: 19,
					// Required by the OpenStreetMap tile usage policy.
					attribution:
						'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
				})
				.addTo(map);
			const line = leaflet
				.polyline(route.coordinates, { color: ROUTE_COLOR, weight: 5, opacity: 0.8 })
				.addTo(map);
			map.invalidateSize();
			map.fitBounds(line.getBounds(), { padding: [16, 16] });
		} catch (error) {
			console.error('Failed to load the trip route:', error);
			if (!destroyed) status = 'error';
		}
	});

	onDestroy(() => {
		destroyed = true;
		map?.remove();
		map = null;
	});
</script>

{#if status === 'loading'}
	<p class="note">{$LL.mobile.trips.detail.mapLoading()}</p>
{:else if status === 'error'}
	<p class="note">{$LL.mobile.trips.detail.mapError()}</p>
{:else if status === 'ready'}
	<div class="map" bind:this={mapEl} data-testid="mobile-trip-map"></div>
{/if}

<style>
	.map {
		height: 220px;
		border-radius: 8px;
		border: 1px solid var(--border-default);
		overflow: hidden;
	}

	.note {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}
</style>
