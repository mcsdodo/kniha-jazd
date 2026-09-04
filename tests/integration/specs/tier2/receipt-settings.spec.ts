/**
 * Tier 2: Receipt Settings & Database Location Integration Tests
 *
 * Tests the new features from tasks 39 & 40:
 * - Receipt scanning settings (API key, folder path)
 * - Database location reporting (backend only - the move-database UI is gone)
 * - The app version the settings page renders from `get_app_version`
 * - Read-only mode banner (via API, hard to test UI without mocking migration)
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { rpc } from '../../utils/db';

// Selectors for new UI elements
const ReceiptSettings = {
  section: '#receipt-scanning',
  geminiApiKeyInput: '#gemini-api-key',
  receiptsFolderInput: '#receipts-folder', // editable <input> since the file dialog went away
  showHideApiKeyBtn: '.icon-btn',
};

/** Version the settings page reads from the backend on mount. */
const APP_VERSION = '[data-test="app-version"]';
/** What the component renders until `get_app_version` answers. */
const APP_VERSION_PLACEHOLDER = '...';

type ReceiptSettingsShape = {
  geminiApiKey: string | null;
  receiptsFolderPath: string | null;
};

async function getReceiptSettings(): Promise<ReceiptSettingsShape | null> {
  try {
    return await rpc<ReceiptSettingsShape>('get_receipt_settings');
  } catch {
    return null;
  }
}

async function setGeminiApiKey(apiKey: string): Promise<void> {
  await rpc<void>('set_gemini_api_key', { apiKey });
}

async function setReceiptsFolderPath(path: string): Promise<void> {
  await rpc<void>('set_receipts_folder_path', { path });
}

async function getDbLocation(): Promise<{
  dbPath: string;
  isCustomPath: boolean;
  backupsPath: string;
}> {
  return rpc('get_db_location');
}

async function getAppMode(): Promise<{
  mode: string;
  isReadOnly: boolean;
  readOnlyReason: string | null;
}> {
  return rpc('get_app_mode');
}

