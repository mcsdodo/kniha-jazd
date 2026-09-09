/**
 * Tier 2: the odometer cascade modal (Task 81, Task 10 of its plan)
 *
 * Editing a distance, inserting a trip or deleting one can shift every later
 * row of the year. The frontend asks the backend with `dryRun: true` first:
 * if the plan moves no other row and does not break the next year's chain,
 * the write happens silently. Otherwise a modal lists every row that moves,
 * and only Confirm writes it -- Cancel writes nothing, not even the edited
 * row itself.
 *
 * The arithmetic of the shift belongs to the backend unit tests (Task 1).
 * This spec only proves the UI -> backend -> display flow: did the save
 * reach the backend, did the modal (or its absence) match the plan, and did
 * the grid repaint with the answer.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

// Visible (non-editing, non-synthetic) trip rows in the grid -- same
// selector as odometer-chain-warnings.spec.ts and datetime-is-order.spec.ts.
const TRIP_ROW_SELECTOR =
  '.trip-grid tbody tr:not(.synthetic-row):not(.editing):not(.empty)';

const CASCADE_MODAL = '[data-testid="cascade-modal"]';
const CASCADE_SUMMARY = '[data-testid="cascade-summary"]';
const CASCADE_CONFIRM = '[data-testid="cascade-confirm"]';
const CASCADE_CANCEL = '[data-testid="cascade-cancel"]';

/**
 * Atomically set the value of a form input identified by `data-testid`
 * (avoids the multi-event/auto-calc pitfalls described in
 * .claude/rules/integration-tests.md). Matches the helper already used in
 * datetime-is-order.spec.ts.
 */
async function setFieldByTestId(testId: string, value: string): Promise<void> {
  const found = await browser.execute(
    (sel: string, newValue: string) => {
      const input = document.querySelector(sel) as HTMLInputElement | null;
      if (!input) return false;
      input.value = newValue;
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    },
    `[data-testid="${testId}"]`,
    value
  );
  if (!found) {
    throw new Error(`setFieldByTestId: no element for [data-testid="${testId}"]`);
  }
}

/** Find the visible trip row whose purpose cell matches exactly. */
async function findRowByPurpose(
  purpose: string
): Promise<WebdriverIO.Element | null> {
  const rows = await $$(TRIP_ROW_SELECTOR);
  for (const row of rows) {
    const text = (await row.$('.col-purpose').getText()).trim();
    if (text === purpose) return row;
  }
  return null;
}

/** Read the ODO cell of the row with this purpose (throws if not found). */
async function odoOf(purpose: string): Promise<string> {
  const row = await findRowByPurpose(purpose);
  if (!row) throw new Error(`odoOf: row with purpose "${purpose}" not found`);
  const text = (await row.$('.col-odo').getText()).trim();
  // Strip a trailing chain-indicator glyph, if the row happens to carry one.
  return text.split(/\s+/)[0];
}

async function waitForCascadeModal(timeout = 5000): Promise<void> {
  await $(CASCADE_MODAL).waitForDisplayed({ timeout });
}

async function waitForNoCascadeModal(timeout = 5000): Promise<void> {
  await $(CASCADE_MODAL).waitForExist({ timeout, reverse: true });
}

/** Number of rows currently in edit mode (0 or 1 in every scenario here). */
async function editingRowCount(): Promise<number> {
  return (await $$('.trip-grid tbody tr.editing')).length;
}

async function waitForEditingRowClosed(
  timeoutMsg: string,
  timeout = 5000
): Promise<void> {
  await browser.waitUntil(async () => (await editingRowCount()) === 0, {
    timeout,
    timeoutMsg,
  });
}

/** Double-click the row with this purpose and wait for it to open in edit mode. */
async function openRowForEdit(purpose: string, timeout = 5000): Promise<void> {
  const row = await findRowByPurpose(purpose);
  if (!row) throw new Error(`openRowForEdit: row "${purpose}" not found`);
  await row.doubleClick();
  await browser.waitUntil(async () => (await editingRowCount()) === 1, {
    timeout,
    timeoutMsg: `row "${purpose}" did not enter edit mode`,
  });
}

