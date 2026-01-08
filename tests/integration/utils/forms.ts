/**
 * Form utilities for integration tests
 *
 * Provides helpers for filling forms, selecting options,
 * and interacting with common UI elements.
 */

import { Nav, Settings, TripGrid } from './assertions';

// =============================================================================
// Generic Form Helpers
// =============================================================================

/**
 * Fill a text input field
 */
export async function fillField(selector: string, value: string): Promise<void> {
  const field = await $(selector);
  await field.waitForDisplayed({ timeout: 5000 });
  await field.clearValue();
  await field.setValue(value);
}

/**
 * Fill a numeric input field
 */
export async function fillNumericField(selector: string, value: number): Promise<void> {
  await fillField(selector, value.toString());
}

/**
 * Select an option from a dropdown by value
 */
export async function selectOption(selector: string, value: string): Promise<void> {
  const dropdown = await $(selector);
  await dropdown.waitForDisplayed({ timeout: 5000 });
  await dropdown.selectByAttribute('value', value);
}

/**
 * Select an option from a dropdown by visible text
 */
export async function selectOptionByText(selector: string, text: string): Promise<void> {
  const dropdown = await $(selector);
  await dropdown.waitForDisplayed({ timeout: 5000 });
  await dropdown.selectByVisibleText(text);
}

/**
 * Check or uncheck a checkbox
 */
export async function setCheckbox(selector: string, checked: boolean): Promise<void> {
  const checkbox = await $(selector);
  await checkbox.waitForDisplayed({ timeout: 5000 });
  const isChecked = await checkbox.isSelected();

  if (isChecked !== checked) {
    await checkbox.click();
  }
}

/**
 * Click a button by selector
 */
export async function clickButton(selector: string): Promise<void> {
  const button = await $(selector);
  await button.waitForClickable({ timeout: 5000 });
  await button.click();
}

/**
 * Click a button by text content
 */
export async function clickButtonByText(text: string): Promise<void> {
  const button = await $(`button*=${text}`);
  await button.waitForClickable({ timeout: 5000 });
  await button.click();
}

// =============================================================================
// Year Picker
// =============================================================================

/**
 * Wait for a specific year to be available in the year picker
 * This is useful after seeding data when the year picker options may need to refresh
 */
export async function waitForYearOption(year: number, timeout = 10000): Promise<void> {
  await browser.waitUntil(
    async () => {
      const yearPicker = await $(Nav.yearPicker);
      const exists = await yearPicker.isExisting();
      if (!exists) return false;

      const options = await yearPicker.$$('option');
      for (const option of options) {
        const value = await option.getAttribute('value');
        if (value === year.toString()) {
          return true;
        }
      }
      return false;
    },
    {
      timeout,
      timeoutMsg: `Year ${year} not available in year picker within ${timeout}ms`,
      interval: 500,
    }
  );
}

/**
 * Select a specific year in the year picker
 * Waits for the year to be available first (handles dynamic population)
 */
export async function selectYear(year: number): Promise<void> {
  const yearPicker = await $(Nav.yearPicker);
  const exists = await yearPicker.isExisting();

  if (!exists) {
    throw new Error('Year picker not found on page');
  }

  // Wait for the year option to be available (handles async year picker population)
  await waitForYearOption(year);

  await yearPicker.selectByAttribute('value', year.toString());
  await browser.pause(500); // Wait for data to reload
}

/**
 * Get the currently selected year
 */
export async function getSelectedYear(): Promise<number> {
  const yearPicker = await $(Nav.yearPicker);
  const value = await yearPicker.getValue();
  return parseInt(value, 10);
}

/**
 * Get all available years in the year picker
 */
export async function getAvailableYears(): Promise<number[]> {
  const yearPicker = await $(Nav.yearPicker);
  const options = await yearPicker.$$('option');
  const years: number[] = [];

  for (const option of options) {
    const value = await option.getAttribute('value');
    if (value) {
      years.push(parseInt(value, 10));
    }
  }

  return years.sort((a, b) => b - a); // Descending order
}

// =============================================================================
// Fuel Fields (ICE + PHEV)
// =============================================================================

/**
 * Options for filling fuel fields
 */
export interface FuelFieldOptions {
  liters: number;
  costEur?: number;
  fullTank?: boolean;
}

/**
 * Fill fuel fields in the trip editing row
 */
export async function fillFuelFields(options: FuelFieldOptions): Promise<void> {
  const { liters, costEur, fullTank = true } = options;

  // Fill liters
  const litersInput = await $(TripGrid.tripForm.fuelLiters);
  const litersExists = await litersInput.isExisting();

  if (!litersExists) {
    throw new Error('Fuel liters field not found - is this an ICE/PHEV vehicle?');
  }

  await litersInput.clearValue();
  await litersInput.setValue(liters.toString());

  // Fill cost if provided
  if (costEur !== undefined) {
    const costInput = await $(TripGrid.tripForm.fuelCost);
    const costExists = await costInput.isExisting();

    if (costExists) {
      await costInput.clearValue();
      await costInput.setValue(costEur.toString());
    }
  }

  // Set full tank checkbox
  const fullTankCheckbox = await $(TripGrid.tripForm.fullTank);
  const checkboxExists = await fullTankCheckbox.isExisting();

  if (checkboxExists) {
    const isChecked = await fullTankCheckbox.isSelected();
    if (isChecked !== fullTank) {
      await fullTankCheckbox.click();
    }
  }
}

