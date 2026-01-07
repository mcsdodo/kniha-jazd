/**
 * Standardized selectors and custom assertions for integration tests
 *
 * Provides consistent element selectors and assertion helpers
 * to reduce test fragility and improve maintainability.
 */

// =============================================================================
// Element Selectors
// =============================================================================

/**
 * Navigation selectors
 */
export const Nav = {
  /** Main navigation links */
  tripsLink: 'a[href="/"]',
  settingsLink: 'a[href="/settings"]',
  dokladyLink: 'a[href="/doklady"]',
  backupsLink: 'a[href="/backups"]',

  /** Year picker */
  yearPicker: '#year-picker',
  yearOption: (year: number) => `option[value="${year}"]`,
} as const;

/**
 * Settings page selectors
 */
export const Settings = {
  /** Company info */
  companyName: '#company-name',
  companyIco: '#company-ico',
  bufferTripPurpose: '#buffer-trip-purpose',

  /** Vehicle management */
  addVehicleBtn: 'button*=vozidlo',
  vehicleList: '.vehicle-list',
  vehicleCard: '.vehicle-card',
  editVehicleBtn: 'button*=Upravit',
  deleteVehicleBtn: 'button*=Vymazat',

  /** Vehicle form */
  vehicleForm: {
    name: '#name',
    licensePlate: '#license-plate',
    vehicleType: '#vehicle-type',
    initialOdometer: '#initial-odometer',
    // ICE fields
    tankSize: '#tank-size',
    tpConsumption: '#tp-consumption',
    // BEV fields
    batteryCapacity: '#battery-capacity',
    baselineConsumption: '#baseline-consumption',
    initialBattery: '#initial-battery',
  },

  /** Vehicle type badges */
  iceBadge: '.badge.type-ice',
  bevBadge: '.badge.type-bev',
  phevBadge: '.badge.type-phev',

  /** Save/Cancel buttons */
  saveBtn: 'button*=Ulozit',
  cancelBtn: 'button*=Zrusit',
} as const;

/**
 * Trip grid selectors
 */
export const TripGrid = {
  /** Main grid elements */
  table: '.trip-grid table',
  headerRow: '.trip-grid thead tr',
  dataRows: '.trip-grid tbody tr',
  editingRow: 'tr.editing',

  /** Column headers */
  dateHeader: 'th*=Datum',
  routeHeader: 'th*=Trasa',
  distanceHeader: 'th*=km',
  odometerHeader: 'th*=Stav',
  fuelHeader: 'th*=Tankovanie',
  consumptionHeader: 'th*=Spotreba',
  remainingHeader: 'th*=Zostatok',

  /** Trip row actions */
  newTripBtn: 'button*=Novy zaznam',
  editTripBtn: 'button.edit-trip',
  deleteTripBtn: 'button.delete-trip',
  saveTripBtn: 'button*=Ulozit',
  cancelEditBtn: 'button*=Zrusit',

  /** Trip form fields (in editing row) */
  tripForm: {
    date: 'tr.editing input[type="date"]',
    origin: 'tr.editing input[name="origin"]',
    destination: 'tr.editing input[name="destination"]',
    distance: 'tr.editing input[name="distance"]',
    odometer: 'tr.editing input[name="odometer"]',
    purpose: 'tr.editing input[name="purpose"]',
    // Fuel fields
    fuelLiters: 'tr.editing input[name="fuelLiters"]',
    fuelCost: 'tr.editing input[name="fuelCost"]',
    fullTank: 'tr.editing input[name="fullTank"]',
    // Energy fields
    energyKwh: 'tr.editing input[name="energyKwh"]',
    energyCost: 'tr.editing input[name="energyCost"]',
    fullCharge: 'tr.editing input[name="fullCharge"]',
    socOverride: 'tr.editing input[name="socOverride"]',
    // Other costs
    otherCosts: 'tr.editing input[name="otherCosts"]',
    otherCostsNote: 'tr.editing input[name="otherCostsNote"]',
  },

  /** Warning indicators */
  consumptionWarning: '.consumption-warning',
  dateWarning: '.date-warning',
  receiptWarning: '.receipt-warning',

  /** Stats/summary */
  totalKm: '.stats .total-km',
  totalFuel: '.stats .total-fuel',
  avgConsumption: '.stats .avg-consumption',
  marginPercent: '.stats .margin-percent',
} as const;

