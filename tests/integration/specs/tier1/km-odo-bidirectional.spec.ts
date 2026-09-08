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

  describe('Odometer anchor inside a tied group', () => {
    it('offers the ODO the grid shows as Km pred for the same row', async () => {
      // The last two rows share one start datetime -- the shape the real book
      // has, and the shape the editor and the grid used to disagree on.
      //
      // The editor took its anchor from the row below in DISPLAY order; the
      // grid derives Km pred from the canonical order. The two agree only
      // while the grid is sorted descending by trip number. Sorted ascending,
      // the row under edit is the LAST row on screen, so there is no row below
      // it and the anchor fell back to the year start (70000): a 150 km trip
      // was offered ODO 70150. The canonical anchor is the row before it in
      // trip order (70200), so the ODO must be 70350.
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
