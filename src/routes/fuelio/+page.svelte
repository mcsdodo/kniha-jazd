<script lang="ts">
	import 'leaflet/dist/leaflet.css';
	import { onDestroy, onMount } from 'svelte';
	import type { Map as LeafletMap, LayerGroup } from 'leaflet';
	import * as api from '$lib/api';
	import type { FuelioReport, FuelioRow, FuelioRowStatus } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { toast } from '$lib/stores/toast';

	// Task 90 POC: read-only cross-check of Fuelio GPS drives against the
	// logbook. The backend matches, measures and flags every row (ADR-008);
	// this page only filters the rows for display and draws the map.

	const GPS_COLOR = '#d63031';
	const ROUTE_COLOR = '#0984e3';

	let report = $state<FuelioReport | null>(null);
	let loading = $state(false);
	let minKm = $state(30);
	let highwayOnly = $state(true);
	let problemsOnly = $state(false);
	let statusFilter = $state<FuelioRowStatus | 'all'>('all');
	const STATUSES: FuelioRowStatus[] = ['matched', 'missing', 'noDrive'];
	let selected = $state<FuelioRow | null>(null);
	let selectedHasRoute = $state<boolean | null>(null);

	let leaflet: typeof import('leaflet') | null = null;
	let map: LeafletMap | null = null;
	let layers: LayerGroup | null = null;
	let mapEl = $state<HTMLDivElement | null>(null);

	// A row's km for the min-km filter: the GPS km, or the logbook km of a trip
	// without GPS. Display filtering only - nothing is calculated here.
	function rowKm(r: FuelioRow): number {
		return r.gpsKm ?? r.tripKm ?? 0;
	}

	function isProblem(r: FuelioRow): boolean {
		return r.status !== 'matched' || r.flags.length > 0;
	}

	let rows = $derived(
		(report?.rows ?? []).filter(
			(r) =>
				rowKm(r) >= (minKm || 0) &&
				(!highwayOnly || r.isHighway) &&
				(!problemsOnly || isProblem(r)) &&
				(statusFilter === 'all' || r.status === statusFilter)
		)
	);

	$effect(() => {
		const vehicle = $activeVehicleStore;
		const year = $selectedYearStore;
		if (!vehicle) {
			report = null;
			return;
		}
		load(vehicle.id, year);
	});

	async function load(vehicleId: string, year: number) {
		loading = true;
		selected = null;
		try {
			report = await api.getFuelioCrosscheck(vehicleId, year);
		} catch (e) {
			toast.error(String(e));
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		leaflet = (await import('leaflet')).default;
	});

	onDestroy(() => {
		map?.remove();
		map = null;
	});

	function ensureMap() {
		if (map || !leaflet || !mapEl) return;
		map = leaflet.map(mapEl).setView([48.7, 19.5], 7);
		leaflet
			.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
				maxZoom: 19,
				// Required by the OpenStreetMap tile usage policy.
				attribution:
					'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
			})
			.addTo(map);
		layers = leaflet.layerGroup().addTo(map);
	}

	async function selectRow(r: FuelioRow) {
		if (r.driveIds.length === 0) return;
		selected = r;
		selectedHasRoute = null;
		try {
			const track = await api.getFuelioTrack(r.driveIds, r.tripId);
			selectedHasRoute = track.route !== null;
			ensureMap();
			if (!map || !leaflet || !layers) return;
			layers.clearLayers();
			const L = leaflet;
			const drawn: import('leaflet').Polyline[] = [];
			if (track.route) {
				drawn.push(
					L.polyline(track.route, { color: ROUTE_COLOR, weight: 6, opacity: 0.6 }).addTo(layers)
				);
			}
			for (const line of track.gps) {
				drawn.push(L.polyline(line, { color: GPS_COLOR, weight: 3, opacity: 0.9 }).addTo(layers));
			}
			const bounds = drawn
				.map((p) => p.getBounds())
				.reduce((a, b) => a.extend(b), drawn[0].getBounds());
			map.invalidateSize();
			map.fitBounds(bounds, { padding: [20, 20] });
		} catch (e) {
			toast.error(String(e));
		}
	}

	// Display formatting of backend values.
	function dt(s: string | null): string {
		if (!s) return '';
		const [date, time] = s.split('T');
		const [y, m, d] = date.split('-');
		return `${Number(d)}.${Number(m)}.${y} ${time.slice(0, 5)}`;
	}
	function hm(s: string | null): string {
		return s ? s.split('T')[1].slice(0, 5) : '';
	}
	function num(n: number | null, digits = 0): string {
		return n === null ? '' : n.toFixed(digits);
	}
	function signed(n: number | null, digits: number, unit: string): string {
		if (n === null) return '';
		return `${n > 0 ? '+' : ''}${n.toFixed(digits)} ${unit}`;
	}
