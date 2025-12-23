<script lang="ts">
	import { activeVehicleStore } from '$lib/stores/vehicles';
</script>

<div class="main-page">
	{#if $activeVehicleStore}
		<div class="vehicle-info">
			<h2>Aktívne vozidlo</h2>
			<div class="info-grid">
				<div class="info-item">
					<span class="label">Názov:</span>
					<span class="value">{$activeVehicleStore.name}</span>
				</div>
				<div class="info-item">
					<span class="label">ŠPZ:</span>
					<span class="value">{$activeVehicleStore.license_plate}</span>
				</div>
				<div class="info-item">
					<span class="label">Objem nádrže:</span>
					<span class="value">{$activeVehicleStore.tank_size_liters} L</span>
				</div>
				<div class="info-item">
					<span class="label">Spotreba (TP):</span>
					<span class="value">{$activeVehicleStore.tp_consumption} L/100km</span>
				</div>
			</div>
		</div>

		<div class="trip-section">
			<h2>Jazdy</h2>
			<p class="placeholder">Tabuľka jázd bude implementovaná v ďalšej fáze.</p>
		</div>

		<div class="actions">
			<a href="/settings" class="button">Nastavenia</a>
		</div>
	{:else}
		<div class="no-vehicle">
			<h2>Žiadne vozidlo</h2>
			<p>Prosím, vyberte vozidlo z hlavného menu alebo ho vytvorte v nastaveniach.</p>
			<a href="/settings" class="button">Prejsť do nastavení</a>
		</div>
	{/if}
</div>

<style>
	.main-page {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.vehicle-info {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.vehicle-info h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}

	.info-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.label {
		font-size: 0.875rem;
		color: #7f8c8d;
		font-weight: 500;
	}

	.value {
		font-size: 1rem;
		color: #2c3e50;
		font-weight: 600;
	}

	.trip-section {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.trip-section h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.placeholder {
		color: #7f8c8d;
		font-style: italic;
	}

	.actions {
		display: flex;
		gap: 1rem;
	}

	.no-vehicle {
		background: white;
		padding: 2rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		text-align: center;
	}

	.no-vehicle h2 {
		margin: 0 0 1rem 0;
		color: #2c3e50;
	}

	.no-vehicle p {
		color: #7f8c8d;
		margin-bottom: 1.5rem;
	}

	.button {
		display: inline-block;
		padding: 0.75rem 1.5rem;
		background-color: #3498db;
		color: white;
		text-decoration: none;
		border-radius: 4px;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.button:hover {
		background-color: #2980b9;
	}
</style>
