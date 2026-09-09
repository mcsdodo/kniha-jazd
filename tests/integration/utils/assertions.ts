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
  yearPicker: '#year-select',
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
  addVehicleBtn: 'button*=vehicle',
  vehicleList: '.vehicle-list',
  vehicleCard: '.vehicle-card',
  editVehicleBtn: 'button*=Edit',
  deleteVehicleBtn: 'button*=Delete',

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

  /** Save/Cancel buttons (in modal) */
  saveBtn: '.modal-footer button.button-primary',
  cancelBtn: '.modal-footer button.button-secondary',
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
  dateHeader: 'th*=Date',
  routeHeader: 'th*=Route',
  distanceHeader: 'th*=km',
  odometerHeader: 'th*=Odo',
  fuelHeader: 'th*=Refuel',
  consumptionHeader: 'th*=Consumption',
  remainingHeader: 'th*=Remaining',

  /** Trip row actions */
  newTripBtn: 'button*=New record',
  editTripBtn: 'button.edit-trip',
  deleteTripBtn: 'button.delete-trip',
  saveTripBtn: 'button*=Save',
  cancelEditBtn: 'button*=Cancel',

  /** Trip form fields (in editing row)
   * Uses data-testid attributes for robust, position-independent selectors
   */
  tripForm: {
    date: '[data-testid="trip-start-datetime"]',
    origin: '[data-testid="trip-origin"]',
    destination: '[data-testid="trip-destination"]',
    distance: '[data-testid="trip-distance"]',
    odometer: '[data-testid="trip-odometer"]',
    purpose: '[data-testid="trip-purpose"]',
    // Fuel fields
    fuelLiters: '[data-testid="trip-fuel-liters"]',
    fuelCost: '[data-testid="trip-fuel-cost"]',
    fullTank: '[data-testid="trip-full-tank"]',
    // Energy fields
    energyKwh: '[data-testid="trip-energy-kwh"]',
    energyCost: '[data-testid="trip-energy-cost"]',
    fullCharge: '[data-testid="trip-full-charge"]',
    socOverride: '[data-testid="trip-soc-override"]',
    // Other costs
    otherCosts: '[data-testid="trip-other-costs"]',
    otherCostsNote: '[data-testid="trip-other-costs-note"]',
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
  scanBtn: 'button*=Scan',
  assignBtn: 'button*=Assign',

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
  verifyBtn: 'button*=Verify',
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
  confirmBtn: 'button*=Confirm',
  cancelBtn: 'button*=Cancel',
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

/**
 * The odometer cascade modal (task 81). Shown instead of a silent write
 * whenever a create, edit or delete would move another row's odometer.
 * Same testids as odometer-cascade.spec.ts, which owns the feature spec.
 */
export const CascadeModal = {
  container: '[data-testid="cascade-modal"]',
  summary: '[data-testid="cascade-summary"]',
  confirm: '[data-testid="cascade-confirm"]',
  cancel: '[data-testid="cascade-cancel"]',
} as const;

/**
 * The old one-line confirmStore dialog (`GlobalConfirm.svelte` /
 * `ConfirmModal.svelte`). Task 81 left it in place for the one case the
 * cascade modal does not cover: a delete that moves no other row. It has no
 * data-testid, so the danger button is the identifying selector -- the
 * cascade modal's own buttons never carry the `danger` class.
 */
export const LegacyConfirmDialog = {
  dangerButton: '.modal .button-small.danger',
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
  const rowCount = await rows.length;
  if (tripRowIndex >= rowCount) {
    throw new Error(`Trip row ${tripRowIndex} does not exist (${rowCount} rows total)`);
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
  const rowCount = await rows.length;
  if (tripRowIndex >= rowCount) {
    throw new Error(`Trip row ${tripRowIndex} does not exist (${rowCount} rows total)`);
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
  const actualCount = await rows.length;

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

/** What `resolveTripDialog` found and settled. */
export type TripDialogKind = 'none' | 'cascade' | 'confirm';

/**
 * Settle whatever dialog (if any) follows a trip create, edit or delete
 * (task 81's odometer cascade; task 11b's stale-spec fix).
 *
 * Call this immediately after triggering the write (Enter, the Save
 * icon-button, or the delete icon-button). The write lands one of three
 * ways, and only one of them ever happens for a single action:
 *  - silently -- the plan moves no other row (creates/edits only; a delete
 *    always shows one of the dialogs below, see TripGrid.svelte
 *    `handleDelete`).
 *  - the cascade modal -- the plan moves at least one later row's odometer.
 *  - delete only: the old one-line confirmStore dialog -- the delete moves
 *    nothing, but still destroys a legal record with no undo.
 *
 * `isDone` is the caller's fast, context-specific proof the write already
 * landed with nothing to confirm -- e.g. the editing row's `.editing` class
 * having dropped for a save. It defaults to "never", which is correct for a
 * delete (there is no silent path to detect) and makes a save caller that
 * omits it wait out the full timeout on the genuinely-silent path -- pass it
 * whenever that path is possible, so the common case does not eat the whole
 * timeout every time.
 */
export async function resolveTripDialog(
  isDone: () => Promise<boolean> = async () => false,
  timeout = 5000
): Promise<TripDialogKind> {
  let kind: TripDialogKind | undefined;
  await browser.waitUntil(
    async () => {
      if (await $(CascadeModal.container).isExisting()) {
        kind = 'cascade';
        return true;
      }
      if (await $(LegacyConfirmDialog.dangerButton).isExisting()) {
        kind = 'confirm';
        return true;
      }
      if (await isDone()) {
        kind = 'none';
        return true;
      }
      return false;
    },
    {
      timeout,
      timeoutMsg:
        'resolveTripDialog: the write neither completed silently nor opened a dialog',
    }
  );

  if (kind === 'cascade') {
    const confirmBtn = await $(CascadeModal.confirm);
    await confirmBtn.waitForClickable({ timeout });
    await confirmBtn.click();
    await $(CascadeModal.container).waitForExist({ timeout, reverse: true });
  } else if (kind === 'confirm') {
    const dangerBtn = await $(LegacyConfirmDialog.dangerButton);
    await dangerBtn.waitForClickable({ timeout });
    await dangerBtn.click();
    await dangerBtn.waitForExist({ timeout, reverse: true });
  }

  return kind as TripDialogKind;
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
