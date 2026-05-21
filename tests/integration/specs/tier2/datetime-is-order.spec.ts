/**
 * Tier 2: start_datetime is the single source of truth for trip order
 *
 * Target behaviour (Task 65):
 * - Trips are ordered strictly by their start_datetime (newest first).
 * - There is no manual ordering: no per-row up/down arrows, no manual-sort
 *   toggle in the grid header.
 * - Inserting trips out of date order (via "+1" or the "+" insert-above button)
 *   still produces a correctly ordered grid — there is no concept of a
 *   "date warning" row anymore (the date-warning CSS class is gone).
 * - Editing a trip's start datetime re-positions it in the grid.
 * - Deleting a trip in the middle preserves the chronological order of the
 *   remaining rows.
 *
 * These scenarios are written before the implementation changes (outside-in
 * TDD). They are expected to fail or error on `main` and pass once Tasks 2-10
 * are merged.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';

// Visible (non-editing, non-synthetic) trip rows in the grid.
const TRIP_ROW_SELECTOR =
  '.trip-grid tbody tr:not(.synthetic-row):not(.editing):not(.empty)';

/**
 * Read the start-date text from every visible trip row, top to bottom.
 * Rows render the start datetime as "DD.MM. HH:MM" — we slice the date prefix
 * so positional assertions can be made against e.g. "21.05.".
 */
async function getVisibleStartDates(): Promise<string[]> {
  const rows = await $$(TRIP_ROW_SELECTOR);
  const out: string[] = [];
  for (const row of rows) {
    const cell = await row.$('.col-start-datetime');
    const text = (await cell.getText()).trim();
    // Take up to the first space — leaves "DD.MM." even if time follows.
    const datePart = text.split(/\s+/)[0];
    out.push(datePart);
  }
  return out;
}

/**
 * Atomically set the value of a form input identified by `data-testid`
 * (avoids the multi-event/auto-calc pitfalls described in
 * .claude/rules/integration-tests.md).
 */
async function setFieldByTestId(testId: string, value: string): Promise<void> {
  await browser.execute(
    (sel: string, newValue: string) => {
      const input = document.querySelector(sel) as HTMLInputElement | null;
      if (input) {
        input.value = newValue;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
      }
    },
    `[data-testid="${testId}"]`,
    value
  );
}

/**
 * Fill the (currently open) editing row with a complete trip and save via
 * Enter. Used by Scenarios 1 / 6 which exercise the UI insertion path.
 */
async function fillEditingRowAndSave(opts: {
  startDatetime: string; // "YYYY-MM-DDTHH:MM"
  endDatetime: string;
  origin: string;
  destination: string;
  distanceKm: string;
  odometer: string;
  purpose: string;
}): Promise<void> {
  await setFieldByTestId('trip-start-datetime', opts.startDatetime);
  await setFieldByTestId('trip-end-datetime', opts.endDatetime);
  await setFieldByTestId('trip-origin', opts.origin);
  await setFieldByTestId('trip-destination', opts.destination);
  await setFieldByTestId('trip-distance', opts.distanceKm);
  await setFieldByTestId('trip-odometer', opts.odometer);
  await setFieldByTestId('trip-purpose', opts.purpose);
  await browser.pause(200);
  await browser.keys('Enter');
  await browser.pause(800);
}

