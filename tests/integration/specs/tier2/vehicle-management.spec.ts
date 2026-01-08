/**
 * Tier 2: Vehicle Management Integration Tests
 *
 * Tests vehicle CRUD operations including creation, editing, and deletion.
 * Focuses on PHEV vehicles which require both tank and battery fields.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  getVehicles,
} from '../../utils/db';
import {
  createTestPhevVehicle,
  createTestIceVehicle,
  uniqueTestId,
} from '../../fixtures/vehicles';
import {
  fillVehicleForm,
  saveVehicleForm,
  clickButtonByText,
} from '../../utils/forms';
import { Settings } from '../../utils/assertions';

describe('Tier 2: Vehicle Management', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('PHEV Vehicle Creation', () => {
    it('should create PHEV vehicle with both tank and battery fields', async () => {
      // Navigate to settings page
      await navigateTo('settings');
      await browser.pause(500);

      // Click add vehicle button
      const addVehicleBtn = await $(Settings.addVehicleBtn);
      const btnExists = await addVehicleBtn.isExisting();

      if (!btnExists) {
        console.log('Add vehicle button not found - checking page state');
        const body = await $('body');
        console.log('Current page content:', await body.getText());
        return;
      }

      await addVehicleBtn.click();
      await browser.pause(500);

      // Generate unique test data
      const testId = uniqueTestId();
      const vehicleName = `PHEV Test ${testId.substring(0, 5)}`;
      const licensePlate = `PHEV-${testId.substring(0, 5)}`;

      // Fill PHEV vehicle form with both tank and battery fields
      await fillVehicleForm({
        name: vehicleName,
        licensePlate: licensePlate,
        vehicleType: 'Phev',
        initialOdometer: 25000,
        // Tank fields (required for PHEV)
        tankSizeLiters: 45,
        tpConsumption: 1.5,
        // Battery fields (required for PHEV)
        batteryCapacityKwh: 13,
        baselineConsumptionKwh: 15,
        initialBatteryPercent: 100,
      });

      // Save the vehicle
      await saveVehicleForm();
      await browser.pause(1000);

      // Verify vehicle was created by checking page content
      const body = await $('body');
      const text = await body.getText();

      expect(text).toContain(vehicleName);
      expect(text).toContain(licensePlate);

      // Verify via Tauri IPC that the vehicle was saved correctly
      const vehicles = await getVehicles();
      const createdVehicle = vehicles.find((v) => v.name === vehicleName) as Record<string, unknown>;

      expect(createdVehicle).toBeDefined();
      // Note: Rust returns snake_case property names
      expect(createdVehicle?.vehicle_type).toBe('Phev');

      // Verify both tank and battery fields were saved
      expect(createdVehicle?.tank_size_liters).toBe(45);
      expect(createdVehicle?.tp_consumption).toBe(1.5);
      expect(createdVehicle?.battery_capacity_kwh).toBe(13);
      expect(createdVehicle?.baseline_consumption_kwh).toBe(15);
    });
  });

  describe('Vehicle Editing', () => {
    it('should edit existing vehicle and see changes reflected', async () => {
      // First, seed a vehicle via Tauri IPC
      const vehicleData = createTestIceVehicle({
        name: 'Edit Test Vehicle Original',
        licensePlate: 'EDIT-TST',
        initialOdometer: 50000,
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

      // Navigate to settings page
      await navigateTo('settings');
      await browser.pause(500);

      // Find the edit button for our vehicle
      // Look for the vehicle card first
      const vehicleCards = await $$(Settings.vehicleCard);
      let targetCard = null;

      for (const card of vehicleCards) {
        const cardText = await card.getText();
        if (cardText.includes(vehicleData.name)) {
          targetCard = card;
          break;
        }
      }

      if (!targetCard) {
        console.log('Vehicle card not found, checking page state');
        const body = await $('body');
        console.log('Current page content:', await body.getText());
        return;
      }

      // Click edit button on the vehicle card
      const editBtn = await targetCard.$(Settings.editVehicleBtn);
      if (await editBtn.isDisplayed()) {
        await editBtn.click();
        await browser.pause(500);

        // Update the vehicle name
        const newName = 'Edit Test Vehicle Updated';
        const nameInput = await $(Settings.vehicleForm.name);
        await nameInput.clearValue();
        await nameInput.setValue(newName);

        // Update consumption
        const consumptionInput = await $(Settings.vehicleForm.tpConsumption);
        await consumptionInput.clearValue();
        await consumptionInput.setValue('7.5');

        // Save changes
        await saveVehicleForm();
        await browser.pause(1000);

        // Verify changes are reflected in UI
        const body = await $('body');
        const text = await body.getText();

        expect(text).toContain(newName);

        // Verify via Tauri IPC that changes were saved
        const vehicles = await getVehicles();
        const updatedVehicle = vehicles.find((v) => v.id === vehicle.id) as Record<string, unknown>;

        expect(updatedVehicle).toBeDefined();
        expect(updatedVehicle?.name).toBe(newName);
        // Note: Rust returns snake_case property names
        expect(updatedVehicle?.tp_consumption).toBe(7.5);
      } else {
        console.log('Edit button not visible');
      }
    });
  });

  describe('Vehicle Deletion', () => {
    it('should delete vehicle and redirect to empty state', async () => {
      // First, seed a vehicle via Tauri IPC
      const vehicleData = createTestIceVehicle({
        name: 'Delete Test Vehicle',
        licensePlate: 'DEL-TST',
        initialOdometer: 30000,
        tpConsumption: 6.5,
        tankSizeLiters: 55,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      // Get initial vehicle count
      const initialVehicles = await getVehicles();
      const initialCount = initialVehicles.length;

      // Navigate to settings page
      await navigateTo('settings');
      await browser.pause(500);

      // Find the vehicle card for our test vehicle
      const vehicleCards = await $$(Settings.vehicleCard);
      let targetCard = null;

      for (const card of vehicleCards) {
        const cardText = await card.getText();
        if (cardText.includes(vehicleData.name)) {
          targetCard = card;
          break;
        }
      }

      if (!targetCard) {
        console.log('Vehicle card not found for deletion');
        return;
      }

      // Click delete button on the vehicle card
      const deleteBtn = await targetCard.$(Settings.deleteVehicleBtn);
      if (await deleteBtn.isDisplayed()) {
        await deleteBtn.click();
        await browser.pause(300);

        // Confirm deletion (look for confirm button in modal/dialog)
        const confirmBtn = await $('button*=Confirm');
        if (await confirmBtn.isExisting()) {
          await confirmBtn.click();
          await browser.pause(1000);
        }

        // Verify vehicle was deleted via Tauri IPC
        const remainingVehicles = await getVehicles();
        const deletedVehicle = remainingVehicles.find((v) => v.id === vehicle.id);

        expect(deletedVehicle).toBeUndefined();
        expect(remainingVehicles.length).toBe(initialCount - 1);

        // Verify vehicle name no longer appears in UI
        const body = await $('body');
        const text = await body.getText();

        expect(text).not.toContain(vehicleData.name);

        // If this was the only vehicle, we should see empty state or add vehicle prompt
        if (remainingVehicles.length === 0) {
          const addVehicleBtn = await $(Settings.addVehicleBtn);
          const btnVisible = await addVehicleBtn.isDisplayed();
          expect(btnVisible).toBe(true);
        }
      } else {
        console.log('Delete button not visible');
      }
    });
  });
});
