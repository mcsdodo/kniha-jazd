import { test, expect } from '@playwright/test';

/**
 * Smoke tests — verify the SvelteKit shell loads and basic navigation works.
 * Language-agnostic: the i18n locale is user-controlled and may be either
 * Slovak ("Kniha Jázd") or English ("Trip Logbook"), so we assert on
 * structure (h1 has content) rather than exact strings.
 */

test('app loads and shows header', async ({ page }) => {
	await page.goto('/');
	const h1 = page.locator('h1').first();
	await expect(h1).toBeVisible();
	await expect(h1).not.toHaveText('');
});

test('navigation to settings works', async ({ page }) => {
	await page.goto('/');
	await page.click('a[href="/settings"]');
	await expect(page.locator('a[href="/settings"].nav-link.active')).toBeVisible();
});
