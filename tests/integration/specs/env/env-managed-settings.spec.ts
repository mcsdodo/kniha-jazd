/**
 * Env-pinned settings suite.
 *
 * Runs ONLY under `npm run test:integration:server:env`, which starts the server
 * with HA_URL / HA_API_TOKEN / PAPERLESS_* exported (see ENV_PINNED_FIXTURE in
 * wdio.server.conf.ts). Those variables make the matching settings read-only
 * app-wide, so this cannot live alongside specs that edit them.
 *
 * Covers the UI contract only — that env-pinned fields render read-only, name
 * their variable, and still expose the connection status. The pinning logic
 * itself is proven by the Rust unit tests in integrations_tests.rs.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { invokeTauri } from '../../utils/db';

const HA_TOKEN_FIXTURE = 'env-pinned-ha-token';
const PAPERLESS_TOKEN_FIXTURE = 'env-pinned-paperless-token';

describe('Env-managed settings', () => {
  before(async () => {
    await waitForAppReady();
    await navigateTo('settings');
  });

  it('reports the pinned fields over the API', async () => {
    const ha = await invokeTauri<{
      url: string | null;
      urlFromEnv: boolean;
      tokenFromEnv: boolean;
      tokenEnvValue: string | null;
    }>('get_ha_settings');

    expect(ha.urlFromEnv).toBe(true);
    expect(ha.tokenFromEnv).toBe(true);
    expect(ha.tokenEnvValue).toBe(HA_TOKEN_FIXTURE);

    const paperless = await invokeTauri<{
      urlFromEnv: boolean;
      tokenFromEnv: boolean;
      enabledFromEnv: boolean;
      tokenEnvValue: string | null;
    }>('get_paperless_settings');

    expect(paperless.urlFromEnv).toBe(true);
    expect(paperless.tokenFromEnv).toBe(true);
    expect(paperless.enabledFromEnv).toBe(true);
    expect(paperless.tokenEnvValue).toBe(PAPERLESS_TOKEN_FIXTURE);
  });

  it('renders pinned inputs as disabled', async () => {
    for (const selector of ['#ha-url', '#ha-token', '#paperless-url', '#paperless-token']) {
      const input = await $(selector);
      await input.waitForDisplayed();
      expect(await input.isEnabled()).toBe(false);
    }

    const toggle = await $('[data-test="paperless-enabled-toggle"]');
    expect(await toggle.isEnabled()).toBe(false);
  });

  it('names the environment variable behind each pinned field', async () => {
    const expected: Record<string, string> = {
      'ha-url-env-badge': 'HA_URL',
      'ha-token-env-badge': 'HA_API_TOKEN',
      'paperless-url-env-badge': 'PAPERLESS_URL',
      'paperless-token-env-badge': 'PAPERLESS_API_TOKEN',
      'paperless-enabled-env-badge': 'PAPERLESS_ENABLED',
    };

    for (const [testId, varName] of Object.entries(expected)) {
      const badge = await $(`[data-test="${testId}"]`);
      expect(await badge.isDisplayed()).toBe(true);
      expect(await badge.getText()).toBe(varName);
    }
  });

  it('reveals the live env token behind the eye icon', async () => {
    const tokenInput = await $('#ha-token');
    // The bound value is the env token even while masked
    expect(await tokenInput.getValue()).toBe(HA_TOKEN_FIXTURE);
    expect(await tokenInput.getAttribute('type')).toBe('password');

    // The eye button sits next to the input inside .input-with-icon
    const eye = await $('#ha-token').parentElement().$('button.icon-btn');
    await eye.click();

    await browser.waitUntil(
      async () => (await tokenInput.getAttribute('type')) === 'text',
      { timeout: 3000, timeoutMsg: 'eye icon did not reveal the token' }
    );
    expect(await tokenInput.getValue()).toBe(HA_TOKEN_FIXTURE);
  });

  it('still shows connection status for both integrations', async () => {
    // Both fixtures point at hosts that do not resolve, so the status settles on
    // "disconnected" — the point is that the block renders at all when the
    // configuration comes from the environment rather than the settings file.
    const haStatus = await $('.ha-status');
    await haStatus.waitForDisplayed({ timeout: 15000 });

    const paperlessStatus = await $('[data-test="paperless-status"]');
    await paperlessStatus.waitForDisplayed({ timeout: 15000 });
  });

  it('does not attempt a save when a pinned field is interacted with', async () => {
    const tokenInput = await $('#ha-token');
    // Disabled inputs swallow input events; assert no rejected-save toast appears.
    await tokenInput.click().catch(() => {
      /* clicking a disabled element is expected to be a no-op or throw */
    });
    await browser.pause(1200); // longer than the 800ms auto-save debounce

    const errorToast = await $('.toast-error');
    expect(await errorToast.isExisting()).toBe(false);
  });

  it('offers a typed receipts-folder path in server mode', async () => {
    // No native directory dialog over HTTP — the browse button is replaced by an input
    const input = await $('[data-test="receipts-folder-input"]');
    expect(await input.isDisplayed()).toBe(true);
    expect(await input.isEnabled()).toBe(true);
  });
});
