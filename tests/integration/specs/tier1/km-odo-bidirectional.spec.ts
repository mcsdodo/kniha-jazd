/**
 * Tier 1: KM ↔ ODO Bidirectional Calculation Tests
 *
 * Tests that editing KM updates ODO and vice versa.
 * Regression test for bug: "first ODO edit subtracts wrong value from KM"
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
  getTripGridData,
  rpc,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';
import {
  waitForTripGrid,
} from '../../utils/assertions';
import {
  fillNumericField,
} from '../../utils/forms';

/**
 * The trip-number header doubles as the sort control, and its arrow is the only
 * rendered state. 'desc' (newest first) is the grid's default.
 */
const SORT_HEADER = '.trip-grid th.col-trip-number';
const SORT_INDICATOR = `${SORT_HEADER} .sort-indicator`;
const SORT_ASC_ARROW = '▲';

/**
 * Wait for the error toast raised by a rejected save (task 81's
 * negative-distance guard) and confirm its message. `waitForExist` alone is
 * not enough -- the toast's `fly` transition can leave `getText()` reading
 * empty for one tick right after the element attaches, so the text itself
 * is polled too.
 */
async function waitForErrorToast(expectedText: string, timeout = 5000): Promise<void> {
  const toast = await $('.toast-error');
  await toast.waitForExist({
    timeout,
    timeoutMsg: 'no error toast appeared for the rejected save',
  });
  await browser.waitUntil(
    async () => (await toast.getText()).includes(expectedText),
    { timeout, timeoutMsg: `error toast never showed "${expectedText}"` }
  );
}

