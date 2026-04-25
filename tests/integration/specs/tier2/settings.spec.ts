/**
 * Tier 2: Settings Integration Tests
 *
 * Tests the settings functionality including:
 * - Saving company information (name and ICO)
 * - Switching language and verifying UI updates
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage, detectCurrentLocale, localizedStrings } from '../../utils/language';
import { seedSettings, invokeTauri } from '../../utils/db';
import {
  fillCompanySettings,
  fillField,
  selectOption,
  clickButtonByText,
} from '../../utils/forms';
import { Settings } from '../../utils/assertions';

type SettingsShape = {
  companyName: string;
  companyIco: string;
  bufferTripPurpose: string;
};

async function getSettings(): Promise<SettingsShape | null> {
  try {
    return await invokeTauri<SettingsShape>('get_settings');
  } catch {
    return null;
  }
}

async function saveSettings(settings: {
  companyName: string;
  companyIco: string;
  bufferTripPurpose?: string;
}): Promise<void> {
  await invokeTauri<void>('save_settings', {
    companyName: settings.companyName,
    companyIco: settings.companyIco,
    bufferTripPurpose: settings.bufferTripPurpose || 'Sluzobna cesta',
  });
}

/**
 * Set locale. There is no Tauri command for this; use localStorage directly.
 */
async function setLocale(locale: 'sk' | 'en'): Promise<void> {
  await browser.execute((loc: string) => {
    localStorage.setItem('kniha-jazd-locale', loc);
  }, locale);
}

describe('Tier 2: Settings', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Company Settings', () => {
    it('should save company name and ICO', async () => {
      // Navigate to settings page
      await navigateTo('settings');
      await browser.pause(500);

      // Define test company data
      const testCompanyName = 'Test Company s.r.o.';
      const testCompanyIco = '12345678';

      // Find company name input
      const companyNameInput = await $(Settings.companyName);
      const nameExists = await companyNameInput.isExisting();

      if (nameExists) {
        // Clear and fill company name
        await companyNameInput.clearValue();
        await companyNameInput.setValue(testCompanyName);

        // Find and fill ICO input
        const companyIcoInput = await $(Settings.companyIco);
        if (await companyIcoInput.isExisting()) {
          await companyIcoInput.clearValue();
          await companyIcoInput.setValue(testCompanyIco);
        }

        // Trigger blur to save immediately (auto-save on blur)
        await browser.keys('Tab');
        // Wait for auto-save debounce (800ms) + buffer
        await browser.pause(1200);

        // Verify settings were saved by checking the UI
        const savedNameValue = await companyNameInput.getValue();
        expect(savedNameValue).toBe(testCompanyName);

        const savedIcoValue = await companyIcoInput.getValue();
        expect(savedIcoValue).toBe(testCompanyIco);

        // Verify via Tauri IPC
        const settings = await getSettings();
        expect(settings).not.toBeNull();
        expect(settings?.companyName).toBe(testCompanyName);
        expect(settings?.companyIco).toBe(testCompanyIco);
      } else {
        // Settings fields not found in UI - save via IPC directly
        await saveSettings({
          companyName: testCompanyName,
          companyIco: testCompanyIco,
          bufferTripPurpose: 'Sluzobna cesta',
        });

        // Verify settings were saved
        const settings = await getSettings();
        expect(settings).not.toBeNull();
        expect(settings?.companyName).toBe(testCompanyName);
        expect(settings?.companyIco).toBe(testCompanyIco);

        // Refresh and verify UI reflects the saved settings
        await browser.refresh();
        await waitForAppReady();
        await navigateTo('settings');
        await browser.pause(500);

        const body = await $('body');
        const text = await body.getText();

        expect(text).toContain(testCompanyName);
        expect(text).toContain(testCompanyIco);
      }
    });
  });

  describe('Language Switching', () => {
    it('should switch language and see UI update', async () => {
      // Navigate to settings page
      await navigateTo('settings');
      await browser.pause(500);

      // Detect current locale
      const initialLocale = await detectCurrentLocale();
      console.log(`Initial locale detected: ${initialLocale}`);

      // Find language switcher
      const languageSwitcher = await $('#language-switcher');
      const switcherExists = await languageSwitcher.isExisting();

      if (switcherExists) {
        // Determine target locale (opposite of current)
        const targetLocale = initialLocale === 'en' ? 'sk' : 'en';

        // Switch language
        await languageSwitcher.selectByAttribute('value', targetLocale);
        await browser.pause(1000); // Wait for UI to update

        // Verify UI updated to new language
        const newLocale = await detectCurrentLocale();
        expect(newLocale).toBe(targetLocale);

        // Verify specific UI elements changed
        const body = await $('body');
        const text = await body.getText();

        // Check for language-specific text
        if (targetLocale === 'en') {
          // English UI elements
          expect(text).toMatch(/Settings|Save|Cancel/i);
        } else {
          // Slovak UI elements
          expect(text).toMatch(/Nastavenia|Ulozit|Zrusit/i);
        }

        // Switch back to original language
        await languageSwitcher.selectByAttribute('value', initialLocale || 'sk');
        await browser.pause(500);
      } else {
        // Language switcher not found - try via IPC
        console.log('Language switcher not found in UI');

        // Get the page title/header to determine current language
        const header = await $('h1');
        const headerText = await header.getText();
        const isEnglish = headerText.toLowerCase().includes('logbook');

        // Try to switch language via IPC
        const targetLocale = isEnglish ? 'sk' : 'en';
        await setLocale(targetLocale);

        // Refresh to apply language change
        await browser.refresh();
        await waitForAppReady();

        // Verify language changed
        const newHeader = await $('h1');
        const newHeaderText = await newHeader.getText();

        if (targetLocale === 'en') {
          expect(newHeaderText.toLowerCase()).toContain('logbook');
        } else {
          expect(newHeaderText.toLowerCase()).toContain('kniha');
        }

        // Check for more language-specific elements
        const body = await $('body');
        const bodyText = await body.getText();

        if (targetLocale === 'en') {
          // Look for English UI text
          const hasEnglishText =
            bodyText.includes('Settings') ||
            bodyText.includes('Save') ||
            bodyText.includes('Trips') ||
            bodyText.includes('Vehicle');
          expect(hasEnglishText).toBe(true);
        } else {
          // Look for Slovak UI text
          const hasSlovakText =
            bodyText.includes('Nastavenia') ||
            bodyText.includes('Ulozit') ||
            bodyText.includes('Jazdy') ||
            bodyText.includes('Vozidlo');
          expect(hasSlovakText).toBe(true);
        }

        // Switch back to Slovak (default for this app)
        await setLocale('sk');
        await browser.refresh();
        await waitForAppReady();
      }
    });
  });
});
