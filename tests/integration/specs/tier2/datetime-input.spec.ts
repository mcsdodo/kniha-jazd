/**
 * Tier 2: Typed start/end datetime in the trip editor
 *
 * The edit row shows the start and end as text "DD.MM HH:MM" (year from the
 * value the field holds). The calendar button next to it follows the
 * "Show datetime picker" setting (Settings > Appearance, default ON).
 *
 * The parse rules (short forms, year boundary, invalid input) are covered by
 * src/lib/datetimeInput.test.ts. This spec verifies the UI wiring only.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { waitForTripGrid } from '../../utils/assertions';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, setActiveVehicle, rpc } from '../../utils/db';

const START = '[data-testid="trip-start-datetime"]';
const END = '[data-testid="trip-end-datetime"]';

async function openNewRow(): Promise<void> {
  const newRecordBtn = await $('button.new-record');
  await newRecordBtn.waitForClickable({ timeout: 5000 });
  await newRecordBtn.click();
  await (await $(START)).waitForDisplayed({ timeout: 5000 });
}

async function typeInto(selector: string, text: string): Promise<void> {
  const input = await $(selector);
  await input.clearValue();
  // clearValue() blurs the element (WebDriver "Element Clear"), so focus it again.
  await input.click();
  await browser.keys(text.split(''));
}

describe('Tier 2: Typed datetime in the trip editor', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    await rpc<void>('set_show_datetime_picker', { enabled: true });
    // The end column must be visible; sibling specs hide 'time'.
    await rpc<void>('set_hidden_columns', { columns: [] });

    const vehicle = await seedVehicle({
      name: 'Datetime Input Vehicle',
      licensePlate: 'DTI-001',
      initialOdometer: 10000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    await setActiveVehicle(vehicle.id as string);

    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
  });

  it('accepts a short form and shows it as DD.MM HH:MM', async () => {
    await openNewRow();
    const start = await $(START);
    const year = ((await start.getAttribute('data-value')) ?? '').slice(0, 4);

    await typeInto(START, '15.3 1430');
    // Blur by a click on the end field.
    await (await $(END)).click();

    expect(await start.getValue()).toBe('15.03 14:30');
    expect(await start.getAttribute('data-value')).toBe(`${year}-03-15T14:30`);
    // A new row's end follows its start.
    expect(await (await $(END)).getAttribute('data-value')).toBe(`${year}-03-15T14:30`);
  });

  it('marks invalid text and does not save the row', async () => {
    await openNewRow();
    await typeInto(START, '32.13 2500');
    await (await $(END)).click();

    const start = await $(START);
    expect(await start.getAttribute('aria-invalid')).toBe('true');

    await browser.keys('Enter');
    await browser.pause(300);
    // The editor is still open: the save was blocked.
    expect(await (await $(START)).isExisting()).toBe(true);
    // The invalid text stays (free text keeps "2500", part-by-part shows
    // "25:00"; which one depends on the focus path), still marked invalid.
    expect(await (await $(START)).getValue()).toMatch(/^32\.13 25:?00$/);
    expect(await (await $(START)).getAttribute('aria-invalid')).toBe('true');
  });

  it('hides the calendar button when the setting is off', async () => {
    await openNewRow();
    expect(await (await $('[data-testid="trip-start-datetime-picker"]')).isExisting()).toBe(true);
    await browser.keys('Escape');

    await navigateTo('settings');
    const toggle = await $('[data-testid="show-datetime-picker-toggle"]');
    await toggle.waitForDisplayed({ timeout: 5000 });
    expect(await toggle.isSelected()).toBe(true);
    await toggle.click();
    await browser.waitUntil(async () => (await rpc<boolean>('get_show_datetime_picker')) === false, {
      timeout: 5000,
      timeoutMsg: 'The setting was not saved',
    });

    await navigateTo('trips');
    await waitForTripGrid();
    await openNewRow();
    expect(await (await $('[data-testid="trip-start-datetime-picker"]')).isExisting()).toBe(false);
    expect(await (await $('[data-testid="trip-end-datetime-picker"]')).isExisting()).toBe(false);
  });
});
