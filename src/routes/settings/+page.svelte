<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { vehiclesStore, activeVehicleStore } from '$lib/stores/vehicles';
	import VehicleModal from '$lib/components/VehicleModal.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Vehicle, Settings, BackupInfo, CleanupPreview, BackupRetention } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { localeStore } from '$lib/stores/locale';
	import type { Locales } from '$lib/i18n/i18n-types';
	import { themeStore } from '$lib/stores/theme';
	import type { ThemeMode } from '$lib/api';
	import { getAppVersion, getReceiptSettings, setGeminiApiKey, setReceiptsFolderPath, getHaSettings, saveHaSettings, testHaConnection, fetchHaOdo, getInferTripTimes, setInferTripTimes, getPaperlessSettings, savePaperlessSettings, testPaperlessConnection, listPaperlessCustomFields, revealSecret } from '$lib/api';
	import type { PaperlessCustomFieldInfo, SecretField } from '$lib/types';
	import type { HaSettings } from '$lib/types';

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

	// App version
	let appVersion = '';

	// Time inference toggle (default OFF until loaded)
	let inferTripTimes = false;

	// Receipt scanning settings state
	let geminiApiKey = '';
	let receiptsFolderPath = '';
	let hasGeminiApiKey = false;
	let geminiKeyFromEnv = false;

	// ── Secret reveal ────────────────────────────────────────────────────────
	// Secrets never arrive with the settings; each reveal is a separate backend
	// call that needs the PIN when the request comes over the network. Values
	// live only in `revealed` and are dropped on re-mask or unmount, so every
	// reveal asks again.
	let revealed: Partial<Record<SecretField, string>> = {};
	let pinPromptFor: SecretField | null = null;
	let pinInput = '';
	let pinError = '';
	let pinBusy = false;

	function isRevealed(field: SecretField, r: typeof revealed): boolean {
		return r[field] !== undefined;
	}

	/** Eye click: hide if shown, else ask for the PIN and reveal. */
	async function toggleReveal(field: SecretField) {
		if (revealed[field] !== undefined) {
			delete revealed[field];
			revealed = revealed;
			return;
		}
		pinInput = '';
		pinError = '';
		pinPromptFor = field;
	}

	async function submitPin() {
		if (!pinPromptFor) return;
		pinBusy = true;
		pinError = '';
		try {
			const value = await revealSecret(pinPromptFor, pinInput);
			revealed = { ...revealed, [pinPromptFor]: value };
			pinPromptFor = null;
			pinInput = '';
		} catch (error) {
			// Keep the modal open so the PIN can be retried; the backend message
			// distinguishes wrong PIN, unconfigured PIN, and lockout.
			pinError = String(error).replace(/^Error:\s*/, '');
		} finally {
			pinBusy = false;
		}
	}

	function cancelPin() {
		pinPromptFor = null;
		pinInput = '';
		pinError = '';
	}

	onDestroy(() => {
		// Don't leave secrets in memory after navigating away
		revealed = {};
	});

	// Track initial values to detect actual changes
	let initialCompanyName = '';
	let initialCompanyIco = '';
	let initialBufferTripPurpose = '';
	let initialGeminiApiKey = '';
	let initialReceiptsFolderPath = '';

	// Home Assistant settings state
	let haUrl = '';
	let haApiToken = '';
	let haHasToken = false;
	let haUrlError = '';
	let initialHaUrl = '';
	let initialHaApiToken = '';
	// Env-var pinning: backend decides, page only renders (ADR-008)
	let haUrlFromEnv = false;
	let haTokenFromEnv = false;
	// Connection status
	const HA_STATUS = {
		IDLE: 'idle',
		TESTING: 'testing',
		CONNECTED: 'connected',
		DISCONNECTED: 'disconnected'
	} as const;
	type HaStatus = typeof HA_STATUS[keyof typeof HA_STATUS];
	let haConnectionStatus: HaStatus = HA_STATUS.IDLE;

	// Paperless settings state
	let paperlessUrl = '';
	let paperlessApiToken = '';
	let paperlessHasToken = false;
	let paperlessUrlError = '';
	let initialPaperlessUrl = '';
	let initialPaperlessApiToken = '';
	let paperlessUrlFromEnv = false;
	let paperlessTokenFromEnv = false;
	let paperlessEnabledFromEnv = false;
	const PAPERLESS_STATUS = {
		IDLE: 'idle',
		TESTING: 'testing',
		CONNECTED: 'connected',
		DISCONNECTED: 'disconnected'
	} as const;
	type PaperlessStatus = typeof PAPERLESS_STATUS[keyof typeof PAPERLESS_STATUS];
	let paperlessConnectionStatus: PaperlessStatus = PAPERLESS_STATUS.IDLE;
	let paperlessEnabled = false;
	// Paperless custom field name overrides
	let paperlessFieldDatetime = '';
	let paperlessFieldLiters = '';
	let paperlessFieldTotal = '';
	let initialPaperlessFieldDatetime = '';
	let initialPaperlessFieldLiters = '';
	let initialPaperlessFieldTotal = '';
	// Live list of custom fields fetched from the Paperless server (drives the dropdowns)
	let paperlessCustomFields: PaperlessCustomFieldInfo[] | null = null;
	let paperlessCustomFieldsLoading = false;
	let paperlessCustomFieldsError = '';
	// Compatibility filter: which Paperless data_types are usable per concept
	const PAPERLESS_FIELD_COMPATIBLE = {
		datetime: ['string', 'date'],
		liters: ['float', 'integer'],
		total: ['float', 'monetary', 'integer'],
	} as const;
	function paperlessCompatibleFields(concept: keyof typeof PAPERLESS_FIELD_COMPATIBLE): PaperlessCustomFieldInfo[] {
		if (!paperlessCustomFields) return [];
		const allowed = PAPERLESS_FIELD_COMPATIBLE[concept] as readonly string[];
		return paperlessCustomFields.filter((f) => allowed.includes(f.dataType));
	}
	const PAPERLESS_DEFAULT_FIELD_NAMES = {
		datetime: 'receipt_datetime',
		liters: 'liters',
		total: 'total_price_eur',
	} as const;

	// Real ODO values from HA (keyed by vehicle ID)
	let vehicleOdoValues: Map<string, number> = new Map();

	// Debounce utility for auto-save
	function debounce<T extends (...args: unknown[]) => void>(fn: T, ms: number): T {
		let timeoutId: ReturnType<typeof setTimeout>;
		return ((...args: unknown[]) => {
			clearTimeout(timeoutId);
			timeoutId = setTimeout(() => fn(...args), ms);
		}) as T;
	}

	// Auto-save functions for company settings
	async function saveCompanySettingsNow() {
		// Only save if values actually changed
		if (
			companyName === initialCompanyName &&
			companyIco === initialCompanyIco &&
			bufferTripPurpose === initialBufferTripPurpose
		) {
			return;
		}
		try {
			const savedSettings = await api.saveSettings(companyName, companyIco, bufferTripPurpose);
			settings = savedSettings;
			initialCompanyName = companyName;
			initialCompanyIco = companyIco;
			initialBufferTripPurpose = bufferTripPurpose;
			toast.success($LL.toast.settingsSaved());
		} catch (error) {
			console.error('Failed to save settings:', error);
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
	}

	// Auto-save functions for receipt settings
	async function saveReceiptSettingsNow() {
		// Only save if values actually changed
		if (
			geminiApiKey === initialGeminiApiKey &&
			receiptsFolderPath === initialReceiptsFolderPath
		) {
			return;
		}
		try {
			// The key is read-only while GEMINI_API_KEY pins it — sending it would
			// be rejected and mask the folder save. It's also write-only now, so a
			// blank field means "unchanged", NOT "clear it".
			if (!geminiKeyFromEnv && geminiApiKey !== '') {
				await setGeminiApiKey(geminiApiKey);
				hasGeminiApiKey = true;
			}
			await setReceiptsFolderPath(receiptsFolderPath);
			initialGeminiApiKey = geminiApiKey;
			initialReceiptsFolderPath = receiptsFolderPath;
			toast.success($LL.settings.receiptSettingsSaved());
		} catch (error) {
			console.error('Failed to save receipt settings:', error);
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
	}

	// Debounced versions (800ms delay)
	const debouncedSaveCompanySettings = debounce(saveCompanySettingsNow, 800);
	const debouncedSaveReceiptSettings = debounce(saveReceiptSettingsNow, 800);

	// Home Assistant settings handlers
	function validateHaUrl(url: string): boolean {
		if (!url) return true; // Empty is valid (clearing)
		if (!url.startsWith('http://') && !url.startsWith('https://')) {
			haUrlError = $LL.homeAssistant.urlInvalid();
			return false;
		}
		try {
			new URL(url);
			haUrlError = '';
			return true;
		} catch {
			haUrlError = $LL.homeAssistant.urlInvalid();
			return false;
		}
	}

	// Test HA connection using stored credentials (via Rust to avoid CORS)
	async function testHaConnectionStatus() {
		// Need both URL and token to test
		if (!haUrl || (!haApiToken && !haHasToken)) {
			haConnectionStatus = HA_STATUS.IDLE;
			return;
		}

		haConnectionStatus = HA_STATUS.TESTING;
		try {
			// Call Rust backend - it has the stored credentials and no CORS issues
			const isConnected = await testHaConnection();
			haConnectionStatus = isConnected ? HA_STATUS.CONNECTED : HA_STATUS.DISCONNECTED;

			// If connected, fetch ODO values for all vehicles with sensors
			if (isConnected) {
				await fetchAllVehicleOdoValues();
			}
		} catch (error) {
			console.error('[HA] Connection test failed:', error);
			haConnectionStatus = HA_STATUS.DISCONNECTED;
		}
	}

	// Fetch ODO values for all vehicles that have HA sensors configured
	async function fetchAllVehicleOdoValues() {
		const vehicles = $vehiclesStore;
		const newValues = new Map<string, number>();

		for (const vehicle of vehicles) {
			if (vehicle.haOdoSensor) {
				try {
					const odo = await fetchHaOdo(vehicle.haOdoSensor);
					if (odo !== null) {
						newValues.set(vehicle.id, odo);
					}
				} catch (error) {
					console.error(`[HA] Failed to fetch ODO for ${vehicle.name}:`, error);
				}
			}
		}

		vehicleOdoValues = newValues;
	}

	async function saveHaSettingsNow() {
		// Nothing here is ours to change
		if (haUrlFromEnv && haTokenFromEnv) {
			return;
		}
		// Only save if values actually changed
		if (haUrl === initialHaUrl && haApiToken === initialHaApiToken) {
			return;
		}
		// Validate URL before saving
		if (!validateHaUrl(haUrl)) {
			return;
		}
		try {
			// null = "leave alone". Sending an env-pinned field would be rejected by
			// the backend guard even when the user only edited the other one.
			await saveHaSettings(
				haUrlFromEnv ? null : haUrl || null,
				haTokenFromEnv ? null : haApiToken || null
			);
			initialHaUrl = haUrl;
			initialHaApiToken = haApiToken;
			haHasToken = !!haApiToken;
			toast.success($LL.toast.settingsSaved());
			// Test connection after save
			await testHaConnectionStatus();
		} catch (error) {
			console.error('Failed to save HA settings:', error);
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
	}

	const debouncedSaveHaSettings = debounce(saveHaSettingsNow, 800);

	// Paperless-ngx settings handlers
	function validatePaperlessUrl(url: string): boolean {
		if (!url) return true;
		if (!url.startsWith('http://') && !url.startsWith('https://')) {
			paperlessUrlError = $LL.paperless.errors.urlInvalid();
			return false;
		}
		try {
			new URL(url);
			paperlessUrlError = '';
			return true;
		} catch {
			paperlessUrlError = $LL.paperless.errors.urlInvalid();
			return false;
		}
	}

	async function testPaperlessConnectionStatus() {
		if (!paperlessUrl || (!paperlessApiToken && !paperlessHasToken)) {
			paperlessConnectionStatus = PAPERLESS_STATUS.IDLE;
			return;
		}
		paperlessConnectionStatus = PAPERLESS_STATUS.TESTING;
		try {
			const isConnected = await testPaperlessConnection();
			paperlessConnectionStatus = isConnected ? PAPERLESS_STATUS.CONNECTED : PAPERLESS_STATUS.DISCONNECTED;
		} catch (error) {
			console.error('[Paperless] Connection test failed:', error);
			paperlessConnectionStatus = PAPERLESS_STATUS.DISCONNECTED;
		}
	}

	async function savePaperlessSettingsNow() {
		if (paperlessUrlFromEnv && paperlessTokenFromEnv) {
			return;
		}
		if (paperlessUrl === initialPaperlessUrl && paperlessApiToken === initialPaperlessApiToken) {
			return;
		}
		if (!validatePaperlessUrl(paperlessUrl)) {
			return;
		}
		try {
			await savePaperlessSettings(
				paperlessUrlFromEnv ? null : paperlessUrl || null,
				paperlessTokenFromEnv ? null : paperlessApiToken || null
			);
			initialPaperlessUrl = paperlessUrl;
			initialPaperlessApiToken = paperlessApiToken;
			// Re-fetch to get true has_token: backend preserves existing token when null is sent.
			const refreshed = await getPaperlessSettings();
			paperlessHasToken = refreshed.hasToken;
			toast.success($LL.toast.settingsSaved());
			await testPaperlessConnectionStatus();
		} catch (error) {
			console.error('Failed to save Paperless settings:', error);
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
	}

	const debouncedSavePaperlessSettings = debounce(savePaperlessSettingsNow, 800);

	async function loadPaperlessCustomFields() {
		// Only attempt when Paperless is actually configured.
		if (!(initialPaperlessUrl && paperlessHasToken)) {
			paperlessCustomFields = null;
			paperlessCustomFieldsError = '';
			return;
		}
		paperlessCustomFieldsLoading = true;
		paperlessCustomFieldsError = '';
		try {
			paperlessCustomFields = await listPaperlessCustomFields();
		} catch (error) {
			console.error('Failed to load Paperless custom fields:', error);
			paperlessCustomFieldsError = String(error);
			paperlessCustomFields = [];
		} finally {
			paperlessCustomFieldsLoading = false;
		}
	}

	async function savePaperlessFieldNamesNow() {
		if (
			paperlessFieldDatetime === initialPaperlessFieldDatetime &&
			paperlessFieldLiters === initialPaperlessFieldLiters &&
			paperlessFieldTotal === initialPaperlessFieldTotal
		) {
			return;
		}
		try {
			// Empty strings clear → backend uses default. null = no change.
			await savePaperlessSettings(
				null, null, null,
				paperlessFieldDatetime,
				paperlessFieldLiters,
				paperlessFieldTotal,
			);
			// Re-fetch so resolved values (with defaults applied for empty input) come back.
			const refreshed = await getPaperlessSettings();
			paperlessFieldDatetime = refreshed.fieldNameDatetime;
			paperlessFieldLiters = refreshed.fieldNameLiters;
			paperlessFieldTotal = refreshed.fieldNameTotal;
			initialPaperlessFieldDatetime = paperlessFieldDatetime;
			initialPaperlessFieldLiters = paperlessFieldLiters;
			initialPaperlessFieldTotal = paperlessFieldTotal;
			toast.success($LL.toast.settingsSaved());
		} catch (error) {
			console.error('Failed to save Paperless field names:', error);
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
	}

	async function togglePaperlessEnabled(value: boolean) {
		paperlessEnabled = value;
		try {
			await savePaperlessSettings(null, null, value);
			toast.success($LL.toast.settingsSaved());
			if (value) {
				await testPaperlessConnectionStatus();
			} else {
				paperlessConnectionStatus = PAPERLESS_STATUS.IDLE;
			}
		} catch (error) {
			console.error('Failed to toggle Paperless:', error);
			paperlessEnabled = !value;
			toast.error($LL.toast.errorSaveSettings({ error: String(error) }));
		}
	}

	async function handleThemeChange(theme: ThemeMode) {
		selectedTheme = theme;
		await themeStore.change(theme);
	}

	async function handleInferTripTimesChange(event: Event) {
		const checkbox = event.target as HTMLInputElement;
		inferTripTimes = checkbox.checked;
		await setInferTripTimes(inferTripTimes);
	}

	// Backup state
	let backups: BackupInfo[] = [];
	let loadingBackups = false;
	let backupInProgress = false;
	let restoreConfirmation: BackupInfo | null = null;
	let deleteConfirmation: BackupInfo | null = null;

	// Backup retention state
	let retentionEnabled = false;
	let retentionKeepCount = 3;
	let cleanupPreview: CleanupPreview | null = null;
	let cleaningUp = false;

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
				// Track initial values for change detection
				initialCompanyName = loadedSettings.companyName;
				initialCompanyIco = loadedSettings.companyIco;
				initialBufferTripPurpose = loadedSettings.bufferTripPurpose;
			}

			await loadBackups();
			await loadRetentionSettings();
			await checkVehiclesWithTrips();

			// Load app version (works in desktop and web/server mode)
			appVersion = await getAppVersion();

			// Load time-inference toggle
			inferTripTimes = await getInferTripTimes();

			// Load receipt settings
			const receiptSettings = await getReceiptSettings();
			if (receiptSettings) {
				hasGeminiApiKey = receiptSettings.hasGeminiApiKey;
				receiptsFolderPath = receiptSettings.receiptsFolderPath || '';
				geminiKeyFromEnv = receiptSettings.geminiApiKeyFromEnv;
				// The key is write-only now, so the input starts empty
				geminiApiKey = '';
				initialGeminiApiKey = '';
				initialReceiptsFolderPath = receiptSettings.receiptsFolderPath || '';
			}

			// Load Home Assistant settings
			const haSettings = await getHaSettings();
			if (haSettings) {
				haUrl = haSettings.url || '';
				haHasToken = haSettings.hasToken;
				initialHaUrl = haSettings.url || '';
				haUrlFromEnv = haSettings.urlFromEnv;
				haTokenFromEnv = haSettings.tokenFromEnv;
				// Test connection if configured
				await testHaConnectionStatus();
			}

			// Load Paperless-ngx settings
			try {
				const paperlessSettings = await getPaperlessSettings();
				paperlessUrl = paperlessSettings.url || '';
				paperlessHasToken = paperlessSettings.hasToken;
				paperlessEnabled = paperlessSettings.enabled;
				paperlessUrlFromEnv = paperlessSettings.urlFromEnv;
				paperlessTokenFromEnv = paperlessSettings.tokenFromEnv;
				paperlessEnabledFromEnv = paperlessSettings.enabledFromEnv;
				paperlessFieldDatetime = paperlessSettings.fieldNameDatetime;
				paperlessFieldLiters = paperlessSettings.fieldNameLiters;
				paperlessFieldTotal = paperlessSettings.fieldNameTotal;
				initialPaperlessUrl = paperlessUrl;
				initialPaperlessFieldDatetime = paperlessFieldDatetime;
				initialPaperlessFieldLiters = paperlessFieldLiters;
				initialPaperlessFieldTotal = paperlessFieldTotal;
				if (paperlessUrl && paperlessHasToken) {
					await testPaperlessConnectionStatus();
					// Populate the custom-field dropdowns so the user can pick names
					// instead of typing them. Best-effort — failure surfaces inline.
					await loadPaperlessCustomFields();
				}
			} catch (error) {
				console.error('Failed to load Paperless settings:', error);
			}

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
		haOdoSensor: string | null;
		haFillupSensor: string | null;
		haFuelLevelSensor: string | null;
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
					haOdoSensor: data.haOdoSensor,
					haFillupSensor: data.haFillupSensor,
					haFuelLevelSensor: data.haFuelLevelSensor,
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

	// Retention settings functions
	async function loadRetentionSettings() {
		try {
			const retention = await api.getBackupRetention();
			if (retention) {
				retentionEnabled = retention.enabled;
				retentionKeepCount = retention.keepCount;
			}
			if (retentionEnabled) {
				cleanupPreview = await api.getCleanupPreview(retentionKeepCount);
			}
		} catch (error) {
			console.error('Failed to load retention settings:', error);
		}
	}

	async function handleRetentionChange() {
		try {
			await api.setBackupRetention({
				enabled: retentionEnabled,
				keepCount: retentionKeepCount
			});
			if (retentionEnabled) {
				cleanupPreview = await api.getCleanupPreview(retentionKeepCount);
			} else {
				cleanupPreview = null;
			}
		} catch (error) {
			console.error('Failed to save retention settings:', error);
			toast.error(String(error));
		}
	}

	async function handleCleanupNow() {
		if (!cleanupPreview || cleanupPreview.toDelete.length === 0) return;
		cleaningUp = true;
		try {
			await api.cleanupPreUpdateBackups(retentionKeepCount);
			await loadBackups();
			cleanupPreview = await api.getCleanupPreview(retentionKeepCount);
			toast.success($LL.toast.cleanupComplete());
		} catch (error) {
			console.error('Failed to cleanup backups:', error);
			toast.error(String(error));
		} finally {
			cleaningUp = false;
		}
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
				<fieldset class="theme-fieldset">
					<legend>{$LL.settings.themeLabel()}</legend>
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
				</fieldset>
			</div>
		</section>

		<!-- Receipt Scanning Section -->
		<section class="settings-section" id="receipt-scanning">
			<h2>{$LL.settings.receiptScanningSection()}</h2>
			<div class="section-content">
				<div class="form-group">
					<label for="gemini-api-key">
						{$LL.settings.geminiApiKey()}
						{#if geminiKeyFromEnv}
							<span class="env-badge" data-test="gemini-key-env-badge" title={$LL.settings.envManagedTitle({ name: 'GEMINI_API_KEY' })}>GEMINI_API_KEY</span>
						{/if}
					</label>
					<div class="input-with-icon">
						<input
							type={isRevealed('geminiApiKey', revealed) ? 'text' : 'password'}
							id="gemini-api-key"
							class="monospace-input"
							value={isRevealed('geminiApiKey', revealed) ? revealed.geminiApiKey : geminiApiKey}
							disabled={geminiKeyFromEnv}
							placeholder={hasGeminiApiKey && !geminiApiKey ? '********' : $LL.settings.geminiApiKeyPlaceholder()}
							on:input={(e) => { geminiApiKey = (e.target as HTMLInputElement).value; debouncedSaveReceiptSettings(); }}
							on:blur={saveReceiptSettingsNow}
						/>
						<button
							type="button"
							class="icon-btn"
							data-test="reveal-gemini-key"
							disabled={!hasGeminiApiKey}
							on:click={() => toggleReveal('geminiApiKey')}
							title={isRevealed('geminiApiKey', revealed) ? $LL.settings.hideApiKey() : $LL.settings.showApiKey()}
						>
							{#if isRevealed('geminiApiKey', revealed)}
								<!-- Eye off icon (Lucide) -->
								<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 11 8 11 8a13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.526 13.526 0 0 0 1 12s4 8 11 8a9.74 9.74 0 0 0 5.39-1.61"/><line x1="2" x2="22" y1="2" y2="22"/></svg>
							{:else}
								<!-- Eye icon (Lucide) -->
								<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>
							{/if}
						</button>
					</div>
					<small class="hint">
						{geminiKeyFromEnv ? $LL.settings.envManaged() : $LL.settings.geminiApiKeyHint()}
					</small>
				</div>

				<div class="form-group">
					<label for="receipts-folder">{$LL.settings.receiptsFolder()}</label>
					<!-- The path is typed: it resolves on the server, not on the
					     browsing device. -->
					<input
						type="text"
						id="receipts-folder"
						data-test="receipts-folder-input"
						bind:value={receiptsFolderPath}
						placeholder={$LL.settings.receiptsFolderPlaceholder()}
						on:input={debouncedSaveReceiptSettings}
						on:blur={saveReceiptSettingsNow}
					/>
					<small class="hint">{$LL.settings.receiptsFolderServerHint()}</small>
				</div>
			</div>
		</section>

		<!-- Home Assistant Section -->
		<section class="settings-section" id="home-assistant">
			<h2>{$LL.homeAssistant.sectionTitle()}</h2>
			<div class="section-content">
				<div class="form-group">
					<label for="ha-url">
						{$LL.homeAssistant.urlLabel()}
						{#if haUrlFromEnv}
							<span class="env-badge" data-test="ha-url-env-badge" title={$LL.settings.envManagedTitle({ name: 'HA_URL' })}>HA_URL</span>
						{/if}
					</label>
					<input
						type="text"
						id="ha-url"
						bind:value={haUrl}
						disabled={haUrlFromEnv}
						placeholder={$LL.homeAssistant.urlPlaceholder()}
						on:input={() => { validateHaUrl(haUrl); debouncedSaveHaSettings(); }}
						on:blur={saveHaSettingsNow}
						class:input-error={haUrlError}
					/>
					{#if haUrlError}
						<small class="error-text">{haUrlError}</small>
					{:else if haUrlFromEnv}
						<small class="hint">{$LL.settings.envManaged()}</small>
					{:else}
						<small class="hint">{$LL.homeAssistant.urlHint()}</small>
					{/if}
				</div>

				<div class="form-group">
					<label for="ha-token">
						{$LL.homeAssistant.tokenLabel()}
						{#if haTokenFromEnv}
							<span class="env-badge" data-test="ha-token-env-badge" title={$LL.settings.envManagedTitle({ name: 'HA_API_TOKEN' })}>HA_API_TOKEN</span>
						{/if}
					</label>
					<div class="input-with-icon">
						<input
							type={isRevealed('haApiToken', revealed) ? 'text' : 'password'}
							id="ha-token"
							class="monospace-input"
							value={isRevealed('haApiToken', revealed) ? revealed.haApiToken : haApiToken}
							disabled={haTokenFromEnv}
							placeholder={haHasToken && !haApiToken ? '********' : $LL.homeAssistant.tokenPlaceholder()}
							on:input={(e) => { haApiToken = (e.target as HTMLInputElement).value; debouncedSaveHaSettings(); }}
							on:blur={saveHaSettingsNow}
						/>
						<button
							type="button"
							class="icon-btn"
							data-test="reveal-ha-token"
							disabled={!haHasToken}
							on:click={() => toggleReveal('haApiToken')}
							title={isRevealed('haApiToken', revealed) ? $LL.settings.hideApiKey() : $LL.settings.showApiKey()}
						>
							{#if isRevealed('haApiToken', revealed)}
								<!-- Eye off icon -->
								<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 11 8 11 8a13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.526 13.526 0 0 0 1 12s4 8 11 8a9.74 9.74 0 0 0 5.39-1.61"/><line x1="2" x2="22" y1="2" y2="22"/></svg>
							{:else}
								<!-- Eye icon -->
								<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>
							{/if}
						</button>
					</div>
					<small class="hint">
						{#if haTokenFromEnv}
							{$LL.settings.envManaged()}
						{:else if haHasToken && !haApiToken}
							{$LL.homeAssistant.tokenSet()}
						{:else}
							{$LL.homeAssistant.tokenHint()}
						{/if}
					</small>
				</div>

				<!-- Connection Status -->
				{#if haConnectionStatus !== HA_STATUS.IDLE}
					<div class="ha-status" class:connected={haConnectionStatus === HA_STATUS.CONNECTED} class:disconnected={haConnectionStatus === HA_STATUS.DISCONNECTED} class:testing={haConnectionStatus === HA_STATUS.TESTING}>
						{#if haConnectionStatus === HA_STATUS.TESTING}
							<span class="status-icon">⏳</span>
							<span>{$LL.homeAssistant.testing()}</span>
						{:else if haConnectionStatus === HA_STATUS.CONNECTED}
							<span class="status-icon">✓</span>
							<span>{$LL.homeAssistant.connected()}</span>
						{:else}
							<span class="status-icon">✗</span>
							<span>{$LL.homeAssistant.disconnected()}</span>
						{/if}
					</div>
				{/if}
			</div>
		</section>

		<!-- Paperless-ngx Section -->
		<section class="settings-section" id="paperless">
			<h2>{$LL.paperless.sectionTitle()}</h2>
			<div class="section-content">
				<p class="hint">{$LL.paperless.description()}</p>

				<div class="form-group">
					<label
						class="checkbox-label"
						class:disabled={paperlessEnabledFromEnv || !initialPaperlessUrl || !paperlessHasToken}
						title={paperlessEnabledFromEnv
							? $LL.settings.envManagedTitle({ name: 'PAPERLESS_ENABLED' })
							: !initialPaperlessUrl || !paperlessHasToken ? $LL.paperless.enableToggleDisabledHint() : ''}
					>
						<input
							type="checkbox"
							data-test="paperless-enabled-toggle"
							checked={paperlessEnabled}
							disabled={paperlessEnabledFromEnv || !initialPaperlessUrl || !paperlessHasToken}
							on:change={(e) => togglePaperlessEnabled((e.target as HTMLInputElement).checked)}
						/>
						{$LL.paperless.enableToggle()}
						{#if paperlessEnabledFromEnv}
							<span class="env-badge" data-test="paperless-enabled-env-badge">PAPERLESS_ENABLED</span>
						{/if}
					</label>
				</div>

				<div class="form-group">
					<label for="paperless-url">
						{$LL.paperless.url()}
						{#if paperlessUrlFromEnv}
							<span class="env-badge" data-test="paperless-url-env-badge" title={$LL.settings.envManagedTitle({ name: 'PAPERLESS_URL' })}>PAPERLESS_URL</span>
						{/if}
					</label>
					<input
						type="text"
						id="paperless-url"
						data-test="paperless-url"
						bind:value={paperlessUrl}
						disabled={paperlessUrlFromEnv}
						placeholder={$LL.paperless.urlPlaceholder()}
						on:input={() => { validatePaperlessUrl(paperlessUrl); debouncedSavePaperlessSettings(); }}
						on:blur={savePaperlessSettingsNow}
						class:input-error={paperlessUrlError}
					/>
					{#if paperlessUrlError}
						<small class="error-text">{paperlessUrlError}</small>
					{:else if paperlessUrlFromEnv}
						<small class="hint">{$LL.settings.envManaged()}</small>
					{/if}
				</div>

				<div class="form-group">
					<label for="paperless-token">
						{$LL.paperless.apiToken()}
						{#if paperlessTokenFromEnv}
							<span class="env-badge" data-test="paperless-token-env-badge" title={$LL.settings.envManagedTitle({ name: 'PAPERLESS_API_TOKEN' })}>PAPERLESS_API_TOKEN</span>
						{/if}
					</label>
					<div class="input-with-icon">
						<input
							type={isRevealed('paperlessApiToken', revealed) ? 'text' : 'password'}
							id="paperless-token"
							data-test="paperless-token"
							class="monospace-input"
							value={isRevealed('paperlessApiToken', revealed) ? revealed.paperlessApiToken : paperlessApiToken}
							disabled={paperlessTokenFromEnv}
							placeholder={paperlessHasToken && !paperlessApiToken ? '********' : $LL.paperless.apiTokenPlaceholder()}
							on:input={(e) => { paperlessApiToken = (e.target as HTMLInputElement).value; debouncedSavePaperlessSettings(); }}
							on:blur={savePaperlessSettingsNow}
						/>
						<button
							type="button"
							class="icon-btn"
							data-test="reveal-paperless-token"
							disabled={!paperlessHasToken}
							on:click={() => toggleReveal('paperlessApiToken')}
							title={isRevealed('paperlessApiToken', revealed) ? $LL.settings.hideApiKey() : $LL.settings.showApiKey()}
						>
							{#if isRevealed('paperlessApiToken', revealed)}
								<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 11 8 11 8a13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.526 13.526 0 0 0 1 12s4 8 11 8a9.74 9.74 0 0 0 5.39-1.61"/><line x1="2" x2="22" y1="2" y2="22"/></svg>
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>
							{/if}
						</button>
					</div>
					{#if paperlessTokenFromEnv}
						<small class="hint">{$LL.settings.envManaged()}</small>
					{:else if paperlessHasToken && !paperlessApiToken}
						<small class="hint">{$LL.paperless.tokenSet()}</small>
					{/if}
				</div>

				{#if initialPaperlessUrl && paperlessHasToken}
					<div class="form-group">
						<div class="custom-fields-header">
							<h3>{$LL.paperless.customFields.sectionTitle()}</h3>
							<button
								type="button"
								class="button-small"
								data-test="paperless-refresh-fields"
								on:click={loadPaperlessCustomFields}
								disabled={paperlessCustomFieldsLoading}
							>
								{$LL.paperless.customFields.refresh()}
							</button>
						</div>
						<p class="hint">{$LL.paperless.customFields.sectionDescription()}</p>
					</div>

					{#if paperlessCustomFieldsLoading}
						<p class="hint">{$LL.paperless.customFields.loading()}</p>
					{:else if paperlessCustomFieldsError}
						<p class="error-text">{$LL.paperless.customFields.loadError()}</p>
					{:else if paperlessCustomFields}
						{@const datetimeFields = paperlessCompatibleFields('datetime')}
						{@const litersFields = paperlessCompatibleFields('liters')}
						{@const totalFields = paperlessCompatibleFields('total')}

						<div class="form-group">
							<label for="paperless-field-datetime">{$LL.paperless.customFields.datetime()}</label>
							<select
								id="paperless-field-datetime"
								data-test="paperless-field-datetime"
								bind:value={paperlessFieldDatetime}
								on:change={savePaperlessFieldNamesNow}
								disabled={datetimeFields.length === 0}
							>
								<option value="">{$LL.paperless.customFields.useDefault({ name: PAPERLESS_DEFAULT_FIELD_NAMES.datetime })}</option>
								{#each datetimeFields as field (field.id)}
									<option value={field.name}>{field.name} ({field.dataType})</option>
								{/each}
							</select>
							{#if datetimeFields.length === 0}
								<small class="hint">{$LL.paperless.customFields.noCompatibleFields()}</small>
							{:else}
								<small class="hint">{$LL.paperless.customFields.useDefault({ name: PAPERLESS_DEFAULT_FIELD_NAMES.datetime })}</small>
							{/if}
						</div>

						<div class="form-group">
							<label for="paperless-field-liters">{$LL.paperless.customFields.liters()}</label>
							<select
								id="paperless-field-liters"
								data-test="paperless-field-liters"
								bind:value={paperlessFieldLiters}
								on:change={savePaperlessFieldNamesNow}
								disabled={litersFields.length === 0}
							>
								<option value="">{$LL.paperless.customFields.useDefault({ name: PAPERLESS_DEFAULT_FIELD_NAMES.liters })}</option>
								{#each litersFields as field (field.id)}
									<option value={field.name}>{field.name} ({field.dataType})</option>
								{/each}
							</select>
							{#if litersFields.length === 0}
								<small class="hint">{$LL.paperless.customFields.noCompatibleFields()}</small>
							{:else}
								<small class="hint">{$LL.paperless.customFields.useDefault({ name: PAPERLESS_DEFAULT_FIELD_NAMES.liters })}</small>
							{/if}
						</div>

						<div class="form-group">
							<label for="paperless-field-total">{$LL.paperless.customFields.total()}</label>
							<select
								id="paperless-field-total"
								data-test="paperless-field-total"
								bind:value={paperlessFieldTotal}
								on:change={savePaperlessFieldNamesNow}
								disabled={totalFields.length === 0}
							>
								<option value="">{$LL.paperless.customFields.useDefault({ name: PAPERLESS_DEFAULT_FIELD_NAMES.total })}</option>
								{#each totalFields as field (field.id)}
									<option value={field.name}>{field.name} ({field.dataType})</option>
								{/each}
							</select>
							{#if totalFields.length === 0}
								<small class="hint">{$LL.paperless.customFields.noCompatibleFields()}</small>
							{:else}
								<small class="hint">{$LL.paperless.customFields.useDefault({ name: PAPERLESS_DEFAULT_FIELD_NAMES.total })}</small>
							{/if}
						</div>
					{/if}
				{/if}

				{#if paperlessConnectionStatus !== PAPERLESS_STATUS.IDLE}
					<div
						data-test="paperless-status"
						class="ha-status"
						class:connected={paperlessConnectionStatus === PAPERLESS_STATUS.CONNECTED}
						class:disconnected={paperlessConnectionStatus === PAPERLESS_STATUS.DISCONNECTED}
						class:testing={paperlessConnectionStatus === PAPERLESS_STATUS.TESTING}
					>
						{#if paperlessConnectionStatus === PAPERLESS_STATUS.TESTING}
							<span class="status-icon">⏳</span>
							<span>{$LL.paperless.status.testing()}</span>
						{:else if paperlessConnectionStatus === PAPERLESS_STATUS.CONNECTED}
							<span class="status-icon">✓</span>
							<span>{$LL.paperless.status.connected()}</span>
						{:else}
							<span class="status-icon">✗</span>
							<span>{$LL.paperless.status.disconnected()}</span>
						{/if}
					</div>
				{/if}
			</div>
		</section>

		<!-- Version Section -->
		<section class="settings-section" id="app-version">
			<h2>{$LL.update.currentVersion()}</h2>
			<div class="section-content">
				<div class="db-path-display">
					<span class="version-display">
						<span class="version-number" data-test="app-version">{appVersion || '...'}</span>
					</span>
					<span class="version-actions">
						<a
							class="link-btn"
							href="https://github.com/mcsdodo/kniha-jazd/blob/main/CHANGELOG.md"
							target="_blank"
							rel="noopener noreferrer"
							title={$LL.update.showChangelog()}
						>
							{$LL.update.showChangelog()}
						</a>
					</span>
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
									{#if vehicleOdoValues.has(vehicle.id)}
										<span class="badge ha-odo" title="Home Assistant ODO">
											🚗 {vehicleOdoValues.get(vehicle.id)?.toLocaleString('sk-SK')} km
										</span>
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
						on:input={debouncedSaveCompanySettings}
						on:blur={saveCompanySettingsNow}
					/>
				</div>

				<div class="form-group">
					<label for="company-ico">{$LL.settings.companyIco()}</label>
					<input
						type="text"
						id="company-ico"
						bind:value={companyIco}
						placeholder={$LL.settings.companyIcoPlaceholder()}
						on:input={debouncedSaveCompanySettings}
						on:blur={saveCompanySettingsNow}
					/>
				</div>

				<div class="form-group">
					<label for="buffer-purpose">{$LL.settings.bufferTripPurpose()}</label>
					<input
						type="text"
						id="buffer-purpose"
						bind:value={bufferTripPurpose}
						placeholder={$LL.settings.bufferTripPurposePlaceholder()}
						on:input={debouncedSaveCompanySettings}
						on:blur={saveCompanySettingsNow}
					/>
					<small class="hint">
						{$LL.settings.bufferTripPurposeHint()}
					</small>
				</div>

				<div class="form-group">
					<label class="checkbox-label">
						<input
							type="checkbox"
							checked={inferTripTimes}
							on:change={handleInferTripTimesChange}
						/>
						{$LL.settings.inferTripTimesLabel()}
					</label>
					<small class="hint">
						{$LL.settings.inferTripTimesDescription()}
					</small>
				</div>
			</div>
		</section>

		<!-- Backup Section -->
		<section class="settings-section">
			<h2>{$LL.settings.backupSection()}</h2>
			<div class="section-content">
				<button class="button" on:click={handleCreateBackup} disabled={backupInProgress}>
					{backupInProgress ? $LL.settings.creatingBackup() : $LL.settings.createBackup()}
				</button>

				<!-- Retention Settings -->
				<div class="retention-settings" data-testid="retention-settings">
					<h4>{$LL.backup.retention.title()}</h4>
					<label class="checkbox-label">
						<input
							type="checkbox"
							bind:checked={retentionEnabled}
							on:change={handleRetentionChange}
							data-testid="retention-enabled"
						/>
						{$LL.backup.retention.enabled()}
						<select
							bind:value={retentionKeepCount}
							data-testid="retention-keep-count"
							on:change={handleRetentionChange}
							disabled={!retentionEnabled}
						>
							<option value={3}>3</option>
							<option value={5}>5</option>
							<option value={10}>10</option>
						</select>
						{$LL.backup.retention.backups()}
					</label>

					{#if retentionEnabled && cleanupPreview}
						<div class="cleanup-preview">
							{#if cleanupPreview.toDelete.length > 0}
								<span>{$LL.backup.retention.toDelete({
									count: cleanupPreview.toDelete.length,
									size: formatFileSize(cleanupPreview.totalBytes)
								})}</span>
								<button
									class="button-small"
									on:click={handleCleanupNow}
									disabled={cleaningUp}
								>
									{cleaningUp ? '...' : $LL.backup.retention.cleanNow()}
								</button>
							{:else}
								<span class="placeholder">{$LL.backup.retention.nothingToClean()}</span>
							{/if}
						</div>
					{/if}
				</div>

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
									{#if backup.backupType === 'pre-update' && backup.updateVersion}
										<span class="backup-badge">{$LL.backup.badge.preUpdate({ version: backup.updateVersion })}</span>
									{/if}
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

{#if pinPromptFor}
	<!-- Reveal PIN prompt (server mode only; the desktop path is trusted).
	     Shown on EVERY reveal — nothing about the PIN is remembered. -->
	<div class="modal-overlay" data-test="reveal-pin-modal">
		<div class="modal">
			<h3>{$LL.settings.revealPinTitle()}</h3>
			<p class="hint">{$LL.settings.revealPinDescription()}</p>
			<form on:submit|preventDefault={submitPin}>
				<label for="reveal-pin">{$LL.settings.revealPinLabel()}</label>
				<!-- svelte-ignore a11y-autofocus -->
				<input
					type="password"
					id="reveal-pin"
					data-test="reveal-pin-input"
					class="monospace-input"
					bind:value={pinInput}
					autocomplete="off"
					autofocus
				/>
				{#if pinError}
					<small class="error-text" data-test="reveal-pin-error">{pinError}</small>
				{/if}
				<div class="modal-actions">
					<button type="button" class="button-secondary" on:click={cancelPin}>
						{$LL.settings.revealPinCancel()}
					</button>
					<button type="submit" data-test="reveal-pin-submit" disabled={pinBusy}>
						{$LL.settings.revealPinSubmit()}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

{#if showVehicleModal}
	<VehicleModal
		vehicle={editingVehicle}
		hasTrips={editingVehicle ? vehiclesWithTrips.has(editingVehicle.id) : false}
		haConfigured={!!(haUrl && haHasToken)}
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
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header h1 {
		margin: 0;
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

	.badge.ha-odo {
		background-color: var(--color-success-bg, #dcfce7);
		color: var(--color-success, #16a34a);
		font-weight: 600;
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

	.badge.custom {
		background-color: var(--btn-primary-light-bg);
		color: var(--btn-primary-light-color);
	}

	.badge.default {
		background-color: var(--bg-surface-alt);
		color: var(--text-secondary);
	}

	.db-path-display {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		background: var(--bg-surface-alt);
		border-radius: 4px;
		border: 1px solid var(--border-default);
	}

	.link-btn {
		background: none;
		border: none;
		color: var(--accent-primary);
		cursor: pointer;
		font-size: 0.875rem;
		padding: 0;
		text-decoration: underline;
		white-space: nowrap;
	}

	.link-btn:hover {
		color: var(--accent-primary-hover, var(--accent-primary));
		text-decoration: none;
	}

	.link-btn:disabled {
		color: var(--text-muted);
		cursor: not-allowed;
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

	.backup-badge {
		font-size: 0.75rem;
		padding: 0.125rem 0.5rem;
		background: var(--accent-light, #e3f2fd);
		color: var(--accent, #1976d2);
		border-radius: 4px;
	}

	.retention-settings {
		margin-top: 1rem;
		padding: 1rem;
		background: var(--bg-surface-alt);
		border-radius: 8px;
		border: 1px solid var(--border-default);
	}

	.retention-settings h4 {
		margin: 0 0 0.75rem 0;
		font-size: 0.9rem;
		color: var(--text-secondary);
	}

	.retention-settings select {
		margin: 0 0.5rem;
		padding: 0.25rem 0.5rem;
	}

	.cleanup-preview {
		margin-top: 0.75rem;
		display: flex;
		align-items: center;
		gap: 1rem;
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

	.theme-fieldset {
		border: none;
		padding: 0;
		margin: 0;
	}

	.theme-fieldset legend {
		font-weight: 600;
		color: var(--text-primary);
		margin-bottom: 0.5rem;
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

	.update-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: var(--bg-surface-alt);
		border-radius: 4px;
		margin-bottom: 1rem;
	}

	.version-display {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
	}

	.version-number {
		font-family: monospace;
		font-size: 0.875rem;
		color: var(--text-primary);
	}

	.version-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		color: var(--text-primary);
	}

	.checkbox-label input[type="checkbox"] {
		width: 18px;
		height: 18px;
		cursor: pointer;
	}

	.input-with-icon {
		position: relative;
		display: flex;
		align-items: center;
	}

	.input-with-icon input {
		flex: 1;
		padding: 0.75rem;
		padding-right: 2.5rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 0.875rem;
		background-color: var(--input-bg);
		color: var(--text-primary);
	}

	.input-with-icon input:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	.monospace-input {
		font-family: var(--font-mono);
	}

	/* Env-var managed fields: read-only, and visibly so. The badge names the
	   variable so the reader knows where to change the value instead. */
	.env-badge {
		display: inline-block;
		margin-left: 0.5rem;
		padding: 0.05rem 0.35rem;
		border: 1px solid var(--border-color);
		border-radius: 3px;
		background: var(--bg-secondary);
		color: var(--text-secondary);
		font-family: var(--font-mono);
		font-size: 0.65rem;
		font-weight: 500;
		letter-spacing: 0.02em;
		vertical-align: middle;
	}

	.section-content input:disabled {
		background: var(--bg-secondary);
		color: var(--text-secondary);
		cursor: not-allowed;
	}

	.icon-btn {
		position: absolute;
		right: 0.5rem;
		background: none;
		border: none;
		padding: 0.25rem;
		cursor: pointer;
		color: var(--text-secondary);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.icon-btn:hover {
		color: var(--text-primary);
	}

	.icon-btn svg {
		display: block;
	}

	/* Error states */
	.input-error {
		border-color: var(--color-error, #dc2626) !important;
	}

	.custom-fields-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.custom-fields-header h3 {
		margin: 0;
	}

	.error-text {
		color: var(--color-error, #dc2626);
		font-size: 0.85rem;
	}

	/* Home Assistant connection status */
	.ha-status {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		margin-top: 0.5rem;
	}

	.ha-status.testing {
		background: var(--bg-surface-alt, #f5f5f5);
		color: var(--text-secondary);
	}

	.ha-status.connected {
		background: var(--color-success-bg, #dcfce7);
		color: var(--color-success, #16a34a);
	}

	.ha-status.disconnected {
		background: var(--color-error-bg, #fee2e2);
		color: var(--color-error, #dc2626);
	}

	.ha-status .status-icon {
		font-size: 1rem;
	}
</style>
