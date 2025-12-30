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
		yearLabel: 'Rok:',
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
		nameLabel: 'Názov vozidla',
		licensePlateLabel: 'Evidenčné číslo (EČV)',
		tankSizeLabel: 'Objem nádrže (litre)',
		tpConsumptionLabel: 'Spotreba z TP (l/100km)',
		initialOdometerLabel: 'Počiatočný stav ODO (km)',
	},

	// Trip grid
	trips: {
		title: 'Jazdy',
		count: '({count:number})',
		newRecord: 'Nový záznam',
		firstRecord: 'Prvý záznam',
		emptyState: 'Žiadne záznamy. Kliknite na "Nový záznam" pre pridanie jazdy.',
		// Column headers
		columns: {
			date: 'Dátum',
			origin: 'Odkiaľ',
			destination: 'Kam',
			km: 'Km',
			odo: 'ODO',
			purpose: 'Účel',
			fuelLiters: 'PHM (L)',
			fuelCost: 'Cena €',
			consumptionRate: 'l/100km',
			remaining: 'Zostatok',
			otherCosts: 'Iné €',
			otherCostsNote: 'Iné pozn.',
			actions: 'Akcie',
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
		// Checkbox
		fullTank: 'Plná',
		// Tooltips/indicators
		partialFillup: 'Čiastočné tankovanie',
		noReceipt: 'Bez dokladu',
		estimatedRate: 'Odhad podľa TP',
		// Legend
		legend: {
			partialFillup: 'čiastočné tankovanie',
			noReceipt: 'bez dokladu',
			highConsumption: 'vysoká spotreba',
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
		// Language section
		languageSection: 'Jazyk',
		language: 'Jazyk aplikácie',
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
		sync: 'Načítať',
		syncing: 'Synchronizujem...',
		processPending: 'Spracovať čakajúce ({count:number})',
		processing: 'Spracovávam...',
		processingProgress: 'Spracovávam {current:number}/{total:number}...',
		// Config warning
		notConfigured: 'Funkcia dokladov nie je nakonfigurovaná.',
		configurePrompt: 'Vytvorte súbor s názvom',
		configurePromptFile: 'local.settings.json',
		configurePromptSuffix: 's nasledujúcim obsahom:',
		configNote: 'Poznámka: Na Windows používajte dvojité spätné lomky (\\\\) v cestách.',
		openConfigFolder: 'Otvoriť priečinok',
		// Filters
		filterAll: 'Všetky',
		filterUnassigned: 'Neoverené',
		filterNeedsReview: 'Na kontrolu',
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
		// Actions
		open: 'Otvoriť',
		reprocess: 'Znovu spracovať',
		reprocessing: 'Spracovávam...',
		assignToTrip: 'Prideliť k jazde',
		// Empty state
		noReceipts: 'Žiadne doklady. Kliknite na Načítať pre načítanie nových.',
	},

	// Trip selector modal
	tripSelector: {
		title: 'Prideliť doklad k jazde',
		noVehicleSelected: 'Nie je vybraté vozidlo',
		loadingTrips: 'Načítavam jazdy...',
		loadError: 'Nepodarilo sa načítať jazdy',
		noTrips: 'Žiadne jazdy na pridelenie.',
		alreadyHas: 'už má:',
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
		receiptDeleted: 'Doklad bol odstránený',
		receiptReprocessed: 'Doklad "{name:string}" bol znovu spracovaný',
		receiptAssigned: 'Doklad bol pridelený k jazde',
		receiptsLoaded: 'Načítaných {count:number} nových dokladov',
		receiptsLoadedWithErrors: 'Načítaných {count:number} dokladov ({errors:number} chýb)',
		receiptsProcessed: 'Spracovaných {count:number} dokladov',
		receiptsProcessedWithErrors: 'Spracovaných {count:number} dokladov ({errors:number} chýb)',
		noNewReceipts: 'Žiadne nové doklady',
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
		// Column headers
		colDate: 'Dátum',
		colOrigin: 'Odkiaľ',
		colDestination: 'Kam',
		colPurpose: 'Účel',
		colKm: 'Km',
		colOdo: 'ODO',
		colFuelLiters: 'PHM L',
		colFuelCost: '€ PHM',
		colOtherCosts: '€ Iné',
		colNote: 'Poznámka',
		colRemaining: 'Zost.',
		colConsumption: 'Spotr.',
		// Footer labels
		footerTotalKm: 'Celkom km',
		footerTotalFuel: 'Celkom PHM',
		footerOtherCosts: 'Iné náklady',
		footerAvgConsumption: 'Priemerná spotreba',
		footerDeviation: 'Odchýlka od TP',
		footerTpNorm: 'TP norma',
		// Print hint
		printHint: 'Pre export do PDF použite Ctrl+P → Uložiť ako PDF',
	},
} satisfies BaseTranslation;

export default sk;
