<script lang="ts">
	// Task 94: the route alternatives of a manual phone trip on one small map.
	// The chosen route is blue and on top; the others are grey and a tap on
	// one chooses it. The backend sends the routes decoded to [lat, lon].
	import 'leaflet/dist/leaflet.css';
	import { onDestroy, onMount } from 'svelte';
	import type { LayerGroup, Map as LeafletMap } from 'leaflet';
	import type { ManualRoute } from '$lib/types';

	let {
		routes,
		active,
		onSelect
	}: { routes: ManualRoute[]; active: number; onSelect: (index: number) => void } = $props();

	const ACTIVE_COLOR = '#0984e3';
	const OTHER_COLOR = '#8a959a';

	let mapEl = $state<HTMLDivElement | null>(null);
	let L = $state<typeof import('leaflet') | null>(null);
	let map: LeafletMap | null = null;
	let layer: LayerGroup | null = null;
	// The routes the map was last fitted to. A new choice of route redraws
	// the lines but keeps the zoom; new routes fit the map again.
	let fitted: ManualRoute[] | null = null;

	onMount(async () => {
		const leaflet = (await import('leaflet')).default;
		if (!mapEl) return;
		// Pinch zoom and drag work. The scroll wheel stays off, so the page
		// still scrolls.
		map = leaflet.map(mapEl, { scrollWheelZoom: false, zoomControl: false });
		leaflet
			.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
				maxZoom: 19,
				// Required by the OpenStreetMap tile usage policy.
				attribution:
					'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
			})
			.addTo(map);
		layer = leaflet.layerGroup().addTo(map);
		L = leaflet;
	});

	$effect(() => {
		const leaflet = L;
		const list = routes;
		const chosen = active;
		if (!leaflet || !map || !layer) return;
		layer.clearLayers();
		const drawn = list.filter((r) => r.coordinates.length >= 2);
		if (drawn.length === 0) return;
		// The others first, so the chosen line is on top.
		list.forEach((route, i) => {
			if (i === chosen || route.coordinates.length < 2) return;
			const line = leaflet
				.polyline(route.coordinates, { color: OTHER_COLOR, weight: 6, opacity: 0.75 })
				.addTo(layer!);
			line.on('click', () => onSelect(i));
		});
		const main = list[chosen];
		if (main && main.coordinates.length >= 2) {
			leaflet
				.polyline(main.coordinates, { color: ACTIVE_COLOR, weight: 6, opacity: 0.9 })
				.addTo(layer);
			const first = main.coordinates[0];
			const last = main.coordinates[main.coordinates.length - 1];
			leaflet
				.circleMarker(first, { radius: 7, color: '#fff', weight: 2, fillColor: '#27ae60', fillOpacity: 1 })
				.addTo(layer);
			leaflet
				.circleMarker(last, { radius: 7, color: '#fff', weight: 2, fillColor: '#d63031', fillOpacity: 1 })
				.addTo(layer);
		}
		if (fitted !== list) {
			fitted = list;
			const bounds = leaflet.latLngBounds(drawn.flatMap((r) => r.coordinates));
			map.invalidateSize();
			map.fitBounds(bounds, { padding: [16, 16] });
		}
	});

	onDestroy(() => {
		map?.remove();
		map = null;
		layer = null;
	});
</script>

<div class="map" bind:this={mapEl} data-testid="m-nova-map"></div>

<style>
	.map {
		height: 200px;
		border-radius: 8px;
		border: 1px solid var(--border-default);
		overflow: hidden;
	}
</style>
