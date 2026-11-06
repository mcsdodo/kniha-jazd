<script lang="ts">
	// Task 93: the phone start page. Big buttons to the add flows and the
	// trip list, then the year stats of the PC page with its suggestions. The
	// values come from the backend (ADR-008). The vehicle settings row of the
	// PC page (name, plate, tank, TP) is not here.
	import { onDestroy, onMount } from 'svelte';
	import {
		calculateTripStats,
		getTripGridData,
		isFuelioAvailable,
		testHaConnection
	} from '$lib/api';
	import type { SuggestedFillup, TripStats } from '$lib/types';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { appModeStore } from '$lib/stores/appMode';
	import { haStore } from '$lib/stores/homeAssistant';
	import { haReadings } from '$lib/haReadings';
	import LL, { locale } from '$lib/i18n/i18n-svelte';
	import MobileBar from '$lib/components/mobile/MobileBar.svelte';
	import MobileVehicleGate from '$lib/components/mobile/MobileVehicleGate.svelte';
	import CompensationBanner from '$lib/components/CompensationBanner.svelte';
	import { fixed, whole } from '$lib/components/mobile/MobileFormat';

	let fuelioAvailable = $state(false);
	let haConnected = $state(false);
	let summary = $state<{
		year: number;
		count: number;
		stats: TripStats;
		maxOdo: number | null;
		suggestedFillup: SuggestedFillup | null;
	} | null>(null);
	let loadError = $state<string | null>(null);

	// The startup writes the vehicle and the year one after the other, so two
	// loads can run at the same time. Only the newest one may write.
	let latestRequest = 0;

	onMount(() => {
		isFuelioAvailable()
			.then((v) => (fuelioAvailable = v))
			.catch((error) => console.error('Failed to check Fuelio:', error));
		testHaConnection()
			.then((v) => (haConnected = v))
			.catch((error) => {
				console.warn('Failed to test HA connection:', error);
				haConnected = false;
			});
	});

	onDestroy(() => haStore.stopPeriodicRefresh());

	$effect(() => {
		const vehicle = $activeVehicleStore;
		const year = $selectedYearStore;
		if (vehicle) load(vehicle.id, year);
	});

	// The same Home Assistant refresh as the PC page.
	$effect(() => {
		const vehicle = $activeVehicleStore;
		if (vehicle?.haOdoSensor && haConnected) {
			haStore.startPeriodicRefresh(vehicle.id, vehicle.haOdoSensor, vehicle.haFuelLevelSensor ?? undefined);
		} else {
			haStore.stopPeriodicRefresh();
		}
	});

	let ha = $derived(
		haReadings(
			$activeVehicleStore ? $haStore.cache.get($activeVehicleStore.id) : null,
			summary?.maxOdo ?? null,
			summary?.stats.fuelRemainingLiters ?? null,
			$activeVehicleStore?.tankSizeLiters
		)
	);

	async function load(vehicleId: string, year: number) {
		const request = ++latestRequest;
		try {
			const [grid, stats] = await Promise.all([
				getTripGridData(vehicleId, year),
				calculateTripStats(vehicleId, year)
			]);
			if (request !== latestRequest) return;
			summary = {
				year,
				count: grid.trips.length,
				stats,
				maxOdo: grid.trips.length > 0 ? Math.max(...grid.trips.map((t) => t.odometer)) : null,
				suggestedFillup: grid.legendSuggestedFillup
			};
			loadError = null;
		} catch (error) {
			if (request !== latestRequest) return;
			loadError = String(error);
		}
	}
</script>

<MobileBar
	title={$LL.mobile.home.title()}
	subtitle={$activeVehicleStore ? `${$activeVehicleStore.name} ${$activeVehicleStore.licensePlate}` : ''}
/>

