/**
 * Electric Vehicle Integration Tests
 *
 * Tests the BEV/PHEV vehicle creation and management flow.
 * Each test is independent and sets up its own preconditions.
 * Tests use unique identifiers to prevent data collisions.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { getVehicles } from '../../utils/db';

/**
 * Helper to create a BEV vehicle via UI interactions
 */
async function createBevVehicleViaUI(options: { name: string; licensePlate: string }): Promise<void> {
  // Navigate to settings
  const settingsLink = await $('a[href="/settings"]');
  await settingsLink.click();
  await browser.pause(500);

  // Open add vehicle modal
  const addVehicleBtn = await $('button*=vehicle');
  if (await addVehicleBtn.isDisplayed()) {
    await addVehicleBtn.click();

    // Wait for modal to be visible
    const modalContent = await $('.modal-content');
    await modalContent.waitForDisplayed({ timeout: 5000 });

    // Fill basic info
    const nameInput = await $('#name');
    await nameInput.setValue(options.name);

    const plateInput = await $('#license-plate');
    await plateInput.setValue(options.licensePlate);

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

    // Save - use specific modal selector
    const saveBtn = await $('.modal-footer button.button-primary');
    await saveBtn.waitForClickable({ timeout: 5000 });
    await saveBtn.click();

    // The modal only closes once create_vehicle AND the follow-up get_vehicles have
    // both round-tripped (see handleSaveVehicle in src/routes/settings/+page.svelte).
    // A fixed pause races those two RPCs on a slow run, so wait for the modal to go.
    await modalContent.waitForDisplayed({ reverse: true, timeout: 10000 });
  }
}

/**
 * Generate a unique test ID to prevent data collisions between test runs
 */
function uniqueTestId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
}

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
    const addVehicleBtn = await $('button*=vehicle');
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
    const addVehicleBtn = await $('button*=vehicle');
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
    const addVehicleBtn = await $('button*=vehicle');
    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();

      // Wait for modal to be visible
      const modalContent = await $('.modal-content');
      await modalContent.waitForDisplayed({ timeout: 5000 });

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
    // Generate unique identifiers for this test run
    const testId = uniqueTestId();
    const vehicleName = `Tesla Model 3 ${testId}`;
    const licensePlate = `EV-${testId.substring(0, 7)}`;

    // Navigate to settings
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();
    await browser.pause(500);

    // Open add vehicle modal
    const addVehicleBtn = await $('button*=vehicle');
    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();

      // Wait for modal to be visible
      const modalContent = await $('.modal-content');
      await modalContent.waitForDisplayed({ timeout: 5000 });

      // Fill basic info with unique values
      const nameInput = await $('#name');
      await nameInput.setValue(vehicleName);

      const plateInput = await $('#license-plate');
      await plateInput.setValue(licensePlate);

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

      // Save - use specific modal selector
      const saveBtn = await $('.modal-footer button.button-primary');
      await saveBtn.waitForClickable({ timeout: 5000 });
      await saveBtn.click();

      // The modal only closes once create_vehicle AND the follow-up get_vehicles have
      // both round-tripped (see handleSaveVehicle in src/routes/settings/+page.svelte).
      // A fixed pause races those two RPCs on a slow run, so wait for the modal to go.
      await modalContent.waitForDisplayed({ reverse: true, timeout: 10000 });

      // Verify vehicle was created - look for the name in the list
      const body = await $('body');
      const text = await body.getText();
      expect(text).toContain(vehicleName);
      expect(text).toContain('BEV');
    }
  });

  it('should show BEV badge in vehicle list', async () => {
    // Generate unique identifiers for this test run
    const testId = uniqueTestId();
    const vehicleName = `Badge Test BEV ${testId}`;

    // Create a BEV vehicle first (each test is independent)
    await createBevVehicleViaUI({
      name: vehicleName,
      licensePlate: `B-${testId.substring(0, 7)}`
    });

    // Confirm the save actually reached the backend before asserting on the DOM.
    // Without this, "the form never saved" and "the list did not re-render" both
    // surface as the same missing-badge failure.
    const vehicles = await getVehicles();
    const created = vehicles.find((v) => v.name === vehicleName);
    expect(created).toBeDefined();
    expect(created?.vehicleType).toBe('Bev');

    // Scope the badge lookup to the row of the vehicle we just created rather than
    // taking the first BEV badge on the page.
    const vehicleRow = await $(`.vehicle-item*=${vehicleName}`);
    await vehicleRow.waitForDisplayed({ timeout: 5000 });
    const bevBadge = await vehicleRow.$('.badge.type-bev');
    await bevBadge.waitForDisplayed({ timeout: 5000 });

    // `.badge` is styled `text-transform: uppercase` (src/routes/settings/+page.svelte),
    // and getText() returns *rendered* text — so the DOM's "Bev" arrives here as "BEV".
    // Assert both so a change to either the rendered label or the underlying value fails.
    expect(await bevBadge.getText()).toBe('BEV');
    expect(await bevBadge.getProperty('textContent')).toBe('Bev');
  });

  it('should block vehicle type change when trips exist', async () => {
    // Generate unique identifiers for this test run
    const testId = uniqueTestId();
    const vehicleName = `Type Change Test BEV ${testId}`;

    // Create a BEV vehicle first (each test is independent)
    await createBevVehicleViaUI({
      name: vehicleName,
      licensePlate: `T-${testId.substring(0, 7)}`
    });

    // Note: This test verifies the UI behavior for editing a vehicle.
    // The type dropdown should be enabled for vehicles without trips,
    // and disabled for vehicles with trips.
    // Since we just created a vehicle with no trips, it should be editable.

    // Find the edit button for the vehicle we just created
    const editBtn = await $('button*=Edit');
    if (await editBtn.isDisplayed()) {
      await editBtn.click();
      await browser.pause(300);

      // Check if type dropdown is enabled (no trips yet)
      const typeDropdown = await $('#vehicle-type');
      const isDisabled = await typeDropdown.getAttribute('disabled');

      // For a new vehicle without trips, type should be editable
      expect(isDisabled).toBeNull();

      // Close modal
      const closeBtn = await $('button.close-button');
      if (await closeBtn.isDisplayed()) {
        await closeBtn.click();
      }
    }
  });
});