/**
 * Doklady (receipts) page selectors
 */
export const Doklady = {
  /** Main elements */
  receiptList: '.receipt-list',
  receiptCard: '.receipt-card',
  scanBtn: 'button*=Skenovat',
  assignBtn: 'button*=Priradit',

  /** Receipt details */
  receiptDate: '.receipt-date',
  receiptLiters: '.receipt-liters',
  receiptPrice: '.receipt-price',
  receiptStation: '.receipt-station',

  /** Status badges */
  pendingBadge: '.status-pending',
  parsedBadge: '.status-parsed',
  needsReviewBadge: '.status-needs-review',
  assignedBadge: '.status-assigned',

  /** Verification */
  verifyBtn: 'button*=Overit',
  verificationResult: '.verification-result',
} as const;

/**
 * Modal/Dialog selectors
 */
export const Modal = {
  backdrop: '.modal-backdrop',
  container: '.modal',
  closeBtn: '.modal .close-button',
  title: '.modal-title',
  content: '.modal-content',
  confirmBtn: 'button*=Potvrdit',
  cancelBtn: 'button*=Zrusit',
} as const;

/**
 * Toast/Notification selectors
 */
export const Toast = {
  container: '.toast',
  success: '.toast.success',
  error: '.toast.error',
  warning: '.toast.warning',
  info: '.toast.info',
  closeBtn: '.toast .close-button',
} as const;

// =============================================================================
// Custom Assertion Helpers
// =============================================================================

/**
 * Assert that an element contains specific text
 */
export async function assertContainsText(
  selector: string,
  expectedText: string,
  timeout = 5000
): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout });
  const actualText = await element.getText();

  if (!actualText.includes(expectedText)) {
    throw new Error(
      `Expected "${selector}" to contain "${expectedText}", but got "${actualText}"`
    );
  }
}

/**
 * Assert that an element has a specific value (for input fields)
 */
export async function assertHasValue(
  selector: string,
  expectedValue: string,
  timeout = 5000
): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout });
  const actualValue = await element.getValue();

  if (actualValue !== expectedValue) {
    throw new Error(
      `Expected "${selector}" to have value "${expectedValue}", but got "${actualValue}"`
    );
  }
}

/**
 * Assert that an element is visible
 */
export async function assertVisible(selector: string, timeout = 5000): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout });
}

/**
 * Assert that an element is not visible
 */
export async function assertNotVisible(selector: string, timeout = 2000): Promise<void> {
  const element = await $(selector);
  try {
    await element.waitForDisplayed({ timeout, reverse: true });
  } catch {
    const isDisplayed = await element.isDisplayed();
    if (isDisplayed) {
      throw new Error(`Expected "${selector}" to not be visible, but it is`);
    }
  }
}

/**
 * Assert that an input field is disabled
 */
export async function assertDisabled(selector: string, timeout = 5000): Promise<void> {
  const element = await $(selector);
  await element.waitForExist({ timeout });
  const isDisabled = await element.getAttribute('disabled');

  if (isDisabled === null) {
    throw new Error(`Expected "${selector}" to be disabled, but it is enabled`);
  }
}

/**
 * Assert that an input field is enabled
 */
export async function assertEnabled(selector: string, timeout = 5000): Promise<void> {
  const element = await $(selector);
  await element.waitForExist({ timeout });
  const isDisabled = await element.getAttribute('disabled');

  if (isDisabled !== null) {
    throw new Error(`Expected "${selector}" to be enabled, but it is disabled`);
  }
}

/**
 * Assert that a trip row has a consumption warning
 */
export async function assertTripHasConsumptionWarning(tripRowIndex: number): Promise<void> {
  const rows = await $$(TripGrid.dataRows);
  if (tripRowIndex >= rows.length) {
    throw new Error(`Trip row ${tripRowIndex} does not exist (${rows.length} rows total)`);
  }

  const row = rows[tripRowIndex];
  const warning = await row.$(TripGrid.consumptionWarning);
  const isDisplayed = await warning.isDisplayed();

  if (!isDisplayed) {
    throw new Error(`Expected trip row ${tripRowIndex} to have consumption warning`);
  }
}

/**
 * Assert that a trip row does NOT have a consumption warning
 */
