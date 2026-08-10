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
const REVEAL_PIN = '4269';

/** Click the eye, answer the PIN prompt, and wait for the value to appear. */
async function revealWithPin(
  eye: WebdriverIO.Element,
  tokenInput: WebdriverIO.Element
): Promise<void> {
  const modal = await $('[data-test="reveal-pin-modal"]');
  await eye.click();
  await modal.waitForDisplayed({ timeout: 5000 });
  await (await $('[data-test="reveal-pin-input"]')).setValue(REVEAL_PIN);
  await (await $('[data-test="reveal-pin-submit"]')).click();
  // Wait for the overlay to go before returning — it intercepts later clicks
  await modal.waitForDisplayed({ reverse: true, timeout: 5000 });
  await browser.waitUntil(async () => (await tokenInput.getValue()) === HA_TOKEN_FIXTURE, {
    timeout: 5000,
    timeoutMsg: 'PIN accepted but the token never appeared',
  });
}

describe('Env-managed settings', () => {
  before(async () => {
    await waitForAppReady();
    await navigateTo('settings');
  });

  it('reports the pinned fields over the API', async () => {
    const ha = await invokeTauri<Record<string, unknown>>('get_ha_settings');
    expect(ha.urlFromEnv).toBe(true);
    expect(ha.tokenFromEnv).toBe(true);
    // The secret must not be anywhere in the response (task 69)
    expect(JSON.stringify(ha)).not.toContain(HA_TOKEN_FIXTURE);

    const paperless = await invokeTauri<Record<string, unknown>>('get_paperless_settings');
    expect(paperless.urlFromEnv).toBe(true);
    expect(paperless.tokenFromEnv).toBe(true);
    expect(paperless.enabledFromEnv).toBe(true);
    expect(JSON.stringify(paperless)).not.toContain(PAPERLESS_TOKEN_FIXTURE);
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

  it('requires the PIN to reveal an env token, and rejects a wrong one', async () => {
    const tokenInput = await $('#ha-token');
    // The secret is NOT in the page — only a mask of the right shape
    expect(await tokenInput.getValue()).not.toBe(HA_TOKEN_FIXTURE);

    const eye = await $('[data-test="reveal-ha-token"]');
    await eye.click();

    const modal = await $('[data-test="reveal-pin-modal"]');
    await modal.waitForDisplayed({ timeout: 5000 });

    // Wrong PIN: stays open, shows the backend's message, reveals nothing
    await (await $('[data-test="reveal-pin-input"]')).setValue('0000');
    await (await $('[data-test="reveal-pin-submit"]')).click();
    const error = await $('[data-test="reveal-pin-error"]');
    await error.waitForDisplayed({ timeout: 5000 });
    expect(await tokenInput.getValue()).not.toBe(HA_TOKEN_FIXTURE);

    // Correct PIN reveals
    await (await $('[data-test="reveal-pin-input"]')).setValue(REVEAL_PIN);
    await (await $('[data-test="reveal-pin-submit"]')).click();
    await modal.waitForDisplayed({ reverse: true, timeout: 5000 });
    await browser.waitUntil(async () => (await tokenInput.getValue()) === HA_TOKEN_FIXTURE, {
      timeout: 5000,
      timeoutMsg: 'correct PIN did not reveal the token',
    });

    // Leave the field masked so later tests start from a known state
    await eye.click();
  });

  it('asks for the PIN again after re-masking', async () => {
    // Start from a clean mount rather than inheriting the previous test's UI state
    await navigateTo('trips');
    await navigateTo('settings');

    const tokenInput = await $('#ha-token');
    const eye = await $('[data-test="reveal-ha-token"]');
    await eye.waitForDisplayed();
    expect(await tokenInput.getValue()).not.toBe(HA_TOKEN_FIXTURE);

    await revealWithPin(eye, tokenInput);

    // Re-mask
    await eye.click();
    await browser.waitUntil(async () => (await tokenInput.getValue()) !== HA_TOKEN_FIXTURE, {
      timeout: 5000,
      timeoutMsg: 'token stayed revealed after re-masking',
    });

    // Nothing was cached, so this prompts again rather than revealing outright
    await revealWithPin(eye, tokenInput);
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
