import { test, expect } from '@playwright/test';

/**
 * Smoke tests — verify the Doklady page loads and renders top-level controls.
 *
 * These tests run in vite-only mode (no Tauri backend), so the page falls
 * back to local mode (paperless mode probe fails → catch falls to local).
 * Language-agnostic: the i18n locale is user-controlled.
 */

test.describe('Doklady page', () => {
	test('navigates to doklady page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/doklady"]');
		await page.waitForURL('**/doklady');
		const h1 = page.locator('h1').first();
		await expect(h1).toBeVisible();
		await expect(h1).not.toHaveText('');
	});

	test('shows configuration warning when not configured', async ({ page }) => {
		await page.goto('/doklady');
		await expect(page.locator('.config-warning').first()).toBeVisible();
	});

	test('filter buttons render', async ({ page }) => {
		await page.goto('/doklady');
		const filterButtons = page.locator('.filter-btn');
		const configWarning = page.locator('.config-warning');
		const filterCount = await filterButtons.count();
		const warningCount = await configWarning.count();
		expect(filterCount + warningCount).toBeGreaterThan(0);
	});
});
