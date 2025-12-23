<script lang="ts">
	import { onMount } from 'svelte';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import VehicleModal from '$lib/components/VehicleModal.svelte';
	import * as api from '$lib/api';
	import type { Vehicle, Settings, BackupInfo } from '$lib/types';

	let showVehicleModal = false;
	let editingVehicle: Vehicle | null = null;

	// Settings state
	let settings: Settings | null = null;
	let companyName = '';
	let companyIco = '';
	let bufferTripPurpose = '';

	// Export state
	let selectedYear = new Date().getFullYear();

	// Backup state
	let backups: BackupInfo[] = [];
	let loadingBackups = false;
	let backupInProgress = false;
	let restoreConfirmation: BackupInfo | null = null;

	onMount(async () => {
		// Load settings
		const loadedSettings = await api.getSettings();
		if (loadedSettings) {
			settings = loadedSettings;
			companyName = loadedSettings.company_name;
			companyIco = loadedSettings.company_ico;
			bufferTripPurpose = loadedSettings.buffer_trip_purpose;
		}

		// Load backups
		await loadBackups();
	});

	function openAddVehicleModal() {
		editingVehicle = null;
		showVehicleModal = true;
	}

	function openEditVehicleModal(vehicle: Vehicle) {
		editingVehicle = vehicle;
		showVehicleModal = true;
	}

	function closeVehicleModal() {
		showVehicleModal = false;
		editingVehicle = null;
	}

	async function handleSaveVehicle(
		name: string,
		licensePlate: string,
		tankSize: number,
		tpConsumption: number,
		initialOdometer: number
	) {
		try {
			if (editingVehicle) {
				// Update existing vehicle
				const updatedVehicle: Vehicle = {
					...editingVehicle,
					name,
					license_plate: licensePlate,
					tank_size_liters: tankSize,
					tp_consumption: tpConsumption,
					initial_odometer: initialOdometer,
					updated_at: new Date().toISOString()
				};
				await api.updateVehicle(updatedVehicle);
			} else {
				// Create new vehicle
				await api.createVehicle(name, licensePlate, tankSize, tpConsumption, initialOdometer);
			}

			// Reload vehicles
			const vehicles = await api.getVehicles();
			vehiclesStore.set(vehicles);

			// Update activeVehicleStore if we edited the active vehicle
			if (editingVehicle && $activeVehicleStore?.id === editingVehicle.id) {
				const updatedActive = vehicles.find((v) => v.id === editingVehicle.id);
				if (updatedActive) {
					activeVehicleStore.set(updatedActive);
				}
			}

			closeVehicleModal();
		} catch (error) {
			console.error('Failed to save vehicle:', error);
			alert('Nepodarilo sa uložiť vozidlo: ' + error);
		}
	}

	async function handleDeleteVehicle(vehicle: Vehicle) {
		if (!confirm(`Naozaj chcete odstrániť vozidlo "${vehicle.name}"?`)) {
			return;
		}

		try {
			await api.deleteVehicle(vehicle.id);

			// Reload vehicles
			const vehicles = await api.getVehicles();
			vehiclesStore.set(vehicles);
		} catch (error) {
			console.error('Failed to delete vehicle:', error);
			alert('Nepodarilo sa odstrániť vozidlo: ' + error);
		}
	}

	async function handleSetActiveVehicle(vehicle: Vehicle) {
		try {
			await api.setActiveVehicle(vehicle.id);

			// Reload vehicles
			const vehicles = await api.getVehicles();
			vehiclesStore.set(vehicles);
		} catch (error) {
			console.error('Failed to set active vehicle:', error);
			alert('Nepodarilo sa nastaviť aktívne vozidlo: ' + error);
		}
	}

	async function handleSaveSettings() {
		try {
			const savedSettings = await api.saveSettings(companyName, companyIco, bufferTripPurpose);
			settings = savedSettings;
			alert('Nastavenia boli úspešne uložené');
		} catch (error) {
			console.error('Failed to save settings:', error);
			alert('Nepodarilo sa uložiť nastavenia: ' + error);
		}
	}

	function handleExportPDF() {
		alert(
			`Export do PDF pre rok ${selectedYear} bude implementovaný v ďalšej fáze.\n\nTáto funkcia vyexportuje všetky jazdy za zvolený rok do PDF súboru v súlade so slovenskou legislatívou.`
		);
	}

	// Backup functions
	async function loadBackups() {
		loadingBackups = true;
		try {
			backups = await api.listBackups();
		} catch (error) {
			console.error('Failed to load backups:', error);
		} finally {
			loadingBackups = false;
		}
	}

	async function handleCreateBackup() {
		backupInProgress = true;
		try {
			const backup = await api.createBackup();
			await loadBackups();
			alert(`Záloha vytvorená: ${backup.filename}`);
		} catch (error) {
			console.error('Failed to create backup:', error);
			alert('Nepodarilo sa vytvoriť zálohu: ' + error);
		} finally {
			backupInProgress = false;
		}
	}

	async function handleRestoreClick(backup: BackupInfo) {
		try {
			// Get full backup info with counts
			restoreConfirmation = await api.getBackupInfo(backup.filename);
		} catch (error) {
			console.error('Failed to get backup info:', error);
			alert('Nepodarilo sa načítať informácie o zálohe: ' + error);
		}
	}

	async function handleConfirmRestore() {
		if (!restoreConfirmation) return;

		try {
			await api.restoreBackup(restoreConfirmation.filename);
			restoreConfirmation = null;
			alert('Záloha bola úspešne obnovená. Aplikácia sa reštartuje.');
			// Reload the app to pick up restored data
			window.location.reload();
		} catch (error) {
			console.error('Failed to restore backup:', error);
			alert('Nepodarilo sa obnoviť zálohu: ' + error);
		}
	}

	function cancelRestore() {
		restoreConfirmation = null;
	}

	function formatBackupDate(isoDate: string): string {
		try {
			const date = new Date(isoDate);
			return date.toLocaleString('sk-SK', {
				day: '2-digit',
				month: '2-digit',
				year: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return isoDate;
		}
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<div class="settings-page">
	<div class="header">
		<h1>Nastavenia</h1>
	</div>

	<div class="sections">
		<!-- Vehicles Section -->
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
									<button class="button-small" on:click={() => openEditVehicleModal(vehicle)}>
										Upraviť
									</button>
									{#if !vehicle.is_active}
										<button
											class="button-small primary"
											on:click={() => handleSetActiveVehicle(vehicle)}
										>
											Nastaviť ako aktívne
										</button>
									{/if}
									<button class="button-small danger" on:click={() => handleDeleteVehicle(vehicle)}>
										Odstrániť
									</button>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<p class="placeholder">Žiadne vozidlá. Vytvorte prvé vozidlo.</p>
				{/if}
				<button class="button" on:click={openAddVehicleModal}>+ Pridať vozidlo</button>
			</div>
		</section>

		<!-- Company Settings Section -->
		<section class="settings-section">
			<h2>Nastavenia spoločnosti</h2>
			<div class="section-content">
				<div class="form-group">
					<label for="company-name">Názov spoločnosti</label>
					<input
						type="text"
						id="company-name"
						bind:value={companyName}
						placeholder="napr. Moja firma s.r.o."
					/>
				</div>

				<div class="form-group">
					<label for="company-ico">IČO</label>
					<input type="text" id="company-ico" bind:value={companyIco} placeholder="napr. 12345678" />
				</div>

				<div class="form-group">
					<label for="filler-purpose">Účel kompenzačnej jazdy</label>
					<input
						type="text"
						id="filler-purpose"
						bind:value={bufferTripPurpose}
						placeholder="napr. testovanie"
					/>
					<small class="hint">
						Tento účel sa použije pri generovaní kompenzačných jázd na dodržanie 20% limitu.
					</small>
				</div>

				<button class="button" on:click={handleSaveSettings}>Uložiť nastavenia</button>
			</div>
		</section>

		<!-- Export Section -->
		<section class="settings-section">
			<h2>Export</h2>
			<div class="section-content">
				<div class="form-group">
					<label for="export-year">Rok pre export</label>
					<select id="export-year" bind:value={selectedYear}>
						{#each Array.from({ length: 10 }, (_, i) => new Date().getFullYear() - i) as year}
							<option value={year}>{year}</option>
						{/each}
					</select>
				</div>

				<button class="button" on:click={handleExportPDF}>Exportovať PDF</button>

				<p class="hint">
					Export vytvorí PDF súbor so všetkými jazdami za zvolený rok v súlade so slovenskou
					legislatívou.
				</p>
			</div>
		</section>

		<!-- Backup Section -->
		<section class="settings-section">
			<h2>Záloha databázy</h2>
			<div class="section-content">
				<button class="button" on:click={handleCreateBackup} disabled={backupInProgress}>
					{backupInProgress ? 'Vytváram zálohu...' : 'Zálohovať'}
				</button>

				<div class="backup-list">
					<h3>Dostupné zálohy</h3>
					{#if loadingBackups}
						<p class="placeholder">Načítavam...</p>
					{:else if backups.length === 0}
						<p class="placeholder">Žiadne zálohy. Vytvorte prvú zálohu.</p>
					{:else}
						{#each backups as backup}
							<div class="backup-item">
								<div class="backup-info">
									<span class="backup-date">{formatBackupDate(backup.created_at)}</span>
									<span class="backup-size">{formatFileSize(backup.size_bytes)}</span>
								</div>
								<button class="button-small" on:click={() => handleRestoreClick(backup)}>
									Obnoviť
								</button>
							</div>
						{/each}
					{/if}
				</div>
			</div>
		</section>
	</div>
</div>

{#if showVehicleModal}
	<VehicleModal vehicle={editingVehicle} onSave={handleSaveVehicle} onClose={closeVehicleModal} />
{/if}

{#if restoreConfirmation}
	<div class="modal-overlay" on:click={cancelRestore} role="button" tabindex="0" on:keydown={(e) => e.key === 'Escape' && cancelRestore()}>
		<div class="modal" on:click|stopPropagation role="dialog" aria-modal="true">
			<h2>Potvrdiť obnovenie</h2>
			<div class="modal-content">
				<p><strong>Dátum zálohy:</strong> {formatBackupDate(restoreConfirmation.created_at)}</p>
				<p><strong>Veľkosť:</strong> {formatFileSize(restoreConfirmation.size_bytes)}</p>
				<p><strong>Obsahuje:</strong> {restoreConfirmation.vehicle_count} vozidiel, {restoreConfirmation.trip_count} jázd</p>
				<p class="warning-text">
					Aktuálne dáta budú prepísané! Pred obnovením sa automaticky vytvorí záloha aktuálneho stavu.
				</p>
			</div>
			<div class="modal-actions">
				<button class="button-small" on:click={cancelRestore}>Zrušiť</button>
				<button class="button-small danger" on:click={handleConfirmRestore}>Obnoviť zálohu</button>
			</div>
		</div>
	</div>
{/if}

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
		flex-wrap: wrap;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.form-group label {
		font-weight: 500;
		color: #2c3e50;
		font-size: 0.875rem;
	}

	.form-group input,
	.form-group select {
		padding: 0.75rem;
		border: 1px solid #d5dbdb;
		border-radius: 4px;
		font-size: 1rem;
		font-family: inherit;
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: #3498db;
		box-shadow: 0 0 0 3px rgba(52, 152, 219, 0.1);
	}

	.hint {
		font-size: 0.75rem;
		color: #7f8c8d;
		font-style: italic;
		margin: 0;
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
		font-size: 1rem;
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

	.button-small.primary {
		background-color: #d4edda;
		color: #155724;
	}

	.button-small.primary:hover {
		background-color: #c3e6cb;
	}

	.button-small.danger {
		background-color: #fee;
		color: #c0392b;
	}

	.button-small.danger:hover {
		background-color: #fdd;
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.backup-list {
		margin-top: 1rem;
	}

	.backup-list h3 {
		font-size: 1rem;
		color: #2c3e50;
		margin: 0 0 0.75rem 0;
	}

	.backup-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		border: 1px solid #e0e0e0;
		border-radius: 4px;
		background: #fafafa;
		margin-bottom: 0.5rem;
	}

	.backup-info {
		display: flex;
		gap: 1rem;
	}

	.backup-date {
		font-weight: 500;
		color: #2c3e50;
	}

	.backup-size {
		color: #7f8c8d;
		font-size: 0.875rem;
	}

	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		max-width: 400px;
		width: 90%;
	}

	.modal h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: #2c3e50;
	}

	.modal-content {
		margin-bottom: 1.5rem;
	}

	.modal-content p {
		margin: 0.5rem 0;
	}

	.warning-text {
		color: #c0392b;
		font-weight: 500;
		margin-top: 1rem !important;
		padding: 0.75rem;
		background: #fee;
		border-radius: 4px;
	}

	.modal-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}
</style>
