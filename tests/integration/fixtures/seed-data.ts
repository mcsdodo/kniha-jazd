/**
 * Test data fixtures and UI-based seeding utilities
 *
 * These helpers create test data through the UI, which also validates
 * that the UI flows work correctly.
 */

import { waitForAppReady, fillField, clickButton } from '../utils/app';

/**
 * Default test vehicle data (ICE vehicle)
 */
export const testVehicle = {
  name: 'Test Vehicle',
  licensePlate: 'TEST-123',
  tankSize: 50,
  tpConsumption: 7.0,
  initialOdometer: 10000
};

/**
 * Default test BEV (Battery Electric Vehicle) data
 */
export const testBevVehicle = {
  name: 'Test BEV',
  licensePlate: 'BEV-123',
  batteryCapacity: 75,
  baselineConsumption: 18,
  initialOdometer: 5000,
  initialBattery: 90
};

/**
 * Default test trip data
 */
export const testTrip = {
  date: '2024-01-15',
  origin: 'Bratislava',
  destination: 'Košice',
  distanceKm: 400,
  purpose: 'Služobná cesta'
};

/**
 * Navigate to settings page if not already there
 */
export async function navigateToSettings(): Promise<void> {
  const settingsLink = await $('a[href="/settings"]');
  await settingsLink.click();
  await browser.pause(500);
}

/**
 * Create a vehicle through the UI
 * Returns when vehicle is created and visible
 */
export async function createVehicleViaUI(options: Partial<typeof testVehicle> = {}): Promise<void> {
  const data = { ...testVehicle, ...options };

  await waitForAppReady();

  // Navigate to settings first
  await navigateToSettings();

  // Click "Pridať vozidlo" or similar button
  const addButton = await $('button*=vozidlo');
  if (await addButton.isDisplayed()) {
    await addButton.click();
  }
  await browser.pause(300);

  // Fill in the vehicle form
  await fillField('input[name="name"], #name', data.name);
  await fillField('input[name="licensePlate"], #licensePlate', data.licensePlate);
  await fillField('input[name="tankSize"], #tankSize', data.tankSize.toString());
  await fillField('input[name="tpConsumption"], #tpConsumption', data.tpConsumption.toString());
  await fillField('input[name="initialOdometer"], #initialOdometer', data.initialOdometer.toString());

  // Submit the form
  await clickButton('Uložiť');

  // Wait for vehicle to appear
  await browser.pause(500);
}

/**
 * Create a BEV (Battery Electric Vehicle) through the UI
 * Returns when vehicle is created and visible
 */
export async function createBevVehicleViaUI(options: Partial<typeof testBevVehicle> = {}): Promise<void> {
  const data = { ...testBevVehicle, ...options };

  await waitForAppReady();

  // Navigate to settings first
  await navigateToSettings();

  // Click "Pridať vozidlo" or similar button
  const addButton = await $('button*=vozidlo');
  if (await addButton.isDisplayed()) {
    await addButton.click();
  }
  await browser.pause(300);

  // Fill basic info
  const nameInput = await $('#name');
  await nameInput.setValue(data.name);

  const plateInput = await $('#license-plate');
  await plateInput.setValue(data.licensePlate);

  // Select BEV vehicle type
  const typeDropdown = await $('#vehicle-type');
  await typeDropdown.selectByAttribute('value', 'Bev');
  await browser.pause(300);

  // Fill ODO
  const odometerInput = await $('#initial-odometer');
  await odometerInput.setValue(data.initialOdometer.toString());

  // Fill battery fields
  const batteryCapacity = await $('#battery-capacity');
  await batteryCapacity.setValue(data.batteryCapacity.toString());

  const baselineConsumption = await $('#baseline-consumption');
  await baselineConsumption.setValue(data.baselineConsumption.toString());

  const initialBattery = await $('#initial-battery');
  await initialBattery.setValue(data.initialBattery.toString());

  // Submit the form
  const saveBtn = await $('button*=Uložiť');
  await saveBtn.click();

  // Wait for vehicle to appear
  await browser.pause(1000);
}

/**
 * Create a trip through the UI
 */
export async function createTripViaUI(options: Partial<typeof testTrip> = {}): Promise<void> {
  const data = { ...testTrip, ...options };

  // Click "Nový záznam" button
  await clickButton('Nový záznam');

  // Wait for editing row to appear
  const editingRow = await $('tr.editing');
  await editingRow.waitForDisplayed({ timeout: 5000 });

  // Fill in trip fields
  // Note: Exact selectors depend on the actual UI implementation
  const dateInput = await $('tr.editing input[type="date"]');
  await dateInput.setValue(data.date);

  const originInput = await $('tr.editing input[name="origin"]');
  await originInput.setValue(data.origin);

  const destInput = await $('tr.editing input[name="destination"]');
  await destInput.setValue(data.destination);

  const distanceInput = await $('tr.editing input[name="distance"]');
  await distanceInput.setValue(data.distanceKm.toString());

  const purposeInput = await $('tr.editing input[name="purpose"]');
  await purposeInput.setValue(data.purpose);

  // Save the trip
  await clickButton('Uložiť');

  // Wait for save to complete
  await browser.pause(500);
}

/**
 * Create a trip with fuel refill
 */
export async function createTripWithFuelViaUI(
  tripData: Partial<typeof testTrip>,
  fuelLiters: number,
  fullTank: boolean = true
): Promise<void> {
  await createTripViaUI(tripData);

  // Additional fuel fields would be filled here
  // This is a simplified version - exact implementation depends on UI structure

  if (fuelLiters > 0) {
    const fuelInput = await $('tr.editing input[name="fuelLiters"]');
    if (await fuelInput.isDisplayed()) {
      await fuelInput.setValue(fuelLiters.toString());
    }

    if (fullTank) {
      const fullTankCheckbox = await $('tr.editing input[name="fullTank"]');
      if (await fullTankCheckbox.isDisplayed()) {
        await fullTankCheckbox.click();
      }
    }
  }
}
