/**
 * Tier 2: Paperless-ngx Integration
 *
 * End-to-end flow against a mock Paperless HTTP server:
 *   1. Configure Paperless URL + token in Settings → connection probe succeeds.
 *   2. Doklady page renders 3 invoice rows from the mock (1 fuel, 2 car).
 *   3. Assigning a fuel doc to a trip persists across a Refresh click.
 *   4. Clearing the Paperless URL via IPC reverts Doklady to local-receipts mode.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  invokeTauri,
  seedVehicle,
  seedTrip,
  setActiveVehicle,
} from '../../utils/db';
import {
  startMockPaperless,
  stopMockPaperless,
  MOCK_PAPERLESS_TOKEN,
} from '../_helpers/mock-paperless-server';

describe('Tier 2: Paperless Integration', () => {
  let mockUrl: string;
  const year = 2026;

  before(async () => {
    mockUrl = await startMockPaperless();
  });

  after(async () => {
    // Always clear Paperless settings so subsequent specs start in local mode.
    // Pass empty strings (not null) — backend treats None as "don't change",
    // empty string as "clear".
    try {
      await invokeTauri<void>('save_paperless_settings', { url: '', token: '' });
    } catch {
      // Best-effort — if the app is gone or already cleared, ignore.
    }
    await stopMockPaperless();
  });

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    // Clear any lingering doc assignments from previous retry attempts —
    // the WDIO test data dir is shared across retries within a session.
    for (const docId of [435, 423, 391]) {
      try {
        await invokeTauri<void>('unassign_paperless_doc', { docId });
      } catch {
        // No-op when there's no link to remove.
      }
    }
  });

  it('configure → render → assign → toggle off restores local mode', async () => {
    // ----- 1. Seed a vehicle and at least one trip in 2026 -------------------
    const vehicle = await seedVehicle({
      name: 'Paperless Test Car',
      licensePlate: 'PAP-001',
      initialOdometer: 10000,
      tankSizeLiters: 60,
      tpConsumption: 6.5,
    });

    await seedTrip({
      vehicleId: vehicle.id as string,
      startDatetime: `${year}-04-26T08:00`,
      endDatetime: `${year}-04-26T09:00`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 10050,
      purpose: 'Sluzobna cesta',
    });

    await seedTrip({
      vehicleId: vehicle.id as string,
      startDatetime: `${year}-04-27T08:00`,
      endDatetime: `${year}-04-27T09:30`,
      origin: 'Trnava',
      destination: 'Nitra',
      distanceKm: 60,
      odometer: 10110,
      purpose: 'Sluzobna cesta',
    });

    await setActiveVehicle(vehicle.id as string);

    // ----- 2. Configure Paperless in Settings via IPC -----------------------
    // We use IPC for save_paperless_settings (more reliable than typing into
    // a debounced input) and only verify the UI status indicator goes green.
    await invokeTauri<void>('save_paperless_settings', {
      url: mockUrl,
      token: MOCK_PAPERLESS_TOKEN,
    });

    await navigateTo('settings');
    await browser.pause(500);

    // Wait for the Paperless URL input to be populated by onMount.
    // Settings onMount runs ~12 sequential IPC calls before reaching the
    // Paperless section; on slow CI runners this can take several seconds.
    // Anchoring on the input value (which is Svelte-bound to `paperlessUrl`)
    // ensures we wait exactly as long as needed — and gives a clear error if
    // onMount aborts early (e.g. an uncaught IPC error earlier in the chain).
    const urlInput = await $('[data-test="paperless-url"]');
    await browser.waitUntil(
      async () => ((await urlInput.getValue()) ?? '').length > 0,
      {
        timeout: 15000,
        timeoutMsg: 'Paperless URL never populated in Settings input — onMount may have failed early',
      }
    );

    // Once paperlessUrl is set, testPaperlessConnectionStatus() fires immediately
    // and flips the status from IDLE → TESTING → CONNECTED. The badge renders
    // the moment status leaves IDLE, so it should appear within ~1-2 seconds.
    const statusBadge = await $('[data-test="paperless-status"]');
    await statusBadge.waitForDisplayed({ timeout: 5000 });
    await browser.waitUntil(
      async () => {
        const cls = (await statusBadge.getAttribute('class')) || '';
        return cls.includes('connected') && !cls.includes('disconnected');
      },
      {
        timeout: 10000,
        timeoutMsg: 'Paperless status badge never reached "connected"',
      }
    );

    // ----- 3. Doklady renders 3 paperless rows from the mock ----------------
    await navigateTo('doklady');
    // Allow live HTTP fetch + Svelte render
    await browser.waitUntil(
      async () => {
        const rows = await $$('[data-test="paperless-row"]');
        return rows.length === 3;
      },
      {
        timeout: 10000,
        timeoutMsg: 'Expected exactly 3 paperless rows to render',
      }
    );

    const rows = await $$('[data-test="paperless-row"]');
    expect(rows.length).toBe(3);

    // Fuel doc 435 — title + liters
    const fuelRow = await $('[data-test="paperless-row"][data-doc-id="435"]');
    await fuelRow.waitForDisplayed({ timeout: 5000 });
    const fuelTitle = await fuelRow.$('[data-test="title"]');
    const fuelTitleText = (await fuelTitle.getText()).trim();
    expect(fuelTitleText).toContain('OMV Slovensko, s.r.o. - Scanned_20260427-1325');

    const fuelLiters = await fuelRow.$('[data-test="liters"]');
    const fuelLitersText = (await fuelLiters.getText()).trim();
    expect(fuelLitersText).toContain('63.34');

    // Car doc 423 — liters cell shows em-dash (non-fuel doc)
    const carRow = await $('[data-test="paperless-row"][data-doc-id="423"]');
    const carLiters = await carRow.$('[data-test="liters"]');
    const carLitersText = (await carLiters.getText()).trim();
    expect(carLitersText).toBe('—');

    // ----- 4. Assign fuel doc 435 to the first trip -------------------------
    const assignBtn = await fuelRow.$('[data-test="assign-btn"]');
    await assignBtn.waitForDisplayed({ timeout: 5000 });
    await assignBtn.click();

    const tripItems = await $$('[data-test="paperless-trip-item"]');
    expect(tripItems.length).toBeGreaterThanOrEqual(1);
    await tripItems[0].click();

    // Wait for the row to re-render with a trip indicator visible.
    await browser.waitUntil(
      async () => {
        const indicator = await $(
          '[data-test="paperless-row"][data-doc-id="435"] [data-test="trip-indicator"]'
        );
        return indicator.isDisplayed();
      },
      {
        timeout: 5000,
        timeoutMsg: 'Trip indicator never appeared after assignment',
      }
    );

    // ----- 5. Click Refresh — assignment persists ---------------------------
    const refreshBtn = await $('[data-test="paperless-refresh"]');
    await refreshBtn.click();

    await browser.waitUntil(
      async () => {
        const indicator = await $(
          '[data-test="paperless-row"][data-doc-id="435"] [data-test="trip-indicator"]'
        );
        return indicator.isDisplayed();
      },
      {
        timeout: 5000,
        timeoutMsg: 'Trip indicator did not persist across Refresh',
      }
    );

    // ----- 6. Disable Paperless toggle → Doklady reverts to local mode ------
    // Use enabled:false — credentials are preserved, only mode switches.
    await invokeTauri<void>('save_paperless_settings', { url: null, token: null, enabled: false });

    // Force a full page remount (SvelteKit may keep route components mounted).
    await navigateTo('trips');
    await browser.pause(300);
    await navigateTo('doklady');
    await browser.pause(800);

    const paperlessRowsAfter = await $$('[data-test="paperless-row"]');
    expect(paperlessRowsAfter.length).toBe(0);

    // The local-mode header (Scan / Recognize buttons) should now be present.
    // Easiest selector-free assertion: paperless-refresh button is gone.
    const refreshAfter = await $('[data-test="paperless-refresh"]');
    expect(await refreshAfter.isExisting()).toBe(false);

    // ----- 7. Re-enable Paperless → rows load again -------------------------
    await invokeTauri<void>('save_paperless_settings', { url: null, token: null, enabled: true });

    await navigateTo('trips');
    await browser.pause(300);
    await navigateTo('doklady');

    await browser.waitUntil(
      async () => {
        const r = await $$('[data-test="paperless-row"]');
        return r.length === 3;
      },
      { timeout: 10000, timeoutMsg: 'Paperless rows did not reload after re-enabling' }
    );
  });

  it('persists custom field names and exposes them via Settings UI', async () => {
    // Configure Paperless first so the section is meaningful.
    await invokeTauri<void>('save_paperless_settings', {
      url: mockUrl,
      token: MOCK_PAPERLESS_TOKEN,
      enabled: true,
    });

    // Set custom field names via IPC (exercises the new save params).
    await invokeTauri<void>('save_paperless_settings', {
      url: null,
      token: null,
      enabled: null,
      fieldNameDatetime: 'Dátum dokladu',
      fieldNameLiters: 'Litre',
      fieldNameTotal: 'Suma',
    });

    // Verify backend persisted them.
    type Resp = {
      url: string | null;
      hasToken: boolean;
      enabled: boolean;
      fieldNameDatetime: string;
      fieldNameLiters: string;
      fieldNameTotal: string;
    };
    const persisted = await invokeTauri<Resp>('get_paperless_settings');
    expect(persisted.fieldNameDatetime).toBe('Dátum dokladu');
    expect(persisted.fieldNameLiters).toBe('Litre');
    expect(persisted.fieldNameTotal).toBe('Suma');

    // Navigate to Settings (force fresh mount per integration-tests rule).
    await navigateTo('trips');
    await browser.pause(200);
    await navigateTo('settings');
    await browser.pause(400);

    // Verify the inputs reflect the persisted custom values.
    const dt = await $('[data-test="paperless-field-datetime"]');
    const lt = await $('[data-test="paperless-field-liters"]');
    const tt = await $('[data-test="paperless-field-total"]');
    await dt.waitForDisplayed({ timeout: 5000 });
    expect(await dt.getValue()).toBe('Dátum dokladu');
    expect(await lt.getValue()).toBe('Litre');
    expect(await tt.getValue()).toBe('Suma');

    // Clear all three back to defaults via IPC (empty string clears).
    await invokeTauri<void>('save_paperless_settings', {
      url: null,
      token: null,
      enabled: null,
      fieldNameDatetime: '',
      fieldNameLiters: '',
      fieldNameTotal: '',
    });

    const defaulted = await invokeTauri<Resp>('get_paperless_settings');
    expect(defaulted.fieldNameDatetime).toBe('receipt_datetime');
    expect(defaulted.fieldNameLiters).toBe('litres');
    expect(defaulted.fieldNameTotal).toBe('total_amount');
  });
});
