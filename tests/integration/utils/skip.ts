/**
 * Skip helpers for dual-mode integration tests.
 *
 * Some tests exercise Tauri-only features (e.g., native file dialogs,
 * window management) that have no equivalent in server mode. Use these
 * helpers to skip such tests gracefully.
 */

// Mocha's `pending()` is injected globally by the runner at test time
// but not declared in @wdio/mocha-framework types.
declare function pending(message?: string): void;

/**
 * Skip the current test when running in server mode (for Tauri-only features).
 *
 * Must be called at the top of an `it()` block — it calls Mocha's `pending()`
 * which marks the test as skipped with a descriptive reason.
 *
 * @example
 * it('should open native file dialog', async () => {
 *   skipInServerMode('native file dialog not available');
 *   // ... Tauri-only test code
 * });
 */
export function skipInServerMode(description: string): void {
  if (process.env.WDIO_SERVER_MODE === '1') {
    pending(`Skipped in server mode: ${description}`);
  }
}

/**
 * Skip the current test when running in Tauri mode (for server-only features).
 *
 * @example
 * it('should show server status indicator', async () => {
 *   skipInTauriMode('server-only UI element');
 *   // ... server-mode-only test code
 * });
 */
export function skipInTauriMode(description: string): void {
  if (process.env.WDIO_SERVER_MODE !== '1') {
    pending(`Skipped in Tauri mode: ${description}`);
  }
}
