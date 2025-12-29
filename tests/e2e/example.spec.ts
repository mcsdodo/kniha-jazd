import { test, expect } from '@playwright/test';

test('app loads and shows header', async ({ page }) => {
	await page.goto('/');

	// Check that the app header is visible
	await expect(page.locator('h1')).toContainText('Kniha Jázd');
});

test('navigation to settings works', async ({ page }) => {
	await page.goto('/');

	// Click settings link
	await page.click('a[href="/settings"]');

	// Verify we're on settings page - check for settings-specific content
	await expect(page.locator('.nav-link.active')).toContainText('Nastavenia');
});