<MobileVehicleGate>
	<div class="body" data-testid="mobile-home">
		{#if !$appModeStore.isReadOnly}
			{#if fuelioAvailable}
				<a class="big" href="/m/fuelio" data-testid="mobile-home-fuelio">
					{$LL.mobile.home.addFuelio()}
					<small>{$LL.mobile.home.addFuelioHint()}</small>
				</a>
			{/if}
			<a class="big alt" href="/m/nova" data-testid="mobile-home-manual">
				{$LL.mobile.home.addManual()}
				<small>{$LL.mobile.home.addManualHint()}</small>
			</a>
		{/if}
		<a class="big alt" href="/m/jazdy" data-testid="mobile-home-trips">
			{$LL.mobile.home.allTrips()}
			{#if summary}
				<small data-testid="mobile-home-trips-summary"
					>{$LL.mobile.home.allTripsHint({
						year: summary.year,
						count: summary.count,
						km: whole(summary.stats.totalKm, $locale)
					})}</small
				>
			{/if}
		</a>
		<a class="big alt" href="/m/miesta" data-testid="mobile-home-places">
			{$LL.mobile.places.homeButton()}
			<small>{$LL.mobile.places.homeHint()}</small>
		</a>
		{#if loadError}
			<p class="error">{$LL.mobile.home.loadError({ error: loadError })}</p>
		{/if}

		{#if summary}
			{@const st = summary.stats}
			{#if st.isOverLimit && st.marginPercent !== null}
				<div data-testid="mobile-home-compensation">
					<CompensationBanner marginPercent={st.marginPercent} bufferKm={st.bufferKm} />
				</div>
			{/if}
			<section class="stats" data-testid="mobile-home-stats">
				<h2>{$LL.mobile.home.statsTitle({ year: summary.year })}</h2>
				<div class="row">
					<span>{$LL.stats.totalDriven()}</span>
					<span data-testid="mobile-stat-km">{whole(st.totalKm, $locale)} km</span>
				</div>
				<div class="row">
					<span>{$LL.stats.fuel()}</span>
					<span data-testid="mobile-stat-fuel"
						>{fixed(st.totalFuelLiters, 1)} l / {fixed(st.totalFuelCostEur, 2)} EUR</span
					>
				</div>
				<div class="row">
					<span>{$LL.stats.remaining()}</span>
					<span class="end" data-testid="mobile-stat-remaining">
						{fixed(st.fuelRemainingLiters, 1)} l
						{#if ha.fuelRefill !== null}
							<small class="ha-fuel" data-testid="mobile-stat-ha-fuel"
								>{ha.fuelRefill >= 0
									? $LL.homeAssistant.fuelToRefill({ liters: fixed(ha.fuelRefill, 1) })
									: $LL.homeAssistant.fuelSurplus({ liters: fixed(Math.abs(ha.fuelRefill), 1) })}</small
							>
						{:else if $haStore.fuelError && $activeVehicleStore?.haFuelLevelSensor}
							<small class="error">{$LL.homeAssistant.fetchError()}</small>
						{/if}
					</span>
				</div>
				<div class="row">
					<span>{$LL.stats.consumption()}</span>
					<span data-testid="mobile-stat-consumption">{fixed(st.avgConsumptionRate, 2)} l/100 km</span>
				</div>
				{#if st.overallDeviationPercent !== null}
					<div class="row" class:warning={st.isOverLimit}>
						<span>{$LL.stats.deviation()}</span>
						<span data-testid="mobile-stat-deviation">{fixed(st.overallDeviationPercent, 1)} %</span>
					</div>
				{/if}
				{#if ha.odo !== null}
					<div class="row" class:warning={ha.odoWarning} data-testid="mobile-stat-ha-odo">
						<span>{$LL.homeAssistant.realOdo()}</span>
						<span class="end">
							{whole(ha.odo, $locale)} km
							{#if ha.odoDelta !== null}
								<small class="delta" class:warning={ha.odoWarning}
									>{whole(Math.abs(ha.odoDelta), $locale)}
									{ha.odoDelta < 0
										? $LL.homeAssistant.deltaToDrive()
										: $LL.homeAssistant.deltaMissingInLog()}</small
								>
							{/if}
						</span>
					</div>
				{:else if $haStore.odoError && $activeVehicleStore?.haOdoSensor}
					<div class="row">
						<span>{$LL.homeAssistant.realOdo()}</span>
						<span class="error">{$LL.homeAssistant.fetchError()}</span>
					</div>
				{/if}
				{#if summary.suggestedFillup}
					<p class="suggestion" data-testid="mobile-stat-suggested-fillup">
						{$LL.trips.legend.suggestedFillup({
							liters: summary.suggestedFillup.liters.toFixed(2),
							rate: summary.suggestedFillup.consumptionRate.toFixed(2)
						})}
					</p>
				{/if}
			</section>
		{/if}
		<a class="plain" href="/" data-testid="mobile-home-pc">{$LL.mobile.home.openPc()}</a>
	</div>
</MobileVehicleGate>

<style>
	.body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.875rem;
		max-width: 640px;
		margin: 0 auto;
	}

	.big {
		display: block;
		min-height: 44px;
		padding: 1.1rem 0.9rem;
		border-radius: 12px;
		border: 2px solid var(--accent-primary);
		background: var(--accent-primary);
		color: #fff;
		font-size: 1.1rem;
		font-weight: 600;
		text-decoration: none;
		box-sizing: border-box;
	}

	.big.alt {
		background: transparent;
		color: var(--accent-primary);
	}

	.big small {
		display: block;
		margin-top: 0.15rem;
		font-size: 0.875rem;
		font-weight: 400;
		opacity: 0.9;
	}

	.plain {
		display: inline-flex;
		align-items: center;
		min-height: 44px;
		color: var(--accent-primary);
		font-weight: 600;
		text-decoration: none;
		align-self: flex-start;
	}

	.stats {
		display: flex;
		flex-direction: column;
		padding: 0.75rem 0.9rem;
		border: 1px solid var(--border-default);
		border-radius: 12px;
		background: var(--bg-surface);
	}

	.stats h2 {
		margin: 0 0 0.4rem;
		font-size: 0.8rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--text-secondary);
	}

	.row {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 0.75rem;
		padding: 0.45rem 0;
		border-bottom: 1px solid var(--border-default);
		font-variant-numeric: tabular-nums;
	}

	.row:last-of-type {
		border-bottom: none;
	}

	.row .end {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		text-align: right;
	}

	.row small {
		font-size: 0.8rem;
		color: var(--text-secondary);
	}

	/* The same colors as the PC stats row: the refill text is always orange;
	   the ODO difference turns orange at the warning; a warning colors the
	   value, not the label. */
	.row small.ha-fuel {
		color: var(--accent-warning);
	}

	.row small.delta.warning {
		color: var(--accent-warning);
		font-weight: 600;
	}

	.row.warning > span:last-child {
		color: var(--accent-warning);
	}

	.suggestion {
		margin: 0.5rem 0 0;
		padding: 0.5rem 0.6rem;
		border-radius: 8px;
		border-left: 3px solid var(--accent-primary);
		background: var(--bg-surface-alt);
		font-size: 0.9rem;
	}

	.error {
		margin: 0;
		color: var(--accent-danger);
	}
</style>