/**
 * Clear fuel fields (for trips without refueling)
 */
export async function clearFuelFields(): Promise<void> {
  const litersInput = await $(TripGrid.tripForm.fuelLiters);
  const exists = await litersInput.isExisting();

  if (exists) {
    await litersInput.clearValue();
  }

  const costInput = await $(TripGrid.tripForm.fuelCost);
  const costExists = await costInput.isExisting();

  if (costExists) {
    await costInput.clearValue();
  }
}

// =============================================================================
// Energy Fields (BEV + PHEV)
// =============================================================================

/**
 * Options for filling energy fields
 */
export interface EnergyFieldOptions {
  kwh: number;
  costEur?: number;
  fullCharge?: boolean;
  socOverridePercent?: number;
}

/**
 * Fill energy fields in the trip editing row
 */
export async function fillEnergyFields(options: EnergyFieldOptions): Promise<void> {
  const { kwh, costEur, fullCharge = true, socOverridePercent } = options;

  // Fill kWh
  const kwhInput = await $(TripGrid.tripForm.energyKwh);
  const kwhExists = await kwhInput.isExisting();

  if (!kwhExists) {
    throw new Error('Energy kWh field not found - is this a BEV/PHEV vehicle?');
  }

  await kwhInput.clearValue();
  await kwhInput.setValue(kwh.toString());

  // Fill cost if provided
  if (costEur !== undefined) {
    const costInput = await $(TripGrid.tripForm.energyCost);
    const costExists = await costInput.isExisting();

    if (costExists) {
      await costInput.clearValue();
      await costInput.setValue(costEur.toString());
    }
  }

  // Set full charge checkbox
  const fullChargeCheckbox = await $(TripGrid.tripForm.fullCharge);
  const checkboxExists = await fullChargeCheckbox.isExisting();

  if (checkboxExists) {
    const isChecked = await fullChargeCheckbox.isSelected();
    if (isChecked !== fullCharge) {
      await fullChargeCheckbox.click();
    }
  }

  // Set SoC override if provided
  if (socOverridePercent !== undefined) {
    const socInput = await $(TripGrid.tripForm.socOverride);
    const socExists = await socInput.isExisting();

    if (socExists) {
      await socInput.clearValue();
      await socInput.setValue(socOverridePercent.toString());
    }
  }
}

/**
 * Clear energy fields (for trips without charging)
 */
export async function clearEnergyFields(): Promise<void> {
  const kwhInput = await $(TripGrid.tripForm.energyKwh);
  const exists = await kwhInput.isExisting();

  if (exists) {
    await kwhInput.clearValue();
  }

  const costInput = await $(TripGrid.tripForm.energyCost);
  const costExists = await costInput.isExisting();

  if (costExists) {
    await costInput.clearValue();
  }

  const socInput = await $(TripGrid.tripForm.socOverride);
  const socExists = await socInput.isExisting();

  if (socExists) {
    await socInput.clearValue();
  }
}

// =============================================================================
// Trip Form Helpers
// =============================================================================

/**
 * Options for filling a complete trip
 */
export interface TripFormOptions {
  date: string; // YYYY-MM-DD
  origin: string;
  destination: string;
  distanceKm: number;
  odometer?: number;
  purpose: string;
  // Fuel (ICE + PHEV)
  fuel?: FuelFieldOptions;
  // Energy (BEV + PHEV)
  energy?: EnergyFieldOptions;
  // Other costs
  otherCostsEur?: number;
  otherCostsNote?: string;
}

/**
 * Fill all fields of a trip form
 */
