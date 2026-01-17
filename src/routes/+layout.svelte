<script lang="ts">
	import '$lib/theme.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore, resetToCurrentYear } from '$lib/stores/year';
	import { localeStore } from '$lib/stores/locale';
	import { themeStore } from '$lib/stores/theme';
	import { updateStore } from '$lib/stores/update';
	import { appModeStore } from '$lib/stores/appMode';
	import { getAutoCheckUpdates } from '$lib/api';
	import { getVehicles, getActiveVehicle, setActiveVehicle, getYearsWithTrips, getOptimalWindowSize, type WindowSize } from '$lib/api';
	import Toast from '$lib/components/Toast.svelte';
	import GlobalConfirm from '$lib/components/GlobalConfirm.svelte';
	import ReceiptIndicator from '$lib/components/ReceiptIndicator.svelte';
	import UpdateModal from '$lib/components/UpdateModal.svelte';
	import LL from '$lib/i18n/i18n-svelte';
	import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

	let { children } = $props();

	// Window size tracking (no defaults - loaded from backend)
	let optimalSize = $state<WindowSize | null>(null);
	let isOptimalSize = $state(true);

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

	async function checkWindowSize() {
		if (!optimalSize) return;
		const win = getCurrentWindow();
		const size = await win.innerSize();
		// Allow small tolerance (±5px) for OS quirks
		isOptimalSize =
			Math.abs(size.width - optimalSize.width) <= 5 &&
			Math.abs(size.height - optimalSize.height) <= 5;
	}

	async function resetWindowSize() {
		if (!optimalSize) return;
		const win = getCurrentWindow();
		await win.setSize(new LogicalSize(optimalSize.width, optimalSize.height));
		await win.center();
		isOptimalSize = true;
	}

	onMount(async () => {
		// Initialize i18n first
		localeStore.init();
		i18nReady = true;

		// Initialize theme (after locale but before async vehicle loading)
		await themeStore.init();

		// Initialize app mode (check for read-only)
		await appModeStore.refresh();

		// Always check for updates in background (for dot indicator)
		// If auto-check disabled, check but don't show modal (use check() which respects dismissed)
		getAutoCheckUpdates().then((enabled) => {
			if (enabled) {
				// Auto-check enabled: show modal if update available (respects previously dismissed)
				updateStore.check().catch((err) => {
					console.error('Update check failed:', err);
				});
			} else {
				// Auto-check disabled: check silently (mark as dismissed so no modal)
				updateStore.checkSilent().catch((err) => {
					console.error('Update check failed:', err);
				});
			}
		}).catch((err) => {
			console.error('Failed to get auto-check setting:', err);
		});

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

			// Load optimal window size and start tracking
			optimalSize = await getOptimalWindowSize();
			await checkWindowSize();
			const win = getCurrentWindow();
			await win.onResized(checkWindowSize);
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
						{#if $updateStore.available && $updateStore.dismissed}
							<span class="update-dot" aria-label="Update available"></span>
						{/if}
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
				{#if optimalSize && !isOptimalSize}
					<button
						class="resize-btn"
						onclick={resetWindowSize}
						title={$LL.app.resetWindowSize()}
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<rect x="3" y="3" width="18" height="18" rx="2"/>
							<path d="M9 3v18M3 9h18"/>
						</svg>
					</button>
				{/if}
			</div>
		</div>
	</header>

	{#if $appModeStore.isReadOnly}
		<div class="read-only-banner">
			<span class="banner-icon">⚠️</span>
			<span class="banner-text">{$LL.settings.readOnlyBanner()}</span>
			<button class="banner-button" onclick={() => updateStore.checkManual()}>
				{$LL.settings.readOnlyCheckUpdates()}
			</button>
		</div>
	{/if}

	<main>
		{@render children()}
	</main>
</div>
{/if}

<Toast />
<GlobalConfirm />

{#if $updateStore.available && !$updateStore.dismissed}
	<UpdateModal
		version={$updateStore.version || ''}
		releaseNotes={$updateStore.releaseNotes}
		downloading={$updateStore.downloading}
		progress={$updateStore.progress}
		onUpdate={() => updateStore.install()}
		onLater={() => updateStore.dismiss()}
	/>
{/if}

<style>
	:global(body) {
		margin: 0;
		padding: 0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu,
			Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
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

	.banner-button {
		padding: 0.5rem 1rem;
		background-color: var(--warning-button-bg, #f59e0b);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-weight: 500;
	}

	.banner-button:hover {
		opacity: 0.9;
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

	.update-dot {
		display: inline-block;
		width: 8px;
		height: 8px;
		background-color: var(--accent-primary);
		border-radius: 50%;
		margin-left: 6px;
		vertical-align: middle;
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

	.resize-btn {
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 4px;
		padding: 0.375rem;
		cursor: pointer;
		color: var(--text-on-header-muted);
		display: flex;
		align-items: center;
		transition: all 0.2s;
	}

	.resize-btn:hover {
		background: rgba(255, 255, 255, 0.2);
		color: var(--text-on-header);
	}

	main {
		flex: 1;
		overflow: auto;
		padding: 1rem 2rem;
		width: 100%;
		box-sizing: border-box;
	}
</style>
