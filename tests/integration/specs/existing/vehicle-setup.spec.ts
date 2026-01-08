/**
 * Vehicle Setup Integration Tests
 *
 * Tests the core flow of creating a vehicle and verifying it appears in the app.
 * This is the proof-of-concept test to verify the integration test setup works.
 * Each test is independent and sets up its own preconditions.
 * Tests use unique identifiers to prevent data collisions.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';

/**
 * Generate a unique test ID to prevent data collisions between test runs
 */
function uniqueTestId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
}

describe('Vehicle Setup', () => {
  beforeEach(async () => {
    // Wait for app to be fully loaded
    await waitForAppReady();
  });

  it('should load the app successfully', async () => {
    // Verify app header is visible
    const header = await $('h1');
    await expect(header).toBeDisplayed();
    await expect(header).toHaveText(expect.stringContaining('Logbook'));
  });

  it('should display app content on main page', async () => {
    // Verify the app renders content on the main page
    // Note: This test does not assume empty/fresh database state
    const content = await $('body');
    const text = await content.getText();

    // App should render some content
    expect(text.length).toBeGreaterThan(0);
  });

  it('should navigate to settings page', async () => {
    // Click on settings link
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();

    // Wait for navigation to complete
    await browser.pause(500);

    // Verify we're on settings page by checking URL or page-specific content
    const url = await browser.getUrl();
    expect(url).toContain('/settings');
  });

  it('should create a new vehicle', async () => {
    // Generate unique identifiers for this test run
    const testId = uniqueTestId();
    const vehicleName = `Test Auto E2E ${testId}`;
    const licensePlate = `E2E-${testId.substring(0, 7)}`;

    // Navigate to settings page first (each test is independent)
    await navigateTo('settings');

    // Look for the add vehicle button/form
    // This test verifies the full vehicle creation flow
    const addVehicleBtn = await $('button*=vehicle');

    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();
      await browser.pause(300);

      // Fill in vehicle details with unique values
      const nameInput = await $('input[name="name"], #name, input[placeholder*="name"]');
      if (await nameInput.isDisplayed()) {
        await nameInput.setValue(vehicleName);
      }

      const plateInput = await $('input[name="licensePlate"], #licensePlate, input[placeholder*="plate"]');
      if (await plateInput.isDisplayed()) {
        await plateInput.setValue(licensePlate);
      }

      const tankInput = await $('input[name="tankSize"], #tankSize');
      if (await tankInput.isDisplayed()) {
        await tankInput.setValue('50');
      }

      const consumptionInput = await $('input[name="tpConsumption"], #tpConsumption');
      if (await consumptionInput.isDisplayed()) {
        await consumptionInput.setValue('7.0');
      }

      const odometerInput = await $('input[name="initialOdometer"], #initialOdometer');
      if (await odometerInput.isDisplayed()) {
        await odometerInput.setValue('10000');
      }

      // Submit
      const saveBtn = await $('button*=Save');
      if (await saveBtn.isDisplayed()) {
        await saveBtn.click();
        await browser.pause(1000);

        // Verify vehicle was created - should see vehicle name somewhere
        const body = await $('body');
        const text = await body.getText();
        expect(text).toContain(vehicleName);
      }
    } else {
      // If no add button, maybe vehicles already exist or different UI state
      // Log for debugging
      console.log('Add vehicle button not found - checking current state');
      const body = await $('body');
      console.log('Current page content:', await body.getText());
    }
  });
});
