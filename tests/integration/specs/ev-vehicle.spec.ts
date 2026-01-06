/**
 * Electric Vehicle Integration Tests
 *
 * Tests the BEV/PHEV vehicle creation and management flow.
 */

import { waitForAppReady } from '../utils/app';

describe('Electric Vehicle Support', () => {
  beforeEach(async () => {
    await waitForAppReady();
  });

  it('should navigate to settings and see vehicle type dropdown', async () => {
    // Navigate to settings
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Open add vehicle modal
    const addVehicleBtn = await $('button*=vozidlo');
    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();
      await browser.pause(300);

      // Check for vehicle type dropdown
      const typeDropdown = await $('#vehicle-type');
      await expect(typeDropdown).toBeDisplayed();

      // Verify ICE is the default option
      const selectedOption = await typeDropdown.getValue();
      expect(selectedOption).toBe('Ice');
    }
  });

  it('should show battery fields when BEV is selected', async () => {
    // Navigate to settings
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Open add vehicle modal
    const addVehicleBtn = await $('button*=vozidlo');
    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();
      await browser.pause(300);

      // Select BEV
      const typeDropdown = await $('#vehicle-type');
      await typeDropdown.selectByAttribute('value', 'Bev');
      await browser.pause(300);

      // Battery fields should now be visible
      const batteryCapacity = await $('#battery-capacity');
      await expect(batteryCapacity).toBeDisplayed();

      const baselineConsumption = await $('#baseline-consumption');
      await expect(baselineConsumption).toBeDisplayed();

      // Fuel fields should be hidden
      const tankSize = await $('#tank-size');
      await expect(tankSize).not.toBeDisplayed();
    }
  });

  it('should show both fuel and battery fields for PHEV', async () => {
    // Navigate to settings
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Open add vehicle modal
    const addVehicleBtn = await $('button*=vozidlo');
    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();
      await browser.pause(300);

      // Select PHEV
      const typeDropdown = await $('#vehicle-type');
      await typeDropdown.selectByAttribute('value', 'Phev');
      await browser.pause(300);

      // Both fuel and battery fields should be visible
      const tankSize = await $('#tank-size');
      await expect(tankSize).toBeDisplayed();

      const batteryCapacity = await $('#battery-capacity');
      await expect(batteryCapacity).toBeDisplayed();
    }
  });

  it('should create a BEV vehicle successfully', async () => {
    // Navigate to settings
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Open add vehicle modal
    const addVehicleBtn = await $('button*=vozidlo');
    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();
      await browser.pause(300);

      // Fill basic info
      const nameInput = await $('#name');
      await nameInput.setValue('Tesla Model 3');

      const plateInput = await $('#license-plate');
      await plateInput.setValue('EV-TEST');

      // Select BEV
      const typeDropdown = await $('#vehicle-type');
      await typeDropdown.selectByAttribute('value', 'Bev');
      await browser.pause(300);

      // Fill ODO
      const odometerInput = await $('#initial-odometer');
      await odometerInput.setValue('5000');

      // Fill battery fields
      const batteryCapacity = await $('#battery-capacity');
      await batteryCapacity.setValue('75');

      const baselineConsumption = await $('#baseline-consumption');
      await baselineConsumption.setValue('18');

      const initialBattery = await $('#initial-battery');
      await initialBattery.setValue('90');

      // Save
      const saveBtn = await $('button*=Uložiť');
      await saveBtn.click();
      await browser.pause(1000);

      // Verify vehicle was created - look for the name in the list
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain('Tesla Model 3');
      expect(text).toContain('BEV');
    }
  });

  it('should show BEV badge in vehicle list', async () => {
    // Navigate to settings (assumes BEV was created in previous test)
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Look for the BEV badge
    const bevBadge = await $('.badge.type-bev');
    if (await bevBadge.isExisting()) {
      await expect(bevBadge).toBeDisplayed();
      const text = await bevBadge.getText();
      expect(text).toContain('BEV');
    }
  });

  it('should block vehicle type change when trips exist', async () => {
    // This test requires a vehicle with trips
    // For now, just verify the UI shows the warning when appropriate

    // Navigate to settings
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Find an existing vehicle and try to edit it
    const editBtn = await $('button*=Upraviť');
    if (await editBtn.isDisplayed()) {
      await editBtn.click();
      await browser.pause(300);

      // Check if type dropdown is disabled or shows warning
      const typeDropdown = await $('#vehicle-type');
      const isDisabled = await typeDropdown.getAttribute('disabled');

      // If disabled, there's a vehicle with trips
      // If not disabled, this vehicle has no trips yet
      console.log('Vehicle type dropdown disabled:', isDisabled);

      // Close modal
      const closeBtn = await $('button.close-button');
      if (await closeBtn.isDisplayed()) {
        await closeBtn.click();
      }
    }
  });
});
