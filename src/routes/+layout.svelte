<script lang="ts">
	import '$lib/theme.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore, resetToCurrentYear } from '$lib/stores/year';
	import { localeStore } from '$lib/stores/locale';
	import { themeStore } from '$lib/stores/theme';
	import { appModeStore } from '$lib/stores/appMode';
	import { getVehicles, getActiveVehicle, setActiveVehicle, getYearsWithTrips } from '$lib/api';
	import Toast from '$lib/components/Toast.svelte';
	import GlobalConfirm from '$lib/components/GlobalConfirm.svelte';
	import ReceiptIndicator from '$lib/components/ReceiptIndicator.svelte';
	import LL from '$lib/i18n/i18n-svelte';

	let { children } = $props();

	let availableYears = $state<number[]>([]);
	let i18nReady = $state(false);

	async function loadYears() {
		if (!$activeVehicleStore) {
			availableYears = [];
			return;
		}
		try {
			const yearsWithData = await getYearsWithTrips($activeVehicleStore.id);
			const currentYear = new Date().getFullYear();
			// Combine current year with years that have data, deduplicate, sort descending
			const allYears = new Set([currentYear, ...yearsWithData]);
			availableYears = [...allYears].sort((a, b) => b - a);

			// If current selection has no data, switch to most recent year with data
			if (yearsWithData.length > 0 && !yearsWithData.includes($selectedYearStore)) {
				const mostRecentWithData = Math.max(...yearsWithData);
				selectedYearStore.set(mostRecentWithData);
			}
		} catch (error) {
			console.error('Failed to load years:', error);
			availableYears = [new Date().getFullYear()];
		}
	}

	onMount(async () => {
		// Initialize i18n first
		localeStore.init();
		i18nReady = true;

		// Initialize theme (after locale but before async vehicle loading)
		await themeStore.init();

		// Initialize app mode (check for read-only)
		await appModeStore.refresh();

		try {
			// PRESERVE parallel loading for performance
			const [vehicles, persistedActiveVehicle] = await Promise.all([
				getVehicles(),
				getActiveVehicle()
			]);
			vehiclesStore.set(vehicles);

			let activeVehicle = persistedActiveVehicle;

			// Auto-select first vehicle if none set but vehicles exist
			if (!activeVehicle && vehicles.length > 0) {
				activeVehicle = vehicles[0];
				await setActiveVehicle(activeVehicle.id);
			}

			// Handle deleted vehicle: if persisted ID not in list, select first
			if (activeVehicle && !vehicles.find(v => v.id === activeVehicle!.id)) {
				if (vehicles.length > 0) {
					activeVehicle = vehicles[0];
					await setActiveVehicle(activeVehicle.id);
				} else {
					activeVehicle = null;
				}
			}

			activeVehicleStore.set(activeVehicle);

			// Reset year to current after auto-select to avoid stale year
			if (activeVehicle) {
				resetToCurrentYear();
			}

			await loadYears();
		} catch (error) {
			console.error('Failed to load initial data:', error);
		}
	});

	async function handleVehicleChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		const vehicleId = select.value;
		if (vehicleId) {
			try {
				await setActiveVehicle(vehicleId);
				const activeVehicle = $vehiclesStore.find((v) => v.id === vehicleId) || null;
				activeVehicleStore.set(activeVehicle);
				resetToCurrentYear();
				await loadYears();
			} catch (error) {
				console.error('Failed to set active vehicle:', error);
			}
		}
	}

	function handleYearChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		selectedYearStore.set(parseInt(select.value, 10));
	}
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{#if i18nReady}
<div class="app">
	<header>
		<div class="header-content">
			<div class="header-left">
				<h1>{$LL.app.title()}</h1>
				<nav class="main-nav">
					<a href="/" class="nav-link" class:active={$page.url.pathname === '/'}>{$LL.app.nav.logbook()}</a>
					<a href="/doklady" class="nav-link" class:active={$page.url.pathname === '/doklady'}>{$LL.app.nav.receipts()}<ReceiptIndicator /></a>
					<a href="/settings" class="nav-link" class:active={$page.url.pathname === '/settings'}>
						{$LL.app.nav.settings()}
					</a>
				</nav>
			</div>
			<div class="header-right">
				<div class="vehicle-selector">
					<label for="vehicle-select">{$LL.app.vehicleLabel()}</label>
					<select
						id="vehicle-select"
						value={$activeVehicleStore?.id || ''}
						onchange={handleVehicleChange}
					>
						{#if $vehiclesStore.length === 0}
							<option value="">{$LL.app.noVehicles()}</option>
						{/if}
						{#each $vehiclesStore as vehicle}
							<option value={vehicle.id}>
								{vehicle.name} ({vehicle.licensePlate})
							</option>
						{/each}
					</select>
				</div>
				{#if $activeVehicleStore}
					<div class="year-selector">
						<label for="year-select">{$LL.app.yearLabel()}</label>
						<select
							id="year-select"
							value={$selectedYearStore}
							onchange={handleYearChange}
						>
							{#each availableYears as year}
								<option value={year}>{year}</option>
							{/each}
						</select>
					</div>
				{/if}
			</div>
		</div>
	</header>

	{#if $appModeStore.isReadOnly}
		<div class="read-only-banner">
			<span class="banner-icon">⚠️</span>
			<span class="banner-text">{$LL.settings.readOnlyBanner()}</span>
		</div>
	{/if}

	<main>
		{@render children()}
	</main>
</div>
{/if}

<Toast />
<GlobalConfirm />

<style>
	:global(body) {
		margin: 0;
		padding: 0;
		font-family: var(--font-sans);
		background-color: var(--bg-body);
		color: var(--text-primary);
	}

	:global(input), :global(select), :global(textarea) {
		color: var(--text-primary);
		background-color: var(--input-bg);
	}

	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	header {
		background-color: var(--bg-header);
		color: var(--text-on-header);
		padding: 1rem 2rem;
		box-shadow: 0 2px 4px var(--shadow-default);
	}

	.read-only-banner {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		background-color: var(--warning-bg, #fef3c7);
		border-bottom: 1px solid var(--warning-border, #f59e0b);
		color: var(--warning-text, #92400e);
	}

	.banner-icon {
		font-size: 1.25rem;
	}

	.banner-text {
		flex: 1;
		font-weight: 500;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
		max-width: 1200px;
		margin: 0 auto;
	}

	h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 2rem;
	}

	.main-nav {
		display: flex;
		gap: 0.5rem;
	}

	.nav-link {
		color: var(--text-on-header-muted);
		text-decoration: none;
		padding: 0.5rem 1rem;
		border-radius: 4px;
		font-weight: 500;
		transition: all 0.2s;
	}

	.nav-link:hover {
		color: var(--text-on-header);
		background: rgba(255, 255, 255, 0.1);
	}

	.nav-link.active {
		color: var(--text-on-header);
		background: rgba(255, 255, 255, 0.2);
	}

	.header-right {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.vehicle-selector {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.year-selector {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	label {
		font-weight: 500;
	}

	select {
		padding: 0.5rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		background-color: var(--input-bg);
		color: var(--text-primary);
		font-size: 1rem;
		cursor: pointer;
		min-width: 200px;
	}

	select:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	main {
		flex: 1;
		overflow: auto;
		padding: 1rem 2rem;
		width: 100%;
		box-sizing: border-box;
	}
</style>
