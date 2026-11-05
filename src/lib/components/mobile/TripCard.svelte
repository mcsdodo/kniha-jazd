<script lang="ts">
	// One trip in the phone list (Task 93). Pills show backend flags only.
	import type { Trip } from '$lib/types';
	import LL, { locale } from '$lib/i18n/i18n-svelte';
	import type { TripLookup } from './TripLookup';
	import { dayLabel, timeRange, whole, fixed } from './MobileFormat';

	let { trip, lookup, onopen }: { trip: Trip; lookup: TripLookup; onopen: (id: string) => void } =
		$props();

	let hasFuel = $derived(lookup.showFuel && trip.fuelLiters != null && trip.fuelLiters > 0);
	let hasCharge = $derived(lookup.showEnergy && trip.energyKwh != null && trip.energyKwh > 0);
	let rate = $derived(lookup.grid.rates[trip.id]);
	let missingInvoice = $derived(
		lookup.missingFuelInvoice.has(trip.id) || lookup.missingOtherInvoice.has(trip.id)
	);
	let overLimit = $derived(lookup.overLimit.has(trip.id));
</script>

<button
	type="button"
	class="card"
	onclick={() => onopen(trip.id)}
	data-testid="mobile-trip-card"
	data-trip-id={trip.id}
>
	<span class="top">
		<span>{dayLabel(trip.startDatetime, $locale)} {timeRange(trip.startDatetime, trip.endDatetime)}</span>
		<span class="km">{whole(trip.distanceKm, $locale)} km</span>
	</span>
	<span class="route">{trip.origin} -&gt; {trip.destination}</span>
	<span class="meta">
		{#if trip.purpose}{trip.purpose}, {/if}{$LL.mobile.trips.odometer({ km: whole(trip.odometer, $locale) })}
	</span>
	{#if hasFuel || hasCharge || overLimit || missingInvoice}
		<span class="pills">
			{#if hasFuel}
				<span class="pill ok" data-testid="mobile-pill-fillup"
					>{$LL.mobile.trips.fillup({ liters: fixed(trip.fuelLiters ?? 0, 2) })}, {trip.fullTank
						? $LL.mobile.trips.fillupFull()
						: $LL.mobile.trips.fillupPartial()}</span
				>
				{#if trip.fullTank && rate !== undefined && !lookup.estimated.has(trip.id)}
					<span class="pill" class:ok={!overLimit} class:warn={overLimit} data-testid="mobile-pill-rate"
						>{fixed(rate, 2)} l/100 km</span
					>
				{/if}
			{/if}
			{#if hasCharge}
				<span class="pill ok" data-testid="mobile-pill-charge"
					>{$LL.mobile.trips.charge({ kwh: fixed(trip.energyKwh ?? 0, 1) })}, {trip.fullCharge
						? $LL.mobile.trips.chargeFull()
						: $LL.mobile.trips.chargePartial()}</span
				>
			{/if}
			{#if overLimit}
				<span class="pill warn" data-testid="mobile-pill-over-limit">{$LL.mobile.trips.overLimit()}</span>
			{/if}
			{#if missingInvoice}
				<span class="pill warn" data-testid="mobile-pill-missing-invoice"
					>{$LL.mobile.trips.missingInvoice()}</span
				>
			{/if}
		</span>
	{/if}
</button>

<style>
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		width: 100%;
		min-height: 44px;
		padding: 0.65rem 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 10px;
		background: var(--bg-surface);
		color: var(--text-primary);
		font: inherit;
		text-align: left;
		cursor: pointer;
		box-sizing: border-box;
	}

	.card:active {
		background: var(--bg-surface-alt);
	}

	.top {
		display: flex;
		justify-content: space-between;
		gap: 0.5rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.km {
		white-space: nowrap;
	}

	.route {
		overflow-wrap: anywhere;
	}

	.meta {
		font-size: 0.875rem;
		color: var(--text-secondary);
		overflow-wrap: anywhere;
	}

	.pills {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-top: 0.125rem;
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
</style>
