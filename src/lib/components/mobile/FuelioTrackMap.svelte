<script lang="ts">
	// The GPS track of Fuelio drives on a small map for the phone add panel
	// (Task 93). The backend sends the track as [lat, lon] lines; the start
	// and end markers show where a new place at the GPS point would go.
	import 'leaflet/dist/leaflet.css';
	import { onDestroy, onMount, tick } from 'svelte';
	import type { Map as LeafletMap } from 'leaflet';
	import { getFuelioTrack } from '$lib/api';
	import LL from '$lib/i18n/i18n-svelte';

	let { driveIds }: { driveIds: string[] } = $props();

	// The same colors as the PC Fuelio page.
	const GPS_COLOR = '#d63031';
	const START_COLOR = '#27ae60';

	let mapEl = $state<HTMLDivElement | null>(null);
	let status = $state<'loading' | 'ready' | 'error' | 'empty'>('loading');
	let map: LeafletMap | null = null;
	let destroyed = false;

	onMount(async () => {
		try {
			const [leaflet, track] = await Promise.all([
				import('leaflet').then((m) => m.default),
				getFuelioTrack(driveIds, null)
			]);
			if (destroyed) return;
			const lines = track.gps.filter((l) => l.length > 0);
			if (lines.length === 0) {
				status = 'empty';
				return;
			}
			status = 'ready';
			// The map needs its container in the DOM with a size.
			await tick();
			if (destroyed || !mapEl) return;
			// Pinch zoom and drag help to see where the drive ended. The
			// scroll wheel stays off, so the page still scrolls.
			map = leaflet.map(mapEl, { scrollWheelZoom: false, zoomControl: false });
			leaflet
				.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
					maxZoom: 19,
					// Required by the OpenStreetMap tile usage policy.
					attribution:
						'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
				})
				.addTo(map);
			const drawn = lines.map((l) =>
				leaflet.polyline(l, { color: GPS_COLOR, weight: 4, opacity: 0.9 }).addTo(map!)
			);
			const first = lines[0][0];
			const lastLine = lines[lines.length - 1];
			const last = lastLine[lastLine.length - 1];
			const marker = (p: [number, number], color: string) =>
				leaflet
					.circleMarker(p, { radius: 7, color: '#fff', weight: 2, fillColor: color, fillOpacity: 1 })
					.addTo(map!);
			marker(first, START_COLOR);
			marker(last, GPS_COLOR);
			const bounds = drawn
				.map((p) => p.getBounds())
				.reduce((a, b) => a.extend(b), drawn[0].getBounds());
			map.invalidateSize();
			map.fitBounds(bounds, { padding: [20, 20] });
		} catch (error) {
			console.error('Failed to load the Fuelio track:', error);
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
	<div class="map" bind:this={mapEl} data-testid="m-fuelio-map"></div>
	<p class="note legend">
		<span class="dot start"></span>{$LL.mobile.add.origin()}
		<span class="dot end"></span>{$LL.mobile.add.destination()}
	</p>
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

	.legend {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		display: inline-block;
	}

	.dot.end {
		margin-left: 0.75rem;
		background: #d63031;
	}

	.dot.start {
		background: #27ae60;
	}
</style>
