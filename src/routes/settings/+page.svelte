<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import VehicleModal from '$lib/components/VehicleModal.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Vehicle, Settings, BackupInfo } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { localeStore } from '$lib/stores/locale';
	import type { Locales } from '$lib/i18n/i18n-types';
	import { themeStore } from '$lib/stores/theme';
	import type { ThemeMode } from '$lib/api';

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

	// Theme state
	let selectedTheme: ThemeMode = 'system';
	let unsubscribeTheme: (() => void) | undefined;

	async function handleThemeChange(theme: ThemeMode) {
		selectedTheme = theme;
		await themeStore.change(theme);
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
		const unsubscribeLocale = localeStore.subscribe((locale) => {
			selectedLocale = locale;
		});

		// Initialize selected theme from store
		unsubscribeTheme = themeStore.subscribe((theme) => {
			selectedTheme = theme;
		});

		// Load settings and backups (async operations)
		(async () => {
			const loadedSettings = await api.getSettings();
			if (loadedSettings) {
				settings = loadedSettings;
				companyName = loadedSettings.companyName;
				companyIco = loadedSettings.companyIco;
				bufferTripPurpose = loadedSettings.bufferTripPurpose;
			}

			await loadBackups();
			await checkVehiclesWithTrips();
		})();

		return () => unsubscribeLocale();
	});

	onDestroy(() => {
		if (unsubscribeTheme) unsubscribeTheme();
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
					licensePlate: data.licensePlate,
					vehicleType: data.vehicleType,
					tankSizeLiters: data.tankSizeLiters,
					tpConsumption: data.tpConsumption,
					batteryCapacityKwh: data.batteryCapacityKwh,
					baselineConsumptionKwh: data.baselineConsumptionKwh,
					initialBatteryPercent: data.initialBatteryPercent,
					initialOdometer: data.initialOdometer,
					vin: data.vin,
					driverName: data.driverName,
					updatedAt: new Date().toISOString()
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

		<!-- Appearance Section -->
		<section class="settings-section">
			<h2>{$LL.settings.appearanceSection()}</h2>
			<div class="section-content">
				<div class="form-group">
					<label>{$LL.settings.themeLabel()}</label>
					<div class="theme-options">
						<label class="theme-option">
							<input
								type="radio"
								name="theme"
								value="system"
								checked={selectedTheme === 'system'}
								on:change={() => handleThemeChange('system')}
							/>
							<span>{$LL.settings.themeSystem()}</span>
						</label>
						<label class="theme-option">
							<input
								type="radio"
								name="theme"
								value="light"
								checked={selectedTheme === 'light'}
								on:change={() => handleThemeChange('light')}
							/>
							<span>{$LL.settings.themeLight()}</span>
						</label>
						<label class="theme-option">
							<input
								type="radio"
								name="theme"
								value="dark"
								checked={selectedTheme === 'dark'}
								on:change={() => handleThemeChange('dark')}
							/>
							<span>{$LL.settings.themeDark()}</span>
						</label>
					</div>
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
									<span class="license-plate">{vehicle.licensePlate}</span>
									<span class="details">
										{#if vehicle.vehicleType === 'Ice'}
											{vehicle.tankSizeLiters ?? 0}L | {vehicle.tpConsumption ?? 0} L/100km
										{:else if vehicle.vehicleType === 'Bev'}
											{vehicle.batteryCapacityKwh ?? 0} kWh | {vehicle.baselineConsumptionKwh ?? 0} kWh/100km
										{:else}
											{vehicle.tankSizeLiters ?? 0}L + {vehicle.batteryCapacityKwh ?? 0} kWh
										{/if}
									</span>
									<span class="badge type-{vehicle.vehicleType.toLowerCase()}">{vehicle.vehicleType}</span>
									{#if vehicle.isActive}
										<span class="badge active">{$LL.vehicle.active()}</span>
									{/if}
								</div>
								<div class="vehicle-actions">
									<button class="button-small" on:click={() => openEditVehicleModal(vehicle)}>
										{$LL.common.edit()}
									</button>
									{#if !vehicle.isActive}
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
									<span class="backup-date">{formatBackupDate(backup.createdAt)}</span>
									<span class="backup-size">{formatFileSize(backup.sizeBytes)}</span>
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
				<p><strong>{$LL.backup.backupDate()}</strong> {formatBackupDate(restoreConfirmation.createdAt)}</p>
				<p><strong>{$LL.backup.backupSize()}</strong> {formatFileSize(restoreConfirmation.sizeBytes)}</p>
				<p><strong>{$LL.backup.backupContains()}</strong> {$LL.backup.vehiclesAndTrips({ vehicles: restoreConfirmation.vehicleCount, trips: restoreConfirmation.tripCount })}</p>
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
				<p><strong>{$LL.backup.backupDate()}</strong> {formatBackupDate(deleteConfirmation.createdAt)}</p>
				<p><strong>{$LL.backup.backupSize()}</strong> {formatFileSize(deleteConfirmation.sizeBytes)}</p>
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
		color: var(--text-primary);
	}

	.sections {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.settings-section {
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px var(--shadow-default);
	}

	.settings-section h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: var(--text-primary);
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
		border: 1px solid var(--border-default);
		border-radius: 4px;
		background: var(--bg-surface-alt);
	}

	.vehicle-info {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.vehicle-info strong {
		font-size: 1rem;
		color: var(--text-primary);
	}

	.license-plate {
		font-size: 0.875rem;
		color: var(--text-secondary);
		font-weight: 500;
	}

	.details {
		font-size: 0.75rem;
		color: var(--text-muted);
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
		background-color: var(--btn-primary-light-bg);
		color: var(--btn-primary-light-color);
	}

	.badge.type-ice {
		background-color: var(--badge-ice-bg);
		color: var(--badge-ice-color);
	}

	.badge.type-bev {
		background-color: var(--badge-bev-bg);
		color: var(--badge-bev-color);
	}

	.badge.type-phev {
		background-color: var(--badge-phev-bg);
		color: var(--badge-phev-color);
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
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.form-group input,
	.form-group select {
		padding: 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 1rem;
		font-family: inherit;
		background-color: var(--input-bg);
		color: var(--text-primary);
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-secondary);
		font-style: italic;
		margin: 0;
	}

	.placeholder {
		color: var(--text-secondary);
		font-style: italic;
		margin: 0;
	}

	.no-data {
		color: var(--text-secondary);
		font-style: italic;
		margin: 0;
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
		font-size: 1rem;
	}

	.button:hover {
		background-color: var(--btn-active-primary-hover);
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}

	.button-small.primary {
		background-color: var(--btn-primary-light-bg);
		color: var(--btn-primary-light-color);
	}

	.button-small.primary:hover {
		background-color: var(--btn-primary-light-hover);
	}

	.button-small.danger {
		background-color: var(--accent-danger-bg);
		color: var(--accent-danger);
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
		color: var(--text-primary);
		margin: 0 0 0.75rem 0;
	}

	.backup-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 4px;
		background: var(--bg-surface-alt);
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
		color: var(--text-primary);
	}

	.backup-size {
		color: var(--text-secondary);
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
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		max-width: 400px;
		width: 90%;
	}

	.modal h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.modal-content {
		margin-bottom: 1.5rem;
	}

	.modal-content p {
		margin: 0.5rem 0;
		color: var(--text-primary);
	}

	.warning-text {
		color: var(--accent-danger);
		font-weight: 500;
		margin-top: 1rem !important;
		padding: 0.75rem;
		background: var(--accent-danger-bg);
		border-radius: 4px;
	}

	.modal-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.theme-options {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.theme-option {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		padding: 0.5rem;
		border-radius: 4px;
		transition: background-color 0.2s;
	}

	.theme-option:hover {
		background-color: var(--bg-surface-alt);
	}

	.theme-option input[type="radio"] {
		width: 18px;
		height: 18px;
		cursor: pointer;
	}

	.theme-option span {
		color: var(--text-primary);
	}
</style>
