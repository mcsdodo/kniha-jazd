/**
 * Tier 2: Multi-Invoice Integration Test (Task 66, Paperless-only in Task 84)
 *
 * UI flow for "1 Fuel + N Other invoices per trip" against a mock Paperless
 * server:
 * 1. Configure Paperless and seed a trip that covers the mock docs' dates.
 * 2. Assign 1 Fuel + 2 Other docs via the unified picker -- no mismatch-confirm
 *    dialog appears for the second Other (C8 regression guard at UI level).
 * 3. Grid DISPLAYS the summed other-costs total.
 * 4. Hand-edit other_costs_eur -> sum-mismatch warning + Slovak tooltip.
 * 5. Unassign one Other -> grid displays the reduced total, warning updates.
 *
 * The arithmetic itself (cent-exact money math, sum-on-assign, snapshot-based
 * unassign) is proven in backend unit tests -- this spec only verifies that the
 * UI triggers the backend and displays the results.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
  updateTrip,
  rpc,
} from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';
import {
  startMockPaperless,
  stopMockPaperless,
  MOCK_PAPERLESS_TOKEN,
} from '../_helpers/mock-paperless-server';

/** Real trip rows only -- excludes first-record/month-end synthetic rows and edit rows */
const TRIP_ROW = '.trip-grid tbody tr:not(.synthetic-row):not(.editing)';

/** Mock Paperless doc IDs (see specs/_helpers/mock-paperless-server.ts). */
const FUEL_DOC = 435; // 63.34 L, 113.95 EUR, 2026-04-27
const OTHER_DOC_1 = 423; // 1.95 EUR, 2026-04-14
const OTHER_DOC_2 = 391; // 110.00 EUR, 2026-03-27

/**
 * Force Slovak locale and reload. The wdio beforeTest hook resets the locale
 * to English before every test; this test asserts the actual translated
 * Slovak tooltip text, so it flips the app back to Slovak first.
 */
async function forceSlovakLocale(): Promise<void> {
  await browser.execute(() => {
    localStorage.setItem('kniha-jazd-locale', 'sk');
  });
  await browser.refresh();
  await waitForAppReady();
}

/**
 * Assign a Paperless doc to a trip through the doklady picker UI.
 * Asserts that NO mismatch-confirm dialog appears (every assignment in this
 * scenario must be a clean "matches" flow -- that is the C8 regression guard).
 */
async function assignViaPicker(
  docId: number,
  assignmentType: 'Fuel' | 'Other',
  tripId: string
): Promise<void> {
  const row = await $(`[data-test="paperless-row"][data-doc-id="${docId}"]`);
  await row.waitForDisplayed({ timeout: 5000 });

  const assignBtn = await row.$('[data-test="assign-btn"]');
  await assignBtn.waitForClickable({ timeout: 5000 });
  await assignBtn.click();

  // Step 1: pick our trip in the selector modal
  const tripItem = await $(`[data-test="trip-item"][data-trip-id="${tripId}"]`);
  await tripItem.waitForClickable({ timeout: 5000 });
  await tripItem.click();

  // Step 2: type selection -- wait for the radios to render
  const typeRadio = await $(`input[name="assignmentType"][value="${assignmentType}"]`);
  await typeRadio.waitForExist({ timeout: 5000 });
  await typeRadio.click();

  // C8 regression guard: the mismatch-confirm dialog must NOT appear
  const mismatchWarning = await $('.mismatch-warning');
  expect(await mismatchWarning.isExisting()).toBe(false);

  // The regular confirm button only renders when there is no mismatch
  const confirmBtn = await $('[data-test="confirm-assign-btn"]');
  await confirmBtn.waitForClickable({ timeout: 5000 });
  await confirmBtn.click();

  // Wait for the modal to close and the page to refresh its data
  const modal = await $('.modal-overlay');
  await modal.waitForDisplayed({ timeout: 5000, reverse: true });
  await browser.pause(400);
}

/** Read the other-costs cell of the single real trip row. */
async function getOtherCostsCell() {
  return $(`${TRIP_ROW} .col-other-costs`);
}

