/**
 * Tier 2: Route Autocomplete Integration Tests
 *
 * Tests the KM field auto-fill feature when selecting a known route.
 * When a user selects an origin and destination that match a previously
 * saved trip, the distance should auto-populate from learned routes.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { waitForTripGrid } from '../../utils/assertions';

describe('Tier 2: Route Autocomplete', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('KM Auto-fill from Known Routes', () => {
    it('should auto-fill KM when selecting a known origin and destination', async () => {
      // Seed a vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Route Autocomplete Test',
        licensePlate: 'AUTO-001',
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

      const year = new Date().getFullYear();

      // Seed an initial trip to create a "learned" route
      // This establishes Bratislava -> Košice = 400 km
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-10`,
        origin: 'Bratislava',
        destination: 'Košice',
        distanceKm: 400,
        odometer: 10400,
        purpose: 'Business trip',
      });

      // Set vehicle as active
      await setActiveVehicle(vehicle.id as string);

      // Navigate to trips page
      await navigateTo('trips');

      // Wait for vehicle info to be displayed
      await browser.waitUntil(
        async () => {
          const vehicleInfo = await $('.vehicle-info');
          return vehicleInfo.isDisplayed();
        },
        { timeout: 10000 }
      );

      // Wait for trip grid
      await waitForTripGrid();
      await browser.pause(500);

      // Click "New record" button
      const newTripBtn = await $('button.new-record');
      await newTripBtn.waitForClickable({ timeout: 5000 });
      await newTripBtn.click();

      // Wait for editing row to appear
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && (await editingRow.isDisplayed());
        },
        { timeout: 10000 }
      );

      // Fill date first (required field)
      const dateInput = await $('[data-testid="trip-date"]');
      await dateInput.waitForDisplayed({ timeout: 5000 });
      await dateInput.setValue(`${year}-01-15`);

      // Type in origin field and select from dropdown
      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.waitForDisplayed({ timeout: 5000 });
      await originInput.click();
      await originInput.setValue('Bratislava');

      // Wait for autocomplete dropdown to appear
      await browser.waitUntil(
        async () => {
          const dropdown = await $('.autocomplete .dropdown');
          return dropdown.isExisting() && (await dropdown.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Origin autocomplete dropdown did not appear' }
      );

      // Click on the suggestion
      const originSuggestion = await $('.autocomplete .dropdown .suggestion');
      await originSuggestion.click();

      // Small pause for the selection to register
      await browser.pause(200);

      // Now fill destination and select from dropdown
      const destInput = await $('[data-testid="trip-destination"]');
      await destInput.waitForDisplayed({ timeout: 5000 });
      await destInput.click();
      await destInput.setValue('Košice');

      // Wait for autocomplete dropdown to appear
      await browser.waitUntil(
        async () => {
          const dropdowns = await $$('.autocomplete .dropdown');
          // Find the dropdown that's currently visible (destination's)
          for (const dropdown of dropdowns) {
            if (await dropdown.isDisplayed()) {
              return true;
            }
          }
          return false;
        },
        { timeout: 5000, timeoutMsg: 'Destination autocomplete dropdown did not appear' }
      );

      // Click on the destination suggestion
      // Need to find the visible dropdown (destination's, not origin's)
      const allDropdowns = await $$('.autocomplete .dropdown');
      for (const dropdown of allDropdowns) {
        if (await dropdown.isDisplayed()) {
          const suggestion = await dropdown.$('.suggestion');
          await suggestion.click();
          break;
        }
      }

      // Wait a moment for auto-fill to trigger
      await browser.pause(300);

      // Verify the KM field was auto-filled with 400
      const distanceInput = await $('[data-testid="trip-distance"]');
      const distanceValue = await distanceInput.getValue();

      expect(parseFloat(distanceValue)).toBe(400);

      // Also verify ODO was auto-calculated (10400 + 400 = 10800)
      const odoInput = await $('[data-testid="trip-odometer"]');
      const odoValue = await odoInput.getValue();

      expect(parseFloat(odoValue)).toBe(10800);
    });

    it('should NOT auto-fill KM if user already entered a distance', async () => {
      // Seed a vehicle
      const vehicleData = createTestIceVehicle({
        name: 'No Overwrite Test',
        licensePlate: 'AUTO-002',
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

      const year = new Date().getFullYear();

      // Seed a trip to create a learned route (Trnava -> Nitra = 65 km)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-02-01`,
        origin: 'Trnava',
        destination: 'Nitra',
        distanceKm: 65,
        odometer: 20065,
        purpose: 'Meeting',
      });

      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');

      await browser.waitUntil(
        async () => {
          const vehicleInfo = await $('.vehicle-info');
          return vehicleInfo.isDisplayed();
        },
        { timeout: 10000 }
      );

      await waitForTripGrid();
      await browser.pause(500);

      // Click "New record"
      const newTripBtn = await $('button.new-record');
      await newTripBtn.waitForClickable({ timeout: 5000 });
      await newTripBtn.click();

      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && (await editingRow.isDisplayed());
        },
        { timeout: 10000 }
      );

      // Fill date
      const dateInput = await $('[data-testid="trip-date"]');
      await dateInput.setValue(`${year}-02-10`);

      // FIRST: Enter a custom distance (100 km) BEFORE selecting route
      const distanceInput = await $('[data-testid="trip-distance"]');
      await distanceInput.setValue('100');

      // Now select origin
      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.click();
      await originInput.setValue('Trnava');

      await browser.waitUntil(
        async () => {
          const dropdown = await $('.autocomplete .dropdown');
          return dropdown.isExisting() && (await dropdown.isDisplayed());
        },
        { timeout: 5000 }
      );

      const originSuggestion = await $('.autocomplete .dropdown .suggestion');
      await originSuggestion.click();
      await browser.pause(200);

      // Select destination
      const destInput = await $('[data-testid="trip-destination"]');
      await destInput.click();
      await destInput.setValue('Nitra');

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
        { timeout: 5000 }
      );

      const allDropdowns = await $$('.autocomplete .dropdown');
      for (const dropdown of allDropdowns) {
        if (await dropdown.isDisplayed()) {
          const suggestion = await dropdown.$('.suggestion');
          await suggestion.click();
          break;
        }
      }

      await browser.pause(300);

      // Verify KM was NOT overwritten - should still be 100 (user's value)
      const finalDistance = await distanceInput.getValue();

      // The autofill only happens when distanceKm is null, not when it has a value
      // So user's 100 should be preserved
      expect(parseFloat(finalDistance)).toBe(100);
    });
  });

  describe('Keyboard Shortcuts', () => {
    it('should submit trip with Enter key (no focus required)', async () => {
      // Seed a vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Enter Submit Test',
        licensePlate: 'ENTER-01',
        initialOdometer: 50000,
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
      await navigateTo('trips');

      await browser.waitUntil(
        async () => {
          const vehicleInfo = await $('.vehicle-info');
          return vehicleInfo.isDisplayed();
        },
        { timeout: 10000 }
      );

      await waitForTripGrid();
      await browser.pause(500);

      // Click "New record"
      const newTripBtn = await $('button.new-record');
      await newTripBtn.waitForClickable({ timeout: 5000 });
      await newTripBtn.click();

      // Wait for editing row
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && (await editingRow.isDisplayed());
        },
        { timeout: 10000 }
      );

      const year = new Date().getFullYear();

      // Fill required fields
      const dateInput = await $('[data-testid="trip-date"]');
      await dateInput.setValue(`${year}-03-15`);

      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.setValue('TestOrigin');

      const destInput = await $('[data-testid="trip-destination"]');
      await destInput.setValue('TestDest');

      // Wait for distance input to be ready
      const distanceInput = await $('[data-testid="trip-distance"]');
      await distanceInput.waitForDisplayed({ timeout: 5000 });

      // Set distance atomically to avoid intermediate input events from setValue()
      // which can corrupt the ODO auto-calculation
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-distance"]', '50');

      // Verify distance was set correctly
      await browser.pause(200);
      const distanceValue = await distanceInput.getValue();
      expect(distanceValue).toBe('50');

      // Also verify ODO was auto-calculated (50000 + 50 = 50050)
      const odoInput = await $('[data-testid="trip-odometer"]');
      const odoValue = await odoInput.getValue();
      expect(odoValue).toBe('50050');

      const purposeInput = await $('[data-testid="trip-purpose"]');
      await purposeInput.setValue('Enter test');

      // Move focus away from purpose field using Tab (more reliable than clicking)
      // This ensures any autocomplete dropdown is closed
      await browser.keys('Tab');
      await browser.pause(300);

      // Press Enter to submit (global handler should catch this)
      await browser.keys('Enter');

      await browser.pause(700);

      // Verify editing row is gone (trip was saved)
      const editingRowAfter = await $('tr.editing');
      const isStillEditing = await editingRowAfter.isExisting();

      expect(isStillEditing).toBe(false);

      // Verify trip was saved by checking page content
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain('TestOrigin');
      expect(text).toContain('TestDest');
    });

    it('should submit with Enter even when autocomplete dropdown is showing', async () => {
      // This tests the specific case: user types in origin field, dropdown appears,
      // user presses Enter WITHOUT selecting from dropdown - should submit form

      const vehicleData = createTestIceVehicle({
        name: 'Dropdown Enter Test',
        licensePlate: 'DROP-01',
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

      const year = new Date().getFullYear();

      // Seed a trip to create learned route (so autocomplete has suggestions)
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-01-01`,
        origin: 'DropdownTest',
        destination: 'SomePlace',
        distanceKm: 100,
        odometer: 70100,
        purpose: 'Setup trip',
      });

      await setActiveVehicle(vehicle.id as string);
      await navigateTo('trips');

      await browser.waitUntil(
        async () => {
          const vehicleInfo = await $('.vehicle-info');
          return vehicleInfo.isDisplayed();
        },
        { timeout: 10000 }
      );

      await waitForTripGrid();
      await browser.pause(500);

      // Click "New record"
      const newTripBtn = await $('button.new-record');
      await newTripBtn.waitForClickable({ timeout: 5000 });
      await newTripBtn.click();

      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && (await editingRow.isDisplayed());
        },
        { timeout: 10000 }
      );

      // Fill date
      const dateInput = await $('[data-testid="trip-date"]');
      await dateInput.setValue(`${year}-04-01`);

      // Type in origin - this should trigger autocomplete dropdown
      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.click();
      await originInput.setValue('Dropdown'); // Partial match for "DropdownTest"

      // Wait for autocomplete dropdown to appear
      await browser.waitUntil(
        async () => {
          const dropdown = await $('.autocomplete .dropdown');
          return dropdown.isExisting() && (await dropdown.isDisplayed());
        },
        { timeout: 5000, timeoutMsg: 'Autocomplete dropdown did not appear' }
      );

      // Verify dropdown is showing
      const dropdownBefore = await $('.autocomplete .dropdown');
      expect(await dropdownBefore.isDisplayed()).toBe(true);

      // Fill other required fields while dropdown might still be showing
      const destInput = await $('[data-testid="trip-destination"]');
      await destInput.setValue('AnotherPlace');

      // Set distance atomically to avoid intermediate input events
      await browser.execute((sel: string, newValue: string) => {
        const input = document.querySelector(sel) as HTMLInputElement;
        if (input) {
          input.value = newValue;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      }, '[data-testid="trip-distance"]', '50');
      await browser.pause(100);

      const purposeInput = await $('[data-testid="trip-purpose"]');
      await purposeInput.setValue('Dropdown enter test');

      // Go back to origin field to trigger dropdown again
      await originInput.click();
      await browser.pause(300);

      // Press Enter while dropdown might be showing (don't select anything)
      await browser.keys('Enter');

      await browser.pause(500);

      // Verify form was submitted (editing row should be gone)
      const editingRowAfter = await $('tr.editing');
      const isStillEditing = await editingRowAfter.isExisting();

      expect(isStillEditing).toBe(false);

      // Verify trip was saved
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain('Dropdown'); // Origin contains "Dropdown"
    });

    it('should cancel editing with Escape key', async () => {
      // Seed a vehicle with an existing trip
      const vehicleData = createTestIceVehicle({
        name: 'Escape Cancel Test',
        licensePlate: 'ESC-001',
        initialOdometer: 60000,
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
      await navigateTo('trips');

      await browser.waitUntil(
        async () => {
          const vehicleInfo = await $('.vehicle-info');
          return vehicleInfo.isDisplayed();
        },
        { timeout: 10000 }
      );

      await waitForTripGrid();
      await browser.pause(500);

      // Click "New record"
      const newTripBtn = await $('button.new-record');
      await newTripBtn.waitForClickable({ timeout: 5000 });
      await newTripBtn.click();

      // Wait for editing row
      await browser.waitUntil(
        async () => {
          const editingRow = await $('tr.editing');
          return editingRow.isExisting() && (await editingRow.isDisplayed());
        },
        { timeout: 10000 }
      );

      // Start filling fields
      const originInput = await $('[data-testid="trip-origin"]');
      await originInput.setValue('ShouldNotSave');

      // Press Escape to cancel
      await browser.keys('Escape');

      await browser.pause(500);

      // Verify editing row is gone
      const editingRowAfter = await $('tr.editing');
      const isStillEditing = await editingRowAfter.isExisting();

      expect(isStillEditing).toBe(false);

      // Verify the trip was NOT saved
      const body = await $('body');
      const text = await body.getText();
      expect(text).not.toContain('ShouldNotSave');
    });
  });
});
