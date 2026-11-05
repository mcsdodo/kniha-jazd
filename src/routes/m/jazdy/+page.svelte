<script lang="ts">
	// Task 93: all trips of the year on the phone, newest first, grouped by
	// month. A tap opens a read-only detail on top of the list. The detail is
	// a shallow-routing history entry, so the browser back button closes it
	// and the list keeps its scroll position. All values come from the
	// backend (ADR-008); this page only sorts and groups them for display.
	import { pushState } from '$app/navigation';
	import { page } from '$app/state';
	import {
		calculateTripStats,
		getPaperlessInvoices,
		getPaperlessSettings,
		getTripGridData,
		getYearsWithTrips
	} from '$lib/api';
	import type { PaperlessInvoiceRow, Trip, TripGridData, TripStats } from '$lib/types';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import LL, { locale } from '$lib/i18n/i18n-svelte';
	import MobileBar from '$lib/components/mobile/MobileBar.svelte';
	import MobileVehicleGate from '$lib/components/mobile/MobileVehicleGate.svelte';
	import TripCard from '$lib/components/mobile/TripCard.svelte';
	import TripDetail from '$lib/components/mobile/TripDetail.svelte';
	import { buildTripLookup, type InvoiceState } from '$lib/components/mobile/TripLookup';
	import { fixed, monthKey, monthLabel, whole } from '$lib/components/mobile/MobileFormat';

	let years = $state<number[]>([new Date().getFullYear()]);
	let grid = $state<TripGridData | null>(null);
	let stats = $state<TripStats | null>(null);
	let loading = $state(true);
	let loadError = $state<string | null>(null);

	// The startup writes the vehicle and then the year, so two loads can run
	// at the same time. Only the newest one may write.
	let latestRequest = 0;

	$effect(() => {
		const vehicle = $activeVehicleStore;
		if (!vehicle) return;
		getYearsWithTrips(vehicle.id)
			.then((withData) => {
				// The same list as the PC header: the current year and every year with data.
				const all = new Set([new Date().getFullYear(), ...withData]);
				years = [...all].sort((a, b) => b - a);
			})
			.catch((error) => console.error('Failed to load years:', error));
	});

	$effect(() => {
		const vehicle = $activeVehicleStore;
		const year = $selectedYearStore;
		if (vehicle) load(vehicle.id, year);
	});

	async function load(vehicleId: string, year: number) {
		const request = ++latestRequest;
		loading = true;
		try {
			const [g, s] = await Promise.all([
				getTripGridData(vehicleId, year),
				calculateTripStats(vehicleId, year)
			]);
			if (request !== latestRequest) return;
			grid = g;
			stats = s;
			loadError = null;
		} catch (error) {
			if (request !== latestRequest) return;
			loadError = String(error);
		} finally {
			if (request === latestRequest) loading = false;
		}
	}

	let lookup = $derived(
		grid && $activeVehicleStore ? buildTripLookup(grid, $activeVehicleStore.vehicleType) : null
	);

	// Newest first in the book order of the backend (trip numbers). A sort on
	// the datetime string would lose the backend tie-break of equal times.
	// The synthetic month-end rows are not trips and do not show here.
	let groups = $derived.by(() => {
		if (!grid) return [];
		const numbers = grid.tripNumbers;
		const sorted = [...grid.trips].sort((a, b) => (numbers[b.id] ?? 0) - (numbers[a.id] ?? 0));
		const out: { key: string; trips: Trip[] }[] = [];
		for (const trip of sorted) {
			const key = monthKey(trip.startDatetime);
			const last = out[out.length - 1];
			if (last && last.key === key) last.trips.push(trip);
			else out.push({ key, trips: [trip] });
		}
		return out;
	});

	// The open trip lives in the history state of a shallow-routing entry.
	let openTripId = $derived((page.state as { tripId?: string }).tripId ?? null);
	let openTrip = $derived(openTripId && grid ? (grid.trips.find((t) => t.id === openTripId) ?? null) : null);

	function openDetail(tripId: string) {
		pushState('', { tripId });
	}

	function closeDetail() {
		history.back();
	}

	// The Paperless rows of the year, loaded on the first detail open. Paperless
	// is slow and can be off, so the list never waits for it.
	let invoiceKey = '';
	let invoiceState = $state<InvoiceState>('loading');
	let invoiceRows = $state<PaperlessInvoiceRow[]>([]);

	$effect(() => {
		const vehicle = $activeVehicleStore;
		const year = $selectedYearStore;
		if (!openTrip || !vehicle) return;
		const key = `${vehicle.id}:${year}`;
		if (key === invoiceKey) return;
		invoiceKey = key;
		invoiceState = 'loading';
		invoiceRows = [];
		// A Paperless that the user switched off in the settings shows nothing.
		getPaperlessSettings()
			.then(async (settings) => {
				if (!settings.enabled) {
					if (invoiceKey === key) invoiceState = 'off';
					return;
				}
				const rows = await getPaperlessInvoices(vehicle.id, year);
				if (invoiceKey !== key) return;
				invoiceRows = rows;
				invoiceState = 'ready';
			})
			.catch((error) => {
				if (invoiceKey !== key) return;
				// "NotConfigured:" is the stable prefix of PaperlessError::NotConfigured.
				invoiceState = String(error).includes('NotConfigured') ? 'off' : 'error';
				if (invoiceState === 'error') console.warn('Failed to load Paperless invoices:', error);
			});
	});

	function handleYearChange(event: Event) {
		selectedYearStore.set(parseInt((event.target as HTMLSelectElement).value, 10));
	}

	let showFuel = $derived($activeVehicleStore?.vehicleType !== 'Bev');
	let tp = $derived($activeVehicleStore?.tpConsumption ?? null);
