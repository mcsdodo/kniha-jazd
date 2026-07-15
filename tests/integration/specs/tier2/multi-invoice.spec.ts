/**
 * Tier 2: Multi-Invoice Integration Test (Task 66)
 *
 * UI flow for "1 Fuel + N Other invoices per trip":
 * 1. Seed trip + 1 Fuel + 2 Other receipts (via seedReceipt helper)
 * 2. Assign Fuel, then both Others via the unified picker — no mismatch-confirm
 *    dialog appears for the second Other (C8 regression guard at UI level)
 * 3. Grid DISPLAYS the summed other-costs total
 * 4. Hand-edit other_costs_eur → sum-mismatch ⚠ + Slovak tooltip in the grid
 * 5. Unassign one Other → grid displays the reduced total, ⚠ state updates
 *
 * The arithmetic itself (cent-exact money math, sum-on-assign, snapshot-based
 * unassign) is proven in backend unit tests — this spec only verifies that the
 * UI triggers the backend and displays the results.
 *
 * Docker mode is skipped: seedReceipt writes placeholder files that the
 * backend must see on the same filesystem.
 */

import { rmSync } from 'fs';
import { join } from 'path';
import { waitForAppReady, navigateTo } from '../../utils/app';
import {
  seedVehicle,
  seedTrip,
  seedReceipt,
  deleteReceipt,
  setActiveVehicle,
  updateTrip,
  invokeTauri,
} from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';
import { describeNotInDockerMode } from '../../utils/skip';
import type { Receipt } from '../../fixtures/types';

/** Real trip rows only — excludes first-record/month-end synthetic rows and edit rows */
const TRIP_ROW = '.trip-grid tbody tr:not(.synthetic-row):not(.editing)';

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
 * Assign a seeded receipt to a trip through the doklady picker UI.
 * Asserts that NO mismatch-confirm dialog appears (every assignment in this
 * scenario must be a clean "matches" flow — that is the C8 regression guard).
 */
