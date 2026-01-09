<script lang="ts">
	import { onMount } from 'svelte';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import VehicleModal from '$lib/components/VehicleModal.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Vehicle, Settings, BackupInfo } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { localeStore } from '$lib/stores/locale';
	import type { Locales } from '$lib/i18n/i18n-types';

	let showVehicleModal = false;
	let editingVehicle: Vehicle | null = null;

	// Settings state
	let settings: Settings | null = null;
	let companyName = '';
	let companyIco = '';
	let bufferTripPurpose = '';

	// Language state
	let selectedLocale: Locales = 'sk';

	function handleLocaleChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		const newLocale = select.value as Locales;
		localeStore.change(newLocale);
		selectedLocale = newLocale;
	}


	// Backup state
	let backups: BackupInfo[] = [];
	let loadingBackups = false;
	let backupInProgress = false;
	let restoreConfirmation: BackupInfo | null = null;
	let deleteConfirmation: BackupInfo | null = null;

	// Vehicle delete confirmation
	let vehicleToDelete: Vehicle | null = null;

	// Track vehicles with trips (for blocking type change)
	let vehiclesWithTrips: Set<string> = new Set();

	async function checkVehiclesWithTrips() {
		const newSet = new Set<string>();
		for (const vehicle of $vehiclesStore) {
			const years = await api.getYearsWithTrips(vehicle.id);
			if (years.length > 0) {
				newSet.add(vehicle.id);
			}
		}
		vehiclesWithTrips = newSet;
	}

	onMount(() => {
		// Initialize selected locale from store
		const unsubscribe = localeStore.subscribe((locale) => {
			selectedLocale = locale;
		});

		// Load settings and backups (async operations)
		(async () => {
			const loadedSettings = await api.getSettings();
			if (loadedSettings) {
				settings = loadedSettings;
				companyName = loadedSettings.company_name;
				companyIco = loadedSettings.company_ico;
				bufferTripPurpose = loadedSettings.buffer_trip_purpose;
			}

			await loadBackups();
			await checkVehiclesWithTrips();
		})();

		return () => unsubscribe();
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

	async function handleSaveVehicle(data: {
		name: string;
		licensePlate: string;
		initialOdometer: number;
		vehicleType: import('$lib/types').VehicleType;
		tankSizeLiters: number | null;
		tpConsumption: number | null;
		batteryCapacityKwh: number | null;
		baselineConsumptionKwh: number | null;
		initialBatteryPercent: number | null;
		vin: string | null;
		driverName: string | null;
	}) {
		try {
			if (editingVehicle) {
				// Update existing vehicle
				const updatedVehicle: Vehicle = {
					...editingVehicle,
					name: data.name,
					license_plate: data.licensePlate,
					vehicle_type: data.vehicleType,
					tank_size_liters: data.tankSizeLiters,
					tp_consumption: data.tpConsumption,
					battery_capacity_kwh: data.batteryCapacityKwh,
					baseline_consumption_kwh: data.baselineConsumptionKwh,
					initial_battery_percent: data.initialBatteryPercent,
					initial_odometer: data.initialOdometer,
					vin: data.vin,
					driver_name: data.driverName,
					updated_at: new Date().toISOString()
				};
				await api.updateVehicle(updatedVehicle);
			} else {
				// Create new vehicle
				await api.createVehicle(
					data.name,
					data.licensePlate,
					data.initialOdometer,
					data.vehicleType,
					data.tankSizeLiters,
					data.tpConsumption,
					data.batteryCapacityKwh,
					data.baselineConsumptionKwh,
					data.initialBatteryPercent,
					data.vin,
					data.driverName
				);
			}

			// Reload vehicles
			const vehicles = await api.getVehicles();
			vehiclesStore.set(vehicles);

			// Update activeVehicleStore if we edited the active vehicle
			if (editingVehicle && $activeVehicleStore?.id === editingVehicle.id) {
				const editedId = editingVehicle.id;
				const updatedActive = vehicles.find((v) => v.id === editedId);
				if (updatedActive) {
					activeVehicleStore.set(updatedActive);
				}
			}

			closeVehicleModal();
			toast.success($LL.toast.vehicleSaved());
		} catch (error) {
			console.error('Failed to save vehicle:', error);
			toast.error($LL.toast.errorSaveVehicle({ error: String(error) }));
		}
	}

	function handleDeleteVehicleClick(vehicle: Vehicle) {
		vehicleToDelete = vehicle;
	}

	async function handleConfirmDeleteVehicle() {
		if (!vehicleToDelete) return;

		try {
			await api.deleteVehicle(vehicleToDelete.id);
			vehicleToDelete = null;

			// Reload vehicles
			const vehicles = await api.getVehicles();
			vehiclesStore.set(vehicles);
			toast.success($LL.toast.vehicleDeleted());
		} catch (error) {
			console.error('Failed to delete vehicle:', error);
			toast.error($LL.toast.errorDeleteVehicle({ error: String(error) }));
		}
	}

	function cancelDeleteVehicle() {
		vehicleToDelete = null;
	}

	async function handleSetActiveVehicle(vehicle: Vehicle) {
		try {
			await api.setActiveVehicle(vehicle.id);

			// Reload vehicles
			const vehicles = await api.getVehicles();
			vehiclesStore.set(vehicles);
		} catch (error) {
			console.error('Failed to set active vehicle:', error);
			toast.error($LL.toast.errorSetActiveVehicle({ error: String(error) }));
		}
	}

	async function handleSaveSettings() {
		try {
			const savedSettings = await api.saveSettings(companyName, companyIco, bufferTripPurpose);
			settings = savedSettings;
			toast.success($LL.toast.settingsSaved());
		} catch (error) {
			console.error('Failed to save settings:', error);
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
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
			await api.createBackup();
			await loadBackups();
			toast.success($LL.toast.backupCreated());
		} catch (error) {
			console.error('Failed to create backup:', error);
			toast.error($LL.toast.errorCreateBackup({ error: String(error) }));
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
			toast.error($LL.toast.errorGetBackupInfo({ error: String(error) }));
		}
	}

	async function handleConfirmRestore() {
		if (!restoreConfirmation) return;

		try {
			await api.restoreBackup(restoreConfirmation.filename);
			restoreConfirmation = null;
			toast.success($LL.toast.backupRestored());
			// Reload the app to pick up restored data
			setTimeout(() => window.location.reload(), 1500);
		} catch (error) {
			console.error('Failed to restore backup:', error);
			toast.error($LL.toast.errorRestoreBackup({ error: String(error) }));
		}
	}

	function cancelRestore() {
		restoreConfirmation = null;
	}

	async function handleDeleteClick(backup: BackupInfo) {
		deleteConfirmation = backup;
	}

	async function handleConfirmDelete() {
		if (!deleteConfirmation) return;

		try {
			await api.deleteBackup(deleteConfirmation.filename);
			deleteConfirmation = null;
			await loadBackups();
			toast.success($LL.toast.backupDeleted());
		} catch (error) {
			console.error('Failed to delete backup:', error);
			toast.error($LL.toast.errorDeleteBackup({ error: String(error) }));
		}
	}

	function cancelDelete() {
		deleteConfirmation = null;
	}

	function formatBackupDate(isoDate: string): string {
		try {
			const date = new Date(isoDate);
			return date.toLocaleString('sk-SK', {
				day: '2-digit',
				month: '2-digit',
				year: 'numeric',
				hour: '2-digit',
				minute: '2-digit',
				second: '2-digit'
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
		<h1>{$LL.settings.title()}</h1>
	</div>

	<div class="sections">
		<!-- Language Section -->
		<section class="settings-section">
			<h2>{$LL.settings.languageSection()}</h2>
			<div class="section-content">
				<div class="form-group">
					<label for="language-select">{$LL.settings.language()}</label>
					<select id="language-select" value={selectedLocale} on:change={handleLocaleChange}>
						<option value="sk">Slovenčina</option>
						<option value="en">English</option>
					</select>
				</div>
			</div>
		</section>

		<!-- Vehicles Section -->
		<section class="settings-section">
			<h2>{$LL.settings.vehiclesSection()}</h2>
			<div class="section-content">
				{#if $vehiclesStore.length > 0}
					<div class="vehicle-list">
						{#each $vehiclesStore as vehicle}
							<div class="vehicle-item">
								<div class="vehicle-info">
									<strong>{vehicle.name}</strong>
									<span class="license-plate">{vehicle.license_plate}</span>
									<span class="details">
										{#if vehicle.vehicle_type === 'Ice'}
											{vehicle.tank_size_liters ?? 0}L | {vehicle.tp_consumption ?? 0} L/100km
										{:else if vehicle.vehicle_type === 'Bev'}
											{vehicle.battery_capacity_kwh ?? 0} kWh | {vehicle.baseline_consumption_kwh ?? 0} kWh/100km
										{:else}
											{vehicle.tank_size_liters ?? 0}L + {vehicle.battery_capacity_kwh ?? 0} kWh
										{/if}
									</span>
									<span class="badge type-{vehicle.vehicle_type.toLowerCase()}">{vehicle.vehicle_type}</span>
									{#if vehicle.is_active}
										<span class="badge active">{$LL.vehicle.active()}</span>
									{/if}
								</div>
								<div class="vehicle-actions">
									<button class="button-small" on:click={() => openEditVehicleModal(vehicle)}>
										{$LL.common.edit()}
									</button>
									{#if !vehicle.is_active}
										<button
											class="button-small primary"
											on:click={() => handleSetActiveVehicle(vehicle)}
										>
											{$LL.vehicle.setAsActive()}
										</button>
									{/if}
									<button class="button-small danger" on:click={() => handleDeleteVehicleClick(vehicle)}>
										{$LL.common.delete()}
									</button>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<p class="placeholder">{$LL.settings.noVehicles()}</p>
				{/if}
				<button class="button" on:click={openAddVehicleModal}>{$LL.settings.addVehicle()}</button>
			</div>
		</section>

		<!-- Company Settings Section -->
		<section class="settings-section">
			<h2>{$LL.settings.companySection()}</h2>
			<div class="section-content">
				<div class="form-group">
					<label for="company-name">{$LL.settings.companyName()}</label>
					<input
						type="text"
						id="company-name"
						bind:value={companyName}
						placeholder={$LL.settings.companyNamePlaceholder()}
					/>
				</div>

				<div class="form-group">
					<label for="company-ico">{$LL.settings.companyIco()}</label>
					<input type="text" id="company-ico" bind:value={companyIco} placeholder={$LL.settings.companyIcoPlaceholder()} />
				</div>

				<div class="form-group">
					<label for="buffer-purpose">{$LL.settings.bufferTripPurpose()}</label>
					<input
						type="text"
						id="buffer-purpose"
						bind:value={bufferTripPurpose}
						placeholder={$LL.settings.bufferTripPurposePlaceholder()}
					/>
					<small class="hint">
						{$LL.settings.bufferTripPurposeHint()}
					</small>
				</div>

				<button class="button" on:click={handleSaveSettings}>{$LL.settings.saveSettings()}</button>
			</div>
		</section>

		<!-- Backup Section -->
		<section class="settings-section">
			<h2>{$LL.settings.backupSection()}</h2>
			<div class="section-content">
				<button class="button" on:click={handleCreateBackup} disabled={backupInProgress}>
					{backupInProgress ? $LL.settings.creatingBackup() : $LL.settings.createBackup()}
				</button>

				<div class="backup-list">
					<h3>{$LL.settings.availableBackups()}</h3>
					{#if loadingBackups}
						<p class="placeholder">{$LL.common.loading()}</p>
					{:else if backups.length === 0}
						<p class="placeholder">{$LL.settings.noBackups()}</p>
					{:else}
						{#each backups as backup}
							<div class="backup-item">
								<div class="backup-info">
									<span class="backup-date">{formatBackupDate(backup.created_at)}</span>
									<span class="backup-size">{formatFileSize(backup.size_bytes)}</span>
								</div>
								<div class="backup-actions">
									<button class="button-small" on:click={() => handleRestoreClick(backup)}>
										{$LL.settings.restore()}
									</button>
									<button class="button-small danger" on:click={() => handleDeleteClick(backup)}>
										{$LL.common.delete()}
									</button>
								</div>
							</div>
						{/each}
					{/if}
				</div>
			</div>
		</section>
	</div>
</div>

{#if showVehicleModal}
	<VehicleModal
		vehicle={editingVehicle}
		hasTrips={editingVehicle ? vehiclesWithTrips.has(editingVehicle.id) : false}
		onSave={handleSaveVehicle}
		onClose={closeVehicleModal}
	/>
{/if}

{#if vehicleToDelete}
	<ConfirmModal
		title={$LL.confirm.deleteVehicleTitle()}
		message={$LL.confirm.deleteVehicleMessage({ name: vehicleToDelete.name })}
		confirmText={$LL.common.delete()}
		cancelText={$LL.common.cancel()}
		danger={true}
		onConfirm={handleConfirmDeleteVehicle}
		onCancel={cancelDeleteVehicle}
	/>
{/if}

{#if restoreConfirmation}
	<div class="modal-overlay" on:click={cancelRestore} role="button" tabindex="0" on:keydown={(e) => e.key === 'Escape' && cancelRestore()}>
		<div class="modal" on:click|stopPropagation on:keydown={() => {}} role="dialog" aria-modal="true" tabindex="-1">
			<h2>{$LL.backup.confirmRestoreTitle()}</h2>
			<div class="modal-content">
				<p><strong>{$LL.backup.backupDate()}</strong> {formatBackupDate(restoreConfirmation.created_at)}</p>
				<p><strong>{$LL.backup.backupSize()}</strong> {formatFileSize(restoreConfirmation.size_bytes)}</p>
				<p><strong>{$LL.backup.backupContains()}</strong> {$LL.backup.vehiclesAndTrips({ vehicles: restoreConfirmation.vehicle_count, trips: restoreConfirmation.trip_count })}</p>
				<p class="warning-text">
					{$LL.backup.restoreWarning()}
				</p>
			</div>
			<div class="modal-actions">
				<button class="button-small" on:click={cancelRestore}>{$LL.common.cancel()}</button>
				<button class="button-small danger" on:click={handleConfirmRestore}>{$LL.backup.restoreBackup()}</button>
			</div>
		</div>
	</div>
{/if}

{#if deleteConfirmation}
	<div class="modal-overlay" on:click={cancelDelete} role="button" tabindex="0" on:keydown={(e) => e.key === 'Escape' && cancelDelete()}>
		<div class="modal" on:click|stopPropagation on:keydown={() => {}} role="dialog" aria-modal="true" tabindex="-1">
			<h2>{$LL.backup.confirmDeleteTitle()}</h2>
			<div class="modal-content">
				<p><strong>{$LL.backup.backupDate()}</strong> {formatBackupDate(deleteConfirmation.created_at)}</p>
				<p><strong>{$LL.backup.backupSize()}</strong> {formatFileSize(deleteConfirmation.size_bytes)}</p>
				<p class="warning-text">
					{$LL.backup.deleteWarning()}
				</p>
			</div>
			<div class="modal-actions">
				<button class="button-small" on:click={cancelDelete}>{$LL.common.cancel()}</button>
				<button class="button-small danger" on:click={handleConfirmDelete}>{$LL.common.delete()}</button>
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

	.badge.type-ice {
		background-color: #e3f2fd;
		color: #1565c0;
	}

	.badge.type-bev {
		background-color: #e8f5e9;
		color: #2e7d32;
	}

	.badge.type-phev {
		background-color: #fff3e0;
		color: #e65100;
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

	.no-data {
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

	.backup-actions {
		display: flex;
		gap: 0.5rem;
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
