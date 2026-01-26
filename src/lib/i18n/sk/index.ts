import type { BaseTranslation } from '../i18n-types';

const sk = {
	// Common
	common: {
		save: 'Uložiť',
		cancel: 'Zrušiť',
		delete: 'Odstrániť',
		edit: 'Upraviť',
		add: 'Pridať',
		close: 'Zavrieť',
		confirm: 'Potvrdiť',
		loading: 'Načítavam...',
		noData: 'Žiadne dáta',
		yes: 'Áno',
		no: 'Nie',
	},

	// App header & navigation
	app: {
		title: 'Kniha Jázd',
		nav: {
			logbook: 'Kniha jázd',
			receipts: 'Doklady',
			settings: 'Nastavenia',
		},
		vehicleLabel: 'Vozidlo:',
		vehiclePlaceholder: '-- Vyberte vozidlo --',
		noVehicles: '-- Žiadne vozidlá --',
		yearLabel: 'Rok:',
		resetWindowSize: 'Optimálna veľkosť okna',
	},

	// Home page
	home: {
		activeVehicle: 'Aktívne vozidlo',
		exportForPrint: 'Export pre tlač',
		exporting: 'Exportujem...',
		noVehicle: 'Žiadne vozidlo',
		noVehicleDescription: 'Prosím, vyberte vozidlo z hlavného menu alebo ho vytvorte v nastaveniach.',
		goToSettings: 'Prejsť do nastavení',
	},

	// Stats
	stats: {
		totalDriven: 'Celkovo najazdené',
		fuel: 'PHM',
		consumption: 'Spotreba',
		deviation: 'Odchýlka',
		remaining: 'Zostatok',
	},

	// Vehicle info
	vehicle: {
		name: 'Názov',
		licensePlate: 'ŠPZ',
		tankSize: 'Objem nádrže',
		tpConsumption: 'Spotreba (TP)',
		initialOdometer: 'Počiatočný stav ODO',
		active: 'Aktívne',
		setAsActive: 'Nastaviť ako aktívne',
		// Vehicle types
		vehicleType: 'Typ vozidla',
		vehicleTypeIce: 'Spaľovacie (ICE)',
		vehicleTypeBev: 'Elektrické (BEV)',
		vehicleTypePhev: 'Plug-in hybrid (PHEV)',
		// Battery fields
		batteryCapacity: 'Kapacita batérie (kWh)',
		baselineConsumption: 'Základná spotreba (kWh/100km)',
		initialBatteryPercent: 'Počiatočný stav batérie (%)',
		// Type change warning
		typeChangeBlocked: 'Typ vozidla nie je možné zmeniť po vytvorení záznamov',
	},

	// Vehicle modal
	vehicleModal: {
		addTitle: 'Pridať vozidlo',
		editTitle: 'Upraviť vozidlo',
		namePlaceholder: 'napr. Škoda Octavia',
		licensePlatePlaceholder: 'napr. BA123XY',
		tankSizePlaceholder: 'napr. 66',
		tpConsumptionPlaceholder: 'napr. 5.1',
		initialOdometerPlaceholder: 'napr. 50000',
		batteryCapacityPlaceholder: 'napr. 75',
		baselineConsumptionPlaceholder: 'napr. 18',
		initialBatteryPlaceholder: 'napr. 100',
		nameLabel: 'Názov vozidla',
		licensePlateLabel: 'Evidenčné číslo (EČV)',
		tankSizeLabel: 'Objem nádrže (litre)',
		tpConsumptionLabel: 'Spotreba z TP (l/100km)',
		initialOdometerLabel: 'Počiatočný stav ODO (km)',
		vinLabel: 'VIN',
		vinPlaceholder: 'napr. ABC123456789',
		driverLabel: 'Vodič',
		driverPlaceholder: 'napr. Janko Hraško',
		vehicleTypeLabel: 'Typ vozidla',
		batteryCapacityLabel: 'Kapacita batérie (kWh)',
		baselineConsumptionLabel: 'Základná spotreba (kWh/100km)',
		initialBatteryLabel: 'Počiatočný stav batérie (%)',
		// Section headers
		fuelSection: 'Palivo',
		batterySection: 'Batéria',
	},

	// Trip grid
	trips: {
		title: 'Jazdy',
		count: '({count:number})',
		newRecord: 'Nový záznam',
		firstRecord: 'Prvý záznam',
		// Date prefill toggle
		datePrefillPrevious: '+1 deň',
		datePrefillToday: 'Dnes',
		datePrefillTooltip: 'Predvyplnený dátum pre nový záznam',
		emptyState: 'Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.',
		// Column headers
		columns: {
			date: 'Dátum',
			time: 'Čas',
			origin: 'Odkiaľ',
			destination: 'Kam',
			km: 'Km',
			odo: 'ODO',
			purpose: 'Účel',
			fuelLiters: 'PHM (L)',
			fuelCost: 'Cena €',
			fuelConsumed: 'Spotr. (L)',
			consumptionRate: 'l/100km',
			remaining: 'Zostatok',
			otherCosts: 'Iné €',
			otherCostsNote: 'Iné pozn.',
			actions: 'Akcie',
			// Energy columns (BEV/PHEV)
			energyKwh: 'Energia (kWh)',
			energyCost: 'Cena nab. €',
			energyRate: 'kWh/100km',
			batteryRemaining: 'Batéria',
			batteryPercent: 'Batéria %',
		},
		// Placeholders
		originPlaceholder: 'Odkiaľ',
		destinationPlaceholder: 'Kam',
		purposePlaceholder: 'Účel',
		// Actions
		moveUp: 'Presunúť hore',
		moveDown: 'Presunúť dole',
		insertAbove: 'Vložiť záznam nad',
		deleteRecord: 'Odstrániť záznam',
		magicFill: 'Automatické doplnenie',
		// Checkbox
		fullTank: 'Plná',
		fullCharge: 'Plné nabitie',
		// SoC override
		socOverride: 'Korekcia stavu batérie (%)',
		socOverrideHint: 'Manuálne nastavenie stavu batérie pre korekciu odchýlok',
		socOverrideIndicator: 'Manuálna korekcia SoC',
		// Tooltips/indicators
		partialFillup: 'Čiastočné tankovanie',
		partialCharge: 'Čiastočné nabitie',
		noReceipt: 'Bez dokladu',
		estimatedRate: 'Odhad podľa TP',
		estimatedEnergyRate: 'Odhad podľa základnej spotreby',
		// Legend
		legend: {
			partialFillup: 'čiastočné tankovanie',
			noReceipt: 'bez dokladu',
			highConsumption: 'vysoká spotreba',
			suggestedFillup: 'Návrh tankovania: {liters} L → {rate} l/100km',
		},
		// Column visibility
		columnVisibility: {
			title: 'Stĺpce',
			time: 'Čas',
			fuelConsumed: 'Spotrebované (L)',
			fuelRemaining: 'Zostatok (L)',
			otherCosts: 'Iné (€)',
			otherCostsNote: 'Iná poznámka',
		},
	},

	// Compensation banner
	compensation: {
		title: 'Prekročený zákonný limit spotreby',
		currentDeviation: 'Aktuálna odchýlka: {percent:string}% (limit: 20%)',
		additionalKmNeeded: 'Potrebných dodatočných km: {km:string} km',
		searchingSuggestion: 'Hľadám vhodný návrh jazdy...',
		suggestionTitle: 'Návrh kompenzačnej jazdy:',
		origin: 'Začiatok:',
		destination: 'Cieľ:',
		distance: 'Vzdialenosť:',
		purpose: 'Účel:',
		bufferNote: 'Poznámka: Toto je kompenzačná jazda (rovnaká poloha začiatku a cieľa)',
		addTrip: 'Pridať jazdu',
		adding: 'Pridávam...',
	},

	// Settings page
	settings: {
		title: 'Nastavenia',
		// Vehicles section
		vehiclesSection: 'Vozidlá',
		noVehicles: 'Žiadne vozidlá. Vytvorte prvé vozidlo.',
		addVehicle: '+ Pridať vozidlo',
		// Company section
		companySection: 'Nastavenia spoločnosti',
		companyName: 'Názov spoločnosti',
		companyNamePlaceholder: 'napr. Moja firma s.r.o.',
		companyIco: 'IČO',
		companyIcoPlaceholder: 'napr. 12345678',
		bufferTripPurpose: 'Účel kompenzačnej jazdy',
		bufferTripPurposePlaceholder: 'napr. služobná cesta',
		bufferTripPurposeHint: 'Tento účel sa použije pri plánovaní jázd na dodržanie 20% limitu spotreby.',
		saveSettings: 'Uložiť nastavenia',
		// Backup section
		backupSection: 'Záloha databázy',
		createBackup: 'Zálohovať',
		creatingBackup: 'Vytváram zálohu...',
		availableBackups: 'Dostupné zálohy',
		noBackups: 'Žiadne zálohy. Vytvorte prvú zálohu.',
		restore: 'Obnoviť',
		revealWindows: 'Zobraziť v Prieskumníkovi',
		revealMac: 'Zobraziť vo Finderi',
		revealLinux: 'Zobraziť v Súboroch',
		// Language section
		languageSection: 'Jazyk',
		language: 'Jazyk aplikácie',
		// Appearance section
		appearanceSection: 'Vzhľad',
		themeLabel: 'Téma',
		themeSystem: 'Podľa systému',
		themeLight: 'Svetlá',
		themeDark: 'Tmavá',
		// Receipt scanning section
		receiptScanningSection: 'Skenovanie dokladov',
		geminiApiKey: 'Gemini API kľúč',
		geminiApiKeyPlaceholder: 'Zadajte API kľúč',
		geminiApiKeyHint: 'API kľúč z Google AI Studio pre rozpoznávanie dokladov.',
		receiptsFolder: 'Priečinok s dokladmi',
		receiptsFolderPlaceholder: 'Vyberte priečinok',
		receiptsFolderHint: 'Priečinok, kde sú uložené fotky dokladov.',
		receiptsFolderChange: 'Zmeniť',
		receiptsFolderNotSet: 'Nie je nastavený',
		browseFolder: 'Vybrať',
		showApiKey: 'Zobraziť',
		hideApiKey: 'Skryť',
		receiptSettingsSaved: 'Nastavenia dokladov boli uložené',
		// Database location section
		dbLocationSection: 'Umiestnenie databázy',
		dbLocationCurrent: 'Aktuálne umiestnenie',
		dbLocationCustom: 'Vlastná cesta',
		dbLocationDefault: 'Zmeniť',
		dbLocationChange: 'Zmeniť umiestnenie...',
		dbLocationResetToDefault: 'Obnoviť predvolené',
		dbLocationOpenFolder: 'Otvoriť priečinok',
		dbLocationSelectFolder: 'Vyberte cieľový priečinok pre databázu',
		dbLocationHint: 'Databázu môžete presunúť na Google Drive, NAS alebo iný zdieľaný priečinok pre použitie na viacerých PC.',
		dbLocationMoving: 'Presúvam databázu...',
		dbLocationMoved: 'Databáza bola úspešne presunutá. Aplikácia sa reštartuje.',
		dbLocationReset: 'Databáza bola presunutá späť do predvoleného umiestnenia. Aplikácia sa reštartuje.',
		dbLocationTargetHasDb: 'Cieľový priečinok už obsahuje databázu. Vyberte iný priečinok.',
		dbLocationConfirmTitle: 'Presunúť databázu',
		dbLocationConfirmMessage: 'Databáza a zálohy budú presunuté do:',
		dbLocationConfirmWarning: 'Aplikácia sa po presune reštartuje.',
		dbLocationConfirmMove: 'Presunúť',
		// Read-only mode
		readOnlyBanner: 'Databáza bola aktualizovaná novšou verziou aplikácie. Režim len na čítanie.',
		readOnlyCheckUpdates: 'Skontrolovať aktualizácie',
	},

	// Backup modals
	backup: {
		confirmRestoreTitle: 'Potvrdiť obnovenie',
		backupDate: 'Dátum zálohy:',
		backupSize: 'Veľkosť:',
		backupContains: 'Obsahuje:',
		vehiclesAndTrips: '{vehicles:number} vozidiel, {trips:number} jázd',
		restoreWarning: 'Aktuálne dáta budú prepísané! Ak si chcete zachovať aktuálny stav, vytvorte si najprv zálohu manuálne.',
		restoreBackup: 'Obnoviť zálohu',
		confirmDeleteTitle: 'Potvrdiť odstránenie',
		deleteWarning: 'Táto záloha bude trvalo odstránená!',
		// Retention settings
		retention: {
			title: 'Automatické čistenie',
			enabled: 'Ponechať iba posledných',
			backups: 'automatických záloh',
			toDelete: 'Na vymazanie: {count:number} záloh ({size:string})',
			cleanNow: 'Vyčistiť teraz',
			nothingToClean: 'Nič na vyčistenie',
		},
		// Badge for pre-update backups
		badge: {
			preUpdate: 'pred {version:string}',
		},
	},

	// Confirm dialogs
	confirm: {
		deleteVehicleTitle: 'Odstrániť vozidlo',
		deleteVehicleMessage: 'Naozaj chcete odstrániť vozidlo "{name:string}"?',
		deleteRecordTitle: 'Odstrániť záznam',
		deleteRecordMessage: 'Naozaj chcete odstrániť tento záznam?',
		deleteReceiptTitle: 'Odstrániť doklad',
		deleteReceiptMessage: 'Naozaj chcete odstrániť doklad "{name:string}"?',
	},

	// Receipts page (doklady)
	receipts: {
		title: 'Doklady',
		// Scan button
		scanFolder: 'Skenovať priečinok',
		scanning: 'Skenujem...',
		// OCR button
		recognizeData: 'Rozpoznať dáta',
		recognizing: 'Rozpoznávam {current:number}/{total:number}...',
		// Legacy (kept for compatibility)
		sync: 'Načítať',
		syncing: 'Synchronizujem...',
		processPending: 'Spracovať čakajúce ({count:number})',
		processing: 'Spracovávam...',
		processingProgress: 'Spracovávam {current:number}/{total:number}...',
		// Currency support
		currency: 'Mena',
		currencyEur: 'EUR (Euro)',
		currencyCzk: 'CZK (Česká koruna)',
		currencyHuf: 'HUF (Maďarský forint)',
		currencyPln: 'PLN (Poľský zlotý)',
		originalAmount: 'Pôvodná suma:',
		eurAmount: 'Suma v EUR:',
		needsConversion: 'Vyžaduje konverziu na EUR',
		convertedFrom: '{amount:number} {currency:string} →',
		// Config warning
		notConfigured: 'Funkcia dokladov nie je nakonfigurovaná.',
		configurePrompt: 'Vytvorte súbor s názvom',
		configurePromptFile: 'local.settings.json',
		configurePromptSuffix: 's nasledujúcim obsahom:',
		configNote: 'Poznámka: Na Windows používajte dvojité spätné lomky (\\\\) v cestách.',
		openConfigFolder: 'Otvoriť priečinok',
		// Not configured (simplified)
		notConfiguredTitle: 'Skenovanie dokladov nie je nakonfigurované',
		notConfiguredDescription: 'Pre používanie tejto funkcie potrebujete:',
		notConfiguredApiKey: 'Nastaviť Gemini API kľúč (z Google AI Studio)',
		notConfiguredFolder: 'Vybrať priečinok s dokladmi',
		goToSettings: 'Prejsť do nastavení',
		// Folder structure warnings
		folderStructureWarning: 'Neplatná štruktúra priečinka',
		folderStructureHint: 'Priečinok musí obsahovať buď len súbory, alebo len priečinky s názvami rokov (2024, 2025, ...)',
		// Date mismatch warning
		dateMismatch: 'Dátum dokladu ({receiptYear:number}) nezodpovedá priečinku ({folderYear:number})',
		// Filters
		filterAll: 'Všetky',
		filterUnassigned: 'Neoverené',
		filterNeedsReview: 'Na kontrolu',
		filterFuel: 'Tankovanie',
		filterOther: 'Iné náklady',
		// Verification summary
		allVerified: '{count:number}/{total:number} dokladov overených',
		verified: '{count:number}/{total:number} overených',
		unverified: '{count:number} neoverených',
		// Receipt details
		date: 'Dátum:',
		liters: 'Litre:',
		price: 'Cena:',
		station: 'Stanica:',
		trip: 'Jazda:',
		// Confidence
		confidenceHigh: 'Vysoká istota',
		confidenceMedium: 'Stredná istota',
		confidenceLow: 'Nízka istota',
		confidenceUnknown: 'Neznáma istota',
		// Status badges
		statusVerified: 'Overený',
		statusNeedsReview: 'Na kontrolu',
		statusUnverified: 'Neoverený',
		// Mismatch reasons
		mismatchMissingData: 'Chýbajú údaje na doklade',
		mismatchNoFuelTrip: 'Žiadna jazda s tankovaním',
		mismatchDate: 'Dátum {receiptDate:string} – jazda je {tripDate:string}',
		mismatchLiters: '{receiptLiters:number} L – jazda má {tripLiters:number} L',
		mismatchPrice: '{receiptPrice:number} € – jazda má {tripPrice:number} €',
		mismatchNoOtherCost: 'Žiadna jazda s touto cenou',
		// Actions
		open: 'Otvoriť',
		reprocess: 'Znovu spracovať',
		reprocessing: 'Spracovávam...',
		assignToTrip: 'Prideliť k jazde',
		// Other costs
		otherCost: 'Iné náklady',
		vendor: 'Predajca:',
		description: 'Popis:',
		assignmentBlocked: 'Jazda už má iné náklady',
		// Empty state
		noReceipts: 'Žiadne doklady. Kliknite na Načítať pre načítanie nových.',
	},

	// Receipt edit modal
	receiptEdit: {
		title: 'Upraviť doklad',
		date: 'Dátum',
		liters: 'Litre',
		amountSection: 'Suma',
		originalAmount: 'Pôvodná suma',
		currency: 'Mena',
		eurAmount: 'Suma v EUR',
		eurAmountRequired: 'Pre cudziu menu je potrebné zadať sumu v EUR',
		stationName: 'Čerpacia stanica',
		stationNamePlaceholder: 'napr. Slovnaft',
		vendorName: 'Predajca',
		vendorNamePlaceholder: 'napr. AutoUmyváreň',
		costDescription: 'Popis',
		costDescriptionPlaceholder: 'napr. Umytie auta',
	},

	// Trip selector modal
	tripSelector: {
		title: 'Prideliť doklad k jazde',
		noVehicleSelected: 'Nie je vybraté vozidlo',
		loadingTrips: 'Načítavam jazdy...',
		loadError: 'Nepodarilo sa načítať jazdy',
		noTrips: 'Žiadne jazdy na pridelenie.',
		alreadyHas: 'už má:',
		matchesReceipt: 'zodpovedá dokladu',
		// Mismatch reasons
		mismatchDate: 'iný dátum',
		mismatchLiters: 'iné litre',
		mismatchPrice: 'iná cena',
		mismatchLitersAndPrice: 'iné litre a cena',
		mismatchDateAndLiters: 'iný dátum a litre',
		mismatchDateAndPrice: 'iný dátum a cena',
		mismatchAll: 'všetko sa líši',
	},

	// Toast messages
	toast: {
		// Success
		vehicleSaved: 'Vozidlo bolo úspešne uložené',
		vehicleDeleted: 'Vozidlo bolo odstránené',
		settingsSaved: 'Nastavenia boli úspešne uložené',
		backupCreated: 'Záloha bola úspešne vytvorená',
		backupRestored: 'Záloha bola úspešne obnovená. Aplikácia sa reštartuje.',
		backupDeleted: 'Záloha bola odstránená',
		cleanupComplete: 'Zálohy boli vyčistené',
		receiptDeleted: 'Doklad bol odstránený',
		receiptUpdated: 'Doklad bol aktualizovaný',
		receiptReprocessed: 'Doklad "{name:string}" bol znovu spracovaný',
		receiptAssigned: 'Doklad bol pridelený k jazde',
		receiptsLoaded: 'Načítaných {count:number} nových dokladov',
		receiptsLoadedWithErrors: 'Načítaných {count:number} dokladov ({errors:number} chýb)',
		receiptsProcessed: 'Spracovaných {count:number} dokladov',
		receiptsProcessedWithErrors: 'Spracovaných {count:number} dokladov ({errors:number} chýb)',
		foundNewReceipts: 'Nájdených {count:number} nových súborov',
		noNewReceipts: 'Žiadne nové súbory',
		noPendingReceipts: 'Žiadne čakajúce doklady',
		// Errors
		errorSaveVehicle: 'Nepodarilo sa uložiť vozidlo: {error:string}',
		errorDeleteVehicle: 'Nepodarilo sa odstrániť vozidlo: {error:string}',
		errorSetActiveVehicle: 'Nepodarilo sa nastaviť aktívne vozidlo: {error:string}',
		errorSaveSettings: 'Nepodarilo sa uložiť nastavenia: {error:string}',
		errorCreateBackup: 'Nepodarilo sa vytvoriť zálohu: {error:string}',
		errorGetBackupInfo: 'Nepodarilo sa načítať informácie o zálohe: {error:string}',
		errorRestoreBackup: 'Nepodarilo sa obnoviť zálohu: {error:string}',
		errorDeleteBackup: 'Nepodarilo sa odstrániť zálohu: {error:string}',
		errorMoveDatabase: 'Nepodarilo sa presunúť databázu: {error:string}',
		errorResetDatabase: 'Nepodarilo sa obnoviť predvolené umiestnenie: {error:string}',
		errorLoadReceipts: 'Nepodarilo sa načítať doklady',
		errorSyncReceipts: 'Nepodarilo sa synchronizovať: {error:string}',
		errorProcessReceipts: 'Nepodarilo sa spracovať: {error:string}',
		errorDeleteReceipt: 'Nepodarilo sa odstrániť doklad',
		errorReprocessReceipt: 'Nepodarilo sa spracovať "{name:string}": {error:string}',
		errorAssignReceipt: 'Nepodarilo sa prideliť doklad: {error:string}',
		errorOpenFile: 'Nepodarilo sa otvoriť súbor',
		errorCreateTrip: 'Nepodarilo sa vytvoriť záznam',
		errorUpdateTrip: 'Nepodarilo sa aktualizovať záznam',
		errorDeleteTrip: 'Nepodarilo sa odstrániť záznam',
		errorMoveTrip: 'Nepodarilo sa presunúť záznam',
		errorAddCompensationTrip: 'Nepodarilo sa pridať jazdu. Skúste to znova.',
		errorExport: 'Export zlyhal: {error:string}',
		errorSelectVehicleFirst: 'Najprv vyberte vozidlo',
		errorSetApiKeyFirst: 'Najprv nastavte priečinok a API kľúč v Nastaveniach',
		errorSetApiKeyOnlyFirst: 'Najprv nastavte API kľúč v Nastaveniach',
	},

	// PDF export labels (passed to Rust)
	export: {
		// Page title
		pageTitle: 'KNIHA JÁZD',
		// Header labels
		headerCompany: 'Firma:',
		headerIco: 'IČO:',
		headerVehicle: 'Vozidlo:',
		headerLicensePlate: 'ŠPZ:',
		headerTankSize: 'Nádrž:',
		headerTpConsumption: 'TP spotreba:',
		headerYear: 'Rok:',
		// Header labels for BEV
		headerBatteryCapacity: 'Batéria:',
		headerBaselineConsumption: 'Základ. spotreba:',
		// VIN and Driver
		headerVin: 'VIN:',
		headerDriver: 'Vodič:',
		// Column headers
		colDate: 'Dátum',
		colTime: 'Čas',
		colOrigin: 'Odkiaľ',
		colDestination: 'Kam',
		colPurpose: 'Účel',
		colKm: 'Km',
		colOdo: 'ODO',
		colFuelLiters: 'PHM L',
		colFuelCost: '€ PHM',
		colFuelConsumed: 'Spotr. L',
		colOtherCosts: '€ Iné',
		colNote: 'Poznámka',
		colRemaining: 'Zost.',
		colConsumption: 'Spotr.',
		// Column headers for BEV
		colEnergyKwh: 'kWh',
		colEnergyCost: '€ Energ.',
		colBatteryRemaining: 'Batéria',
		colEnergyRate: 'kWh/100',
		// Footer labels
		footerTotalKm: 'Celkom km',
		footerTotalFuel: 'Celkom PHM',
		footerOtherCosts: 'Iné náklady',
		footerAvgConsumption: 'Priemerná spotreba',
		footerDeviation: 'Odchýlka od TP',
		footerTpNorm: 'TP norma',
		// Footer labels for BEV
		footerTotalEnergy: 'Celkom energia',
		footerAvgEnergyRate: 'Priemerná spotreba',
		footerBaselineNorm: 'Základ. norma',
		// Print hint
		printHint: 'Pre export do PDF použite Ctrl+P → Uložiť ako PDF',
	},

	// Update section
	update: {
		sectionTitle: 'Aktualizácie',
		currentVersion: 'Verzia',
		checkForUpdates: 'Skontrolovať aktualizácie',
		updateNow: 'Aktualizovať',
		later: 'Neskôr',
		checking: 'Kontrolujem...',
		upToDate: 'Aplikácia je aktuálna',
		available: 'Dostupná aktualizácia',
		availableVersion: 'Dostupná verzia {version:string}',
		downloading: 'Sťahujem...',
		downloadProgress: 'Sťahovanie: {percent:string}%',
		installing: 'Inštalujem...',
		modalTitle: 'Dostupná aktualizácia',
		modalBody: 'Verzia {version:string} je pripravená na inštaláciu.',
		releaseNotes: 'Čo je nové:',
		errorChecking: 'Nepodarilo sa skontrolovať aktualizácie',
		errorNetwork: 'Skontrolujte pripojenie k internetu',
		errorDownloadInterrupted: 'Sťahovanie prerušené, skúste znova',
		errorSignature: 'Neplatná signatúra aktualizácie',
		errorDiskSpace: 'Nedostatok miesta na disku',
		errorServerUnavailable: 'Server aktualizácií nedostupný',
		retry: 'Skúsiť znova',
		buttonUpdate: 'Aktualizovať',
		buttonLater: 'Neskôr',
		autoCheckOnStart: 'Automaticky kontrolovať pri štarte',
		showChangelog: 'Zobraziť zmeny',
		// Backup step during update
		backupStep: 'Záloha vytvorená',
		backupInProgress: 'Vytváranie zálohy...',
		backupFailed: 'Záloha zlyhala',
		backupFailedMessage: 'Nepodarilo sa vytvoriť zálohu databázy. Chcete pokračovať v aktualizácii bez zálohy?',
		continueWithoutBackup: 'Pokračovať bez zálohy',
	},
} satisfies BaseTranslation;

export default sk;