async function assignViaPicker(
  receipt: Receipt,
  assignmentType: 'Fuel' | 'Other',
  tripId: string
): Promise<void> {
  // Find the receipt card by its unique file name (unassigned section).
  // Strip the extension — WDIO treats selectors ending in ".png" as an
  // image-file locator strategy instead of a partial-text match.
  const nameToken = (receipt.fileName ?? '').replace(/\.[a-z0-9]+$/i, '');
  const card = await $(`.receipt-card*=${nameToken}`);
  await card.waitForDisplayed({ timeout: 5000 });

  // The only primary button on an unassigned card is "Assign to trip"
  const assignBtn = await card.$('.button-small.primary');
  await assignBtn.waitForClickable({ timeout: 5000 });
  await assignBtn.click();

  // Step 1: pick our trip in the selector modal
  const tripItem = await $(`[data-test="trip-item"][data-trip-id="${tripId}"]`);
  await tripItem.waitForClickable({ timeout: 5000 });
  await tripItem.click();

  // Step 2: type selection — wait for the radios to render
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

describeNotInDockerMode('Tier 2: Multi-Invoice (1 Fuel + N Other per trip)', () => {
  const seededReceiptIds: string[] = [];

  // Receipts are NOT vehicle-scoped and the beforeTest DB cleanup does not
  // reliably clear cross-spec state — leftover seeded receipts poison later
  // specs (receipts.spec picks them up via getReceipts). Delete rows AND the
  // placeholder files so a stray re-scan cannot resurrect them.
  after(async () => {
    for (const id of seededReceiptIds) {
      try {
        await deleteReceipt(id);
      } catch {
        // already gone or app shutting down — best-effort cleanup
      }
    }
    const dataDir = process.env.KNIHA_JAZD_DATA_DIR;
    if (dataDir) {
      rmSync(join(dataDir, 'seeded-receipts'), { recursive: true, force: true });
    }
  });

  it('assigns 1 Fuel + 2 Other via picker, displays sum, flags and updates mismatch', async function () {
    // Long scenario: 3 picker assignments + several full page refreshes
    this.timeout(120000);

    await waitForAppReady();
    await forceSlovakLocale();

    const year = new Date().getFullYear();

    // ----- 1. Seed vehicle, trip, and 3 receipts ---------------------------
    const vehicle = await seedVehicle({
      name: 'Multi-Invoice Test Vehicle',
      licensePlate: 'MULTI-01',
      initialOdometer: 10000,
      tankSizeLiters: 60,
      tpConsumption: 6.5,
    });
    const vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);

    // Trip with fuel data matching the Fuel receipt exactly (clean "matches"
    // picker flow) and NO other costs yet (first Other populates the field).
    const trip = await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-10T08:00`,
      endDatetime: `${year}-05-10T18:00`,
      origin: 'Bratislava',
      destination: 'Kosice',
      distanceKm: 400,
      odometer: 10400,
      purpose: 'Sluzobna cesta',
      fuelLiters: 40.0,
      fuelCostEur: 60.0,
      fullTank: true,
    });
    const tripId = trip.id as string;

    // All receipt datetimes inside the trip range (08:00–18:00 on 05-10)
    const fuelReceipt = await seedReceipt({
      assignmentType: 'Fuel',
      liters: 40.0,
      totalPriceEur: 60.0,
      receiptDatetime: `${year}-05-10T10:00`,
    });
    const otherReceipt1 = await seedReceipt({
      assignmentType: 'Other',
      totalPriceEur: 10.0,
      receiptDatetime: `${year}-05-10T11:00`,
      vendorName: 'Parking Central',
      costDescription: 'Parkovanie 2h',
    });
    const otherReceipt2 = await seedReceipt({
      assignmentType: 'Other',
      totalPriceEur: 5.01,
      receiptDatetime: `${year}-05-10T12:00`,
      vendorName: 'AutoWash Express',
      costDescription: 'Umytie auta',
    });
    seededReceiptIds.push(
      fuelReceipt.id as string,
      otherReceipt1.id as string,
      otherReceipt2.id as string
    );

    // ----- 2. Assign all three via the picker (no mismatch dialogs) --------
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('doklady');
    await browser.pause(500);

    await assignViaPicker(fuelReceipt, 'Fuel', tripId);
    await assignViaPicker(otherReceipt1, 'Other', tripId);
    // Second Other on a trip that already carries an Other invoice —
    // assignViaPicker asserts no mismatch-confirm dialog appears (C8).
    await assignViaPicker(otherReceipt2, 'Other', tripId);

    // ----- 3. Grid displays the summed other-costs total -------------------
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    let otherCell = await getOtherCostsCell();
    await otherCell.waitForDisplayed({ timeout: 5000 });
    expect(await otherCell.getText()).toContain('15.01'); // 10.00 + 5.01

    // Totals match the attached invoices — no warning of any kind
    let indicator = await otherCell.$('.receipt-indicator');
    expect(await indicator.isExisting()).toBe(false);

    // Fuel column is covered by the Fuel receipt — no missing-fuel warning
    const fuelCell = await $(`${TRIP_ROW} .col-fuel-liters`);
    const fuelIndicator = await fuelCell.$('.receipt-indicator');
    expect(await fuelIndicator.isExisting()).toBe(false);

    // ----- 4. Hand-edit other_costs_eur → sum-mismatch ⚠ + Slovak tooltip --
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
      otherCostsEur: 25.0, // != 15.01 invoice sum
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
      'Suma iných nákladov nesedí so súčtom priradených dokladov (25.00 € vs 15.01 €)'
    );

    // ----- 5. Unassign one Other → reduced total, ⚠ state updates ----------
    // Backend subtracts the applied snapshot (5.01): 25.00 - 5.01 = 19.99
    await invokeTauri<void>('unassign_invoice', {
      invoiceRef: { source: 'receipt', id: otherReceipt2.id as string },
    });

    await browser.refresh();
    await waitForAppReady();
    await waitForTripGrid();
    await browser.pause(500);

    otherCell = await getOtherCostsCell();
    expect(await otherCell.getText()).toContain('19.99');

    // Still mismatched (19.99 vs remaining invoice sum 10.00) — the ⚠ state
    // updated to reflect both the new total and the reduced invoice sum
    mismatchIcon = await otherCell.$('.receipt-indicator.mismatch');
    await mismatchIcon.waitForExist({ timeout: 5000 });
    expect(await mismatchIcon.getAttribute('title')).toBe(
      'Suma iných nákladov nesedí so súčtom priradených dokladov (19.99 € vs 10.00 €)'
    );
  });
});
