<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import { getVehicles, getActiveVehicle, setActiveVehicle } from '$lib/api';

	let { children } = $props();

	onMount(async () => {
		try {
			const [vehicles, activeVehicle] = await Promise.all([
				getVehicles(),
				getActiveVehicle()
			]);
			vehiclesStore.set(vehicles);
			activeVehicleStore.set(activeVehicle);
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
			} catch (error) {
				console.error('Failed to set active vehicle:', error);
			}
		}
	}
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="app">
	<header>
		<div class="header-content">
			<h1>Kniha Jázd</h1>
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

	.vehicle-selector {
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
