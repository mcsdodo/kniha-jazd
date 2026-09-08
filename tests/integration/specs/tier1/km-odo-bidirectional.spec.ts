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

describe('Tier 1: KM ↔ ODO Bidirectional Calculation', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Editing existing trip', () => {
    it('should recalculate KM when ODO is changed', async () => {
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

      // Change ODO from 10100 to 10150 (should make KM = 10150 - 10000 = 150)
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

      // Verify KM was recalculated to 150
      const newKm = await kmInput.getValue();
      expect(newKm).toBe('150');
    });

    it('should maintain correct KM when ODO is edited multiple times', async () => {
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

      // First edit: ODO 20050 -> 20060 (KM should be 60)
      // IMPORTANT: Set value atomically to avoid intermediate input events
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-odometer"]', '20060');
      await browser.pause(100);

      let newKm = await kmInput.getValue();
      expect(newKm).toBe('60');

      // Second edit: ODO 20060 -> 20075 (KM should be 75)
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-odometer"]', '20075');
      await browser.pause(100);

      newKm = await kmInput.getValue();
      expect(newKm).toBe('75');

      // Third edit: ODO 20075 -> 20030 (KM should be 30)
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-odometer"]', '20030');
      await browser.pause(100);

      newKm = await kmInput.getValue();
      expect(newKm).toBe('30');
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
    });

    it('still snaps an ODO the user types below the anchor, and saves it', async () => {
      // The snap must survive the gate: an ODO the user actually typed below
      // the anchor becomes anchor + 1 and is saved that way.
      //
      // The snap here comes from handleOdoBlur, on the `change` event, which
      // is unconditional -- so by the time handleSave runs, its own clamp has
      // nothing left to do. handleSave's own clamp has two branches: the
      // odoFollowsKm one is covered by the case in the next describe block,
      // where Save beats the `change` and the preview alike; the
      // manualOdoEdit one is covered by the case right below this one, where
      // Enter beats the `change` event the same way.
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

      // Type an ODO below the anchor (110200) and finalise it.
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
      expect(parseFloat(await odoInput.getValue())).toBe(110201);

      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const savedIndex = await browser.execute((d: string) => {
        const rows = Array.from(document.querySelectorAll('.trip-grid tbody tr'));
        return rows.findIndex(
          (r) => r.querySelector('.col-destination')?.textContent?.trim() === d
        );
      }, SlovakCities.martin);
      const savedSelector = `.trip-grid tbody tr:nth-of-type(${savedIndex + 1})`;
      expect(parseFloat(await (await $(`${savedSelector} .col-odo`)).getText())).toBe(110201);
    });

    it('gives the same anchor + 1 answer when Enter saves before the ODO blurs', async () => {
      // handleSave's own clamp, on the manualOdoEdit branch this time --
      // reached the only way it can be. Enter is bound with
      // <svelte:window on:keydown>, and handleGlobalKeydown calls
      // preventDefault() and then handleSave() synchronously while the ODO
      // input still has focus. No blur fires, so no `change` event, so
      // handleOdoBlur's snap never runs first -- and there is no <form>
      // wrapping the row, so there is no implicit-submit blur either.
      // handleSave therefore reaches its clamp with manualOdoEdit set and the
      // raw typed value still in formData.odometer.
      //
      // One script: type the ODO, then Enter. Nothing can run in between, so
      // this is the manualOdoEdit branch, not handleOdoBlur's snap. The
      // answer must match what the blur snap would have given: anchor + 1,
      // km 1.
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
      await browser.execute((sel: string, value: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        input.value = value;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
        );
      }, 'tr.editing [data-testid="trip-odometer"]', '139000');

      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(500);

      const savedRow = await rowFor(SlovakCities.presov);
      // handleOdoBlur never ran: if it had, the field would already show
      // 140276 before Save fired. This is handleSave's own clamp giving the
      // same answer on the manualOdoEdit branch.
      expect(parseFloat(await (await $(`${savedRow} .col-km`)).getText())).toBe(1);
      expect(parseFloat(await (await $(`${savedRow} .col-odo`)).getText())).toBe(140276);
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

    async function saveEditor(): Promise<void> {
      await (await $('tr.editing .icon-btn.save')).click();
      await browser.waitUntil(async () => !(await $('tr.editing').isExisting()), {
        timeout: 5000,
        timeoutMsg: 'The editor stayed open after save',
      });
      await browser.pause(300);
    }

    it('stays put when an earlier row moved its anchor since the last km edit', async () => {
      // A row component serves its grid row for the whole page lifetime, so a
      // km edit made in one edit session must not still be "behind" the
      // odometer in the next one. Here the anchor MOVES between the two
      // sessions, so a leftover km edit is visible: it would rewrite an
      // odometer the user never retyped, and Save would persist it.
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

      // Session 1 on X: a real km edit. Anchor 90100 + 120.
      await openEditor(await rowSelectorFor(SlovakCities.nitra));
      await typeKm('120');
      await browser.waitUntil(async () => (await (await odoInput()).getValue()) === '90220', {
        timeout: 5000,
        timeoutMsg: 'A km edit must fill the ODO from the backend',
      });
      await saveEditor();

      // Session 2 on W: its km changes, so its odometer does -- and that is X's
      // anchor. X keeps the 90220 it was saved with (nothing recalculates it).
      await openEditor(await rowSelectorFor(SlovakCities.trnava));
      await typeKm('150');
      await browser.waitUntil(async () => (await (await odoInput()).getValue()) === '90150', {
        timeout: 5000,
        timeoutMsg: "The earlier row's ODO did not follow its km",
      });
      await saveEditor();

      // Session 3 on X: opened, not edited. Its ODO must be the saved 90220,
      // not the 90270 a leftover km edit would derive from the new anchor.
      await openEditor(await rowSelectorFor(SlovakCities.nitra));
      await browser.pause(500);
      expect(await (await odoInput()).getValue()).toBe('90220');
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
});
