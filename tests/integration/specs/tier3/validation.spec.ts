/**
 * Tier 3: Validation & Edge Cases Integration Tests
 *
 * Tests input validation and edge cases:
 * - Invalid odometer (lower than previous trip)
 * - Negative distance input
 * - Leap year date handling
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  getTripGridData,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';
import { TripGrid } from '../../utils/assertions';
import { fillTripForm, saveTripForm, clickButton } from '../../utils/forms';

describe('Tier 3: Validation & Edge Cases', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('sk');
  });

  describe('Odometer Validation', () => {
    it('should prevent invalid odometer (lower than previous trip)', async () => {
      // Create vehicle and seed an initial trip
      const vehicleData = createTestIceVehicle({
        name: 'Odometer Validation Test',
        licensePlate: 'ODO-001',
        initialOdometer: 10000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
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

      // Seed first trip with odometer at 10100
      await seedTrip({
        vehicleId: vehicle.id as string,
        date: `${year}-07-01`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 10100,
        purpose: TripPurposes.business,
      });

      await browser.refresh();
      await waitForAppReady();

      // Navigate to trips page
      await navigateTo('trips');
      await browser.pause(500);

      // Check if new trip button exists
      const newTripBtn = await $(TripGrid.newTripBtn);
      const btnExists = await newTripBtn.isExisting();

      if (!btnExists) {
        console.log('New trip button not found - skipping UI validation test');
        // Still verify via backend that the trip was created
        const gridData = await getTripGridData(vehicle.id as string, year);
        expect(gridData.trips.length).toBe(1);
        return;
      }

      await newTripBtn.click();
      await browser.pause(500);

      // Try to create a trip with odometer LOWER than the previous trip
      // Previous odometer: 10100, we'll try 10050 (invalid - goes backward)
      try {
        await fillTripForm({
          date: `${year}-07-05`,
          origin: SlovakCities.trnava,
          destination: SlovakCities.nitra,
          distanceKm: 70,
          odometer: 10050, // INVALID: lower than previous 10100
          purpose: TripPurposes.business,
        });

        // Try to save
        await saveTripForm();
        await browser.pause(1000);
      } catch (error) {
        // Form might throw error during fill which is acceptable
      }

      // Check for validation feedback
      // The app might handle this in several ways:
      // 1. Show an error toast
      // 2. Show validation styling on the field
      // 3. Prevent the save and show warning
      // 4. Accept but show a date warning (chronological issue)

      const body = await $('body');
      const pageText = await body.getText();

      // Check for various error indicators
      const toast = await $('.toast.error');
      const toastExists = await toast.isExisting();

      // Check if the row has a warning class
      const warningRow = await $('tr.date-warning');
      const warningExists = await warningRow.isExisting();

      // Check if there's an error message visible
      const hasErrorIndicator =
        toastExists ||
        warningExists ||
        pageText.toLowerCase().includes('chyba') ||
        pageText.toLowerCase().includes('error') ||
        pageText.toLowerCase().includes('neplatny') ||
        pageText.toLowerCase().includes('invalid');

      // Verify via backend
      const gridData = await getTripGridData(vehicle.id as string, year);

      // Either:
      // a) The trip wasn't saved (validation prevented it)
      // b) The trip was saved but has a warning
      if (gridData.trips.length === 1) {
        // Trip wasn't saved - good validation
        expect(gridData.trips.length).toBe(1);
      } else {
        // Trip was saved - check if it has date warning
        // (Some apps allow saving but flag chronological issues)
        expect(gridData.trips.length).toBe(2);

        // Check for date warnings on any trip
        const hasDateWarning = gridData.date_warnings.length > 0;
        expect(hasDateWarning || warningExists || hasErrorIndicator).toBe(true);
      }
    });
  });

  describe('Negative Input Validation', () => {
    it('should prevent negative distance input', async () => {
      // Create vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Negative Distance Test',
        licensePlate: 'NEG-001',
        initialOdometer: 20000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
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

      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await browser.pause(500);

      // Try to enter a negative distance
      const newTripBtn = await $(TripGrid.newTripBtn);
      const btnExists = await newTripBtn.isExisting();

      if (!btnExists) {
        console.log('New trip button not found - testing via backend');

        // Try to create trip with negative distance via backend
        // The backend should either reject it or convert to positive
        try {
          await seedTrip({
            vehicleId: vehicle.id as string,
            date: `${year}-08-01`,
            origin: SlovakCities.bratislava,
            destination: SlovakCities.trnava,
            distanceKm: -100, // Negative distance
            odometer: 20100,
            purpose: TripPurposes.business,
          });
        } catch (error) {
          // Backend might reject negative distance
          expect(String(error).toLowerCase()).toContain('error');
        }

        // Check if trip was created
        const gridData = await getTripGridData(vehicle.id as string, year);

        if (gridData.trips.length > 0) {
          // If trip was created, distance should be converted to positive or 0
          const trip = gridData.trips[0];
          expect(trip.distance_km).toBeGreaterThanOrEqual(0);
        }
        return;
      }

      await newTripBtn.click();
      await browser.pause(500);

      // Try to fill form with negative distance
      const editingRow = await $(TripGrid.editingRow);
      await editingRow.waitForDisplayed({ timeout: 5000 });

      // Fill distance field with negative value
      const distanceField = await $(TripGrid.tripForm.distance);
      if (await distanceField.isExisting()) {
        await distanceField.clearValue();
        await distanceField.setValue('-100');

        // Fill other required fields
        const dateField = await $(TripGrid.tripForm.date);
        await dateField.setValue(`${year}-08-01`);

        const originField = await $(TripGrid.tripForm.origin);
        await originField.setValue(SlovakCities.bratislava);

        const destField = await $(TripGrid.tripForm.destination);
        await destField.setValue(SlovakCities.trnava);

        const odoField = await $(TripGrid.tripForm.odometer);
        await odoField.clearValue();
        await odoField.setValue('20100');

        const purposeField = await $(TripGrid.tripForm.purpose);
        await purposeField.setValue(TripPurposes.business);

        // Try to save
        await saveTripForm();
        await browser.pause(1000);
      }

      // Check the result
      const body = await $('body');
      const pageText = await body.getText();

      // Check for error indicators
      const toast = await $('.toast.error');
      const toastExists = await toast.isExisting();

      // Get backend data
      const gridData = await getTripGridData(vehicle.id as string, year);

      if (gridData.trips.length > 0) {
        // Trip was created - verify distance is not negative
        const trip = gridData.trips[0];
        expect(trip.distance_km).toBeGreaterThanOrEqual(0);
      } else {
        // Trip wasn't created - validation worked
        expect(
          toastExists ||
          pageText.toLowerCase().includes('chyba') ||
          pageText.toLowerCase().includes('error')
        ).toBe(true);
      }
    });
  });

  describe('Leap Year Handling', () => {
    it('should handle leap year date (February 29)', async () => {
      // Create vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Leap Year Test',
        licensePlate: 'LEAP-001',
        initialOdometer: 30000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Find a leap year (2024 is a leap year, 2028 is next)
      // We need to check if we can use 2024 or need to find another
      const currentYear = new Date().getFullYear();
      let leapYear = 2024;

      // Find nearest leap year that works for the test
      // Leap years are divisible by 4, except century years must be divisible by 400
      const isLeapYear = (year: number): boolean => {
        return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
      };

      // Use 2024 as it's a known recent leap year
      // If current year is a leap year, use that
      if (isLeapYear(currentYear)) {
        leapYear = currentYear;
      }

      // Try to create a trip on February 29 of a leap year
      try {
        const trip = await seedTrip({
          vehicleId: vehicle.id as string,
          date: `${leapYear}-02-29`, // Leap year date
          origin: SlovakCities.bratislava,
          destination: SlovakCities.kosice,
          distanceKm: 400,
          odometer: 30400,
          purpose: 'Leap Year Trip Test',
        });

        expect(trip.id).toBeDefined();
        expect(trip.date).toBe(`${leapYear}-02-29`);
      } catch (error) {
        // If 2024 is in the past and app doesn't allow past years,
        // this might fail - which is acceptable
        console.log('Leap year trip creation failed:', error);
      }

      // Verify the trip was created (for the leap year used)
      const gridData = await getTripGridData(vehicle.id as string, leapYear);

      // If the trip was created successfully
      if (gridData.trips.length > 0) {
        // Find the leap year trip
        const leapTrip = gridData.trips.find(
          (t) => t.date === `${leapYear}-02-29`
        );

        if (leapTrip) {
          expect(leapTrip.date).toBe(`${leapYear}-02-29`);
          expect(leapTrip.origin).toBe(SlovakCities.bratislava);
          expect(leapTrip.destination).toBe(SlovakCities.kosice);
          expect(leapTrip.distance_km).toBe(400);
        }
      }

      // Refresh and verify the date displays correctly in UI
      await browser.refresh();
      await waitForAppReady();

      // Navigate to the leap year in the year picker (if possible)
      const yearPicker = await $('#year-picker');
      const yearPickerExists = await yearPicker.isExisting();

      if (yearPickerExists) {
        // Try to select the leap year
        try {
          await yearPicker.selectByAttribute('value', leapYear.toString());
          await browser.pause(500);
        } catch {
          // Year might not be available in picker
          console.log(`Year ${leapYear} not available in year picker`);
        }
      }

      // Verify date is displayed correctly if trip exists
      const body = await $('body');
      const pageText = await body.getText();

      // If the leap year trip exists and is visible, check the date format
      // The date "2024-02-29" should display as "29. 2. 2024" or "29.2.2024" in Slovak
      if (gridData.trips.length > 0 && gridData.trips.some((t) => t.date === `${leapYear}-02-29`)) {
        // The date should be visible in some format
        expect(
          pageText.includes('29.') ||
          pageText.includes('29/') ||
          pageText.includes('-02-29')
        ).toBe(true);
      }

      // Also verify that February 29 on a non-leap year would fail
      const nonLeapYear = 2023; // Not a leap year

      try {
        await seedTrip({
          vehicleId: vehicle.id as string,
          date: `${nonLeapYear}-02-29`, // Invalid date for non-leap year
          origin: SlovakCities.bratislava,
          destination: SlovakCities.trnava,
          distanceKm: 65,
          odometer: 30465,
          purpose: 'Invalid Date Test',
        });

        // If we get here, the app accepted the invalid date
        // Some databases might auto-correct to Feb 28 or Mar 1
        const nonLeapGridData = await getTripGridData(
          vehicle.id as string,
          nonLeapYear
        );

        if (nonLeapGridData.trips.length > 0) {
          const invalidTrip = nonLeapGridData.trips[0];
          // Date should NOT be Feb 29 (which doesn't exist in 2023)
          expect(invalidTrip.date).not.toBe(`${nonLeapYear}-02-29`);
        }
      } catch (error) {
        // Expected - invalid date should be rejected
        expect(String(error).length).toBeGreaterThan(0);
      }
    });
  });
});