describe('Tier 2: Receipt Settings & Database Location', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Receipt Scanning Settings UI', () => {
    it('should display receipt scanning section on settings page', async () => {
      await navigateTo('settings');
      await browser.pause(500);

      // Check for the receipt scanning section
      const section = await $(ReceiptSettings.section);
      const sectionExists = await section.isExisting();
      expect(sectionExists).toBe(true);

      // Check for API key input
      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      const apiKeyExists = await apiKeyInput.isExisting();
      expect(apiKeyExists).toBe(true);

      // Check for the receipts folder input
      const folderInput = await $(ReceiptSettings.receiptsFolderInput);
      const folderExists = await folderInput.isExisting();
      expect(folderExists).toBe(true);
    });

    it('should mask the API key and gate revealing it', async () => {
      // The key is write-only now (task 69): it never arrives with the settings,
      // and revealing it is a separate backend call that needs a PIN.
      await setGeminiApiKey('reveal-flow-key');
      await navigateTo('trips');
      await navigateTo('settings');
      await browser.pause(500);

      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      const toggleBtn = await $(ReceiptSettings.showHideApiKeyBtn);
      expect(await apiKeyInput.getAttribute('type')).toBe('password');

      await toggleBtn.click();
      await browser.pause(300);

      // No KNIHA_JAZD_REVEAL_PIN in this suite, so the prompt appears and the
      // backend refuses. The full PIN flow lives in specs/env.
      const modal = await $('[data-test="reveal-pin-modal"]');
      expect(await modal.isDisplayed()).toBe(true);
      expect(await apiKeyInput.getAttribute('type')).toBe('password');

      await setGeminiApiKey('');
    });

    it('should save receipt settings via IPC', async () => {
      // Set settings via IPC
      const testApiKey = 'test-api-key-12345';
      await setGeminiApiKey(testApiKey);

      // Small pause to ensure file system sync in CI
      await browser.pause(100);

      // Verify settings were saved
      const settings = await getReceiptSettings();
      expect(settings).not.toBeNull();
      // The key itself is never returned — only whether one is configured
      expect(settings?.hasGeminiApiKey).toBe(true);

      // Clear the API key after test
      await setGeminiApiKey('');
    });

    it('should show a placeholder instead of the saved API key', async function () {
      await setGeminiApiKey('test-display-key');
      await navigateTo('trips');
      await navigateTo('settings');

      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      // The settings page issues ~10 sequential RPCs before receipt settings land,
      // which is slow over HTTP — wait for the field to settle rather than guessing
      // a duration. Write-only: the field stays empty and advertises the stored key.
      await browser.waitUntil(
        async () => (await apiKeyInput.getAttribute('placeholder')) === '********',
        { timeout: 15000, timeoutMsg: 'receipt settings never loaded into the API key field' }
      );
      expect(await apiKeyInput.getValue()).toBe('');

      await setGeminiApiKey('');
    });

    it('should display database location info via IPC', async () => {
      const dbLocation = await getDbLocation();

      expect(dbLocation).toBeDefined();
      expect(dbLocation.dbPath).toBeDefined();
      expect(typeof dbLocation.dbPath).toBe('string');
      expect(dbLocation.dbPath.length).toBeGreaterThan(0);
      expect(dbLocation.dbPath).toContain('kniha-jazd');
    });

    it('should correctly report default path via IPC', async () => {
      // This test verifies the backend correctly reports whether using default path
      // UI badge display was removed in favor of simpler UI
      const dbLocation = await getDbLocation();

      // For a fresh test environment, path should be default (not custom)
      expect(dbLocation.isCustomPath).toBe(false);
    });
  });

  describe('App Version', () => {
    it('should render the backend app version on the settings page', async () => {
      // Desktop read the version from Tauri; the browser build gets it over the
      // `get_app_version` RPC, so this is the only place that wiring is proven.
      await navigateTo('trips');
      await navigateTo('settings');

      const versionEl = await $(APP_VERSION);
      await versionEl.waitForDisplayed({
        timeout: 15000,
        timeoutMsg: `Version element (${APP_VERSION}) never rendered on the settings page`,
      });

      // The settings page fires ~10 sequential RPCs on mount; wait for the
      // placeholder to be replaced rather than guessing a duration.
      await browser.waitUntil(
        async () => (await versionEl.getText()).trim() !== APP_VERSION_PLACEHOLDER,
        {
          timeout: 15000,
          timeoutMsg: 'app version stayed on its "..." placeholder - get_app_version never answered',
        }
      );

      const rendered = (await versionEl.getText()).trim();
      expect(rendered).toMatch(/^\d+\.\d+\.\d+/);

      // ...and it is the backend's version, not something the page invented.
      const fromBackend = await rpc<string>('get_app_version');
      expect(rendered).toBe(fromBackend);
    });
  });

  describe('App Mode (Read-Only)', () => {
    it('should return normal mode by default', async () => {
      const appMode = await getAppMode();

      expect(appMode).toBeDefined();
      expect(appMode.mode).toBe('Normal');
      expect(appMode.isReadOnly).toBe(false);
      expect(appMode.readOnlyReason).toBeNull();
    });

    it('should not display read-only banner in normal mode', async () => {
      // Wait for app to load
      await waitForAppReady();

      // Check that read-only banner is not visible
      const banner = await $('.read-only-banner');
      const bannerExists = await banner.isExisting();

      // In normal mode, banner should not exist or not be visible
      if (bannerExists) {
        const isDisplayed = await banner.isDisplayed();
        expect(isDisplayed).toBe(false);
      } else {
        expect(bannerExists).toBe(false);
      }
    });
  });

  describe('Receipt Settings Auto-Save Flow', () => {
    it('should persist settings through IPC', async () => {
      // Test setting values directly via IPC (which is what the UI ultimately does)
      // This avoids Svelte binding issues in WebDriver and focuses on the backend

      const testApiKey = 'ipc-flow-test-key-' + Date.now();

      // Set via IPC directly
      await setGeminiApiKey(testApiKey);

      // Small pause to ensure file system sync in CI
      await browser.pause(100);

      // Verify the setting was persisted
      const settings = await getReceiptSettings();
      expect(settings?.hasGeminiApiKey).toBe(true);

      // Clean up
      await setGeminiApiKey('');

      // Verify cleanup - empty string is stored as null
      const cleanSettings = await getReceiptSettings();
      expect(cleanSettings?.hasGeminiApiKey).toBe(false);
    });
  });
});
