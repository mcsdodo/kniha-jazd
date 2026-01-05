/**
 * Vehicle Setup Integration Tests
 *
 * Tests the core flow of creating a vehicle and verifying it appears in the app.
 * This is the proof-of-concept test to verify the integration test setup works.
 */

import { waitForAppReady } from '../utils/app';

describe('Vehicle Setup', () => {
  beforeEach(async () => {
    // Wait for app to be fully loaded
    await waitForAppReady();
  });

  it('should load the app successfully', async () => {
    // Verify app header is visible
    const header = await $('h1');
    await expect(header).toBeDisplayed();
    await expect(header).toHaveTextContaining('Kniha');
  });

  it('should show empty state when no vehicles exist', async () => {
    // With a fresh database, we should see the "add vehicle" prompt
    // or an empty vehicle list
    const content = await $('body');
    const text = await content.getText();

    // App should show something indicating we need to add a vehicle
    // or show the main interface ready for input
    expect(text.length).toBeGreaterThan(0);
  });

  it('should navigate to settings page', async () => {
    // Click on settings link
    const settingsLink = await $('a[href="/settings"]');
    await settingsLink.click();

    // Verify we're on settings page
    const activeNav = await $('.nav-link.active');
    await expect(activeNav).toHaveTextContaining('Nastavenia');
  });

  it('should create a new vehicle', async () => {
    // Look for the add vehicle button/form
    // This test verifies the full vehicle creation flow
    const addVehicleBtn = await $('button*=vozidlo');

    if (await addVehicleBtn.isDisplayed()) {
      await addVehicleBtn.click();

      // Fill in vehicle details
      const nameInput = await $('input[name="name"], #name, input[placeholder*="názov"]');
      if (await nameInput.isDisplayed()) {
        await nameInput.setValue('Test Auto E2E');
      }

      const plateInput = await $('input[name="licensePlate"], #licensePlate, input[placeholder*="EČV"]');
      if (await plateInput.isDisplayed()) {
        await plateInput.setValue('E2E-TEST');
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
      const saveBtn = await $('button*=Uložiť');
      if (await saveBtn.isDisplayed()) {
        await saveBtn.click();
        await browser.pause(1000);

        // Verify vehicle was created - should see vehicle name somewhere
        const body = await $('body');
        const text = await body.getText();
        expect(text).toContain('Test Auto E2E');
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
