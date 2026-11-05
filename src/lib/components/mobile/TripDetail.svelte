<script lang="ts">
	// One trip, read only (Task 93). It shows the values of one row of the PC
	// grid, one per line. No edit and no delete on the phone.
	import type { PaperlessInvoiceRow, Trip } from '$lib/types';
	import LL, { locale } from '$lib/i18n/i18n-svelte';
	import type { InvoiceState, TripLookup } from './TripLookup';
	import MobileBar from './MobileBar.svelte';
	import TripRouteMap from './TripRouteMap.svelte';
	import { fullDayLabel, timeRange, whole, fixed, shortDate } from './MobileFormat';

	let {
		trip,
		lookup,
		invoiceState,
		invoiceRows,
		onclose
	}: {
		trip: Trip;
		lookup: TripLookup;
		invoiceState: InvoiceState;
		invoiceRows: PaperlessInvoiceRow[];
		onclose: () => void;
	} = $props();

	let grid = $derived(lookup.grid);
	let id = $derived(trip.id);
	let rate = $derived(grid.rates[id]);
	let energyRate = $derived(grid.energyRates[id]);
	let fuelRemaining = $derived(grid.fuelRemaining[id]);
	let batteryKwh = $derived(grid.batteryRemainingKwh[id]);
	let batteryPercent = $derived(grid.batteryRemainingPercent[id]);
	let odometerStart = $derived(grid.odometerStart[id]);
	let tripNumber = $derived(grid.tripNumbers[id]);
	let overLimit = $derived(lookup.overLimit.has(id));
	let hasFuel = $derived(trip.fuelLiters != null && trip.fuelLiters > 0);
	let hasCharge = $derived(trip.energyKwh != null && trip.energyKwh > 0);
	let hasOther = $derived(
		(trip.otherCostsEur != null && trip.otherCostsEur > 0) || !!trip.otherCostsNote
	);
	let tripInvoices = $derived(invoiceRows.filter((r) => r.tripId === id));

	// "DD.MM. HH:MM", the format of the PC grid warnings.
	function shortDatetime(s: string): string {
		const [date, time = ''] = s.split('T');
		const [, m, d] = date.split('-');
		return `${d}.${m}. ${time.slice(0, 5)}`;
	}

	// The warnings of this trip in plain words, from the backend flags.
	let warnings = $derived.by(() => {
		const out: string[] = [];
		const tripText = `${shortDatetime(trip.startDatetime)} - ${trip.endDatetime ? shortDatetime(trip.endDatetime) : '-'}`;
		if (overLimit) out.push($LL.mobile.trips.detail.overLimit());
		if (lookup.missingFuelInvoice.has(id)) out.push($LL.trips.legend.missingFuelInvoice());
		if (lookup.missingOtherInvoice.has(id)) out.push($LL.trips.legend.missingOtherInvoice());
		if (lookup.otherSumMismatch.has(id)) {
			out.push(
				$LL.trips.legend.otherSumMismatch({
					total: fixed(trip.otherCostsEur ?? 0, 2),
					sum: fixed(grid.otherInvoiceSums[id] ?? 0, 2)
				})
			);
		}
		if (lookup.fuelDatetime.has(id) && !lookup.fuelOverride.has(id)) {
			for (const r of grid.fuelReceiptDatetimes[id] ?? []) {
				out.push($LL.trips.legend.fuelReceiptOutsideTrip({ receipt: shortDatetime(r), trip: tripText }));
			}
		}
		if (lookup.otherDatetime.has(id) && !lookup.otherOverride.has(id)) {
			for (const r of grid.otherReceiptDatetimes[id] ?? []) {
				out.push($LL.trips.legend.otherReceiptOutsideTrip({ receipt: shortDatetime(r), trip: tripText }));
			}
		}
		if (lookup.duplicateDatetime.has(id)) out.push($LL.trips.legend.duplicateDatetimeTooltip());
		if (lookup.odometerSpan.has(id)) {
			out.push(
				$LL.trips.legend.odometerSpanMismatchTooltip({
					span: whole(grid.odometerSpans[id] ?? 0, $locale),
					km: whole(trip.distanceKm, $locale)
				})
			);
		}
		return out;
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onclose();
	}
</script>

<svelte:window {onkeydown} />

<div class="overlay" role="dialog" aria-modal="true" aria-label={fullDayLabel(trip.startDatetime, $locale)} data-testid="mobile-trip-detail" data-trip-id={id}>
	<MobileBar
		title={fullDayLabel(trip.startDatetime, $locale)}
		subtitle={tripNumber !== undefined ? $LL.mobile.trips.detail.tripNumber({ number: tripNumber }) : ''}
		backLabel={$LL.mobile.trips.detail.back()}
		onback={onclose}
	/>
	<div class="body">
		<section class="field">
			<span class="label">{$LL.mobile.trips.detail.route()}</span>
			<span class="value" data-testid="mobile-detail-route"
				><b>{trip.origin} -&gt; {trip.destination}</b><br />{timeRange(
					trip.startDatetime,
					trip.endDatetime
				)}, {whole(trip.distanceKm, $locale)} km</span
			>
		</section>

		{#if lookup.routeMap.has(id)}
			{#key id}<TripRouteMap tripId={id} />{/key}
		{/if}

		{#if trip.purpose}
			<section class="field">
				<span class="label">{$LL.mobile.trips.detail.purpose()}</span>
				<span class="value" data-testid="mobile-detail-purpose">{trip.purpose}</span>
			</section>
		{/if}

		<div class="card" data-testid="mobile-detail-values">
			{#if odometerStart !== undefined}
				<div class="row">
					<span>{$LL.mobile.trips.detail.odometerStart()}</span><span>{whole(odometerStart, $locale)} km</span>
				</div>
			{/if}
			<div class="row">
				<span>{$LL.mobile.trips.detail.odometerEnd()}</span><span>{whole(trip.odometer, $locale)} km</span>
			</div>
			{#if lookup.showFuel && rate !== undefined}
				<div class="row" data-testid="mobile-detail-rate">
					<span>{$LL.mobile.trips.detail.consumption()}</span>
					<span class="end">
						{fixed(rate, 2)} l/100 km
						{#if lookup.estimated.has(id)}<span class="pill muted">{$LL.trips.estimatedRate()}</span>{/if}
						{#if overLimit}<span class="pill warn">{$LL.mobile.trips.overLimit()}</span>{/if}
					</span>
				</div>
			{/if}
			{#if lookup.showFuel && fuelRemaining !== undefined}
				<div class="row" data-testid="mobile-detail-fuel-remaining">
					<span>{$LL.mobile.trips.detail.fuelRemaining()}</span><span>{fixed(fuelRemaining, 1)} l</span>
				</div>
			{/if}
			{#if lookup.showEnergy && energyRate !== undefined}
				<div class="row" data-testid="mobile-detail-energy-rate">
					<span>{$LL.mobile.trips.detail.energyRate()}</span>
					<span class="end">
						{fixed(energyRate, 1)} kWh/100 km
						{#if lookup.estimatedEnergy.has(id)}<span class="pill muted">{$LL.trips.estimatedEnergyRate()}</span>{/if}
					</span>
				</div>
			{/if}
			{#if lookup.showEnergy && batteryKwh !== undefined}
				<div class="row" data-testid="mobile-detail-battery">
					<span>{$LL.mobile.trips.detail.battery()}</span>
					<span
						>{fixed(batteryKwh, 1)} kWh{#if batteryPercent !== undefined}
							({fixed(batteryPercent, 0)} %){/if}</span
					>
				</div>
			{/if}
		</div>

		{#if hasFuel}
			<div class="card" data-testid="mobile-detail-fillup">
				<div class="top">
					<span>{$LL.mobile.trips.detail.fillup()}</span>
					<span class="pill" class:ok={trip.fullTank} class:muted={!trip.fullTank}
						>{trip.fullTank ? $LL.mobile.trips.detail.fullTank() : $LL.trips.partialFillup()}</span
					>
				</div>
				<div class="row">
					<span>{$LL.mobile.trips.detail.amount()}</span><span>{fixed(trip.fuelLiters ?? 0, 2)} l</span>
				</div>
				{#if trip.fuelCostEur != null}
					<div class="row">
						<span>{$LL.mobile.trips.detail.cost()}</span><span>{fixed(trip.fuelCostEur, 2)} EUR</span>
					</div>
				{/if}
			</div>
		{/if}

		{#if hasCharge}
			<div class="card" data-testid="mobile-detail-charge">
				<div class="top">
					<span>{$LL.mobile.trips.detail.charge()}</span>
					<span class="pill" class:ok={trip.fullCharge} class:muted={!trip.fullCharge}
						>{trip.fullCharge ? $LL.mobile.trips.detail.fullCharge() : $LL.trips.partialCharge()}</span
					>
				</div>
				<div class="row">
					<span>{$LL.mobile.trips.detail.amount()}</span><span>{fixed(trip.energyKwh ?? 0, 1)} kWh</span>
				</div>
				{#if trip.energyCostEur != null}
					<div class="row">
						<span>{$LL.mobile.trips.detail.cost()}</span><span>{fixed(trip.energyCostEur, 2)} EUR</span>
					</div>
				{/if}
			</div>
		{/if}

		{#if hasOther}
			<div class="card" data-testid="mobile-detail-other">
				<div class="top"><span>{$LL.mobile.trips.detail.otherCosts()}</span></div>
				{#if trip.otherCostsEur != null}
					<div class="row">
						<span>{$LL.mobile.trips.detail.cost()}</span><span>{fixed(trip.otherCostsEur, 2)} EUR</span>
					</div>
				{/if}
				{#if trip.otherCostsNote}
					<div class="row">
						<span>{$LL.mobile.trips.detail.otherNote()}</span><span class="end">{trip.otherCostsNote}</span>
					</div>
				{/if}
			</div>
		{/if}

		{#if invoiceState !== 'off'}
			<div class="card" data-testid="mobile-detail-invoices">
				<div class="top"><span>{$LL.mobile.trips.detail.invoices()}</span></div>
				{#if invoiceState === 'loading'}
					<p class="note">{$LL.mobile.trips.detail.invoicesLoading()}</p>
				{:else if invoiceState === 'error'}
					<p class="note">{$LL.mobile.trips.detail.invoicesError()}</p>
				{:else if tripInvoices.length === 0}
					<p class="note">{$LL.mobile.trips.detail.noInvoices()}</p>
				{:else}
					{#each tripInvoices as inv (inv.paperlessDocumentId)}
						<a
							class="row invoice"
							href={inv.paperlessUrl}
							target="_blank"
							rel="noopener noreferrer"
							data-testid="mobile-detail-invoice"
						>
							<span class="end-start"
								>{inv.title}, {shortDate(inv.receiptDatetime ?? inv.createdDate)}</span
							>
							<span
								>{#if inv.totalPriceEur != null}{fixed(inv.totalPriceEur, 2)} EUR{/if} &gt;</span
							>
						</a>
					{/each}
				{/if}
			</div>
		{/if}

		{#if warnings.length > 0}
			<div class="warnbox" data-testid="mobile-detail-warnings">
				<b>{$LL.mobile.trips.detail.warnings()}</b>
				<ul>
					{#each warnings as w}<li>{w}</li>{/each}
				</ul>
			</div>
		{/if}

		<p class="note">{$LL.mobile.trips.detail.readOnly()}</p>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 50;
		overflow-y: auto;
		overscroll-behavior: contain;
		background: var(--bg-body);
	}

	.body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 640px;
		margin: 0 auto;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.label {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.value {
		font-variant-numeric: tabular-nums;
		overflow-wrap: anywhere;
	}

	.card {
		border: 1px solid var(--border-default);
		border-radius: 10px;
		padding: 0.625rem 0.75rem;
		background: var(--bg-surface);
		display: flex;
		flex-direction: column;
	}

	.top {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-weight: 600;
		padding-bottom: 0.25rem;
	}

	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
		min-height: 2rem;
		padding: 0.25rem 0;
		border-bottom: 1px solid var(--border-default);
		font-size: 0.925rem;
		font-variant-numeric: tabular-nums;
	}

	.row:last-child {
		border-bottom: none;
	}

	.end {
		text-align: right;
		overflow-wrap: anywhere;
	}

	.end-start {
		overflow-wrap: anywhere;
	}

	.invoice {
		min-height: 44px;
		color: var(--accent-primary);
		text-decoration: none;
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

	.pill.muted {
		background: var(--bg-surface-alt);
		color: var(--text-secondary);
	}

	.warnbox {
		background: var(--warning-bg);
		border: 1px solid var(--warning-border);
		border-radius: 8px;
		padding: 0.5rem 0.75rem;
		font-size: 0.9rem;
	}

	.warnbox b {
		color: var(--warning-color);
	}

	.warnbox ul {
		margin: 0.25rem 0 0;
		padding-left: 1.1rem;
	}

	.note {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
	}
</style>