export async function assertTripNoConsumptionWarning(tripRowIndex: number): Promise<void> {
  const rows = await $$(TripGrid.dataRows);
  if (tripRowIndex >= rows.length) {
    throw new Error(`Trip row ${tripRowIndex} does not exist (${rows.length} rows total)`);
  }

  const row = rows[tripRowIndex];
  const warning = await row.$(TripGrid.consumptionWarning);
  const exists = await warning.isExisting();

  if (exists) {
    const isDisplayed = await warning.isDisplayed();
    if (isDisplayed) {
      throw new Error(`Expected trip row ${tripRowIndex} to NOT have consumption warning`);
    }
  }
}

/**
 * Assert the current URL contains a specific path
 */
export async function assertUrlContains(expectedPath: string): Promise<void> {
  const url = await browser.getUrl();
  if (!url.includes(expectedPath)) {
    throw new Error(`Expected URL to contain "${expectedPath}", but got "${url}"`);
  }
}

/**
 * Assert that a select dropdown has a specific option selected
 */
export async function assertSelectedOption(
  selector: string,
  expectedValue: string,
  timeout = 5000
): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout });
  const actualValue = await element.getValue();

  if (actualValue !== expectedValue) {
    throw new Error(
      `Expected "${selector}" to have selected value "${expectedValue}", but got "${actualValue}"`
    );
  }
}

/**
 * Assert trip count in the grid
 */
export async function assertTripCount(expectedCount: number): Promise<void> {
  const rows = await $$(TripGrid.dataRows);
  const actualCount = rows.length;

  if (actualCount !== expectedCount) {
    throw new Error(`Expected ${expectedCount} trips, but found ${actualCount}`);
  }
}

/**
 * Assert vehicle type badge is displayed
 */
export async function assertVehicleTypeBadge(
  vehicleType: 'Ice' | 'Bev' | 'Phev'
): Promise<void> {
  const badgeSelector =
    vehicleType === 'Ice'
      ? Settings.iceBadge
      : vehicleType === 'Bev'
        ? Settings.bevBadge
        : Settings.phevBadge;

  const badge = await $(badgeSelector);
  await badge.waitForDisplayed({ timeout: 5000 });
}

// =============================================================================
// Wait Helpers
// =============================================================================

/**
 * Wait for a toast message to appear and optionally contain specific text
 */
export async function waitForToast(
  type: 'success' | 'error' | 'warning' | 'info' = 'success',
  expectedText?: string,
  timeout = 5000
): Promise<void> {
  const toastSelector =
    type === 'success'
      ? Toast.success
      : type === 'error'
        ? Toast.error
        : type === 'warning'
          ? Toast.warning
          : Toast.info;

  const toast = await $(toastSelector);
  await toast.waitForDisplayed({ timeout });

  if (expectedText) {
    await assertContainsText(toastSelector, expectedText);
  }
}

/**
 * Wait for the trip grid to be loaded
 */
export async function waitForTripGrid(timeout = 10000): Promise<void> {
  const grid = await $(TripGrid.table);
  await grid.waitForDisplayed({ timeout });
}

/**
 * Wait for a modal to appear
 */
export async function waitForModal(timeout = 5000): Promise<void> {
  const modal = await $(Modal.container);
  await modal.waitForDisplayed({ timeout });
}

/**
 * Wait for a modal to close
 */
export async function waitForModalClose(timeout = 5000): Promise<void> {
  const modal = await $(Modal.container);
  await modal.waitForDisplayed({ timeout, reverse: true });
}

// =============================================================================
// Data Extraction Helpers
// =============================================================================

/**
 * Get the current margin percent from the stats display
 */
export async function getMarginPercent(): Promise<number | null> {
  const element = await $(TripGrid.marginPercent);
  const exists = await element.isExisting();

  if (!exists) {
    return null;
  }

  const text = await element.getText();
  const match = text.match(/(\d+(?:\.\d+)?)/);
  return match ? parseFloat(match[1]) : null;
}

/**
 * Get the total km from the stats display
 */
export async function getTotalKm(): Promise<number | null> {
  const element = await $(TripGrid.totalKm);
  const exists = await element.isExisting();

  if (!exists) {
    return null;
  }

  const text = await element.getText();
  const match = text.match(/(\d+(?:\.\d+)?)/);
  return match ? parseFloat(match[1]) : null;
}

/**
 * Get the currently selected year from the year picker
 */
export async function getSelectedYear(): Promise<number> {
  const yearPicker = await $(Nav.yearPicker);
  const value = await yearPicker.getValue();
  return parseInt(value, 10);
}
