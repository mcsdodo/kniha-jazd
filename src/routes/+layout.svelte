<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore, resetToCurrentYear } from '$lib/stores/year';
	import { getVehicles, getActiveVehicle, setActiveVehicle, getYearsWithTrips } from '$lib/api';

	let { children } = $props();

	let availableYears = $state<number[]>([]);

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
		} catch (error) {
			console.error('Failed to load years:', error);
			availableYears = [new Date().getFullYear()];
		}
	}

	onMount(async () => {
		try {
			const [vehicles, activeVehicle] = await Promise.all([
				getVehicles(),
				getActiveVehicle()
			]);
			vehiclesStore.set(vehicles);
			activeVehicleStore.set(activeVehicle);
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

<div class="app">
	<header>
		<div class="header-content">
			<div class="header-left">
				<h1>Kniha Jázd</h1>
				<nav class="main-nav">
					<a href="/" class="nav-link" class:active={$page.url.pathname === '/'}>Kniha jázd</a>
					<a href="/settings" class="nav-link" class:active={$page.url.pathname === '/settings'}>Nastavenia</a>
				</nav>
			</div>
			<div class="header-right">
				<div class="vehicle-selector">
					<label for="vehicle-select">Vozidlo:</label>
					<select
						id="vehicle-select"
						value={$activeVehicleStore?.id || ''}
						onchange={handleVehicleChange}
					>
						<option value="">-- Vyberte vozidlo --</option>
						{#each $vehiclesStore as vehicle}
							<option value={vehicle.id}>
								{vehicle.name} ({vehicle.license_plate})
							</option>
						{/each}
					</select>
				</div>
				{#if $activeVehicleStore}
					<div class="year-selector">
						<label for="year-select">Rok:</label>
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

	<main>
		{@render children()}
	</main>
</div>

<style>
	:global(body) {
		margin: 0;
		padding: 0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu,
			Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
		background-color: #f5f5f5;
	}

	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}

	header {
		background-color: #2c3e50;
		color: white;
		padding: 1rem 2rem;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
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
		color: rgba(255, 255, 255, 0.7);
		text-decoration: none;
		padding: 0.5rem 1rem;
		border-radius: 4px;
		font-weight: 500;
		transition: all 0.2s;
	}

	.nav-link:hover {
		color: white;
		background: rgba(255, 255, 255, 0.1);
	}

	.nav-link.active {
		color: white;
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
		border: 1px solid #ddd;
		border-radius: 4px;
		background-color: white;
		font-size: 1rem;
		cursor: pointer;
		min-width: 200px;
	}

	select:focus {
		outline: none;
		border-color: #3498db;
		box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.1);
	}

	main {
		flex: 1;
		overflow: auto;
		padding: 1rem 2rem;
		width: 100%;
		box-sizing: border-box;
	}
</style>
