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
	import FuelioOverwriteModal from '$lib/components/FuelioOverwriteModal.svelte';
	import FuelioAddModal from '$lib/components/FuelioAddModal.svelte';

	// Task 90 POC: read-only cross-check of Fuelio GPS drives against the
	// logbook. The backend matches, measures and flags every row (ADR-008);
	// this page only filters the rows for display and draws the map.

	const GPS_COLOR = '#d63031';
	const ROUTE_COLOR = '#0984e3';

	let report = $state<FuelioReport | null>(null);
	let loading = $state(false);
	// Empty: no km filter.
	let minKm = $state<number | null>(null);
	// Filter pills: a pill that is not selected does not filter. State pills
	// combine with OR (none selected = every state); the groups with AND.
	let highwayOnly = $state(false);
	let problemsOnly = $state(false);
	const STATUSES: FuelioRowStatus[] = ['matched', 'missing'];
	let selectedStatuses = $state<FuelioRowStatus[]>([]);

	function toggleStatus(st: FuelioRowStatus) {
		selectedStatuses = selectedStatuses.includes(st)
			? selectedStatuses.filter((x) => x !== st)
			: [...selectedStatuses, st];
	}
	let selected = $state<FuelioRow | null>(null);
	let selectedHasRoute = $state<boolean | null>(null);
	let overwriting = $state<FuelioRow | null>(null);
	let adding = $state<FuelioRow | null>(null);
	let syncing = $state(false);

	async function syncDropbox() {
		const vehicle = $activeVehicleStore;
		const year = $selectedYearStore;
		if (!vehicle) return;
		syncing = true;
		try {
			const r = await api.syncFuelioDropbox(year);
			const message = $LL.fuelio.sync.done({
				year: r.year,
				downloaded: r.downloaded,
				total: r.inDropbox
			});
			if (r.failed.length > 0) {
				toast.error(`${message} ${$LL.fuelio.sync.failed({ count: r.failed.length })}`);
			} else {
				toast.success(message);
			}
			await load(vehicle.id, year);
		} catch (e) {
			toast.error($LL.fuelio.sync.error({ error: String(e) }));
		} finally {
			syncing = false;
		}
	}

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
				(selectedStatuses.length === 0 || selectedStatuses.includes(r.status))
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

			{#if report.dropboxConfigured}
				<div class="sync-bar">
					<button
						type="button"
						class="sync-btn"
						disabled={syncing}
						onclick={syncDropbox}
						data-testid="fuelio-sync-dropbox"
						>{syncing
							? $LL.fuelio.sync.running()
							: $LL.fuelio.sync.button({ year: $selectedYearStore })}</button
					>
					<span class="muted small">{$LL.fuelio.sync.hint()}</span>
				</div>
			{/if}

			<div class="filters">
				<label>
					{$LL.fuelio.minKm()}
					<input class="text-input km" type="number" min="0" bind:value={minKm} data-testid="fuelio-min-km" />
				</label>
				<div class="pills" data-testid="fuelio-filter-pills">
					{#each STATUSES as st}
						<button
							type="button"
							class="pill"
							aria-pressed={selectedStatuses.includes(st)}
							onclick={() => toggleStatus(st)}
							data-testid="fuelio-pill-{st}">{$LL.fuelio.status[st]()}</button
						>
					{/each}
					<button
						type="button"
						class="pill"
						aria-pressed={problemsOnly}
						onclick={() => (problemsOnly = !problemsOnly)}
						data-testid="fuelio-pill-problems">{$LL.fuelio.problemsOnly()}</button
					>
					<button
						type="button"
						class="pill"
						aria-pressed={highwayOnly}
						title={$LL.fuelio.highwayHint()}
						onclick={() => (highwayOnly = !highwayOnly)}
						data-testid="fuelio-pill-highway">{$LL.fuelio.highwayOnly()}</button
					>
				</div>
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
								<th></th>
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
									<td class="status-cell">
										<span
											class="status-icon status-icon-{r.status}"
											role="img"
											aria-label={$LL.fuelio.status[r.status]()}
											title={$LL.fuelio.status[r.status]()}
											data-testid="fuelio-status-{r.status}"
										>
											{#if r.status === 'matched'}
												<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/></svg>
											{:else}
												<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>
											{/if}
										</span>
									</td>
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
											<span
												class="flag-icon"
												role="img"
												aria-label={$LL.fuelio.flag[f]()}
												title={$LL.fuelio.flag[f]()}
												data-testid="fuelio-flag-{f}"
											>
												{#if f === 'timeDiffers'}
													<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
												{:else if f === 'kmDiffers'}
													<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21.3 15.3a2.4 2.4 0 0 1 0 3.4l-2.6 2.6a2.4 2.4 0 0 1-3.4 0L2.7 8.7a2.41 2.41 0 0 1 0-3.4l2.6-2.6a2.41 2.41 0 0 1 3.4 0Z"/><path d="m14.5 12.5 2-2"/><path d="m11.5 9.5 2-2"/><path d="m8.5 6.5 2-2"/><path d="m17.5 15.5 2-2"/></svg>
												{:else if f === 'differentRoute'}
													<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="6" cy="19" r="3"/><path d="M9 19h8.5a3.5 3.5 0 0 0 0-7h-11a3.5 3.5 0 0 1 0-7H15"/><circle cx="18" cy="5" r="3"/></svg>
												{:else if f === 'partialGps'}
													<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12.75 7.09a3 3 0 0 1 2.16 2.16"/><path d="M17.072 17.072c-1.634 2.17-3.527 3.912-4.471 4.727a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 1.432-4.568"/><path d="m2 2 20 20"/><path d="M8.475 2.818A8 8 0 0 1 20 10c0 1.183-.31 2.377-.81 3.533"/><path d="M9.13 9.13a3 3 0 0 0 3.74 3.74"/></svg>
												{:else}
													<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m18.84 12.25 1.72-1.71h-.02a5.004 5.004 0 0 0-.12-7.07 5.006 5.006 0 0 0-6.95 0l-1.72 1.71"/><path d="m5.17 11.75-1.71 1.71a5.004 5.004 0 0 0 .12 7.07 5.006 5.006 0 0 0 6.95 0l1.71-1.71"/><line x1="8" x2="8" y1="2" y2="5"/><line x1="2" x2="5" y1="8" y2="8"/><line x1="16" x2="16" y1="19" y2="22"/><line x1="19" x2="22" y1="16" y2="16"/></svg>
												{/if}
											</span>
										{/each}
									</td>
									<td>
										{#if r.tripId}
											<button
												class="overwrite-btn"
												onclick={(e) => {
													e.stopPropagation();
													overwriting = r;
												}}
												data-testid="fuelio-overwrite">{$LL.fuelio.overwrite.button()}</button
											>
										{:else if r.status === 'missing'}
											<button
												class="overwrite-btn"
												onclick={(e) => {
													e.stopPropagation();
													adding = r;
												}}
												data-testid="fuelio-add">{$LL.fuelio.add.button()}</button
											>
										{/if}
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

{#if adding && $activeVehicleStore}
	<FuelioAddModal
		row={adding}
		vehicleId={$activeVehicleStore.id}
		year={$selectedYearStore}
		onCancel={() => (adding = null)}
		onDone={() => {
			adding = null;
			load($activeVehicleStore!.id, $selectedYearStore);
		}}
	/>
{/if}

{#if overwriting && $activeVehicleStore}
	<FuelioOverwriteModal
		row={overwriting}
		vehicleId={$activeVehicleStore.id}
		year={$selectedYearStore}
		onCancel={() => (overwriting = null)}
		onDone={() => {
			overwriting = null;
			load($activeVehicleStore!.id, $selectedYearStore);
		}}
	/>
{/if}

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
	.sync-bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex-wrap: wrap;
	}
	.sync-btn {
		padding: 0.4rem 0.9rem;
		border: none;
		border-radius: 4px;
		background: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		font-size: 0.875rem;
		cursor: pointer;
	}
	.sync-btn:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}
	.pill {
		padding: 0.25rem 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 999px;
		background: var(--bg-surface);
		color: var(--text-primary);
		font-size: 0.8rem;
		cursor: pointer;
	}
	.pill:hover {
		background: var(--accent-primary-light-bg);
	}
	.pill[aria-pressed='true'] {
		background: var(--btn-active-primary-bg);
		border-color: var(--btn-active-primary-color);
		color: var(--btn-active-primary-color);
		font-weight: 600;
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
	.status-cell {
		text-align: center;
	}
	.status-icon {
		display: inline-flex;
		vertical-align: middle;
	}
	.status-icon-matched {
		color: var(--accent-success);
	}
	.status-icon-missing {
		color: var(--badge-danger-color);
	}
	.flag-icon {
		display: inline-flex;
		vertical-align: middle;
		margin-right: 0.3rem;
		color: var(--warning-color);
	}
	.overwrite-btn {
		padding: 0.2rem 0.5rem;
		font-size: 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		background: var(--btn-secondary-bg);
		color: var(--text-primary);
		cursor: pointer;
		white-space: nowrap;
	}
	.overwrite-btn:hover {
		background: var(--btn-secondary-hover);
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
