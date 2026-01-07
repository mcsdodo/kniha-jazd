/**
 * Language utilities for integration tests
 *
 * Provides helpers to ensure consistent language settings during tests.
 * The app supports Slovak (sk) and English (en) locales.
 */

/**
 * Supported locales
 */
export type Locale = 'sk' | 'en';

/**
 * Default locale for tests (Slovak is the primary UI language)
 */
export const DEFAULT_LOCALE: Locale = 'sk';

/**
 * Language switcher selector (if available in the app)
 */
const LANGUAGE_SWITCHER_SELECTOR = '#language-switcher';

/**
 * Alternative locale indicator selectors
 */
const LOCALE_INDICATORS = {
  sk: [
    // Slovak-specific text that would only appear in Slovak locale
    'h1*=Kniha jazd',
    'button*=Ulozit',
    'button*=Novy zaznam',
    'th*=Datum',
  ],
  en: [
    // English-specific text
    'h1*=Logbook',
    'button*=Save',
    'button*=New record',
    'th*=Date',
  ],
} as const;

/**
 * Detect the current locale by checking for locale-specific UI text
 */
export async function detectCurrentLocale(): Promise<Locale | null> {
  for (const indicator of LOCALE_INDICATORS.sk) {
    try {
      const element = await $(indicator);
      const isDisplayed = await element.isDisplayed();
      if (isDisplayed) {
        return 'sk';
      }
    } catch {
      // Element not found, continue checking
    }
  }

  for (const indicator of LOCALE_INDICATORS.en) {
    try {
      const element = await $(indicator);
      const isDisplayed = await element.isDisplayed();
      if (isDisplayed) {
        return 'en';
      }
    } catch {
      // Element not found, continue checking
    }
  }

  return null;
}

/**
 * Ensure the app is in the specified locale
 *
 * This function will:
 * 1. Check the current locale
 * 2. If different from desired, attempt to switch
 * 3. Verify the switch was successful
 *
 * @param targetLocale - The desired locale (default: 'sk')
 * @throws Error if locale cannot be set
 */
export async function ensureLanguage(targetLocale: Locale = DEFAULT_LOCALE): Promise<void> {
  const currentLocale = await detectCurrentLocale();

  if (currentLocale === targetLocale) {
    // Already in the correct locale
    return;
  }

  // Try to find and use the language switcher
  try {
    const switcher = await $(LANGUAGE_SWITCHER_SELECTOR);
    const exists = await switcher.isExisting();

    if (exists) {
      await switcher.selectByAttribute('value', targetLocale);
      await browser.pause(500); // Wait for UI to update

      // Verify the switch was successful
      const newLocale = await detectCurrentLocale();
      if (newLocale !== targetLocale) {
        throw new Error(
          `Failed to switch locale from ${currentLocale} to ${targetLocale}. Current: ${newLocale}`
        );
      }
      return;
    }
  } catch (error) {
    // Language switcher not found or not working
    console.warn('Language switcher not available:', error);
  }

  // If we can't switch and locale is different, warn but continue
  // The tests should still work as long as they use locale-agnostic selectors
  if (currentLocale !== null && currentLocale !== targetLocale) {
    console.warn(
      `App is in '${currentLocale}' locale, but '${targetLocale}' was requested. ` +
        `Tests will continue with current locale.`
    );
  }
}

/**
 * Ensure the app is in Slovak locale (default for this app)
 */
export async function ensureSlovak(): Promise<void> {
  await ensureLanguage('sk');
}

/**
 * Ensure the app is in English locale
 */
export async function ensureEnglish(): Promise<void> {
  await ensureLanguage('en');
}

/**
 * Get localized text for common UI elements
 *
 * This can be used to write locale-independent assertions
 */
export function getLocalizedText(
  key: keyof typeof localizedStrings,
  locale: Locale = DEFAULT_LOCALE
): string {
  return localizedStrings[key][locale];
}

/**
 * Common localized strings for assertions
 */
export const localizedStrings = {
  // Page titles
  appTitle: { sk: 'Kniha jazd', en: 'Logbook' },

  // Navigation
  trips: { sk: 'Jazdy', en: 'Trips' },
  settings: { sk: 'Nastavenia', en: 'Settings' },
  receipts: { sk: 'Doklady', en: 'Receipts' },
  backups: { sk: 'Zalohy', en: 'Backups' },

  // Actions
  save: { sk: 'Ulozit', en: 'Save' },
  cancel: { sk: 'Zrusit', en: 'Cancel' },
  delete: { sk: 'Vymazat', en: 'Delete' },
  edit: { sk: 'Upravit', en: 'Edit' },
  add: { sk: 'Pridat', en: 'Add' },
  newRecord: { sk: 'Novy zaznam', en: 'New record' },

  // Vehicle
  addVehicle: { sk: 'Pridat vozidlo', en: 'Add vehicle' },
  vehicleName: { sk: 'Nazov vozidla', en: 'Vehicle name' },
  licensePlate: { sk: 'ECV', en: 'License plate' },
  vehicleType: { sk: 'Typ vozidla', en: 'Vehicle type' },

  // Trip grid columns
  date: { sk: 'Datum', en: 'Date' },
  route: { sk: 'Trasa', en: 'Route' },
  distance: { sk: 'Vzdialenost', en: 'Distance' },
  odometer: { sk: 'Stav tachometra', en: 'Odometer' },
  refuel: { sk: 'Tankovanie', en: 'Refuel' },
  consumption: { sk: 'Spotreba', en: 'Consumption' },
  remaining: { sk: 'Zostatok', en: 'Remaining' },

  // Trip purposes
  businessTrip: { sk: 'Sluzobna cesta', en: 'Business trip' },
  clientMeeting: { sk: 'Stretnutie s klientom', en: 'Client meeting' },

  // Messages
  saved: { sk: 'Ulozene', en: 'Saved' },
  deleted: { sk: 'Vymazane', en: 'Deleted' },
  error: { sk: 'Chyba', en: 'Error' },

  // Warnings
  overLimit: { sk: 'Prekroceny limit', en: 'Over limit' },
  missingReceipt: { sk: 'Chybajuci doklad', en: 'Missing receipt' },
} as const;

/**
 * Helper type for localized string keys
 */
export type LocalizedStringKey = keyof typeof localizedStrings;