/** Wait for the live preview to fill the ODO field with this exact value. */
async function waitForOdometerPreview(
  expected: string,
  timeout = 5000
): Promise<void> {
  await browser.waitUntil(
    async () => (await $('[data-testid="trip-odometer"]').getValue()) === expected,
    { timeout, timeoutMsg: `odometer preview did not settle to ${expected}` }
  );
}

/** "YYYY-MM-DDTHH:MM" for the given day offset from Jan 1 of `year` (UTC-safe). */
function dateAt(year: number, dayOffset: number): string {
  const d = new Date(Date.UTC(year, 0, 1));
  d.setUTCDate(d.getUTCDate() + dayOffset);
  const mm = String(d.getUTCMonth() + 1).padStart(2, '0');
  const dd = String(d.getUTCDate()).padStart(2, '0');
  return `${year}-${mm}-${dd}T08:00`;
}

describe('Odometer cascade on save', () => {
  let vehicleId: string;
  // 2026 matches the project-wide `currentDate` in CLAUDE.md, keeping dates
  // inside the year picker's currently-active range.
  const year = 2026;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    const vehicle = await seedVehicle({
      name: 'Cascade Vehicle',
      licensePlate: 'CASCADE-01',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  /** Seed the standard three-row chain A(20km) -> B(30km) -> C(25km). */
  async function seedThreeRowChain(): Promise<void> {
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 60), // 2026-03-02
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 20,
      odometer: 50020,
      purpose: 'Row A',
    });
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 61), // 2026-03-03
      origin: 'Trnava',
      destination: 'Nitra',
      distanceKm: 30,
      odometer: 50050,
      purpose: 'Row B',
    });
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 62), // 2026-03-04
      origin: 'Nitra',
      destination: 'Zilina',
      distanceKm: 25,
      odometer: 50075,
      purpose: 'Row C',
    });
  }

  it('moves every later row when a distance is edited', async () => {
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    // Grow Row B's distance 30 -> 50. Its own ODO becomes anchor(50020) + 50
    // = 50070, and the +20 delta must carry forward onto Row C.
    await openRowForEdit('Row B');
    await setFieldByTestId('trip-distance', '50');
    await waitForOdometerPreview('50070');
    await browser.keys('Enter');

    await waitForCascadeModal();
    // The summary decomposes the shift (R3) -- proves the plan reached the
    // screen, not only that a modal of some kind appeared. A whole-kilometre
    // shift now prints as "+20", matching the odometers in the table below it
    // and the grid behind it (task 81, M2). A fractional shift still prints
    // its decimal.
    expect(await $(CASCADE_SUMMARY).getText()).toContain('+20 km');

    await $(CASCADE_CONFIRM).click();
    await waitForNoCascadeModal();
    await waitForEditingRowClosed(
      'Row B did not close after confirming the cascade'
    );

    await browser.waitUntil(async () => (await odoOf('Row C')) === '50095', {
      timeout: 5000,
      timeoutMsg: 'Row C odometer did not shift to 50095 after the cascade',
    });
    expect(await odoOf('Row B')).toBe('50070');
  });

  it('writes nothing when the modal is cancelled', async () => {
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    await openRowForEdit('Row B');
    await setFieldByTestId('trip-distance', '50');
    await waitForOdometerPreview('50070');
    await browser.keys('Enter');

    await waitForCascadeModal();
    await $(CASCADE_CANCEL).click();
    await waitForNoCascadeModal();

    // The row editor stays open on exactly what the user typed -- Cancel
    // does not revert the draft, it just never writes it.
    expect(await editingRowCount()).toBe(1);
    expect(await $('[data-testid="trip-distance"]').getValue()).toBe('50');

    // Nothing reached the database -- reload and check BOTH the edited row
    // and the later row it would have shifted. The two are one write; a
    // check on Row C alone would miss half the guarantee.
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    expect(await odoOf('Row B')).toBe('50050');
    expect(await odoOf('Row C')).toBe('50075');
  });

  it('blocks every cascade control while a row is open for edit', async () => {
    // Task 81, C1. TripRow re-seeds its `formData` from the `trip` prop only
    // while it is NOT editing, so a cascade started from another row moves the
    // open row's stored odometer while its editor keeps the old number. The
    // next save then submits an unchanged km with a changed odometer, the
    // backend takes its odo-wins branch, and it writes distance_km = odometer
    // - anchor -- inflated by exactly the shift. Nothing catches it: the value
    // grows rather than going negative, the span equals the km by
    // construction, and an inflated distance lowers the computed l/100km, so
    // it hides a BIZ-003 breach instead of inventing one.
    //
    // The fix is the gate this test pins: while an editor is open, no control
    // that can start a cascade is actionable.
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    await openRowForEdit('Row B');

    // TRIP_ROW_SELECTOR skips the editing row, so this is Row C's display row.
    const rowC = await findRowByPurpose('Row C');
    expect(rowC).not.toBeNull();
    expect(await rowC!.$('button.icon-btn.delete').isEnabled()).toBe(false);
    expect(await rowC!.$('button.icon-btn.insert').isEnabled()).toBe(false);
    // The control this gate was copied from, asserted here so the three stay
    // one pattern.
    expect(await rowC!.$('button.icon-btn.copy').isEnabled()).toBe(false);
    // A new trip dated before the open row cascades it just as an insert does.
    expect(await $('button.new-record').isEnabled()).toBe(false);

    // Not merely styled as disabled: a real click reaches nothing. A disabled
    // button fires no click handler, so this is the strongest form of the
    // check available in the page.
    for (const selector of ['button.icon-btn.delete', 'button.icon-btn.insert']) {
      const state = await browser.execute(
        (purpose: string, sel: string) => {
          const rows = Array.from(
            document.querySelectorAll(
              '.trip-grid tbody tr:not(.synthetic-row):not(.editing):not(.empty)'
            )
          );
          const row = rows.find(
            (r) => r.querySelector('.col-purpose')?.textContent?.trim() === purpose
          );
          const button = row?.querySelector(sel) as HTMLButtonElement | null;
          if (!button) return 'missing';
          button.click();
          return button.disabled ? 'disabled' : 'enabled';
        },
        'Row C',
        selector
      );
      expect(state).toBe('disabled');
    }

    // A second editor is the same seam by another door: saving it would
    // cascade the first one just as well.
    await rowC!.doubleClick();
    await browser.pause(500);
    expect(await editingRowCount()).toBe(1);

    // Nothing started: no modal of any kind is up -- neither the cascade
    // modal nor the plain delete confirmation (ConfirmModal.svelte), which
    // share the `.modal-overlay` wrapper.
    expect(await $(CASCADE_MODAL).isExisting()).toBe(false);
    expect(await $('.modal-overlay').isExisting()).toBe(false);

    // The gate lifts when the editor closes -- it is a gate, not a lock.
    await browser.keys('Escape');
    await waitForEditingRowClosed('Escape did not close the editor');
    const rowCAgain = await findRowByPurpose('Row C');
    expect(rowCAgain).not.toBeNull();
    await browser.waitUntil(
      async () => await rowCAgain!.$('button.icon-btn.delete').isEnabled(),
      { timeout: 5000, timeoutMsg: 'delete stayed disabled after the editor closed' }
    );
    expect(await rowCAgain!.$('button.icon-btn.insert').isEnabled()).toBe(true);
    expect(await $('button.new-record').isEnabled()).toBe(true);

    // The book is untouched by any of it.
    expect(await odoOf('Row B')).toBe('50050');
    expect(await odoOf('Row C')).toBe('50075');
  });

  it('does not open a row editor while an unsaved new row is open', async () => {
    // The mirror of the gate above (task 81, C1). A new row dated before an
    // existing one cascades it on save, so an editor opened next to a pending
    // new row is the same stale-editor seam by another door.
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    const newRecordBtn = await $('button.new-record');
    await newRecordBtn.waitForClickable({ timeout: 5000 });
    await newRecordBtn.click();
    // The new row is itself an editing row, so this is the only one open.
    await browser.waitUntil(async () => (await editingRowCount()) === 1, {
      timeout: 5000,
      timeoutMsg: 'new-record did not open a new editing row',
    });

    const rowC = await findRowByPurpose('Row C');
    expect(rowC).not.toBeNull();
    await rowC!.doubleClick();
    await browser.pause(500);
    expect(await editingRowCount()).toBe(1);
    expect(await findRowByPurpose('Row C')).not.toBeNull();
  });

  it('keeps the scroll position after a cascade', async function () {
    // 55 sequential seedTrip round trips alone can approach the default 30s
    // mocha timeout (wdio.server.conf.ts mochaOpts.timeout) before the test
    // even opens the grid. A non-arrow function is required for `this`.
    this.timeout(180000);

    // Seed enough rows that the page needs to scroll. Spread across two
    // months (2026 is not a leap year, so Feb tops out at 28) so every
    // seeded date stays valid.
    const rowCount = 55;
    for (let i = 1; i <= rowCount; i++) {
      await seedTrip({
        vehicleId,
        startDatetime: dateAt(year, i - 1), // Jan 1 .. Feb 24
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 10,
        odometer: 50000 + i * 10,
        purpose: `Scroll ${i}`,
      });
    }

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    // Which element actually scrolls: +layout.svelte pins `.app` to
    // `height: 100vh` and gives `main` (its flex-1 child, wrapping every
    // routed page) `overflow: auto`. So `document.documentElement` -- and
    // therefore `window.scrollY` -- never moves; `main` is the real
    // scroller. Measured directly here rather than assumed: an earlier
    // version of this test asserted on `window.scrollY` and the sanity
    // check below failed every time (scrollHeight === innerHeight exactly),
    // which is what exposed the difference.
    const scroller = 'main';

    // Sanity: the container must actually be tall enough to need scrolling,
    // or "scroll position unchanged" would pass trivially at 0 === 0.
    const scrollable = await browser.execute((sel: string) => {
      const el = document.querySelector(sel) as HTMLElement | null;
      return !!el && el.scrollHeight > el.clientHeight + 200;
    }, scroller);
    expect(scrollable).toBe(true);

    // Scroll a mid-list row into the middle of the scroller's viewport, so
    // it is a normal, on-screen row rather than one sitting at the very top
    // or bottom edge. Computed inside the page so it accounts for `main`'s
    // own scroll offset, not the document's.
    const targetPurpose = 'Scroll 30';
    const targetScrollTop = await browser.execute(
      (sel: string, purpose: string) => {
        const main = document.querySelector(sel) as HTMLElement | null;
        if (!main) return null;
        const rows = Array.from(
          document.querySelectorAll(
            '.trip-grid tbody tr:not(.synthetic-row):not(.editing):not(.empty)'
          )
        );
        const row = rows.find(
          (r) => r.querySelector('.col-purpose')?.textContent?.trim() === purpose
        );
        if (!row) return null;
        const rowRect = row.getBoundingClientRect();
        const mainRect = main.getBoundingClientRect();
        const offsetWithinMain = main.scrollTop + (rowRect.top - mainRect.top);
        return Math.max(0, Math.round(offsetWithinMain - main.clientHeight / 2));
      },
      scroller,
      targetPurpose
    );
    expect(targetScrollTop).not.toBeNull();
    await browser.execute(
      (sel: string, top: number) => {
        (document.querySelector(sel) as HTMLElement).scrollTop = top;
      },
      scroller,
      targetScrollTop as number
    );

    // Open the editor, THEN capture the baseline. A real WebDriver
    // double-click nudges `main.scrollTop` on its own (measured: it moved
    // our manually-set position every time, independent of the app code --
    // most likely the sticky `<thead>` making the driver's hit-testing
    // treat the row as partly covered). That nudge is native browser
    // behaviour, not the R6 regression under test, so the baseline is taken
    // once the editor is already open and the page has settled.
    await openRowForEdit(targetPurpose, 10000);
    const before = await browser.execute(
      (sel: string) => (document.querySelector(sel) as HTMLElement).scrollTop,
      scroller
    );
    expect(before).toBeGreaterThan(0);

    // Grow Scroll 30's distance 10 -> 25 (+15 km). This must cascade onto
    // Scroll 31 without TripGrid unmounting -- the regression under test.
    // Every wait below gets a longer timeout than the 3-row tests: a 55-row
    // reload (onTripsChanged + loadRoutes + loadPurposes + loadPlaces) is
    // heavier than the small-chain tests' reload.
    await setFieldByTestId('trip-distance', '25');
    await waitForOdometerPreview('50315', 10000); // anchor(Scroll 29 = 50290) + 25
    await browser.keys('Enter');

    await waitForCascadeModal(10000);
    await $(CASCADE_CONFIRM).click();
    await waitForNoCascadeModal(10000);
    await waitForEditingRowClosed(
      'Scroll 30 did not close after confirming the cascade',
      10000
    );

    // Proves the cascade actually executed (not a no-op that would
    // trivially preserve scroll for the wrong reason) before reading scroll.
    await browser.waitUntil(
      async () => (await odoOf('Scroll 31')) === '50325',
      {
        timeout: 10000,
        timeoutMsg: 'Scroll 31 odometer did not shift after the cascade',
      }
    );

    const after = await browser.execute(
      (sel: string) => (document.querySelector(sel) as HTMLElement).scrollTop,
      scroller
    );
    expect(after).toBe(before);
  });

  it('does not open the modal when only the purpose changes', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 60),
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 20,
      odometer: 50020,
      purpose: 'Original purpose',
    });
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 61),
      origin: 'Trnava',
      destination: 'Nitra',
      distanceKm: 30,
      odometer: 50050,
      purpose: 'Row B',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    await openRowForEdit('Original purpose');
    await setFieldByTestId('trip-purpose', 'Updated purpose');
    await browser.keys('Enter');

    // If a modal had opened, nothing here would ever click it, so this
    // would time out -- the common edit must stay a single silent write.
    await waitForEditingRowClosed(
      'a purpose-only save must not block on the cascade modal'
    );
    expect(await $(CASCADE_MODAL).isExisting()).toBe(false);
    expect(await findRowByPurpose('Updated purpose')).not.toBeNull();
  });

  it('shifts the later rows when a trip is inserted mid-year', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 60), // 2026-03-02
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 20,
      odometer: 50020,
      purpose: 'Row A',
    });
    // Gap on 03-03, filled in below by the insert.
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 62), // 2026-03-04
      origin: 'Nitra',
      destination: 'Zilina',
      distanceKm: 25,
      odometer: 50045,
      purpose: 'Row C',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    const rowA = await findRowByPurpose('Row A');
    expect(rowA).not.toBeNull();
    const insertBtn = await rowA!.$('button.icon-btn.insert');
    await insertBtn.waitForClickable({ timeout: 5000 });
    await insertBtn.click();
    await browser.waitUntil(async () => (await editingRowCount()) === 1, {
      timeout: 5000,
      timeoutMsg: 'insert-above did not open a new editing row',
    });

    await setFieldByTestId('trip-start-datetime', dateAt(year, 61)); // 03-03
    await setFieldByTestId('trip-end-datetime', dateAt(year, 61));
    await setFieldByTestId('trip-origin', 'Trnava');
    await setFieldByTestId('trip-destination', 'Nitra');
    await setFieldByTestId('trip-distance', '15');
    await setFieldByTestId('trip-purpose', 'Row B');
    await browser.pause(200);
    await browser.keys('Enter');

    await waitForCascadeModal();
    await $(CASCADE_CONFIRM).click();
    await waitForNoCascadeModal();
    await waitForEditingRowClosed(
      'the inserted row did not close after confirming the cascade'
    );

    expect(await findRowByPurpose('Row B')).not.toBeNull();
    await browser.waitUntil(async () => (await odoOf('Row C')) === '50060', {
      timeout: 5000,
      timeoutMsg: 'Row C odometer did not shift after the mid-year insert',
    });
  });

  it('does not open the modal when a trip is appended to the newest year', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 60), // 2026-03-02
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 20,
      odometer: 50020,
      purpose: 'Row A',
    });
    await seedTrip({
      vehicleId,
      startDatetime: dateAt(year, 61), // 2026-03-03
      origin: 'Trnava',
      destination: 'Nitra',
      distanceKm: 30,
      odometer: 50050,
      purpose: 'Row B',
    });

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    const newRecordBtn = await $('button.new-record');
    await newRecordBtn.waitForClickable({ timeout: 5000 });
    await newRecordBtn.click();
    await browser.waitUntil(async () => (await editingRowCount()) === 1, {
      timeout: 5000,
      timeoutMsg: 'new-record did not open a new editing row',
    });

    // Dated after both existing rows -- the daily action of appending to the
    // newest year. Nothing moves and there is no next year, so this must
    // stay a single silent write.
    await setFieldByTestId('trip-start-datetime', dateAt(year, 62)); // 03-04
    await setFieldByTestId('trip-end-datetime', dateAt(year, 62));
    await setFieldByTestId('trip-origin', 'Nitra');
    await setFieldByTestId('trip-destination', 'Zilina');
    await setFieldByTestId('trip-distance', '10');
    await setFieldByTestId('trip-purpose', 'Row C');
    await browser.pause(200);
    await browser.keys('Enter');

    await waitForEditingRowClosed(
      'appending the newest trip must stay a single silent write'
    );
    expect(await $(CASCADE_MODAL).isExisting()).toBe(false);
    expect(await findRowByPurpose('Row C')).not.toBeNull();
  });

  it('closes the gap when a trip is deleted', async () => {
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    const rowB = await findRowByPurpose('Row B');
    expect(rowB).not.toBeNull();
    const deleteBtn = await rowB!.$('button.icon-btn.delete');
    await deleteBtn.waitForClickable({ timeout: 5000 });
    await deleteBtn.click();

    // Deleting the middle row moves Row C, so the cascade modal gates it --
    // not the plain one-line delete confirmation.
    await waitForCascadeModal();
    await $(CASCADE_CONFIRM).click();
    await waitForNoCascadeModal();

    await browser.waitUntil(async () => (await findRowByPurpose('Row B')) === null, {
      timeout: 5000,
      timeoutMsg: 'Row B is still present after confirming the delete',
    });
    // Row C's start becomes Row B's start (Row A's ODO), plus Row C's own
    // distance: 50020 + 25 = 50045.
    await browser.waitUntil(async () => (await odoOf('Row C')) === '50045', {
      timeout: 5000,
      timeoutMsg: 'Row C did not close the gap after Row B was deleted',
    });
  });

  it('deletes nothing when the delete modal is cancelled', async () => {
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    const rowB = await findRowByPurpose('Row B');
    expect(rowB).not.toBeNull();
    const deleteBtn = await rowB!.$('button.icon-btn.delete');
    await deleteBtn.waitForClickable({ timeout: 5000 });
    await deleteBtn.click();

    await waitForCascadeModal();
    await $(CASCADE_CANCEL).click();
    await waitForNoCascadeModal();

    // Nothing happened yet, even before a reload.
    expect(await findRowByPurpose('Row B')).not.toBeNull();

    // Confirm against the stored book too: the row and every later
    // odometer must be exactly as they were.
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    expect(await findRowByPurpose('Row B')).not.toBeNull();
    expect(await odoOf('Row B')).toBe('50050');
    expect(await odoOf('Row C')).toBe('50075');
  });

  it('cancels an armed cascade when the click lands on the grid behind it', async () => {
    await seedThreeRowChain();

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(300);

    // Arm a cascade with no row in edit mode. Delete is the only write that
    // does this: the row it removes is a display row, so nothing is open
    // when the modal appears.
    const rowB = await findRowByPurpose('Row B');
    expect(rowB).not.toBeNull();
    const deleteBtn = await rowB!.$('button.icon-btn.delete');
    await deleteBtn.waitForClickable({ timeout: 5000 });
    await deleteBtn.click();
    await waitForCascadeModal();
    expect(await editingRowCount()).toBe(0);

    // Double-click Row C, one of the rows this delete would move. The modal
    // overlay is fixed over the whole viewport, so the first click hits the
    // overlay and cancels; only the second reaches the row. This is what
    // makes an editor opened here safe, and it is the reason ADR-046 needs
    // no `cascadePending` term in `TripRow.handleEdit` -- see the ADR.
    const rowC = await findRowByPurpose('Row C');
    expect(rowC).not.toBeNull();
    await rowC!.doubleClick();
    await browser.pause(500);

    // The cascade is off, and nothing was written: Row B is still there.
    await waitForNoCascadeModal();
    expect(await findRowByPurpose('Row B')).not.toBeNull();

    // Row C opened on its stored odometer, not on a pre-cascade copy of it.
    // 50075 is what the book holds, because the delete never ran.
    expect(await editingRowCount()).toBe(1);
    expect(await $('[data-testid="trip-odometer"]').getValue()).toBe('50075');
  });
});