export async function fillTripForm(options: TripFormOptions): Promise<void> {
  const {
    date,
    origin,
    destination,
    distanceKm,
    odometer,
    purpose,
    fuel,
    energy,
    otherCostsEur,
    otherCostsNote,
  } = options;

  // Wait for editing row to be visible
  const editingRow = await $(TripGrid.editingRow);
  await editingRow.waitForDisplayed({ timeout: 5000 });

  // Fill basic fields
  await fillField(TripGrid.tripForm.date, date);
  await fillField(TripGrid.tripForm.origin, origin);
  await fillField(TripGrid.tripForm.destination, destination);
  await fillNumericField(TripGrid.tripForm.distance, distanceKm);

  if (odometer !== undefined) {
    await fillNumericField(TripGrid.tripForm.odometer, odometer);
  }

  await fillField(TripGrid.tripForm.purpose, purpose);

  // Fill fuel fields if provided
  if (fuel) {
    await fillFuelFields(fuel);
  }

  // Fill energy fields if provided
  if (energy) {
    await fillEnergyFields(energy);
  }

  // Fill other costs if provided
  if (otherCostsEur !== undefined) {
    const otherCostsInput = await $(TripGrid.tripForm.otherCosts);
    const exists = await otherCostsInput.isExisting();

    if (exists) {
      await otherCostsInput.clearValue();
      await otherCostsInput.setValue(otherCostsEur.toString());
    }
  }

  if (otherCostsNote !== undefined) {
    const noteInput = await $(TripGrid.tripForm.otherCostsNote);
    const exists = await noteInput.isExisting();

    if (exists) {
      await noteInput.clearValue();
      await noteInput.setValue(otherCostsNote);
    }
  }
}

/**
 * Save the current trip (click Save button)
 */
export async function saveTripForm(): Promise<void> {
  await clickButton(TripGrid.saveTripBtn);
  await browser.pause(500); // Wait for save to complete
}

/**
 * Cancel editing the current trip
 */
export async function cancelTripEdit(): Promise<void> {
  await clickButton(TripGrid.cancelEditBtn);
  await browser.pause(300);
}

// =============================================================================
// Vehicle Form Helpers
// =============================================================================

/**
 * Options for filling a vehicle form
 */
export interface VehicleFormOptions {
  name: string;
  licensePlate: string;
  vehicleType?: 'Ice' | 'Bev' | 'Phev';
  initialOdometer: number;
  // ICE fields
  tankSizeLiters?: number;
  tpConsumption?: number;
  // BEV fields
  batteryCapacityKwh?: number;
  baselineConsumptionKwh?: number;
  initialBatteryPercent?: number;
}

/**
 * Fill the vehicle form
 */
export async function fillVehicleForm(options: VehicleFormOptions): Promise<void> {
  const {
    name,
    licensePlate,
    vehicleType = 'Ice',
    initialOdometer,
    tankSizeLiters,
    tpConsumption,
    batteryCapacityKwh,
    baselineConsumptionKwh,
    initialBatteryPercent,
  } = options;

  // Wait for modal to be visible
  const modalContent = await $('.modal-content');
  await modalContent.waitForDisplayed({ timeout: 5000 });

  // Fill basic fields
  await fillField(Settings.vehicleForm.name, name);
  await fillField(Settings.vehicleForm.licensePlate, licensePlate);
  await fillNumericField(Settings.vehicleForm.initialOdometer, initialOdometer);

  // Select vehicle type
  await selectOption(Settings.vehicleForm.vehicleType, vehicleType);
  await browser.pause(300); // Wait for form to update based on type

  // Fill type-specific fields
  if (vehicleType === 'Ice' || vehicleType === 'Phev') {
    if (tankSizeLiters !== undefined) {
      await fillNumericField(Settings.vehicleForm.tankSize, tankSizeLiters);
    }
    if (tpConsumption !== undefined) {
      await fillNumericField(Settings.vehicleForm.tpConsumption, tpConsumption);
    }
  }

  if (vehicleType === 'Bev' || vehicleType === 'Phev') {
    if (batteryCapacityKwh !== undefined) {
      await fillNumericField(Settings.vehicleForm.batteryCapacity, batteryCapacityKwh);
    }
    if (baselineConsumptionKwh !== undefined) {
      await fillNumericField(Settings.vehicleForm.baselineConsumption, baselineConsumptionKwh);
    }
    if (initialBatteryPercent !== undefined) {
      await fillNumericField(Settings.vehicleForm.initialBattery, initialBatteryPercent);
    }
  }
}

/**
 * Save the vehicle form
 */
export async function saveVehicleForm(): Promise<void> {
  // Wait for modal footer to be visible before clicking
  const modalFooter = await $('.modal-footer');
  await modalFooter.waitForDisplayed({ timeout: 5000 });

  await clickButton(Settings.saveBtn);
  await browser.pause(500); // Wait for save to complete
}

/**
 * Cancel the vehicle form
 */
export async function cancelVehicleForm(): Promise<void> {
  await clickButton(Settings.cancelBtn);
  await browser.pause(300);
}

// =============================================================================
// Settings Form Helpers
// =============================================================================

/**
 * Options for filling company settings
 */
export interface CompanySettingsOptions {
  companyName: string;
  companyIco: string;
  bufferTripPurpose?: string;
}

/**
 * Fill company settings form
 */
export async function fillCompanySettings(options: CompanySettingsOptions): Promise<void> {
  const { companyName, companyIco, bufferTripPurpose } = options;

  await fillField(Settings.companyName, companyName);
  await fillField(Settings.companyIco, companyIco);

  if (bufferTripPurpose !== undefined) {
    await fillField(Settings.bufferTripPurpose, bufferTripPurpose);
  }
}