describe('Tier 1: KM ↔ ODO Bidirectional Calculation', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Editing existing trip', () => {
    it('leaves KM in the field alone when ODO is typed, and derives it on save', async () => {
      // Task 8 deleted handleOdoChange's km derivation: typing an ODO no
      // longer touches the km field live. The backend still derives km as
      // (odometer - anchor) on save (ADR-046), so the field must stay put
      // and only the SAVED row may show the derived value.
      // Seed a vehicle with initialOdometer = 10000
      const vehicleData = createTestIceVehicle({
        name: 'ODO-KM Test Vehicle',
        licensePlate: 'ODOKM-01',
        initialOdometer: 10000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      // Seed a trip: KM=100, ODO=10100 (previousOdo=10000, so 10100-10000=100)
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-01-15T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 100,
        odometer: 10100,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Double-click the trip row to edit
      // IMPORTANT: Use 'tbody tr' to exclude thead row which also matches :not(.first-record)
      const tripRow = await $('tbody tr:not(.first-record):not(.editing)');
      await tripRow.waitForDisplayed({ timeout: 5000 });

      // Use selector-based JS dispatch - more reliable than WebDriver doubleClick in CI
      await browser.execute(() => {
        const row = document.querySelector('tbody tr:not(.first-record):not(.editing)') as HTMLElement;
        if (row) {
          row.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
        }
      });

      // Wait for editing mode
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && await editingRow.isDisplayed();
        },
        {
          timeout: 5000,
          timeoutMsg: 'Editing row did not appear after double-click'
        }
      );

      // Get initial KM value
      const kmInput = await $('[data-testid="trip-distance"]');
      const initialKm = await kmInput.getValue();
      expect(initialKm).toBe('100');

      // Get initial ODO value
      const odoInput = await $('[data-testid="trip-odometer"]');
      const initialOdo = await odoInput.getValue();
      expect(initialOdo).toBe('10100');

      // Change ODO from 10100 to 10150. No field-level recalculation any more --
      // the km input must stay exactly where it was.
      // IMPORTANT: Set value atomically to avoid intermediate input events from clearValue()/setValue()
      // which would cause cumulative delta calculations
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-odometer"]', '10150');

      await browser.pause(100);

      // The km FIELD never moved.
      expect(await kmInput.getValue()).toBe('100');

      // Only the last row of the year, so this save is silent (no cascade to
      // approve). The backend derives km = 10150 - 10000 = 150 on save.
      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const grid = await getTripGridData(vehicle.id as string, year);
      const saved = grid.trips.find((t) => t.destination === SlovakCities.kosice);
      expect(saved).toBeDefined();
      expect(saved!.distanceKm).toBe(150);
      expect(saved!.odometer).toBe(10150);
    });

    it('derives KM from only the FINAL ODO on save, not each keystroke', async () => {
      // Regression guard, restated: the km field must never move while ODO
      // is typed, however many times it changes before Save. The backend
      // derives km once, from whatever ODO is in the field at save time.
      // Seed a vehicle with initialOdometer = 20000
      const vehicleData = createTestIceVehicle({
        name: 'Multi-Edit Test Vehicle',
        licensePlate: 'MULTI-01',
        initialOdometer: 20000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      // Seed a trip: KM=50, ODO=20050
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-02-01T08:00`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 50,
        odometer: 20050,
        purpose: TripPurposes.clientMeeting,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Double-click the trip row to edit
      // IMPORTANT: Use 'tbody tr' to exclude thead row which also matches :not(.first-record)
      const tripRow = await $('tbody tr:not(.first-record):not(.editing)');
      await tripRow.waitForDisplayed({ timeout: 5000 });

      // Use selector-based JS dispatch - more reliable than WebDriver doubleClick in CI
      await browser.execute(() => {
        const row = document.querySelector('tbody tr:not(.first-record):not(.editing)') as HTMLElement;
        if (row) {
          row.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
        }
      });

      // Wait for editing mode
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && await editingRow.isDisplayed();
        },
        {
          timeout: 5000,
          timeoutMsg: 'Editing row did not appear after double-click'
        }
      );

      const kmInput = await $('[data-testid="trip-distance"]');
      const odoInput = await $('[data-testid="trip-odometer"]');

      const typeOdo = async (value: string) => {
        await browser.execute((sel: string, newValue: string) => {
          const input = document.querySelector(sel) as HTMLInputElement;
          if (input) {
            input.value = newValue;
            input.dispatchEvent(new Event('input', { bubbles: true }));
          }
        }, sel, value);
        await browser.pause(100);
      };
      const sel = '[data-testid="trip-odometer"]';

      // Three edits in a row. The km field must stay exactly where it was
      // (the original stored value) after every one of them -- there is no
      // more live derivation to check per keystroke.
      await typeOdo('20060');
      expect(await kmInput.getValue()).toBe('50');
      await typeOdo('20075');
      expect(await kmInput.getValue()).toBe('50');
      await typeOdo('20030');
      expect(await kmInput.getValue()).toBe('50');

      // Only the FINAL ODO (20030) reaches the backend. Anchor is 20000, so
      // the derived km is 30 -- not 60, not 75, and not any running total.
      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const grid = await getTripGridData(vehicle.id as string, year);
      const saved = grid.trips.find((t) => t.destination === SlovakCities.nitra);
      expect(saved).toBeDefined();
      expect(saved!.distanceKm).toBe(30);
      expect(saved!.odometer).toBe(20030);
    });

    it('should recalculate ODO when KM is changed', async () => {
      // Seed a vehicle with initialOdometer = 30000
      const vehicleData = createTestIceVehicle({
        name: 'KM-ODO Test Vehicle',
        licensePlate: 'KMODO-01',
        initialOdometer: 30000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      // Seed a trip: KM=80, ODO=30080
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-03-01T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 80,
        odometer: 30080,
        purpose: TripPurposes.delivery,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Double-click the trip row to edit
      // IMPORTANT: Use 'tbody tr' to exclude thead row which also matches :not(.first-record)
      const tripRow = await $('tbody tr:not(.first-record):not(.editing)');
      await tripRow.waitForDisplayed({ timeout: 5000 });

      // Use selector-based JS dispatch - more reliable than WebDriver doubleClick in CI
      await browser.execute(() => {
        const row = document.querySelector('tbody tr:not(.first-record):not(.editing)') as HTMLElement;
        if (row) {
          row.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
        }
      });

      // Wait for editing mode
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && await editingRow.isDisplayed();
        },
        {
          timeout: 5000,
          timeoutMsg: 'Editing row did not appear after double-click'
        }
      );

      const kmInput = await $('[data-testid="trip-distance"]');
      const odoInput = await $('[data-testid="trip-odometer"]');

      // Change KM from 80 to 120 (should make ODO = 30000 + 120 = 30120)
      // IMPORTANT: Set value atomically to avoid intermediate input events
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-distance"]', '120');

      // The ODO now arrives with the preview response, so wait for it instead
      // of reading after a fixed pause.
      await browser.waitUntil(
        async () => (await odoInput.getValue()) === '30120',
        {
          timeout: 5000,
          timeoutMsg: 'ODO never followed the new KM to 30120',
        }
      );
      expect(await odoInput.getValue()).toBe('30120');
    });
  });

  describe('The ODO follows a km edit, nothing else', () => {
    it('leaves the ODO alone when the row is only opened, or only its fuel is typed', async () => {
      // The editor asks for a preview whenever it opens, when litres are
      // typed, when the full-tank box is toggled and after magic fill. None of
      // those is a km edit, so none of them may move the odometer of a row the
      // user did not retype -- on a row whose stored ODO disagrees with the
      // canonical chain that would silently rewrite a legal record.
      //
      // Seeded to disagree on purpose: the anchor is 80000 and the row records
      // 100 km, so the chain says 80100, but 80150 is stored.
      const vehicleData = createTestIceVehicle({
        name: 'Opened Not Edited',
        licensePlate: 'OPENED-1',
        initialOdometer: 80000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-05-02T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 80150,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      await browser.execute(() => {
        const row = document.querySelector(
          'tbody tr:not(.first-record):not(.editing)'
        ) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      });

      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear after double-click' }
      );

      // The preview marker on the rate cell says the response landed, so the
      // assertion below is not just outrunning the request.
      const previewMark = await $('tr.editing td.col-consumption-rate.preview');
      await previewMark.waitForExist({
        timeout: 5000,
        timeoutMsg: 'The editor never received a preview to react to',
      });

      const odoInput = await $('tr.editing [data-testid="trip-odometer"]');
      expect(await odoInput.getValue()).toBe('80150');

      // Typing litres asks for another preview. Still not a km edit.
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, 'tr.editing [data-testid="trip-fuel-liters"]', '30');
      await browser.pause(500);

      expect(await odoInput.getValue()).toBe('80150');

      // A km edit DOES move it, to the canonical chain: 80000 + 120.
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, 'tr.editing [data-testid="trip-distance"]', '120');

      await browser.waitUntil(async () => (await odoInput.getValue()) === '80120', {
        timeout: 5000,
        timeoutMsg: 'A km edit must still fill the ODO from the backend',
      });
    });
  });

  describe('Saving a row that sits below its anchor', () => {
    /**
     * A row can legitimately hold an odometer BELOW the one the canonical
     * chain gives it: the production book has such a row, and the odometer
     * span warning exists to show it rather than to erase it. Saving that row
     * after editing only text must leave both numbers exactly as they were.
     */
    it('keeps the numbers when only a text field was edited', async () => {
      const vehicleData = createTestIceVehicle({
        name: 'Below Anchor Row',
        licensePlate: 'BELOW-01',
        initialOdometer: 100000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-07-04T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.zilina,
        distanceKm: 300,
        odometer: 100300,
        purpose: TripPurposes.business,
      });
      // The row under test: its anchor is 100300, its odometer is 100200.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-07-04T14:00`,
        origin: SlovakCities.zilina,
        destination: SlovakCities.poprad,
        distanceKm: 352,
        odometer: 100200,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const rowFor = async (dest: string): Promise<string> => {
        const index = await browser.execute((d: string) => {
          const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
          return rows.findIndex(
            (r) => r.querySelector('.col-destination')?.textContent?.trim() === d
          );
        }, dest);
        expect(index).toBeGreaterThan(-1);
        return `.trip-grid tbody tr:nth-of-type(${index + 1})`;
      };

      const rowSelector = await rowFor(SlovakCities.poprad);
      await browser.execute((sel: string) => {
        const row = document.querySelector(sel) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      }, rowSelector);

      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear after double-click' }
      );
      await $('tr.editing td.col-consumption-rate.preview').waitForExist({
        timeout: 5000,
        timeoutMsg: 'The editor never received a preview to react to',
      });

      // Edit text only. The numbers are not touched.
      await browser.execute((sel: string, value: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = value;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, 'tr.editing [data-testid="trip-purpose"]', 'Opraveny ucel');

      await (await $('tr.editing .icon-btn.save')).click();
      await browser.pause(500);
      // Neither number moved (km unchanged, odometer unchanged), so the
      // backend plan is a no-op: no modal, no confirmation to give.
      expect(await $('[data-testid="cascade-modal"]').isExisting()).toBe(false);
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const savedRow = await rowFor(SlovakCities.poprad);
      // The text edit did land, so this is a real save, not a no-op.
      expect((await (await $(`${savedRow} .col-purpose`)).getText()).trim()).toBe(
        'Opraveny ucel'
      );
      // The numbers are untouched. An ungated clamp collapses the distance to
      // 1 km and writes 100301 into the odometer of a legal record.
      expect(parseFloat(await (await $(`${savedRow} .col-km`)).getText())).toBe(352);
      expect(parseFloat(await (await $(`${savedRow} .col-odo`)).getText())).toBe(100200);
      // The pre-existing mismatch (anchor 100300, so this row's span is
      // 100200 - 100300 = -100 km against its recorded 352) is still flagged:
      // a text-only edit does not repair it (contrast the number edit below,
      // which does, per ADR-046).
      expect(
        await $(`${savedRow} .col-odo .chain-indicator`).isExisting()
      ).toBe(true);
    });

    it('rejects an ODO typed below the anchor instead of saving a negative distance', async () => {
      // Task 8 deleted both clamps (handleOdoBlur's snap-on-change and
      // handleSave's own clamp). There is no snap left anywhere: the field
      // still shows exactly what was typed (ADR-042: no silent correction).
      //
      // km is untouched here (the odometer field alone was edited), so the
      // backend's "km differs -> km wins; else odo differs -> odo wins" rule
      // (plan_odometer_cascade) takes the odo-wins branch: new_distance_km =
      // submitted_odometer - anchor = 109000 - 110200 = -1200. Task 81
      // (fix round: "reject a negative distance_km in the odometer cascade")
      // added a guard that rejects any plan whose derived distance_km would
      // be negative -- distance_km feeds calculate_closed_period_totals and
      // the 20% legal margin (BIZ-003), so a negative value would silently
      // corrupt that calculation, and the odometer span warning can never
      // catch it on this branch (the pair IS internally consistent, just
      // consistent with a car whose odometer ran backwards). The guard wins:
      // this save must be refused, not written.
      const vehicleData = createTestIceVehicle({
        name: 'Typed Below Anchor',
        licensePlate: 'BELOW-02',
        initialOdometer: 110000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-08-05T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trencin,
        distanceKm: 200,
        odometer: 110200,
        purpose: TripPurposes.business,
      });
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-08-05T14:00`,
        origin: SlovakCities.trencin,
        destination: SlovakCities.martin,
        distanceKm: 100,
        odometer: 110300,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const rowIndex = await browser.execute((d: string) => {
        const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
        return rows.findIndex(
          (r) => r.querySelector('.col-destination')?.textContent?.trim() === d
        );
      }, SlovakCities.martin);
      expect(rowIndex).toBeGreaterThan(-1);
      const rowSelector = `.trip-grid tbody tr:nth-of-type(${rowIndex + 1})`;

      await browser.execute((sel: string) => {
        const row = document.querySelector(sel) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      }, rowSelector);
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear after double-click' }
      );

      // Type an ODO below the anchor (110200) and finalise it with a
      // `change` event too -- there is no listener left on it, so it must
      // have no effect either way.
      await browser.execute((sel: string, value: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = value;
          input.dispatchEvent(new Event('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
        }
      }, 'tr.editing [data-testid="trip-odometer"]', '109000');
      await browser.pause(300);

      const odoInput = await $('tr.editing [data-testid="trip-odometer"]');
      // No clamp: the field still shows exactly what was typed.
      expect(parseFloat(await odoInput.getValue())).toBe(109000);

      await (await $('tr.editing .icon-btn.save')).click();

      // The guard rejects this before any cascade plan could need approval:
      // handleUpdate's own dry-run preview call is what throws, so the
      // catch runs and no modal -- cascade or otherwise -- ever appears.
      // errorUpdateTrip() in en/index.ts -- read from i18n, not guessed.
      await waitForErrorToast('Failed to update record');
      expect(await $('[data-testid="cascade-modal"]').isExisting()).toBe(false);

      // Nothing was written: the row stays open exactly as the user left it
      // (doSave's `if (!saved) return;` -- handleUpdate's catch returns
      // false, so isEditing never flips and formData is never re-seeded).
      expect(await $('tr.editing').isExisting()).toBe(true);
      expect(parseFloat(await odoInput.getValue())).toBe(109000);

      // The stored book is untouched -- the seeded pair, not the rejected one.
      const grid = await getTripGridData(vehicle.id as string, year);
      const saved = grid.trips.find((t) => t.destination === SlovakCities.martin);
      expect(saved).toBeDefined();
      expect(saved!.odometer).toBe(110300);
      expect(saved!.distanceKm).toBe(100);
    });

    it('Enter rejects the typed ODO below anchor, same as a Save click', async () => {
      // Both clamps are gone (Task 8): handleOdoBlur, which fired on the
      // `change` event before handleSave ever ran, and handleSave's own
      // clamp. Enter is bound with <svelte:window on:keydown>, and
      // handleGlobalKeydown calls preventDefault() then handleSave()
      // synchronously while the ODO input still has focus -- no blur, no
      // `change` event, no implicit-submit blur (there is no <form> wrapping
      // the row). So this path was the more exacting of the two clamp
      // branches to reach the backend's negative-distance guard (task 81,
      // see the Save-click case above for why the guard exists) through --
      // it must refuse exactly the same way a Save click does.
      const vehicleData = createTestIceVehicle({
        name: 'Enter Beats Blur',
        licensePlate: 'BELOW-03',
        initialOdometer: 140000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-09-10T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 275,
        odometer: 140275,
        purpose: TripPurposes.business,
      });
      // The row under test: its anchor is 140275, its stored odometer is 140150.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-09-10T14:00`,
        origin: SlovakCities.kosice,
        destination: SlovakCities.presov,
        distanceKm: 50,
        odometer: 140150,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const rowFor = async (dest: string): Promise<string> => {
        const index = await browser.execute((d: string) => {
          const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
          return rows.findIndex(
            (r) => r.querySelector('.col-destination')?.textContent?.trim() === d
          );
        }, dest);
        expect(index).toBeGreaterThan(-1);
        return `.trip-grid tbody tr:nth-of-type(${index + 1})`;
      };

      await browser.execute((sel: string) => {
        const row = document.querySelector(sel) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      }, await rowFor(SlovakCities.presov));
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear after double-click' }
      );

      // Type an ODO below the anchor (140275) and press Enter -- input only,
      // no change event, both in one synchronous script.
      const odoInput = await $('tr.editing [data-testid="trip-odometer"]');
      await browser.execute((sel: string, value: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        input.value = value;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
        );
      }, 'tr.editing [data-testid="trip-odometer"]', '139000');

      // km is unchanged (50, same as stored), so the backend's odo-wins
      // branch would derive distance = 139000 - 140275 (anchor) = -1275 --
      // the same negative-distance guard as the Save-click case rejects it
      // before any cascade plan could need approval.
      await waitForErrorToast('Failed to update record');
      expect(await $('[data-testid="cascade-modal"]').isExisting()).toBe(false);

      // Nothing was written: the row stays open on exactly what was typed.
      expect(await $('tr.editing').isExisting()).toBe(true);
      expect(parseFloat(await odoInput.getValue())).toBe(139000);

      // The stored book is untouched -- the seeded pair, not the rejected one.
      const grid = await getTripGridData(vehicle.id as string, year);
      const saved = grid.trips.find((t) => t.destination === SlovakCities.presov);
      expect(saved).toBeDefined();
      expect(saved!.odometer).toBe(140150);
      expect(saved!.distanceKm).toBe(50);
    });
  });

  describe('Saving before the preview lands', () => {
    it('keeps the km the user typed and moves the ODO to the chain', async () => {
      // handleSave's own clamp, reached the only way it can be: the km input
      // and the Save click in ONE synchronous script, so no preview response
      // and no `change` event can come between them. The row sits below its
      // canonical anchor, so the clamp fires with a stale odometer in the
      // field and a km the user typed a moment ago.
      //
      // The km is the number to keep. Before this fix the clamp kept the
      // odometer instead and collapsed a 400 km leg to 1 km.
      const vehicleData = createTestIceVehicle({
        name: 'Save Beats Preview',
        licensePlate: 'RACE-001',
        initialOdometer: 120000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-09-06T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.zvolen,
        distanceKm: 300,
        odometer: 120300,
        purpose: TripPurposes.business,
      });
      // Under test: anchor 120300, stored odometer 120200.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-09-06T14:00`,
        origin: SlovakCities.zvolen,
        destination: SlovakCities.michalovce,
        distanceKm: 352,
        odometer: 120200,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const rowFor = async (dest: string): Promise<string> => {
        const index = await browser.execute((d: string) => {
          const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
          return rows.findIndex(
            (r) => r.querySelector('.col-destination')?.textContent?.trim() === d
          );
        }, dest);
        expect(index).toBeGreaterThan(-1);
        return `.trip-grid tbody tr:nth-of-type(${index + 1})`;
      };

      await browser.execute((sel: string) => {
        const row = document.querySelector(sel) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      }, await rowFor(SlovakCities.michalovce));
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear after double-click' }
      );

      // One script: type the km, then Save. Nothing can run in between.
      await browser.execute(() => {
        const km = document.querySelector(
          'tr.editing [data-testid="trip-distance"]'
        ) as HTMLInputElement;
        km.value = '400';
        km.dispatchEvent(new Event('input', { bubbles: true }));
        (document.querySelector('tr.editing .icon-btn.save') as HTMLElement).click();
      });

      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const savedRow = await rowFor(SlovakCities.michalovce);
      // The typed distance survives, and the odometer is where the chain puts
      // it: 120300 + 400. The old clamp saved 1 km and 120301.
      expect(parseFloat(await (await $(`${savedRow} .col-km`)).getText())).toBe(400);
      expect(parseFloat(await (await $(`${savedRow} .col-odo`)).getText())).toBe(120700);
    });
  });

  describe('The ODO of a re-opened row', () => {
    /** The grid row whose destination cell reads `dest`, as a CSS selector. */
    async function rowSelectorFor(dest: string): Promise<string> {
      const index = await browser.execute((d: string) => {
        const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
        return rows.findIndex(
          (r) => r.querySelector('.col-destination')?.textContent?.trim() === d
        );
      }, dest);
      expect(index).toBeGreaterThan(-1);
      return `.trip-grid tbody tr:nth-of-type(${index + 1})`;
    }

    async function openEditor(rowSelector: string): Promise<void> {
      await browser.execute((sel: string) => {
        const row = document.querySelector(sel) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      }, rowSelector);
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: `No editor opened for ${rowSelector}` }
      );
      // The preview marker on the rate cell says the response landed, so an
      // assertion after this is not merely outrunning the request.
      await $('tr.editing td.col-consumption-rate.preview').waitForExist({
        timeout: 5000,
        timeoutMsg: 'The editor never received a preview to react to',
      });
    }

    async function typeKm(km: string): Promise<void> {
      await browser.execute((sel: string, value: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = value;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, 'tr.editing [data-testid="trip-distance"]', km);
    }

    /** Strict: the row must close on its own, with no cascade modal to answer. */
    async function saveEditor(): Promise<void> {
      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(300);
    }

    /**
     * A save that DOES move a later row's odometer asks first (task 81,
     * Task 7). Assert the modal actually appeared -- a helper that merely
     * tolerated one could hide it popping up where it shouldn't -- then
     * confirm it. Returns the modal's summary text, read before it closes.
     */
    async function saveEditorConfirmingCascade(): Promise<string> {
      await (await $('tr.editing .icon-btn.save')).click();
      await $('[data-testid="cascade-modal"]').waitForExist({
        timeout: 5000,
        timeoutMsg: 'Expected a cascade modal (this save moves a later row) but none appeared',
      });
      const summary = await (await $('[data-testid="cascade-summary"]')).getText();
      await (await $('[data-testid="cascade-confirm"]')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after confirming the cascade',
      });
      await browser.pause(300);
      return summary;
    }

    it('reflects a cascade it never opened for, on re-open (task 81, R6)', async () => {
      // TripGrid keys its rows by trip id, so ONE TripRow instance serves
      // display and every edit session of that row for the page's whole
      // lifetime -- it is never remounted. `formData` is only seeded once,
      // at construction.
      //
      // This is the ONLY spec that exercises that guarantee through the row
      // EDITOR: the grid's own display cell (`.col-odo`) binds straight to
      // the `trip` prop and would read the fresh value even if the reseed
      // guard were deleted outright. Only re-opening the editor and reading
      // `formData` (the input's value) can tell the two apart -- which is
      // exactly what Task 10's cascade spec (grid-cell assertions only)
      // does not do.
      const vehicleData = createTestIceVehicle({
        name: 'Re-opened Row',
        licensePlate: 'REOPEN-1',
        initialOdometer: 90000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      // The earlier row, W.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-06-03T09:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 90100,
        purpose: TripPurposes.business,
      });
      // The row under test, X.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-06-03T11:00`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 90200,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const odoInput = () => $('tr.editing [data-testid="trip-odometer"]');

      // Session 1 on X: a real km edit. Anchor 90100 + 120. X is the last
      // row of the year, so nothing else moves -- this save is silent.
      await openEditor(await rowSelectorFor(SlovakCities.nitra));
      await typeKm('120');
      await browser.waitUntil(async () => (await (await odoInput()).getValue()) === '90220', {
        timeout: 5000,
        timeoutMsg: 'A km edit must fill the ODO from the backend',
      });
      await saveEditor();

      // Session 2 on W: its km changes, so its odometer does -- and that is
      // X's anchor. This DOES move a later row (X), so the cascade modal
      // must appear; confirm it so the shift is actually written.
      await openEditor(await rowSelectorFor(SlovakCities.trnava));
      await typeKm('150');
      await browser.waitUntil(async () => (await (await odoInput()).getValue()) === '90150', {
        timeout: 5000,
        timeoutMsg: "The earlier row's ODO did not follow its km",
      });
      const summary = await saveEditorConfirmingCascade();
      // Sanity on the modal's own claim, not just that one appeared.
      expect(summary).toContain('1');

      // Session 3 on X: opened, not edited. The cascade moved X's anchor
      // from 90100 to 90150 and confirming wrote X's own odometer forward
      // by the same +50 -- 90220 -> 90270. Re-opening X must show that FRESH
      // value: if the formData re-seed guard ($: if (trip && !isEditing) in
      // TripRow.svelte) were missing, the field would still hold the STALE
      // 90220 from session 1's construction-time formData.
      const grid = await getTripGridData(vehicle.id as string, year);
      const persistedX = grid.trips.find((t) => t.destination === SlovakCities.nitra);
      expect(persistedX).toBeDefined();
      expect(persistedX!.odometer).toBe(90270);

      await openEditor(await rowSelectorFor(SlovakCities.nitra));
      await browser.pause(500);
      expect(await (await odoInput()).getValue()).toBe('90270');
    });
  });

  describe('Odometer anchor: the canonical order, not the display neighbour', () => {
    it('offers Km pred + km for a row with no row below it in display order', async () => {
      // The editor took its anchor from the row below in DISPLAY order; the
      // grid derives Km pred from the canonical order. The two agree only
      // while the grid is sorted descending by trip number, its default.
      // Sorted ascending, the row under edit is the LAST row on screen, so
      // there is no row below it and the anchor fell back to the year start
      // (70000): a 150 km trip was offered ODO 70150. The canonical anchor is
      // the row before it in trip order (70200), so the ODO must be 70350.
      //
      // The last two rows share one start datetime because that is the shape
      // the real book has. The tie itself is covered in Rust, by
      // test_preview_of_an_edited_row_anchors_on_its_canonical_predecessor.
      const vehicleData = createTestIceVehicle({
        name: 'Tied Group Anchor',
        licensePlate: 'TIEDG-01',
        initialOdometer: 70000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);
      // Km pred must be visible - another spec can leave it hidden.
      await rpc<void>('set_hidden_columns', { columns: [] });

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-01T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 70100,
        purpose: TripPurposes.business,
      });
      // Tied pair: same start datetime, seeded in canonical order.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-01T12:00`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 100,
        odometer: 70200,
        purpose: TripPurposes.business,
      });
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-01T12:00`,
        origin: SlovakCities.nitra,
        destination: SlovakCities.kosice,
        distanceKm: 100,
        odometer: 70300,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Sort ascending, which puts the tied pair's second row at the bottom.
      const indicator = await $(SORT_INDICATOR);
      await indicator.waitForDisplayed({ timeout: 10000 });
      if ((await indicator.getText()) !== SORT_ASC_ARROW) {
        await (await $(SORT_HEADER)).click();
        await browser.waitUntil(
          async () => (await (await $(SORT_INDICATOR)).getText()) === SORT_ASC_ARROW,
          { timeout: 5000, timeoutMsg: 'Grid never sorted ascending' }
        );
      }

      // The row under edit, found by its destination so month-end rows and the
      // first-record row cannot be picked by position.
      const rowIndex = await browser.execute((dest: string) => {
        const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
        return rows.findIndex(
          (r) => r.querySelector('.col-destination')?.textContent?.trim() === dest
        );
      }, SlovakCities.kosice);
      expect(rowIndex).toBeGreaterThan(-1);

      const rowSelector = `.trip-grid tbody tr:nth-of-type(${rowIndex + 1})`;
      const kmPredText = await (await $(`${rowSelector} .col-odo-start`)).getText();
      const kmPred = parseFloat(kmPredText);
      // The grid reads the canonical order: the tied row before this one.
      expect(kmPred).toBe(70200);

      await browser.execute((sel: string) => {
        const row = document.querySelector(sel) as HTMLElement;
        row?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      }, rowSelector);

      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Editing row did not appear after double-click' }
      );

      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, 'tr.editing [data-testid="trip-distance"]', '150');

      const odoInput = await $('tr.editing [data-testid="trip-odometer"]');
      const expectedOdo = String(kmPred + 150);
      await browser.waitUntil(
        async () => (await odoInput.getValue()) === expectedOdo,
        {
          timeout: 5000,
          timeoutMsg: `Editor never offered ${expectedOdo} (Km pred ${kmPred} + 150 km)`,
        }
      );
      expect(await odoInput.getValue()).toBe(expectedOdo);
    });
  });

  describe('A fractional km on a new row', () => {
    it('saves the odometer the chain gives it, not 0', async () => {
      // The whole editor chain runs on the backend preview (task 80). The
      // preview command took `distance_km` as an i32, so a km like 100.5
      // failed to parse, the preview came back null, the new row had no
      // anchor, the save clamp did not fire, and `formData.odometer` -- null
      // on a new row -- was saved as 0. A legal odometer of 0.
      //
      // Fractional km are real in this book: they arrive from manual edits.
      const vehicleData = createTestIceVehicle({
        name: 'Fractional Km New Row',
        licensePlate: 'FRAC-001',
        initialOdometer: 80000,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-05-02T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 80100,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      const newTripBtn = await $('button.new-record');
      await newTripBtn.waitForClickable({ timeout: 5000 });
      await newTripBtn.click();
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
        },
        { timeout: 10000, timeoutMsg: 'The new row did not open' }
      );

      // Type the text fields first. Typing does not auto-fill the distance --
      // only picking a suggestion does -- so the km typed below stands.
      await browser.execute(
        (origin: string, destination: string, purpose: string) => {
          const set = (testId: string, value: string) => {
            const input = document.querySelector(
              `tr.editing [data-testid="${testId}"]`
            ) as HTMLInputElement;
            input.value = value;
            input.dispatchEvent(new Event('input', { bubbles: true }));
          };
          set('trip-origin', origin);
          set('trip-destination', destination);
          set('trip-purpose', purpose);
        },
        SlovakCities.trnava,
        SlovakCities.nitra,
        TripPurposes.business
      );

      // The fractional km. The anchor is 80100, so the row ends at 80200.5.
      await browser.execute(() => {
        const km = document.querySelector(
          'tr.editing [data-testid="trip-distance"]'
        ) as HTMLInputElement;
        km.value = '100.5';
        km.dispatchEvent(new Event('input', { bubbles: true }));
      });

      // Give the preview time to land and fill the ODO. Do not assert it here:
      // on the broken build it never lands, and the number this test is about
      // is the one that reaches the database.
      const odoInput = await $('tr.editing [data-testid="trip-odometer"]');
      await browser
        .waitUntil(async () => (await odoInput.getValue()) === '80200.5', {
          timeout: 5000,
        })
        .catch(() => undefined);

      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      // Read the stored value, not the grid cell: the grid rounds to whole km.
      const grid = await getTripGridData(vehicle.id as string, year);
      const saved = grid.trips.find((t) => t.destination === SlovakCities.nitra);
      expect(saved).toBeDefined();
      expect(saved!.distanceKm).toBe(100.5);
      expect(saved!.odometer).toBe(80200.5);
    });
  });
});
