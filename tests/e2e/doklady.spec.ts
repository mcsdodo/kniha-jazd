import { test, expect } from '@playwright/test';

test.describe('Doklady page', () => {
	test('navigates to doklady page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/doklady"]');
		await expect(page.locator('h1')).toContainText('Doklady');
	});

	test('shows configuration warning when not configured', async ({ page }) => {
		await page.goto('/doklady');
		await expect(page.locator('.config-warning')).toBeVisible();
		await expect(page.locator('.config-warning')).toContainText('nie je nakonfigurovana');
	});

	test('sync button is disabled when not configured', async ({ page }) => {
		await page.goto('/doklady');
		const syncButton = page.locator('button:has-text("Sync")');
		await expect(syncButton).toBeDisabled();
	});

	test('filter buttons work', async ({ page }) => {
		await page.goto('/doklady');

		// All filters should be visible
		await expect(page.locator('.filter-btn:has-text("Vsetky")')).toBeVisible();
		await expect(page.locator('.filter-btn:has-text("Nepridelene")')).toBeVisible();
		await expect(page.locator('.filter-btn:has-text("Na kontrolu")')).toBeVisible();

		// Click unassigned filter
		await page.click('.filter-btn:has-text("Nepridelene")');
		await expect(page.locator('.filter-btn:has-text("Nepridelene")')).toHaveClass(/active/);
	});
});