</script>

<MobileBar title={$LL.mobile.trips.title()} backHref="/m" backLabel={$LL.mobile.trips.back()}>
	{#snippet right()}
		<label class="year">
			<span class="sr-only">{$LL.mobile.trips.yearLabel()}</span>
			<select value={$selectedYearStore} onchange={handleYearChange} data-testid="mobile-year-select">
				{#each years as year}
					<option value={year}>{year}</option>
				{/each}
			</select>
		</label>
	{/snippet}
</MobileBar>

<MobileVehicleGate>
	<div class="body">
		{#if stats}
			<div class="card summary" class:over={stats.isOverLimit} data-testid="mobile-trips-summary">
				{#if showFuel}
					<div class="top">
						<span data-testid="mobile-summary-consumption"
							>{$LL.mobile.trips.consumption({ rate: fixed(stats.avgConsumptionRate, 2) })}</span
						>
						{#if tp !== null}
							<!-- The year as % of TP (BIZ-033). The warning style comes from
							     the per-period check: one period over the limit is enough. -->
							<span
								class="pill"
								class:ok={!stats.isOverLimit}
								class:warn={stats.isOverLimit}
								data-testid="mobile-summary-deviation"
								>{stats.overallDeviationPercent !== null
									? $LL.mobile.trips.yearDeviation({
											percent: fixed(stats.overallDeviationPercent, 1),
											tp: fixed(tp, 1)
										})
									: $LL.mobile.trips.tpNoFillup({ tp: fixed(tp, 1) })}</span
							>
						{/if}
					</div>
					<div class="meta">
						{$LL.mobile.trips.totalKm({ km: whole(stats.totalKm, $locale) })},
						{$LL.mobile.trips.remaining({ liters: fixed(stats.fuelRemainingLiters, 1) })}
					</div>
					{#if stats.isOverLimit}
						<div class="meta small warn-text" data-testid="mobile-summary-period-over">
							{$LL.mobile.trips.periodOverLimit()}
						</div>
					{/if}
				{:else}
					<div class="top">
						<span>{$LL.mobile.trips.totalKm({ km: whole(stats.totalKm, $locale) })}</span>
					</div>
				{/if}
			</div>
		{/if}

		{#if loadError}
			<p class="error">{$LL.mobile.trips.loadError({ error: loadError })}</p>
		{:else if loading && !grid}
			<p class="muted">{$LL.mobile.trips.loading()}</p>
		{:else if grid && grid.trips.length === 0}
			<p class="muted" data-testid="mobile-trips-empty">{$LL.mobile.trips.empty({ year: $selectedYearStore })}</p>
		{/if}

		{#if lookup}
			{#each groups as group (group.key)}
				<section class="month" data-testid="mobile-trips-month">
					<h2>{monthLabel(group.key, $locale)}</h2>
					{#each group.trips as trip (trip.id)}
						<TripCard {trip} {lookup} onopen={openDetail} />
					{/each}
				</section>
			{/each}
		{/if}
	</div>

	{#if openTrip && lookup}
		<TripDetail trip={openTrip} {lookup} {invoiceState} {invoiceRows} onclose={closeDetail} />
	{/if}
</MobileVehicleGate>

<style>
	.body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 640px;
		margin: 0 auto;
	}

	.year select {
		min-height: 44px;
		padding: 0 0.5rem;
		border: 1px solid var(--border-input);
		border-radius: 8px;
		background: var(--input-bg);
		color: var(--text-primary);
		font-size: 1rem;
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}

	.card {
		border: 1px solid var(--border-default);
		border-radius: 10px;
		padding: 0.65rem 0.75rem;
		background: var(--bg-surface);
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.summary.over {
		border-color: var(--warning-border);
	}

	.top {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: center;
		gap: 0.375rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.meta {
		font-size: 0.875rem;
		color: var(--text-secondary);
	}

	.meta.small {
		font-size: 0.8rem;
	}

	.warn-text {
		color: var(--warning-color);
		font-weight: 600;
	}

	.pill {
		display: inline-block;
		padding: 0.125rem 0.5rem;
		border-radius: 999px;
		font-size: 0.8rem;
		font-weight: 600;
	}

	.pill.ok {
		background: var(--accent-success-bg);
		color: var(--accent-success);
	}

	.pill.warn {
		background: var(--warning-bg);
		color: var(--warning-color);
	}

	.month {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.month h2 {
		margin: 0.5rem 0 0;
		font-size: 0.95rem;
		font-weight: 700;
	}

	.muted {
		margin: 0;
		color: var(--text-secondary);
	}

	.error {
		margin: 0;
		color: var(--accent-danger);
	}
</style>
