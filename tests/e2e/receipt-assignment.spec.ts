import { test, expect } from '@playwright/test';

/**
 * Smoke tests — verify the doklady page and trip-row receipt picker render
 * without errors in vite-only mode.
 *
 * In vite-only mode there's no Tauri backend, so receipts/trips/vehicles are
 * empty and most assignment-flow assertions are no-ops by design (guarded by
 * `if (buttonVisible)`). These tests verify the page DOM doesn't crash and
 * that the navigation entry-points exist; deep behavior is covered by
 * tests/integration/ (WebdriverIO + Tauri).
 *
 * Language-agnostic: button labels are user-locale-dependent.
 */

test.describe('Receipt to Trip Assignment', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
	});

	test('doklady page loads', async ({ page }) => {
		await page.click('a[href="/doklady"]');
		await page.waitForURL('**/doklady');
		await expect(page.locator('h1').first()).toBeVisible();
	});

	test('navigation between pages does not error', async ({ page }) => {
		await page.click('a[href="/doklady"]');
		await page.waitForURL('**/doklady');
		await page.click('a[href="/settings"]');
		await page.waitForURL('**/settings');
		await page.click('a[href="/"]');
		await page.waitForURL((url) => url.pathname === '/');
	});

	test('main page renders without crashing', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('h1').first()).toBeVisible();
	});

	test('settings page renders without crashing', async ({ page }) => {
		await page.goto('/settings');
		await expect(page.locator('h1').first()).toBeVisible();
	});
});