</script>

<div class="fuelio-page">
	<section class="fuelio-section">
		<h2 class="heading">{$LL.fuelio.title()}</h2>
		<p class="muted">{$LL.fuelio.intro()}</p>

		{#if !$activeVehicleStore}
			<p>{$LL.fuelio.noVehicle()}</p>
		{:else if loading && !report}
			<p>{$LL.fuelio.loading()}</p>
		{:else if report}
			<p class="muted small">
				{$LL.fuelio.folder({ folder: report.folder })}
				{#if !report.folderExists}
					<strong class="warn">{$LL.fuelio.noFolder()}</strong>
				{:else}
					- {$LL.fuelio.driveCount({ year: $selectedYearStore, count: report.driveCount })}
				{/if}
			</p>

			<div class="filters">
				<label>
					{$LL.fuelio.minKm()}
					<input class="text-input km" type="number" min="0" bind:value={minKm} data-testid="fuelio-min-km" />
				</label>
				<label><input type="checkbox" bind:checked={highwayOnly} /> {$LL.fuelio.highwayOnly()}</label>
				<label><input type="checkbox" bind:checked={problemsOnly} /> {$LL.fuelio.problemsOnly()}</label>
				<label>
					{$LL.fuelio.col.status()}
					<select class="text-input" bind:value={statusFilter} data-testid="fuelio-status-filter">
						<option value="all">{$LL.fuelio.allStatuses()}</option>
						{#each STATUSES as st}
							<option value={st}>{$LL.fuelio.status[st]()}</option>
						{/each}
					</select>
				</label>
				<span class="muted small">{$LL.fuelio.shown({ count: rows.length })}</span>
			</div>

			<div class="layout">
				<div class="table-wrap">
					<table data-testid="fuelio-table">
						<thead>
							<tr>
								<th>{$LL.fuelio.col.status()}</th>
								<th>{$LL.fuelio.col.logbook()}</th>
								<th>{$LL.fuelio.col.route()}</th>
								<th class="r">{$LL.fuelio.col.logbookKm()}</th>
								<th>{$LL.fuelio.col.gps()}</th>
								<th class="r">{$LL.fuelio.col.gpsKm()}</th>
								<th class="r">{$LL.fuelio.col.fast()}</th>
								<th class="r">{$LL.fuelio.col.maxKmh()}</th>
								<th>{$LL.fuelio.col.diff()}</th>
								<th>{$LL.fuelio.col.flags()}</th>
							</tr>
						</thead>
						<tbody>
							{#each rows as r (r.tripId ?? r.driveIds.join('-'))}
								<tr
									class="status-{r.status}"
									class:clickable={r.driveIds.length > 0}
									class:selected={selected === r}
									onclick={() => selectRow(r)}
									data-testid="fuelio-row"
								>
									<td><span class="badge badge-{r.status}">{$LL.fuelio.status[r.status]()}</span></td>
									<td class="nowrap">{dt(r.tripStart)}</td>
									<td>
										{#if r.origin}{r.origin} &rarr; {r.destination}{/if}
									</td>
									<td class="r">{num(r.tripKm)}</td>
									<td class="nowrap">
										{#if r.gpsStart}
											{r.tripStart ? hm(r.gpsStart) : dt(r.gpsStart)} - {hm(r.gpsEnd)}
											{#if r.driveIds.length > 1}
												<span class="muted small">({$LL.fuelio.drives({ count: r.driveIds.length })})</span>
											{/if}
										{/if}
									</td>
									<td class="r">{num(r.gpsKm, 1)}</td>
									<td class="r">{num(r.fastMinutes)}</td>
									<td class="r">{num(r.maxKmh)}</td>
									<td class="nowrap small">
										{signed(r.startDiffMin, 0, 'min')}
										{#if r.kmDiffPct !== null}<br />{signed(r.kmDiffPct, 1, '%')}{/if}
										{#if r.offRoutePct !== null}<br />{$LL.fuelio.offRoute({ pct: Math.round(r.offRoutePct) })}{/if}
									</td>
									<td>
										{#each r.flags as f}
											<span class="badge badge-flag">{$LL.fuelio.flag[f]()}</span>
										{/each}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="map-panel">
					<h3>{$LL.fuelio.mapTitle()}</h3>
					<div class="legend small">
						<span><i style="background:{GPS_COLOR}"></i>{$LL.fuelio.mapLegendGps()}</span>
						<span><i style="background:{ROUTE_COLOR}"></i>{$LL.fuelio.mapLegendRoute()}</span>
					</div>
					{#if !selected}
						<p class="muted small">{$LL.fuelio.mapHint()}</p>
					{:else}
						<p class="small">
							{#if selected.origin}{selected.origin} &rarr; {selected.destination}{:else}{dt(selected.gpsStart)}{/if}
							{#if selected.tripId && selectedHasRoute === false}
								<br /><span class="muted">{$LL.fuelio.noRoute()}</span>
							{/if}
						</p>
					{/if}
					<div class="map" bind:this={mapEl} data-testid="fuelio-map"></div>
				</div>
			</div>
		{/if}
	</section>
</div>

<style>
	.fuelio-page {
		max-width: 1600px;
		margin: 0 auto;
	}
	.fuelio-section {
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px var(--shadow-default);
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.heading {
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}
	.muted {
		color: var(--text-secondary);
		margin: 0;
	}
	.small {
		font-size: 0.8rem;
	}
	.warn {
		color: var(--accent-danger);
	}
	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		align-items: center;
		font-size: 0.875rem;
		color: var(--text-primary);
	}
	.text-input {
		padding: 0.3rem 0.5rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		background-color: var(--input-bg);
		color: var(--text-primary);
	}
	.km {
		width: 5rem;
	}
	.layout {
		display: grid;
		grid-template-columns: minmax(0, 3fr) minmax(320px, 2fr);
		gap: 1rem;
		align-items: start;
	}
	@media (max-width: 1100px) {
		.layout {
			grid-template-columns: 1fr;
		}
	}
	.table-wrap {
		overflow: auto;
		max-height: 75vh;
	}
	table {
		border-collapse: collapse;
		width: 100%;
		font-size: 0.8rem;
		color: var(--text-primary);
	}
	th,
	td {
		padding: 0.3rem 0.4rem;
		border-bottom: 1px solid var(--border-default);
		text-align: left;
		vertical-align: top;
	}
	th {
		position: sticky;
		top: 0;
		background: var(--bg-surface-alt);
	}
	.r {
		text-align: right;
	}
	.nowrap {
		white-space: nowrap;
	}
	tr.clickable {
		cursor: pointer;
	}
	tr.clickable:hover {
		background: var(--accent-primary-light-bg);
	}
	tr.selected {
		background: var(--accent-primary-light-hover);
	}
	.badge {
		display: inline-block;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
		font-size: 0.75rem;
		white-space: nowrap;
	}
	.badge-matched {
		background: var(--accent-success-bg);
		color: var(--accent-success);
	}
	.badge-missing {
		background: var(--badge-danger-bg);
		color: var(--badge-danger-color);
	}
	.badge-noDrive {
		background: var(--bg-surface-alt);
		color: var(--text-secondary);
	}
	.badge-flag {
		background: var(--warning-bg);
		color: var(--warning-color);
		margin: 0 0.2rem 0.2rem 0;
	}
	.map-panel {
		position: sticky;
		top: 1rem;
	}
	.map-panel h3 {
		margin: 0 0 0.25rem;
		font-size: 1rem;
		color: var(--text-primary);
	}
	.legend {
		display: flex;
		gap: 1rem;
		color: var(--text-secondary);
	}
	.legend i {
		display: inline-block;
		width: 1rem;
		height: 0.25rem;
		margin-right: 0.3rem;
		vertical-align: middle;
	}
	.map {
		height: 60vh;
		min-height: 320px;
		border-radius: 6px;
		border: 1px solid var(--border-default);
	}
</style>
