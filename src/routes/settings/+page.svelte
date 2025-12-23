<script lang="ts">
	import { vehiclesStore } from '$lib/stores/vehicles';
</script>

<div class="settings-page">
	<div class="header">
		<h1>Nastavenia</h1>
		<a href="/" class="back-link">← Späť na hlavnú stránku</a>
	</div>

	<div class="sections">
		<section class="settings-section">
			<h2>Vozidlá</h2>
			<div class="section-content">
				{#if $vehiclesStore.length > 0}
					<div class="vehicle-list">
						{#each $vehiclesStore as vehicle}
							<div class="vehicle-item">
								<div class="vehicle-info">
									<strong>{vehicle.name}</strong>
									<span class="license-plate">{vehicle.license_plate}</span>
									<span class="details">
										{vehicle.tank_size_liters}L | {vehicle.tp_consumption} L/100km
									</span>
									{#if vehicle.is_active}
										<span class="badge active">Aktívne</span>
									{/if}
								</div>
								<div class="vehicle-actions">
									<button class="button-small">Upraviť</button>
									<button class="button-small danger">Odstrániť</button>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<p class="placeholder">Žiadne vozidlá. Vytvorte prvé vozidlo.</p>
				{/if}
				<button class="button">+ Pridať vozidlo</button>
			</div>
		</section>

		<section class="settings-section">
			<h2>Nastavenia spoločnosti</h2>
			<div class="section-content">
				<p class="placeholder">
					Nastavenia spoločnosti (názov, IČO, účel vyrovnávacích jázd) budú implementované v
					ďalšej fáze.
				</p>
			</div>
		</section>
	</div>
</div>

<style>
	.settings-page {
		max-width: 800px;
		margin: 0 auto;
	}

	.header {
		margin-bottom: 2rem;
	}

	.header h1 {
		margin: 0 0 0.5rem 0;
		color: #2c3e50;
	}

	.back-link {
		color: #3498db;
		text-decoration: none;
		font-size: 0.875rem;
	}

	.back-link:hover {
		text-decoration: underline;
	}

	.sections {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.settings-section {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.settings-section h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.section-content {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.vehicle-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.vehicle-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border: 1px solid #e0e0e0;
		border-radius: 4px;
		background: #fafafa;
	}

	.vehicle-info {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.vehicle-info strong {
		font-size: 1rem;
		color: #2c3e50;
	}

	.license-plate {
		font-size: 0.875rem;
		color: #7f8c8d;
		font-weight: 500;
	}

	.details {
		font-size: 0.75rem;
		color: #95a5a6;
	}

	.badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
	}

	.badge.active {
		background-color: #d4edda;
		color: #155724;
	}

	.vehicle-actions {
		display: flex;
		gap: 0.5rem;
	}

	.placeholder {
		color: #7f8c8d;
		font-style: italic;
		margin: 0;
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: #3498db;
		color: white;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button:hover {
		background-color: #2980b9;
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: #ecf0f1;
		color: #2c3e50;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button-small:hover {
		background-color: #d5dbdb;
	}

	.button-small.danger {
		background-color: #fee;
		color: #c0392b;
	}

	.button-small.danger:hover {
		background-color: #fdd;
	}
</style>
