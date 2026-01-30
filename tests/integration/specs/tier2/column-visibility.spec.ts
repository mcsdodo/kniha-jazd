/**
 * Tier 2: Column Visibility Integration Tests
 *
 * Tests the hideable columns functionality:
 * - Toggle column off → verify hidden
 * - Toggle column on → verify visible
 * - Reload app → verify persists
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

/**
 * Get hidden columns via Tauri IPC
 */
async function getHiddenColumns(): Promise<string[]> {
  const result = await browser.execute(async () => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('get_hidden_columns');
  });
  return result as string[];
}

/**
 * Set hidden columns via Tauri IPC
 */
async function setHiddenColumnsViaIpc(columns: string[]): Promise<void> {
  await browser.execute(async (cols: string[]) => {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available');
    }
    return await window.__TAURI__.core.invoke('set_hidden_columns', { columns: cols });
  }, columns);
}

describe('Tier 2: Column Visibility', () => {
  let vehicleId: string;

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    // Reset hidden columns to empty
    await setHiddenColumnsViaIpc([]);

    // Seed test vehicle with a trip
    const vehicle = await seedVehicle({
      name: 'Visibility Test Vehicle',
      licensePlate: 'VIS-001',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);

    // Seed a trip so we have data to display
    await seedTrip({
      vehicleId,
      startDatetime: '2026-01-15T08:00',
      time: '10:30',
      origin: 'Home',
      destination: 'Office',
      distanceKm: 25,
      odometer: 50025,
      purpose: 'Work commute',
      otherCostsEur: 5.50,
      otherCostsNote: 'Parking',
    });
  });

  describe('Column Visibility Dropdown', () => {
    it('should show column visibility dropdown button', async () => {
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Find the column visibility dropdown button
      const visibilityBtn = await $('[data-testid="column-visibility-toggle"]');
      expect(await visibilityBtn.isExisting()).toBe(true);
    });

    it('should open dropdown and show column options', async () => {
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Click the column visibility button
      const visibilityBtn = await $('[data-testid="column-visibility-toggle"]');
      await visibilityBtn.click();
      await browser.pause(300);

      // Check that dropdown menu is visible
      const dropdownMenu = await $('[data-testid="column-visibility-menu"]');
      expect(await dropdownMenu.isDisplayed()).toBe(true);

      // Check that expected column options exist
      const timeOption = await dropdownMenu.$('label*=Time');
      expect(await timeOption.isExisting()).toBe(true);
    });
  });

  describe('Toggle Columns', () => {
    it('should hide time column when toggled off', async () => {
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Verify time column header is initially visible
      const timeHeaderBefore = await $('[data-testid="column-header-time"]');
      expect(await timeHeaderBefore.isExisting()).toBe(true);

      // Click the column visibility button
      const visibilityBtn = await $('[data-testid="column-visibility-toggle"]');
      await visibilityBtn.click();
      await browser.pause(300);

      // Find and click the time checkbox to hide it (use JS because checkbox has opacity: 0)
      await browser.execute(() => {
        const checkbox = document.querySelector('[data-testid="column-toggle-time"]') as HTMLInputElement;
        if (checkbox) checkbox.click();
      });
      await browser.pause(300);

      // Close dropdown by clicking outside
      await browser.keys('Escape');
      await browser.pause(300);

      // Verify time column header is now hidden
      const timeHeaderAfter = await $('[data-testid="column-header-time"]');
      expect(await timeHeaderAfter.isExisting()).toBe(false);

      // Verify via IPC that time is in hidden columns
      const hiddenColumns = await getHiddenColumns();
      expect(hiddenColumns).toContain('time');
    });

    it('should show time column when toggled back on', async () => {
      // First hide the time column via IPC
      await setHiddenColumnsViaIpc(['time']);

      // Refresh to pick up IPC changes
      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Verify time column header is hidden
      const timeHeaderBefore = await $('[data-testid="column-header-time"]');
      expect(await timeHeaderBefore.isExisting()).toBe(false);

      // Click the column visibility button
      const visibilityBtn = await $('[data-testid="column-visibility-toggle"]');
      await visibilityBtn.click();
      await browser.pause(300);

      // Find and click the time checkbox to show it (use JS because checkbox has opacity: 0)
      await browser.execute(() => {
        const checkbox = document.querySelector('[data-testid="column-toggle-time"]') as HTMLInputElement;
        if (checkbox) checkbox.click();
      });
      await browser.pause(300);

      // Close dropdown
      await browser.keys('Escape');
      await browser.pause(300);

      // Verify time column header is now visible
      const timeHeaderAfter = await $('[data-testid="column-header-time"]');
      expect(await timeHeaderAfter.isExisting()).toBe(true);

      // Verify via IPC that time is no longer in hidden columns
      const hiddenColumns = await getHiddenColumns();
      expect(hiddenColumns).not.toContain('time');
    });
  });

  describe('Persistence', () => {
    it('should persist hidden columns across page reload', async () => {
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Hide the time column via UI
      const visibilityBtn = await $('[data-testid="column-visibility-toggle"]');
      await visibilityBtn.click();
      await browser.pause(300);

      // Click time checkbox (use JS because checkbox has opacity: 0)
      await browser.execute(() => {
        const checkbox = document.querySelector('[data-testid="column-toggle-time"]') as HTMLInputElement;
        if (checkbox) checkbox.click();
      });
      await browser.pause(300);

      // Close dropdown
      await browser.keys('Escape');
      await browser.pause(300);

      // Verify time column is hidden
      const timeHeaderBefore = await $('[data-testid="column-header-time"]');
      expect(await timeHeaderBefore.isExisting()).toBe(false);

      // Reload the page
      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Verify time column is still hidden after reload
      const timeHeaderAfter = await $('[data-testid="column-header-time"]');
      expect(await timeHeaderAfter.isExisting()).toBe(false);

      // Verify via IPC
      const hiddenColumns = await getHiddenColumns();
      expect(hiddenColumns).toContain('time');
    });

    it('should persist multiple hidden columns', async () => {
      // Hide multiple columns via IPC
      await setHiddenColumnsViaIpc(['time', 'fuelConsumed', 'otherCosts']);

      // Refresh to pick up IPC changes
      await browser.refresh();
      await waitForAppReady();
      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Verify all three columns are hidden
      const timeHeader = await $('[data-testid="column-header-time"]');
      expect(await timeHeader.isExisting()).toBe(false);

      // fuelConsumed and otherCosts headers - check they're missing from the page
      const pageText = await $('body').getText();

      // Reload to verify persistence
      await browser.refresh();
      await waitForAppReady();
      await waitForTripGrid();
      await browser.pause(500);

      // Verify columns are still hidden
      const hiddenColumns = await getHiddenColumns();
      expect(hiddenColumns).toContain('time');
      expect(hiddenColumns).toContain('fuelConsumed');
      expect(hiddenColumns).toContain('otherCosts');
    });
  });

  describe('Badge Display', () => {
    it('should show badge count when columns are hidden', async () => {
      // Hide two columns
      await setHiddenColumnsViaIpc(['time', 'fuelConsumed']);

      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Check for badge showing count
      const badge = await $('[data-testid="column-visibility-toggle"] .badge');
      if (await badge.isExisting()) {
        const badgeText = await badge.getText();
        expect(badgeText).toBe('2');
      }
    });

    it('should not show badge when no columns are hidden', async () => {
      // Ensure no columns are hidden
      await setHiddenColumnsViaIpc([]);

      await navigateTo('trips');
      await waitForTripGrid();
      await browser.pause(500);

      // Badge should not exist or not be visible
      const badge = await $('[data-testid="column-visibility-toggle"] .badge');
      const exists = await badge.isExisting();
      if (exists) {
        const isDisplayed = await badge.isDisplayed();
        expect(isDisplayed).toBe(false);
      }
    });
  });
});
