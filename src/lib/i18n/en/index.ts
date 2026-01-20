import type { Translation } from '../i18n-types';

const en = {
	// Common
	common: {
		save: 'Save',
		cancel: 'Cancel',
		delete: 'Delete',
		edit: 'Edit',
		add: 'Add',
		close: 'Close',
		confirm: 'Confirm',
		loading: 'Loading...',
		noData: 'No data',
		yes: 'Yes',
		no: 'No',
	},

	// App header & navigation
	app: {
		title: 'Trip Logbook',
		nav: {
			logbook: 'Logbook',
			receipts: 'Receipts',
			settings: 'Settings',
		},
		vehicleLabel: 'Vehicle:',
		vehiclePlaceholder: '-- Select vehicle --',
		noVehicles: '-- No vehicles --',
		yearLabel: 'Year:',
		resetWindowSize: 'Optimal window size',
	},

	// Home page
	home: {
		activeVehicle: 'Active vehicle',
		exportForPrint: 'Export for print',
		exporting: 'Exporting...',
		noVehicle: 'No vehicle',
		noVehicleDescription: 'Please select a vehicle from the main menu or create one in settings.',
		goToSettings: 'Go to settings',
	},

	// Stats
	stats: {
		totalDriven: 'Total driven',
		fuel: 'Fuel',
		consumption: 'Consumption',
		deviation: 'Deviation',
		remaining: 'Remaining',
	},

	// Vehicle info
	vehicle: {
		name: 'Name',
		licensePlate: 'License plate',
		tankSize: 'Tank size',
		tpConsumption: 'Consumption (TP)',
		initialOdometer: 'Initial ODO',
		active: 'Active',
		setAsActive: 'Set as active',
		// Vehicle types
		vehicleType: 'Vehicle type',
		vehicleTypeIce: 'Combustion (ICE)',
		vehicleTypeBev: 'Electric (BEV)',
		vehicleTypePhev: 'Plug-in hybrid (PHEV)',
		// Battery fields
		batteryCapacity: 'Battery capacity (kWh)',
		baselineConsumption: 'Baseline consumption (kWh/100km)',
		initialBatteryPercent: 'Initial battery state (%)',
		// Type change warning
		typeChangeBlocked: 'Vehicle type cannot be changed after trips are recorded',
	},

	// Vehicle modal
	vehicleModal: {
		addTitle: 'Add vehicle',
		editTitle: 'Edit vehicle',
		namePlaceholder: 'e.g. Škoda Octavia',
		licensePlatePlaceholder: 'e.g. BA123XY',
		tankSizePlaceholder: 'e.g. 66',
		tpConsumptionPlaceholder: 'e.g. 5.1',
		initialOdometerPlaceholder: 'e.g. 50000',
		batteryCapacityPlaceholder: 'e.g. 75',
		baselineConsumptionPlaceholder: 'e.g. 18',
		initialBatteryPlaceholder: 'e.g. 100',
		nameLabel: 'Vehicle name',
		licensePlateLabel: 'License plate',
		tankSizeLabel: 'Tank size (liters)',
		tpConsumptionLabel: 'TP consumption (l/100km)',
		initialOdometerLabel: 'Initial ODO (km)',
		vinLabel: 'VIN',
		vinPlaceholder: 'e.g. ABC123456789',
		driverLabel: 'Driver',
		driverPlaceholder: 'e.g. John Doe',
		vehicleTypeLabel: 'Vehicle type',
		batteryCapacityLabel: 'Battery capacity (kWh)',
		baselineConsumptionLabel: 'Baseline consumption (kWh/100km)',
		initialBatteryLabel: 'Initial battery state (%)',
		// Section headers
		fuelSection: 'Fuel',
		batterySection: 'Battery',
	},

	// Trip grid
	trips: {
		title: 'Trips',
		count: '({count})',
		newRecord: 'New record',
		firstRecord: 'First record',
		emptyState: 'No records. Click "New record" to add a trip.',
		// Column headers
		columns: {
			date: 'Date',
			origin: 'From',
			destination: 'To',
			km: 'Km',
			odo: 'ODO',
			purpose: 'Purpose',
			fuelLiters: 'Fuel (L)',
			fuelCost: 'Cost €',
			consumptionRate: 'l/100km',
			remaining: 'Remaining',
			otherCosts: 'Other €',
			otherCostsNote: 'Other note',
			actions: 'Actions',
			// Energy columns (BEV/PHEV)
			energyKwh: 'Energy (kWh)',
			energyCost: 'Charge cost €',
			energyRate: 'kWh/100km',
			batteryRemaining: 'Battery',
			batteryPercent: 'Battery %',
		},
		// Placeholders
		originPlaceholder: 'From',
		destinationPlaceholder: 'To',
		purposePlaceholder: 'Purpose',
		// Actions
		moveUp: 'Move up',
		moveDown: 'Move down',
		insertAbove: 'Insert record above',
		deleteRecord: 'Delete record',
		// Checkbox
		fullTank: 'Full',
		fullCharge: 'Full charge',
		// SoC override
		socOverride: 'Battery state correction (%)',
		socOverrideHint: 'Manually set battery state to correct deviations',
		socOverrideIndicator: 'Manual SoC correction',
		// Tooltips/indicators
		partialFillup: 'Partial fillup',
		partialCharge: 'Partial charge',
		noReceipt: 'No receipt',
		estimatedRate: 'Estimated from TP',
		estimatedEnergyRate: 'Estimated from baseline consumption',
		// Legend
		legend: {
			partialFillup: 'partial fillup',
			noReceipt: 'no receipt',
			highConsumption: 'high consumption',
		},
	},

	// Compensation banner
	compensation: {
		title: 'Legal consumption limit exceeded',
		currentDeviation: 'Current deviation: {percent}% (limit: 20%)',
		additionalKmNeeded: 'Additional km needed: {km} km',
		searchingSuggestion: 'Searching for a suitable trip suggestion...',
		suggestionTitle: 'Compensation trip suggestion:',
		origin: 'Start:',
		destination: 'Destination:',
		distance: 'Distance:',
		purpose: 'Purpose:',
		bufferNote: 'Note: This is a compensation trip (same start and destination)',
		addTrip: 'Add trip',
		adding: 'Adding...',
	},

	// Settings page
	settings: {
		title: 'Settings',
		// Vehicles section
		vehiclesSection: 'Vehicles',
		noVehicles: 'No vehicles. Create your first vehicle.',
		addVehicle: '+ Add vehicle',
		// Company section
		companySection: 'Company settings',
		companyName: 'Company name',
		companyNamePlaceholder: 'e.g. My Company Ltd.',
		companyIco: 'Company ID',
		companyIcoPlaceholder: 'e.g. 12345678',
		bufferTripPurpose: 'Compensation trip purpose',
		bufferTripPurposePlaceholder: 'e.g. business trip',
		bufferTripPurposeHint: 'This purpose will be used when planning trips to stay within the 20% consumption limit.',
		saveSettings: 'Save settings',
		// Backup section
		backupSection: 'Database backup',
		createBackup: 'Backup',
		creatingBackup: 'Creating backup...',
		availableBackups: 'Available backups',
		noBackups: 'No backups. Create your first backup.',
		restore: 'Restore',
		revealWindows: 'Show in Explorer',
		revealMac: 'Reveal in Finder',
		revealLinux: 'Show in Files',
		// Language section
		languageSection: 'Language',
		language: 'Application language',
		// Appearance section
		appearanceSection: 'Appearance',
		themeLabel: 'Theme',
		themeSystem: 'System default',
		themeLight: 'Light',
		themeDark: 'Dark',
		// Receipt scanning section
		receiptScanningSection: 'Receipt Scanning',
		geminiApiKey: 'Gemini API Key',
		geminiApiKeyPlaceholder: 'Enter API key',
		geminiApiKeyHint: 'API key from Google AI Studio for receipt recognition.',
		receiptsFolder: 'Receipts Folder',
		receiptsFolderPlaceholder: 'Select folder',
		receiptsFolderHint: 'Folder where receipt photos are stored.',
		receiptsFolderChange: 'Change',
		receiptsFolderNotSet: 'Not set',
		browseFolder: 'Browse',
		showApiKey: 'Show',
		hideApiKey: 'Hide',
		receiptSettingsSaved: 'Receipt settings saved',
		// Database location section
		dbLocationSection: 'Database Location',
		dbLocationCurrent: 'Current Location',
		dbLocationCustom: 'Custom Path',
		dbLocationDefault: 'Change',
		dbLocationChange: 'Change Location...',
		dbLocationResetToDefault: 'Reset to Default',
		dbLocationOpenFolder: 'Open Folder',
		dbLocationSelectFolder: 'Select destination folder for database',
		dbLocationHint: 'You can move the database to Google Drive, NAS, or another shared folder for multi-PC usage.',
		dbLocationMoving: 'Moving database...',
		dbLocationMoved: 'Database moved successfully. Application will restart.',
		dbLocationReset: 'Database moved back to default location. Application will restart.',
		dbLocationTargetHasDb: 'Target folder already contains a database. Please select a different folder.',
		dbLocationConfirmTitle: 'Move Database',
		dbLocationConfirmMessage: 'The database and backups will be moved to:',
		dbLocationConfirmWarning: 'The application will restart after the move.',
		dbLocationConfirmMove: 'Move',
		// Read-only mode
		readOnlyBanner: 'Database was updated by a newer app version. Read-only mode.',
		readOnlyCheckUpdates: 'Check for Updates',
	},

	// Backup modals
	backup: {
		confirmRestoreTitle: 'Confirm restore',
		backupDate: 'Backup date:',
		backupSize: 'Size:',
		backupContains: 'Contains:',
		vehiclesAndTrips: '{vehicles} vehicles, {trips} trips',
		restoreWarning: 'Current data will be overwritten! If you want to preserve the current state, create a backup first.',
		restoreBackup: 'Restore backup',
		confirmDeleteTitle: 'Confirm deletion',
		deleteWarning: 'This backup will be permanently deleted!',
	},

	// Confirm dialogs
	confirm: {
		deleteVehicleTitle: 'Delete vehicle',
		deleteVehicleMessage: 'Are you sure you want to delete vehicle "{name}"?',
		deleteRecordTitle: 'Delete record',
		deleteRecordMessage: 'Are you sure you want to delete this record?',
		deleteReceiptTitle: 'Delete receipt',
		deleteReceiptMessage: 'Are you sure you want to delete receipt "{name}"?',
	},

	// Receipts page
	receipts: {
		title: 'Receipts',
		// Scan button
		scanFolder: 'Scan folder',
		scanning: 'Scanning...',
		// OCR button
		recognizeData: 'Recognize data',
		recognizing: 'Recognizing {current}/{total}...',
		// Legacy (kept for compatibility)
		sync: 'Sync',
		syncing: 'Syncing...',
		processPending: 'Process pending ({count})',
		processing: 'Processing...',
		processingProgress: 'Processing {current}/{total}...',
		// Config warning
		notConfigured: 'Receipts feature is not configured.',
		configurePrompt: 'Create a file named',
		configurePromptFile: 'local.settings.json',
		configurePromptSuffix: 'with the following content:',
		configNote: 'Note: On Windows, use double backslashes (\\\\) in paths.',
		openConfigFolder: 'Open folder',
		// Not configured (simplified)
		notConfiguredTitle: 'Receipt scanning is not configured',
		notConfiguredDescription: 'To use this feature you need to:',
		notConfiguredApiKey: 'Set up Gemini API key (from Google AI Studio)',
		notConfiguredFolder: 'Select a folder with receipts',
		goToSettings: 'Go to settings',
		// Folder structure warnings
		folderStructureWarning: 'Invalid folder structure',
		folderStructureHint: 'Folder must contain either only files, or only folders named with years (2024, 2025, ...)',
		// Date mismatch warning
		dateMismatch: 'Receipt date ({receiptYear}) does not match folder ({folderYear})',
		// Filters
		filterAll: 'All',
		filterUnassigned: 'Unverified',
		filterNeedsReview: 'Needs review',
		filterFuel: 'Fuel',
		filterOther: 'Other costs',
		// Verification summary
		allVerified: '{count}/{total} receipts verified',
		verified: '{count}/{total} verified',
		unverified: '{count} unverified',
		// Receipt details
		date: 'Date:',
		liters: 'Liters:',
		price: 'Price:',
		station: 'Station:',
		trip: 'Trip:',
		// Confidence
		confidenceHigh: 'High confidence',
		confidenceMedium: 'Medium confidence',
		confidenceLow: 'Low confidence',
		confidenceUnknown: 'Unknown confidence',
		// Status badges
		statusVerified: 'Verified',
		statusNeedsReview: 'Needs review',
		statusUnverified: 'Unverified',
		// Mismatch reasons
		mismatchMissingData: 'Receipt data missing',
		mismatchNoFuelTrip: 'No trip with fuel data',
		mismatchDate: 'Date {receiptDate} – trip is {tripDate}',
		mismatchLiters: '{receiptLiters} L – trip has {tripLiters} L',
		mismatchPrice: '{receiptPrice} € – trip has {tripPrice} €',
		mismatchNoOtherCost: 'No trip with this price',
		// Actions
		open: 'Open',
		reprocess: 'Reprocess',
		reprocessing: 'Processing...',
		assignToTrip: 'Assign to trip',
		// Other costs
		otherCost: 'Other costs',
		vendor: 'Vendor:',
		description: 'Description:',
		assignmentBlocked: 'Trip already has other costs',
		// Empty state
		noReceipts: 'No receipts. Click Sync to load new ones.',
	},

	// Trip selector modal
	tripSelector: {
		title: 'Assign receipt to trip',
		noVehicleSelected: 'No vehicle selected',
		loadingTrips: 'Loading trips...',
		loadError: 'Failed to load trips',
		noTrips: 'No trips to assign.',
		alreadyHas: 'already has:',
		matchesReceipt: 'matches receipt',
		// Mismatch reasons
		mismatchDate: 'different date',
		mismatchLiters: 'different liters',
		mismatchPrice: 'different price',
		mismatchLitersAndPrice: 'different liters & price',
		mismatchDateAndLiters: 'different date & liters',
		mismatchDateAndPrice: 'different date & price',
		mismatchAll: 'all data differs',
	},

	// Toast messages
	toast: {
		// Success
		vehicleSaved: 'Vehicle saved successfully',
		vehicleDeleted: 'Vehicle deleted',
		settingsSaved: 'Settings saved successfully',
		backupCreated: 'Backup created successfully',
		backupRestored: 'Backup restored successfully. The app will restart.',
		backupDeleted: 'Backup deleted',
		receiptDeleted: 'Receipt deleted',
		receiptReprocessed: 'Receipt "{name}" reprocessed',
		receiptAssigned: 'Receipt assigned to trip',
		receiptsLoaded: 'Loaded {count} new receipts',
		receiptsLoadedWithErrors: 'Loaded {count} receipts ({errors} errors)',
		receiptsProcessed: 'Processed {count} receipts',
		receiptsProcessedWithErrors: 'Processed {count} receipts ({errors} errors)',
		foundNewReceipts: 'Found {count} new files',
		noNewReceipts: 'No new files',
		noPendingReceipts: 'No pending receipts',
		// Errors
		errorSaveVehicle: 'Failed to save vehicle: {error}',
		errorDeleteVehicle: 'Failed to delete vehicle: {error}',
		errorSetActiveVehicle: 'Failed to set active vehicle: {error}',
		errorSaveSettings: 'Failed to save settings: {error}',
		errorCreateBackup: 'Failed to create backup: {error}',
		errorGetBackupInfo: 'Failed to load backup info: {error}',
		errorRestoreBackup: 'Failed to restore backup: {error}',
		errorDeleteBackup: 'Failed to delete backup: {error}',
		errorMoveDatabase: 'Failed to move database: {error}',
		errorResetDatabase: 'Failed to reset database location: {error}',
		errorLoadReceipts: 'Failed to load receipts',
		errorSyncReceipts: 'Failed to sync: {error}',
		errorProcessReceipts: 'Failed to process: {error}',
		errorDeleteReceipt: 'Failed to delete receipt',
		errorReprocessReceipt: 'Failed to process "{name}": {error}',
		errorAssignReceipt: 'Failed to assign receipt: {error}',
		errorOpenFile: 'Failed to open file',
		errorCreateTrip: 'Failed to create record',
		errorUpdateTrip: 'Failed to update record',
		errorDeleteTrip: 'Failed to delete record',
		errorMoveTrip: 'Failed to move record',
		errorAddCompensationTrip: 'Failed to add trip. Please try again.',
		errorExport: 'Export failed: {error}',
		errorSelectVehicleFirst: 'Please select a vehicle first',
		errorSetApiKeyFirst: 'Set folder and API key in Settings first',
		errorSetApiKeyOnlyFirst: 'Set API key in Settings first',
	},

	// PDF export labels (passed to Rust)
	export: {
		// Page title
		pageTitle: 'TRIP LOGBOOK',
		// Header labels
		headerCompany: 'Company:',
		headerIco: 'ID:',
		headerVehicle: 'Vehicle:',
		headerLicensePlate: 'Plate:',
		headerTankSize: 'Tank:',
		headerTpConsumption: 'TP consumption:',
		headerYear: 'Year:',
		// Header labels for BEV
		headerBatteryCapacity: 'Battery:',
		headerBaselineConsumption: 'Baseline cons.:',
		// VIN and Driver
		headerVin: 'VIN:',
		headerDriver: 'Driver:',
		// Column headers
		colDate: 'Date',
		colOrigin: 'From',
		colDestination: 'To',
		colPurpose: 'Purpose',
		colKm: 'Km',
		colOdo: 'ODO',
		colFuelLiters: 'Fuel L',
		colFuelCost: '€ Fuel',
		colOtherCosts: '€ Other',
		colNote: 'Note',
		colRemaining: 'Rem.',
		colConsumption: 'Cons.',
		// Column headers for BEV
		colEnergyKwh: 'kWh',
		colEnergyCost: '€ Energy',
		colBatteryRemaining: 'Battery',
		colEnergyRate: 'kWh/100',
		// Footer labels
		footerTotalKm: 'Total km',
		footerTotalFuel: 'Total fuel',
		footerOtherCosts: 'Other costs',
		footerAvgConsumption: 'Average consumption',
		footerDeviation: 'Deviation from TP',
		footerTpNorm: 'TP norm',
		// Footer labels for BEV
		footerTotalEnergy: 'Total energy',
		footerAvgEnergyRate: 'Average consumption',
		footerBaselineNorm: 'Baseline norm',
		// Print hint
		printHint: 'To export to PDF use Ctrl+P → Save as PDF',
	},

	// Update section
	update: {
		sectionTitle: 'Updates',
		currentVersion: 'Version',
		checkForUpdates: 'Check for updates',
		updateNow: 'Update Now',
		later: 'Later',
		checking: 'Checking...',
		upToDate: 'App is up to date',
		available: 'Update available',
		availableVersion: 'Version {version} available',
		downloading: 'Downloading...',
		downloadProgress: 'Downloading: {percent}%',
		installing: 'Installing...',
		modalTitle: 'Update Available',
		modalBody: 'Version {version} is ready to install.',
		releaseNotes: "What's new:",
		errorChecking: 'Failed to check for updates',
		errorNetwork: 'Check your internet connection',
		errorDownloadInterrupted: 'Download interrupted, try again',
		errorSignature: 'Invalid update signature',
		errorDiskSpace: 'Insufficient disk space',
		errorServerUnavailable: 'Update server unavailable',
		retry: 'Retry',
		buttonUpdate: 'Update',
		buttonLater: 'Later',
		autoCheckOnStart: 'Automatically check on startup',
		showChangelog: 'Show changelog',
	},
} satisfies Translation;

export default en;
