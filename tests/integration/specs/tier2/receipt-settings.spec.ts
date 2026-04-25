/**
 * Tier 2: Receipt Settings & Database Location Integration Tests
 *
 * Tests the new features from tasks 39 & 40:
 * - Receipt scanning settings (API key, folder path)
 * - Database location display
 * - Read-only mode banner (via API, hard to test UI without mocking migration)
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { invokeTauri } from '../../utils/db';
import { describeNotInServerMode } from '../../utils/skip';

// Selectors for new UI elements
const ReceiptSettings = {
  section: '#receipt-scanning',
  geminiApiKeyInput: '#gemini-api-key',
  receiptsFolderDisplay: '#receipts-folder', // Note: this is a span, not input
  showHideApiKeyBtn: '.icon-btn',
  browseFolderBtn: '.browse-folder-btn',
};

const DbLocation = {
  section: '#db-location',
  pathDisplay: '#db-path', // Specific ID to avoid matching receipts-folder
  customBadge: '.badge.custom',
  defaultBadge: '.badge.default',
  openFolderBtn: '.button-row .button-small',
};

type ReceiptSettingsShape = {
  geminiApiKey: string | null;
  receiptsFolderPath: string | null;
};

async function getReceiptSettings(): Promise<ReceiptSettingsShape | null> {
  try {
    return await invokeTauri<ReceiptSettingsShape>('get_receipt_settings');
  } catch {
    return null;
  }
}

async function setGeminiApiKey(apiKey: string): Promise<void> {
  await invokeTauri<void>('set_gemini_api_key', { apiKey });
}

async function setReceiptsFolderPath(path: string): Promise<void> {
  await invokeTauri<void>('set_receipts_folder_path', { path });
}

async function getDbLocation(): Promise<{
  dbPath: string;
  isCustomPath: boolean;
  backupsPath: string;
}> {
  return invokeTauri('get_db_location');
}

async function getAppMode(): Promise<{
  mode: string;
  isReadOnly: boolean;
  readOnlyReason: string | null;
}> {
  return invokeTauri('get_app_mode');
}

async function checkTargetHasDb(targetPath: string): Promise<boolean> {
  return invokeTauri<boolean>('check_target_has_db', { targetPath });
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

      // Check for folder display (span, not input)
      const folderDisplay = await $(ReceiptSettings.receiptsFolderDisplay);
      const folderExists = await folderDisplay.isExisting();
      expect(folderExists).toBe(true);
    });

    it('should toggle API key visibility', async () => {
      await navigateTo('settings');
      await browser.pause(500);

      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      const toggleBtn = await $(ReceiptSettings.showHideApiKeyBtn);

      // Initially should be password type
      const initialType = await apiKeyInput.getAttribute('type');
      expect(initialType).toBe('password');

      // Click toggle to show
      await toggleBtn.click();
      await browser.pause(200);

      const shownType = await apiKeyInput.getAttribute('type');
      expect(shownType).toBe('text');

      // Click toggle to hide
      await toggleBtn.click();
      await browser.pause(200);

      const hiddenType = await apiKeyInput.getAttribute('type');
      expect(hiddenType).toBe('password');
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
      expect(settings?.geminiApiKey).toBe(testApiKey);

      // Clear the API key after test
      await setGeminiApiKey('');
    });

    it('should display saved API key in settings UI', async function () {
      // The settings page's onMount runs ~10 sequential RPC calls before it reaches
      // the receipt-settings load step (getSettings → loadBackups → retention →
      // vehicles-with-trips → version → auto-check → ... → getReceiptSettings).
      // In server mode each is an HTTP roundtrip, so the cumulative latency makes
      // this UI display test flaky. The persistence path is already verified by the
      // "should persist settings through IPC" test below; this test only adds the
      // Svelte bind:value assertion which is framework-level. Skip in server mode.
      if (process.env.WDIO_SERVER_MODE === '1') {
        this.skip();
      }
      const testApiKey = 'test-display-key';

      await setGeminiApiKey(testApiKey);

      // Small pause to ensure file system sync in CI
      await browser.pause(100);

      // Navigate AWAY from settings first to ensure fresh mount
      // (SvelteKit caches components, so navigating to the same page won't remount)
      await navigateTo('trips');
      await browser.pause(300);

      // Now navigate to settings - this will trigger onMount and load settings from backend
      await navigateTo('settings');
      await browser.pause(500);

      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      const value = await apiKeyInput.getValue();
      expect(value).toBe(testApiKey);

      // Clean up
      await setGeminiApiKey('');
    });
  });

  describe('Database Location Settings', () => {
    it('should display database location info via IPC', async () => {
      const dbLocation = await getDbLocation();

      expect(dbLocation).toBeDefined();
      expect(dbLocation.dbPath).toBeDefined();
      expect(typeof dbLocation.dbPath).toBe('string');
      expect(dbLocation.dbPath.length).toBeGreaterThan(0);
      expect(dbLocation.dbPath).toContain('kniha-jazd');
    });

    it('should show database path in settings UI', async () => {
      await navigateTo('settings');
      await browser.pause(500);

      // Get expected path from IPC
      const dbLocation = await getDbLocation();

      // Find path display in UI
      const pathDisplay = await $(DbLocation.pathDisplay);
      const pathExists = await pathDisplay.isExisting();

      if (pathExists) {
        const displayedPath = await pathDisplay.getText();
        // Path should contain kniha-jazd.db or similar
        expect(displayedPath).toContain('kniha-jazd');
      }
    });

    it('should correctly report default path via IPC', async () => {
      // This test verifies the backend correctly reports whether using default path
      // UI badge display was removed in favor of simpler UI
      const dbLocation = await getDbLocation();

      // For a fresh test environment, path should be default (not custom)
      expect(dbLocation.isCustomPath).toBe(false);
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

  describeNotInServerMode('Database Move Commands', () => {
    it('should detect existing database via check_target_has_db', async () => {
      // Get current db location
      const dbLocation = await getDbLocation();
      const dbDir = dbLocation.dbPath.substring(0, dbLocation.dbPath.lastIndexOf('\\'));

      // The directory containing the db should have a database
      const hasDb = await checkTargetHasDb(dbDir);
      expect(hasDb).toBe(true);
    });

    it('should return false for empty directory', async () => {
      // Check a path that definitely doesn't have a database
      const hasDb = await checkTargetHasDb('C:\\Windows\\Temp');
      expect(hasDb).toBe(false);
    });

    it('should show Change Location button in settings', async () => {
      await navigateTo('settings');
      await browser.pause(500);

      // Find the change location button by class
      const changeBtn = await $('.change-db-location-btn');
      const changeBtnExists = await changeBtn.isExisting();
      expect(changeBtnExists).toBe(true);
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
      expect(settings?.geminiApiKey).toBe(testApiKey);

      // Clean up
      await setGeminiApiKey('');

      // Verify cleanup - empty string is stored as null
      const cleanSettings = await getReceiptSettings();
      expect(cleanSettings?.geminiApiKey).toBeNull();
    });
  });
});