describe('Tier 2: Multi-Invoice (1 Fuel + N Other per trip)', () => {
  let mockUrl: string;
  const year = 2026;

  before(async () => {
    mockUrl = await startMockPaperless();
  });

  after(async () => {
    // Always clear Paperless settings so subsequent specs start unconfigured.
    // Pass empty strings (not null) -- backend treats None as "don't change",
    // empty string as "clear".
    try {
      await rpc<void>('save_paperless_settings', { url: '', token: '' });
    } catch {
      // Best-effort -- if the app is gone or already cleared, ignore.
    }
    await stopMockPaperless();
  });

  it('assigns 1 Fuel + 2 Other via picker, displays sum, flags and updates mismatch', async function () {
    // Long scenario: 3 picker assignments + several full page refreshes
    this.timeout(120000);

    await waitForAppReady();
    await forceSlovakLocale();

    // ----- 1. Configure Paperless --------------------------------------------
    await rpc<void>('save_paperless_settings', {
      url: mockUrl,
      token: MOCK_PAPERLESS_TOKEN,
      enabled: true,
    });

    // ----- 2. Seed vehicle and a trip covering every mock doc datetime --------
    const vehicle = await seedVehicle({
      name: 'Multi-Invoice Test Vehicle',
      licensePlate: 'MULTI-01',
      initialOdometer: 10000,
      tankSizeLiters: 60,
      tpConsumption: 6.5,
    });
    const vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);

    // The three mock docs fall on 2026-03-27, 2026-04-14 and 2026-04-27, so
    // the trip must span them: any doc datetime outside [start, end] would
    // raise a datetime warning and obscure the sum-mismatch assertions.
    const trip = await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-27T00:00`,
      endDatetime: `${year}-04-27T23:59`,
      origin: 'Bratislava',
      destination: 'Kosice',
      distanceKm: 400,
      odometer: 10400,
      purpose: 'Sluzobna cesta',
      fuelLiters: 63.34,
      fuelCostEur: 113.95,
      fullTank: true,
    });
    const tripId = trip.id as string;

    // ----- 3. Assign all three via the picker (no mismatch dialogs) ----------
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('doklady');
    await browser.waitUntil(
      async () => (await $$('[data-test="paperless-row"]')).length === 3,
      { timeout: 10000, timeoutMsg: 'Expected 3 paperless rows to render' }
    );

    await assignViaPicker(FUEL_DOC, 'Fuel', tripId);
    await assignViaPicker(OTHER_DOC_1, 'Other', tripId);
    // Second Other on a trip that already carries an Other invoice --
    // assignViaPicker asserts no mismatch-confirm dialog appears (C8).
    await assignViaPicker(OTHER_DOC_2, 'Other', tripId);

    // ----- 4. Grid displays the summed other-costs total --------------------
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    let otherCell = await getOtherCostsCell();
    await otherCell.waitForDisplayed({ timeout: 5000 });
    expect(await otherCell.getText()).toContain('111.95'); // 1.95 + 110.00

    // Totals match the attached invoices -- no warning of any kind
    let indicator = await otherCell.$('.receipt-indicator');
    expect(await indicator.isExisting()).toBe(false);

    // Fuel column is covered by the Fuel doc -- no missing-fuel warning
    const fuelCell = await $(`${TRIP_ROW} .col-fuel-liters`);
    const fuelIndicator = await fuelCell.$('.receipt-indicator');
    expect(await fuelIndicator.isExisting()).toBe(false);

    // ----- 5. Hand-edit other_costs_eur -> sum-mismatch warning + tooltip ---
    await updateTrip({
      id: tripId,
      startDatetime: trip.startDatetime,
      endDatetime: trip.endDatetime,
      origin: trip.origin,
      destination: trip.destination,
      distanceKm: trip.distanceKm,
      odometer: trip.odometer,
      purpose: trip.purpose,
      fuelLiters: trip.fuelLiters,
      fuelCostEur: trip.fuelCostEur,
      fullTank: trip.fullTank,
      otherCostsEur: 25.0, // != 111.95 invoice sum
    });

    await browser.refresh();
    await waitForAppReady();
    await waitForTripGrid();
    await browser.pause(500);

    otherCell = await getOtherCostsCell();
    expect(await otherCell.getText()).toContain('25.00');

    let mismatchIcon = await otherCell.$('.receipt-indicator.mismatch');
    await mismatchIcon.waitForExist({ timeout: 5000 });
    // Assert the visible translated Slovak text (not an i18n key)
    expect(await mismatchIcon.getAttribute('title')).toBe(
      'Suma iných nákladov nesedí so súčtom priradených dokladov (25.00 € vs 111.95 €)'
    );

    // ----- 6. Unassign one Other -> reduced total, warning updates ----------
    // Backend subtracts the applied snapshot (1.95): 25.00 - 1.95 = 23.05
    await rpc<void>('unassign_paperless_invoice', { docId: OTHER_DOC_1 });

    await browser.refresh();
    await waitForAppReady();
    await waitForTripGrid();
    await browser.pause(500);

    otherCell = await getOtherCostsCell();
    expect(await otherCell.getText()).toContain('23.05');

    // Still mismatched (23.05 vs remaining invoice sum 110.00) -- the warning
    // updated to reflect both the new total and the reduced invoice sum
    mismatchIcon = await otherCell.$('.receipt-indicator.mismatch');
    await mismatchIcon.waitForExist({ timeout: 5000 });
    expect(await mismatchIcon.getAttribute('title')).toBe(
      'Suma iných nákladov nesedí so súčtom priradených dokladov (23.05 € vs 110.00 €)'
    );
  });
});
