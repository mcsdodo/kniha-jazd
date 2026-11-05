<script lang="ts">
	// Task 93: the phone start page. Big buttons to the add flows and the
	// trip list. The counts come from the backend (ADR-008).
	import { onMount } from 'svelte';
	import { calculateTripStats, getTripsForYear, isFuelioAvailable } from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { appModeStore } from '$lib/stores/appMode';
	import LL, { locale } from '$lib/i18n/i18n-svelte';
	import MobileBar from '$lib/components/mobile/MobileBar.svelte';
	import MobileVehicleGate from '$lib/components/mobile/MobileVehicleGate.svelte';
	import { whole } from '$lib/components/mobile/MobileFormat';

	let fuelioAvailable = $state(false);
	let summary = $state<{ year: number; count: number; km: number } | null>(null);
	let loadError = $state<string | null>(null);

	// The startup writes the vehicle and the year one after the other, so two
	// loads can run at the same time. Only the newest one may write.
	let latestRequest = 0;

	onMount(() => {
		isFuelioAvailable()
			.then((v) => (fuelioAvailable = v))
			.catch((error) => console.error('Failed to check Fuelio:', error));
	});

	$effect(() => {
		const vehicle = $activeVehicleStore;
		const year = $selectedYearStore;
		if (vehicle) load(vehicle.id, year);
	});

	async function load(vehicleId: string, year: number) {
		const request = ++latestRequest;
		try {
			const [trips, stats] = await Promise.all([
				getTripsForYear(vehicleId, year),
				calculateTripStats(vehicleId, year)
			]);
			if (request !== latestRequest) return;
			summary = { year, count: trips.length, km: stats.totalKm };
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
						km: whole(summary.km, $locale)
					})}</small
				>
			{/if}
		</a>
		{#if loadError}
			<p class="error">{$LL.mobile.home.loadError({ error: loadError })}</p>
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

	.error {
		margin: 0;
		color: var(--accent-danger);
	}
</style>
