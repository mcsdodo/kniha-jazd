<script lang="ts">
	// Task 96: the pin map of the phone place editor. The Leaflet setup is the
	// one of PlaceModal.svelte. This component only draws the pin and reports a
	// tap or a drag; the editor owns the position.
	import 'leaflet/dist/leaflet.css';
	import { onMount, onDestroy } from 'svelte';
	import type { DivIcon, Map as LeafletMap, Marker } from 'leaflet';

	let {
		position,
		onpick,
		readOnly = false
	}: {
		position: { lat: number; lon: number } | null;
		onpick: (lat: number, lon: number) => void;
		/** No tap and no drag changes the position. */
		readOnly?: boolean;
	} = $props();

	/** Roughly Slovakia: the view for a place with no position yet. */
	const UNPLACED_CENTER: [number, number] = [48.7, 19.7];
	const UNPLACED_ZOOM = 7;
	const PLACED_ZOOM = 15;

	let mapEl = $state<HTMLDivElement | null>(null);
	let leafletReady = $state(false);
	let mapReady = $state(false);
	// The zoom level, shown as data-zoom for the integration test.
	let zoom = $state<number | null>(null);

	// Plain handles: Leaflet objects must not become effect dependencies.
	let leaflet: typeof import('leaflet') | null = null;
	let map: LeafletMap | null = null;
	let marker: Marker | null = null;
	let pinIcon: DivIcon | null = null;
	// The position that the map shows now. A change from outside differs from it.
	let shownLat: number | null = null;
	let shownLon: number | null = null;

	onMount(async () => {
		// Leaflet touches `window` at import time.
		leaflet = (await import('leaflet')).default;
		leafletReady = true;
	});

	onDestroy(() => {
		marker = null;
		map?.remove();
		map = null;
	});

	$effect(() => {
		if (!leafletReady || !mapEl || map) return;
		const L = leaflet;
		if (!L) return;
		pinIcon = L.divIcon({ className: 'place-pin', iconSize: [18, 18], iconAnchor: [9, 9] });
		const start = position;
		map = L.map(mapEl, { scrollWheelZoom: false }).setView(
			start ? [start.lat, start.lon] : UNPLACED_CENTER,
			start ? PLACED_ZOOM : UNPLACED_ZOOM
		);
		L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
			maxZoom: 19,
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
		}).addTo(map);
		zoom = map.getZoom();
		map.on('zoomend', () => {
			zoom = map?.getZoom() ?? null;
		});
		map.on('click', (event) => {
			if (readOnly) return;
			shownLat = event.latlng.lat;
			shownLon = event.latlng.lng;
			onpick(event.latlng.lat, event.latlng.lng);
		});
		if (start) {
			shownLat = start.lat;
			shownLon = start.lon;
		}
		mapReady = true;
	});

	// Keep the pin where the position is. Pan only if the change came from outside.
	$effect(() => {
		const pinLat = position?.lat ?? null;
		const pinLon = position?.lon ?? null;
		if (!mapReady || !map || !leaflet || !pinIcon) return;

		if (pinLat === null || pinLon === null) {
			if (marker) {
				map.removeLayer(marker);
				marker = null;
			}
			return;
		}

		if (!marker) {
			marker = leaflet.marker([pinLat, pinLon], { icon: pinIcon, draggable: !readOnly }).addTo(map);
			marker.on('dragend', () => {
				const moved = marker?.getLatLng();
				if (!moved) return;
				shownLat = moved.lat;
				shownLon = moved.lng;
				onpick(moved.lat, moved.lng);
			});
			// The first pin of a map that started without a position: a pin from
			// outside (search, GPS) pans to it. A tap sets shownLat first, so it does not.
			if (pinLat !== shownLat || pinLon !== shownLon) {
				map.setView([pinLat, pinLon], PLACED_ZOOM);
			}
			shownLat = pinLat;
			shownLon = pinLon;
			return;
		}
		marker.setLatLng([pinLat, pinLon]);
		if (pinLat !== shownLat || pinLon !== shownLon) {
			shownLat = pinLat;
			shownLon = pinLon;
			map.setView([pinLat, pinLon], PLACED_ZOOM);
		}
	});
</script>

<div class="map" bind:this={mapEl} data-testid="m-place-map" data-zoom={zoom}></div>

<style>
	.map {
		height: 220px;
		border: 1px solid var(--border-default);
		border-radius: 10px;
		overflow: hidden;
	}

	:global(.place-pin) {
		box-sizing: border-box;
		border-radius: 50%;
		background: var(--accent-primary);
		border: 3px solid var(--bg-surface);
		box-shadow: 0 1px 4px var(--shadow-default);
		cursor: grab;
	}
</style>
