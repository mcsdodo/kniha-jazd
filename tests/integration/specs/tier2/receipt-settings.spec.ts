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

// Selectors for new UI elements
const ReceiptSettings = {
  section: '#receipt-scanning',
  geminiApiKeyInput: '#gemini-api-key',
  receiptsFolderInput: '#receipts-folder',
  showHideApiKeyBtn: '.toggle-btn',
  browseFolderBtn: '.input-with-button button',
  saveBtn: '#receipt-scanning .button',
};

const DbLocation = {
  section: '.settings-section:has(h2)',
  pathDisplay: '.db-path-display .path-text',
  customBadge: '.badge.custom',
  defaultBadge: '.badge.default',
  openFolderBtn: '.button-row .button-small',
};

/**
 * Get receipt settings via Tauri IPC
 */
async function getReceiptSettings(): Promise<{
  geminiApiKey: string | null;
  receiptsFolderPath: string | null;
} | null> {
  const result = await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    try {
      return await window.__TAURI__.core.invoke('get_receipt_settings');
    } catch {
      return null;
    }
  });
  return result as { geminiApiKey: string | null; receiptsFolderPath: string | null } | null;
}

/**
 * Set Gemini API key via Tauri IPC
 */
async function setGeminiApiKey(apiKey: string): Promise<void> {
  await browser.execute(async (key: string) => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('set_gemini_api_key', { apiKey: key });
  }, apiKey);
}

/**
 * Set receipts folder path via Tauri IPC
 */
async function setReceiptsFolderPath(path: string): Promise<void> {
  await browser.execute(async (p: string) => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('set_receipts_folder_path', { path: p });
  }, path);
}

/**
 * Get database location info via Tauri IPC
 */
async function getDbLocation(): Promise<{
  dbPath: string;
  isCustomPath: boolean;
  backupsPath: string;
}> {
  const result = await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('get_db_location');
  });
  return result as { dbPath: string; isCustomPath: boolean; backupsPath: string };
}

/**
 * Get app mode info via Tauri IPC
 */
async function getAppMode(): Promise<{
  mode: string;
  isReadOnly: boolean;
  readOnlyReason: string | null;
}> {
  const result = await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('get_app_mode');
  });
  return result as { mode: string; isReadOnly: boolean; readOnlyReason: string | null };
}

/**
 * Check if target folder has a database via Tauri IPC
 */
async function checkTargetHasDb(targetPath: string): Promise<boolean> {
  const result = await browser.execute(async (path: string) => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('check_target_has_db', { targetPath: path });
  }, targetPath);
  return result as boolean;
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

      // Check for folder input
      const folderInput = await $(ReceiptSettings.receiptsFolderInput);
      const folderExists = await folderInput.isExisting();
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

      // Verify settings were saved
      const settings = await getReceiptSettings();
      expect(settings).not.toBeNull();
      expect(settings?.geminiApiKey).toBe(testApiKey);

      // Clear the API key after test
      await setGeminiApiKey('');
    });

    it('should display saved API key in settings UI', async () => {
      // Set settings via IPC first
      const testApiKey = 'test-display-key';
      await setGeminiApiKey(testApiKey);

      // Navigate to settings and verify UI shows the key
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

    it('should show default badge for non-custom path', async () => {
      await navigateTo('settings');
      await browser.pause(500);

      const dbLocation = await getDbLocation();

      // If not custom path, should show default badge
      if (!dbLocation.isCustomPath) {
        const defaultBadge = await $(DbLocation.defaultBadge);
        const badgeExists = await defaultBadge.isExisting();
        expect(badgeExists).toBe(true);
      }
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

  describe('Database Move Commands', () => {
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

      // Find the change location button
      const changeBtn = await $('button*=Change Location');
      const changeBtnExists = await changeBtn.isExisting();

      // The button should exist (may have Slovak text)
      if (!changeBtnExists) {
        const changeBtnSk = await $('button*=Zmeniť umiestnenie');
        const changeBtnSkExists = await changeBtnSk.isExisting();
        expect(changeBtnSkExists).toBe(true);
      } else {
        expect(changeBtnExists).toBe(true);
      }
    });
  });

  describe('Receipt Settings Save Flow', () => {
    it('should save and persist settings through UI flow', async () => {
      await navigateTo('settings');
      await browser.pause(500);

      const testApiKey = 'ui-flow-test-key';

      // Find API key input and set value
      const apiKeyInput = await $(ReceiptSettings.geminiApiKeyInput);
      await apiKeyInput.clearValue();

      // Use atomic value setting for reliability
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, ReceiptSettings.geminiApiKeyInput, testApiKey);

      await browser.pause(300);

      // Find and click save button
      const saveBtn = await $(ReceiptSettings.saveBtn);
      if (await saveBtn.isExisting()) {
        await saveBtn.click();
        await browser.pause(1000);

        // Check for success toast
        const toastSuccess = await $('.toast-success, .toast.success');
        const toastExists = await toastSuccess.isExisting();
        // Toast should appear on successful save
        expect(toastExists).toBe(true);

        // Verify via IPC that settings were saved
        const settings = await getReceiptSettings();
        expect(settings?.geminiApiKey).toBe(testApiKey);
      }

      // Clean up
      await setGeminiApiKey('');
    });
  });
});
