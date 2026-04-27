/**
 * Tier 2: Time Inference Toggle + Toast + Undo Integration Tests
 *
 * Tests the route-based start/end-time inference feature:
 * - Default OFF: backend returns no inferred times, user-typed values are kept.
 * - Toggle ON: backend supplies inferred times, frontend shows undo toast.
 * - Undo restores the user's typed values.
 *
 * Backend math (jitter, RNG bounds) is covered by Rust unit tests.
 * This spec verifies the UI wiring.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, invokeTauri } from '../../utils/db';

const ORIGIN = 'Bratislava';
const DESTINATION = 'Trnava';
const ROW_DATE = '2026-04-27';
const TYPED_START = `${ROW_DATE}T07:30`;
const TYPED_END = `${ROW_DATE}T09:30`;

async function setInferTripTimesViaIpc(enabled: boolean): Promise<void> {
  await invokeTauri<void>('set_infer_trip_times', { enabled });
}

async function getInferTripTimesViaIpc(): Promise<boolean> {
  return invokeTauri<boolean>('get_infer_trip_times');
}

/**
 * Atomically set the value of an input (avoids per-keystroke side effects).
 */
async function setInputValue(selector: string, value: string): Promise<void> {
  await browser.execute(
    (sel: string, val: string) => {
      const input = document.querySelector(sel) as HTMLInputElement | null;
      if (input) {
        input.value = val;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
      }
    },
    selector,
    value
  );
}

/** Open a brand-new (blank) trip row. */
async function openNewRow(): Promise<void> {
  const newRecordBtn = await $('button.new-record');
  await newRecordBtn.waitForClickable({ timeout: 5000 });
  await newRecordBtn.click();
  await browser.pause(300);
}

/**
 * Pick an autocomplete suggestion via the dropdown — this is the only path
 * that fires the component's onSelect callback, which in turn triggers
 * TripRow's handleOriginSelect / handleDestinationSelect → tryInferTimes().
 *
 * Pattern mirrored from tier2/route-autocomplete.spec.ts.
 */
async function pickFromAutocomplete(
  testId: string,
  partial: string
): Promise<void> {
  const input = await $(`[data-testid="${testId}"]`);
  await input.waitForDisplayed({ timeout: 5000 });
  await input.click();
  await input.setValue(partial);

  // Wait for the dropdown showing matching suggestions to appear.
  await browser.waitUntil(
    async () => {
      const dropdowns = await $$('.autocomplete .dropdown');
      for (const dropdown of dropdowns) {
        if (await dropdown.isDisplayed()) {
          return true;
        }
      }
      return false;
    },
    { timeout: 5000, timeoutMsg: `Autocomplete dropdown for ${testId} did not appear` }
  );

  // Click the first visible suggestion (only one will be visible at a time).
  const dropdowns = await $$('.autocomplete .dropdown');
  for (const dropdown of dropdowns) {
    if (await dropdown.isDisplayed()) {
      const suggestion = await dropdown.$('.suggestion');
      await suggestion.click();
      break;
    }
  }
  await browser.pause(200);
}

describe('Tier 2: Time Inference Toggle + Toast + Undo', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    // Reset to default (OFF) at the start of every test.
    await setInferTripTimesViaIpc(false);

    // Seed test vehicle with a historical Bratislava → Trnava trip the
    // backend can infer from. Historical times become the basis for
    // inference (with jitter applied in Rust).
    const vehicle = await seedVehicle({
      name: 'Inference Test Vehicle',
      licensePlate: 'INF-001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);

    // Historical trip: BA → TT yesterday, 08:00 → 09:00.
    await seedTrip({
      vehicleId,
      startDatetime: '2026-04-26T08:00',
      endDatetime: '2026-04-26T09:00',
      origin: ORIGIN,
      destination: DESTINATION,
      distanceKm: 50,
      odometer: 50050,
      purpose: 'Business trip',
    });

    await browser.refresh();
    await waitForAppReady();
  });

  it('keeps typed times when setting is OFF (default)', async () => {
    expect(await getInferTripTimesViaIpc()).toBe(false);

    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    await openNewRow();

    // Type the user's start/end times BEFORE picking origin/destination,
    // so we can verify they survive the route-pick step.
    await setInputValue('[data-testid="trip-start-datetime"]', TYPED_START);
    await setInputValue('[data-testid="trip-end-datetime"]', TYPED_END);

    // Pick origin/destination via autocomplete to fire onSelect callbacks.
    await pickFromAutocomplete('trip-origin', ORIGIN.slice(0, 4));
    await pickFromAutocomplete('trip-destination', DESTINATION.slice(0, 4));

    // Allow the gated backend call to settle (returns None when OFF).
    await browser.pause(800);

    // Backend returned None, so typed times must still be intact.
    const startVal = await $('[data-testid="trip-start-datetime"]').getValue();
    const endVal = await $('[data-testid="trip-end-datetime"]').getValue();
    expect(startVal).toBe(TYPED_START);
    expect(endVal).toBe(TYPED_END);

    // No toast should have been shown.
    const toasts = await $$('.toast');
    expect(toasts.length).toBe(0);
  });

  it('replaces typed times AND shows undo toast when setting is ON', async () => {
    await setInferTripTimesViaIpc(true);
    expect(await getInferTripTimesViaIpc()).toBe(true);

    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    await openNewRow();

    await setInputValue('[data-testid="trip-start-datetime"]', TYPED_START);
    await setInputValue('[data-testid="trip-end-datetime"]', TYPED_END);

    await pickFromAutocomplete('trip-origin', ORIGIN.slice(0, 4));
    await pickFromAutocomplete('trip-destination', DESTINATION.slice(0, 4));

    // Wait for the toast (action button) to appear.
    const actionBtn = await $('.toast-action');
    await actionBtn.waitForDisplayed({ timeout: 3000 });

    // Action label must match either Slovak or English copy.
    const actionLabel = (await actionBtn.getText()).trim();
    expect(actionLabel).toMatch(/^(Vrátiť|Undo)$/);

    // Start datetime should no longer be the user's typed value
    // (backend supplied a different time; jitter means we don't pin exact).
    const startVal = await $('[data-testid="trip-start-datetime"]').getValue();
    expect(startVal).not.toBe(TYPED_START);
    expect(startVal.startsWith(ROW_DATE)).toBe(true); // Same row date
  });

  it('Undo button restores the typed start and end times', async () => {
    await setInferTripTimesViaIpc(true);

    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    await openNewRow();

    await setInputValue('[data-testid="trip-start-datetime"]', TYPED_START);
    await setInputValue('[data-testid="trip-end-datetime"]', TYPED_END);

    await pickFromAutocomplete('trip-origin', ORIGIN.slice(0, 4));
    await pickFromAutocomplete('trip-destination', DESTINATION.slice(0, 4));

    const actionBtn = await $('.toast-action');
    await actionBtn.waitForDisplayed({ timeout: 3000 });

    // Click the Undo action.
    await actionBtn.click();
    await browser.pause(300);

    // Typed values must be restored.
    const startVal = await $('[data-testid="trip-start-datetime"]').getValue();
    const endVal = await $('[data-testid="trip-end-datetime"]').getValue();
    expect(startVal).toBe(TYPED_START);
    expect(endVal).toBe(TYPED_END);
  });
});
