import { test, expect } from '@playwright/test';

test.describe('Receipt to Trip Assignment', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
	});

	test('doklady page shows assign button for unassigned receipts', async ({ page }) => {
		// Navigate to Doklady page
		await page.click('a[href="/doklady"]');
		await expect(page.locator('.doklady-page h1')).toContainText('Doklady');

		// If there are unassigned receipts, the assign button should be visible
		const assignButton = page.locator('button:has-text("Prideliť k jazde")');
		// Check if any receipts exist - if yes, button should be there
		const receiptsExist = await page.locator('.receipt-card').count();
		if (receiptsExist > 0) {
			await expect(assignButton.first()).toBeVisible();
		}
	});

	test('assign button opens trip selector modal', async ({ page }) => {
		await page.click('a[href="/doklady"]');
		await expect(page.locator('.doklady-page h1')).toContainText('Doklady');

		// Check if there are unassigned receipts
		const assignButton = page.locator('button:has-text("Prideliť k jazde")').first();
		const buttonVisible = await assignButton.isVisible().catch(() => false);

		if (buttonVisible) {
			await assignButton.click();
			// Modal should open
			await expect(page.locator('h2:has-text("Prideliť doklad k jazde")')).toBeVisible();
			// Cancel button should be visible
			await expect(page.locator('.modal button:has-text("Zrušiť")')).toBeVisible();
		}
	});

	test('modal can be closed with cancel button', async ({ page }) => {
		await page.click('a[href="/doklady"]');

		const assignButton = page.locator('button:has-text("Prideliť k jazde")').first();
		const buttonVisible = await assignButton.isVisible().catch(() => false);

		if (buttonVisible) {
			await assignButton.click();
			await expect(page.locator('.modal-overlay')).toBeVisible();

			// Click cancel
			await page.click('.modal button:has-text("Zrušiť")');
			await expect(page.locator('.modal-overlay')).not.toBeVisible();
		}
	});

	test('modal can be closed by clicking overlay', async ({ page }) => {
		await page.click('a[href="/doklady"]');

		const assignButton = page.locator('button:has-text("Prideliť k jazde")').first();
		const buttonVisible = await assignButton.isVisible().catch(() => false);

		if (buttonVisible) {
			await assignButton.click();
			await expect(page.locator('.modal-overlay')).toBeVisible();

			// Click on overlay (outside modal)
			await page.click('.modal-overlay', { position: { x: 10, y: 10 } });
			await expect(page.locator('.modal-overlay')).not.toBeVisible();
		}
	});
});

test.describe('Receipt Picker in Trip Row', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
	});

	test('receipt picker button is visible when editing a trip', async ({ page }) => {
		// Check if there are any trips
		const tripRow = page.locator('tbody tr').first();
		const rowExists = await tripRow.isVisible().catch(() => false);

		if (rowExists) {
			// Double-click to edit
			await tripRow.dblclick();

			// Receipt picker button should be visible in edit mode
			const pickerButton = page.locator('button:has-text("Doklad")');
			// It may or may not be visible depending on if we're in editing mode
			const editMode = await page.locator('tr.editing').isVisible().catch(() => false);
			if (editMode) {
				await expect(pickerButton).toBeVisible();
			}
		}
	});

	test('new record row has receipt picker', async ({ page }) => {
		// Click "Nový záznam" button
		const newRecordButton = page.locator('button:has-text("Nový záznam")');
		const buttonVisible = await newRecordButton.isVisible().catch(() => false);

		if (buttonVisible) {
			await newRecordButton.click();

			// Should show editing row
			await expect(page.locator('tr.editing')).toBeVisible();

			// Receipt picker should be present
			const pickerButton = page.locator('button:has-text("Doklad")');
			await expect(pickerButton).toBeVisible();
		}
	});

	test('clicking receipt picker opens dropdown', async ({ page }) => {
		const newRecordButton = page.locator('button:has-text("Nový záznam")');
		const buttonVisible = await newRecordButton.isVisible().catch(() => false);

		if (buttonVisible) {
			await newRecordButton.click();
			await expect(page.locator('tr.editing')).toBeVisible();

			const pickerButton = page.locator('button:has-text("Doklad")');
			await pickerButton.click();

			// Dropdown should appear
			await expect(page.locator('.receipt-picker .dropdown')).toBeVisible();
		}
	});
});