describe('Tier 2: start_datetime is the single source of trip order', () => {
  let vehicleId: string;
  const year = 2026;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    const vehicle = await seedVehicle({
      name: 'Datetime Order Vehicle',
      licensePlate: 'DT-ORDER',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  it('Scenario 1: creating trips out of order via UI yields chronological grid with no warning rows', async () => {
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    // 1) "New record" → 21.05 trip
    const newRecordBtn = await $('button.new-record');
    await newRecordBtn.waitForClickable({ timeout: 5000 });
    await newRecordBtn.click();
    await browser.pause(300);
    await fillEditingRowAndSave({
      startDatetime: `${year}-05-21T08:00`,
      endDatetime: `${year}-05-21T09:00`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: '50',
      odometer: '50050',
      purpose: '21st trip',
    });

    // 2) Click "+" (insert above) on the 21.05 row → 18.05 trip
    let firstRow = (await $$(TRIP_ROW_SELECTOR))[0];
    const insertBtn1 = await firstRow.$('button.icon-btn.insert');
    await insertBtn1.waitForClickable({ timeout: 5000 });
    await insertBtn1.click();
    await browser.pause(300);
    await fillEditingRowAndSave({
      startDatetime: `${year}-05-18T08:00`,
      endDatetime: `${year}-05-18T09:00`,
      origin: 'Bratislava',
      destination: 'Nitra',
      distanceKm: '60',
      odometer: '50060',
      purpose: '18th trip',
    });

    // 3) Find the 18.05 row and click its "+" → 20.05 trip
    const rowsBefore3rd = await $$(TRIP_ROW_SELECTOR);
    let rowForMay18 = null;
    for (const row of rowsBefore3rd) {
      const cell = await row.$('.col-start-datetime');
      const text = (await cell.getText()).trim();
      if (text.startsWith('18.05.')) {
        rowForMay18 = row;
        break;
      }
    }
    expect(rowForMay18).not.toBeNull();
    const insertBtn2 = await rowForMay18!.$('button.icon-btn.insert');
    await insertBtn2.waitForClickable({ timeout: 5000 });
    await insertBtn2.click();
    await browser.pause(300);
    await fillEditingRowAndSave({
      startDatetime: `${year}-05-20T08:00`,
      endDatetime: `${year}-05-20T09:00`,
      origin: 'Bratislava',
      destination: 'Kosice',
      distanceKm: '70',
      odometer: '50070',
      purpose: '20th trip',
    });

    // Assert: visible order top → bottom is 21.05, 20.05, 18.05.
    const visibleDates = await getVisibleStartDates();
    expect(visibleDates.length).toBe(3);
    expect(visibleDates[0]).toBe('21.05.');
    expect(visibleDates[1]).toBe('20.05.');
    expect(visibleDates[2]).toBe('18.05.');

    // Assert: zero rows have the date-warning CSS class.
    const warningRows = await $$('tr.date-warning');
    expect(warningRows.length).toBe(0);
  });

  it("Scenario 2: editing a trip's start datetime moves it to the new chronological position", async () => {
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-05T08:00`,
      endDatetime: `${year}-05-05T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50010,
      purpose: 'May 5',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-12T08:00`,
      endDatetime: `${year}-05-12T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50020,
      purpose: 'May 12',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-21T08:00`,
      endDatetime: `${year}-05-21T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50030,
      purpose: 'May 21',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    // Find the row for 12.05 and double-click to enter edit mode.
    const rowsBefore = await $$(TRIP_ROW_SELECTOR);
    let rowForMay12 = null;
    for (const row of rowsBefore) {
      const cell = await row.$('.col-start-datetime');
      const text = (await cell.getText()).trim();
      if (text.startsWith('12.05.')) {
        rowForMay12 = row;
        break;
      }
    }
    expect(rowForMay12).not.toBeNull();
    await rowForMay12!.doubleClick();
    await browser.pause(400);

    // Change start_datetime to 25.05 and save.
    await setFieldByTestId('trip-start-datetime', `${year}-05-25T08:00`);
    await setFieldByTestId('trip-end-datetime', `${year}-05-25T09:00`);
    await browser.pause(200);
    await browser.keys('Enter');
    await browser.pause(800);

    // Assert new chronological order top → bottom: 25.05, 21.05, 5.05.
    const visibleDates = await getVisibleStartDates();
    expect(visibleDates.length).toBe(3);
    expect(visibleDates[0]).toBe('25.05.');
    expect(visibleDates[1]).toBe('21.05.');
    expect(visibleDates[2]).toBe('05.05.');
  });

  it('Scenario 3: up/down reorder arrows do not exist on any row', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-05T08:00`,
      endDatetime: `${year}-05-05T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50010,
      purpose: 'May 5',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-12T08:00`,
      endDatetime: `${year}-05-12T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50020,
      purpose: 'May 12',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-21T08:00`,
      endDatetime: `${year}-05-21T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50030,
      purpose: 'May 21',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    // No row should expose up/down reorder controls.
    const moveUpButtons = await $$('.trip-grid tbody button.icon-btn.move-up');
    const moveDownButtons = await $$(
      '.trip-grid tbody button.icon-btn.move-down'
    );
    expect(moveUpButtons.length).toBe(0);
    expect(moveDownButtons.length).toBe(0);
  });

  it('Scenario 4: manual sort mode toggle does not exist in the grid header', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-05T08:00`,
      endDatetime: `${year}-05-05T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50010,
      purpose: 'May 5',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    // No sortable column headers — start_datetime is the only ordering, and
    // the column header for it must not advertise itself as sortable.
    const sortableHeaders = await $$('.trip-grid thead th.sortable');
    expect(sortableHeaders.length).toBe(0);

    // No sort-direction indicators (▲/▼) anywhere in the header row.
    const sortIndicators = await $$('.trip-grid thead .sort-indicator');
    expect(sortIndicators.length).toBe(0);
  });

  it('Scenario 5: deleting a middle trip preserves chronological order', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-05T08:00`,
      endDatetime: `${year}-05-05T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50010,
      purpose: 'May 5',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-12T08:00`,
      endDatetime: `${year}-05-12T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50020,
      purpose: 'May 12',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-05-21T08:00`,
      endDatetime: `${year}-05-21T09:00`,
      origin: 'A',
      destination: 'B',
      distanceKm: 10,
      odometer: 50030,
      purpose: 'May 21',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    // Find the 12.05 row and click its delete button.
    const rowsBefore = await $$(TRIP_ROW_SELECTOR);
    let rowForMay12 = null;
    for (const row of rowsBefore) {
      const cell = await row.$('.col-start-datetime');
      const text = (await cell.getText()).trim();
      if (text.startsWith('12.05.')) {
        rowForMay12 = row;
        break;
      }
    }
    expect(rowForMay12).not.toBeNull();
    const deleteBtn = await rowForMay12!.$('button.icon-btn.delete');
    await deleteBtn.waitForClickable({ timeout: 5000 });
    await deleteBtn.click();
    await browser.pause(300);

    // Confirm in the modal — the danger button carries the localized "Delete"
    // label (common.delete → "Delete" in English).
    const confirmBtn = await $('.modal .button-small.danger');
    await confirmBtn.waitForClickable({ timeout: 5000 });
    await confirmBtn.click();
    await browser.pause(800);

    // Remaining rows: 21.05 (top), 5.05 (bottom).
    const visibleDates = await getVisibleStartDates();
    expect(visibleDates.length).toBe(2);
    expect(visibleDates[0]).toBe('21.05.');
    expect(visibleDates[1]).toBe('05.05.');
  });

  it('Scenario 6: regression guard — date-warning rows are impossible after out-of-order UI insertion', async () => {
    // Same setup as Scenario 1; this is the explicit regression guard for the
    // red "date-warning" row class, which must not exist anywhere in the DOM
    // after creating trips out of chronological order via the UI.
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    const newRecordBtn = await $('button.new-record');
    await newRecordBtn.waitForClickable({ timeout: 5000 });
    await newRecordBtn.click();
    await browser.pause(300);
    await fillEditingRowAndSave({
      startDatetime: `${year}-05-21T08:00`,
      endDatetime: `${year}-05-21T09:00`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: '50',
      odometer: '50050',
      purpose: '21st trip',
    });

    let firstRow = (await $$(TRIP_ROW_SELECTOR))[0];
    const insertBtn1 = await firstRow.$('button.icon-btn.insert');
    await insertBtn1.waitForClickable({ timeout: 5000 });
    await insertBtn1.click();
    await browser.pause(300);
    await fillEditingRowAndSave({
      startDatetime: `${year}-05-18T08:00`,
      endDatetime: `${year}-05-18T09:00`,
      origin: 'Bratislava',
      destination: 'Nitra',
      distanceKm: '60',
      odometer: '50060',
      purpose: '18th trip',
    });

    const rowsBefore3rd = await $$(TRIP_ROW_SELECTOR);
    let rowForMay18 = null;
    for (const row of rowsBefore3rd) {
      const cell = await row.$('.col-start-datetime');
      const text = (await cell.getText()).trim();
      if (text.startsWith('18.05.')) {
        rowForMay18 = row;
        break;
      }
    }
    expect(rowForMay18).not.toBeNull();
    const insertBtn2 = await rowForMay18!.$('button.icon-btn.insert');
    await insertBtn2.waitForClickable({ timeout: 5000 });
    await insertBtn2.click();
    await browser.pause(300);
    await fillEditingRowAndSave({
      startDatetime: `${year}-05-20T08:00`,
      endDatetime: `${year}-05-20T09:00`,
      origin: 'Bratislava',
      destination: 'Kosice',
      distanceKm: '70',
      odometer: '50070',
      purpose: '20th trip',
    });

    // Explicit regression guard: no <tr> may carry the date-warning class.
    const warningRows = await $$('tr.date-warning');
    expect(warningRows.length).toBe(0);

    // And no element should match any "date-warning" descendant class either —
    // this guards against accidental migration to a child-element warning.
    const anyDateWarning = await $$('[class*="date-warning"]');
    expect(anyDateWarning.length).toBe(0);
  });
});
