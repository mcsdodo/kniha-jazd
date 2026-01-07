/**
 * App utilities for integration tests
 */

/**
 * Wait for the app to be fully loaded and ready
 * Checks for:
 * 1. Main header element (DOM ready)
 * 2. Tauri IPC bridge (window.__TAURI__ available)
 */
export async function waitForAppReady(): Promise<void> {
  // First wait for DOM to be ready (h1 visible)
  await browser.waitUntil(
    async () => {
      const header = await $('h1');
      return header.isDisplayed();
    },
    {
      timeout: 30000,
      timeoutMsg: 'App did not load within 30 seconds'
    }
  );

  // Then wait for Tauri v2 IPC bridge to be available
  // In Tauri v2 with withGlobalTauri: true, API is at window.__TAURI__.core.invoke
  await browser.waitUntil(
    async () => {
      return browser.execute(() => {
        return typeof (window as any).__TAURI__ !== 'undefined' &&
               typeof (window as any).__TAURI__.core !== 'undefined' &&
               typeof (window as any).__TAURI__.core.invoke === 'function';
      });
    },
    {
      timeout: 10000,
      timeoutMsg: 'Tauri IPC bridge did not initialize within 10 seconds'
    }
  );
}

/**
 * Navigate to a specific page in the app
 */
export async function navigateTo(path: 'trips' | 'settings' | 'doklady' | 'backups'): Promise<void> {
  const hrefs: Record<string, string> = {
    trips: '/',
    settings: '/settings',
    doklady: '/doklady',
    backups: '/backups'
  };

  const link = await $(`a[href="${hrefs[path]}"]`);
  await link.click();

  // Wait for navigation to complete
  await browser.pause(500);
}

/**
 * Take a screenshot with a descriptive name (for debugging failed tests)
 */
export async function takeScreenshot(name: string): Promise<void> {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  await browser.saveScreenshot(`./tests/integration/screenshots/${timestamp}-${name}.png`);
}

/**
 * Fill in a form field by label or placeholder
 */
export async function fillField(selector: string, value: string): Promise<void> {
  const field = await $(selector);
  await field.clearValue();
  await field.setValue(value);
}

/**
 * Click a button by its text content
 */
export async function clickButton(text: string): Promise<void> {
  const button = await $(`button=${text}`);
  await button.click();
}

/**
 * Wait for a toast/notification message to appear
 */
export async function waitForToast(textContains?: string): Promise<void> {
  const toast = await $('.toast');
  await toast.waitForDisplayed({ timeout: 5000 });

  if (textContains) {
    await browser.waitUntil(
      async () => {
        const text = await toast.getText();
        return text.includes(textContains);
      },
      { timeout: 5000 }
    );
  }
}
