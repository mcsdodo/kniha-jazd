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
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';
import {
  waitForTripGrid,
} from '../../utils/assertions';
import {
  fillNumericField,
} from '../../utils/forms';

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
        date: `${year}-01-15`,
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
        date: `${year}-02-01`,
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
        date: `${year}-03-01`,
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
      await browser.pause(100);

      // Verify ODO was recalculated to 30120
      const newOdo = await odoInput.getValue();
      expect(newOdo).toBe('30120');
    });
  });
});
