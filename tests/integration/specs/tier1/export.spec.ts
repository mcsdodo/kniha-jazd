/**
 * Tier 1: Export Integration Tests
 *
 * Proves the printed logbook ships what the user is actually looking at: their
 * trip data, the columns they left visible, the sort direction they picked in
 * the grid, and the synthetic year-opening row. In server/web mode the backend
 * returns the report as an HTML string, the page wraps it in a blob and shows
 * it with `window.open()` — these tests read that second window.
 *
 * NOT covered here, because it is already proven in Rust: the totals
 * arithmetic, the consumption math and the HTML template (`export_tests.rs`),
 * and the argument plumbing (`dispatcher_async.rs` ::
 * `export_html_honours_hidden_columns_and_sort_direction`). What this spec owns
 * is the wiring — click Export, and the options the user actually set reach the
 * backend and come back visible in the document.
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  seedSettings,
  getTripGridData,
  setActiveVehicle,
  invokeTauri,
} from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';
import { describeNotInTauriMode } from '../../utils/skip';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';
import { testCompanySettings } from '../../fixtures/scenarios';

/** Export trigger on the home page. */
const EXPORT_BTN = 'button.export-btn';

/**
 * The trip-number header doubles as the sort control, and its arrow is the only
 * rendered state — 'desc' (newest first) is the grid's default. The direction
 * is bound straight through to the export, so flipping it here changes the
 * printed document.
 */
const SORT_HEADER = '.trip-grid th.col-trip-number';
const SORT_INDICATOR = `${SORT_HEADER} .sort-indicator`;
const SORT_DESC_ARROW = '▼';
const SORT_ASC_ARROW = '▲';

/**
 * Column-visibility key the export honours (`is_visible("time")` in export.rs).
 * NOTE: it renders the `col_end_datetime` label — 'End' in the `en` locale this
 * suite runs in — NOT `col_time`, which nothing renders. Same pairing the
 * column-visibility UI uses (`column-header-end`).
 */
const TIME_COLUMN_KEY = 'time';
const TIME_COLUMN_HEADER = 'End';

/** Always-printed header, used to prove we read a real header row. */
const ALWAYS_VISIBLE_HEADER = 'Start';

/** Purpose the backend stamps on the synthetic year-opening row. */
const FIRST_RECORD_PURPOSE = 'Prvý záznam';

/**
 * Seed the user's column-visibility choice. `handleExport` re-reads it on every
 * click, so no page reload is needed for the export to see the change.
 * (Same command the column-visibility spec drives.)
 */
async function setHiddenColumnsViaIpc(columns: string[]): Promise<void> {
  await invokeTauri<void>('set_hidden_columns', { columns });
}

/** What a test can assert on after the export window has been read and closed. */
interface ExportedDocument {
  /** Rendered text of the whole printed page. */
  text: string;
  /** Exact text of every `<th>` in the printed table's header row. */
  headers: string[];
}

/**
 * Click Export, switch into the preview window it opens, read it, close it and
 * switch back.
 *
 * Every failure mode throws: a missing export button, a button that never
 * enables, and a preview window that never opens are all real failures. Do not
 * reintroduce "if the window opened" guards here — they made this spec pass
 * while asserting nothing.
 */
async function exportPreview(): Promise<ExportedDocument> {
  const originalHandle = await browser.getWindowHandle();
  const originalHandles = await browser.getWindowHandles();

  const exportBtn = await $(EXPORT_BTN);
  await exportBtn.waitForDisplayed({
    timeout: 10000,
    timeoutMsg: `Export button (${EXPORT_BTN}) never appeared on the home page`,
  });
  await browser.waitUntil(async () => exportBtn.isEnabled(), {
    timeout: 10000,
    timeoutMsg: 'Export button stayed disabled - the grid never loaded any trips',
  });
  await exportBtn.click();

  let exportHandle: string | undefined;
  await browser.waitUntil(
    async () => {
      const handles = await browser.getWindowHandles();
      exportHandle = handles.find((h) => !originalHandles.includes(h));
      return exportHandle !== undefined;
    },
    {
      timeout: 20000,
      timeoutMsg:
        'Export opened no preview window (blocked popup, or export_html returned an error)',
    }
  );

  await browser.switchToWindow(exportHandle as string);
  try {
    const table = await $('table');
    await table.waitForExist({
      timeout: 15000,
      timeoutMsg: 'Export preview window rendered no table',
    });

    const body = await $('body');
    const text = await body.getText();
    const headers = (await browser.execute(() =>
      Array.from(document.querySelectorAll('thead th')).map((th) =>
        (th.textContent || '').trim()
      )
    )) as string[];

    return { text, headers };
  } finally {
    await browser.closeWindow();
    await browser.switchToWindow(originalHandle);
  }
}

/** Toggle the grid's sort direction and wait for its arrow to flip. */
async function flipSortDirection(): Promise<void> {
  const indicator = await $(SORT_INDICATOR);
  await indicator.waitForDisplayed({
    timeout: 10000,
    timeoutMsg: `Sort control (${SORT_INDICATOR}) never rendered`,
  });
  const before = await indicator.getText();

  const header = await $(SORT_HEADER);
  await header.click();

  await browser.waitUntil(
    async () => {
      const current = await $(SORT_INDICATOR);
      return (await current.getText()) !== before;
    },
    {
      timeout: 5000,
      timeoutMsg: `Sort direction never changed (arrow still ${before})`,
    }
  );
}

/** Current sort arrow shown in the grid header. */
async function currentSortArrow(): Promise<string> {
  const indicator = await $(SORT_INDICATOR);
  await indicator.waitForDisplayed({ timeout: 10000 });
  return indicator.getText();
}

/**
 * Skipped under the desktop/Tauri config, where there is nothing for WebDriver
 * to read: `capabilities.features.openExternal` is true there, so `handleExport`
 * calls `export_to_browser` and the logbook opens in the user's *system*
 * browser. No new WebDriver window is ever created there — which is precisely
 * why the old version of this spec could only assert inside an `if` that never
 * ran. The blob-window path exercised here exists only in server/web mode.
 */
describeNotInTauriMode('Tier 1: Export', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
    // Column visibility lives in the settings file, which the per-test database
    // reset does not touch - start every test from "everything visible".
    await setHiddenColumnsViaIpc([]);
  });

  afterEach(async () => {
    // ...and leave it that way for every other spec in the run.
    await setHiddenColumnsViaIpc([]);
  });

  describe('Export Preview', () => {
    it('should open export preview with trip data', async () => {
      // Seed company settings
      await seedSettings({
        companyName: testCompanySettings.companyName,
        companyIco: testCompanySettings.companyIco,
        bufferTripPurpose: testCompanySettings.bufferTripPurpose,
      });

      // Create vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Export Preview Vehicle',
        licensePlate: 'EPV-001',
        initialOdometer: 50000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      const year = new Date().getFullYear();

      // Create several trips to export
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-03-01T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 65,
        odometer: 50065,
        purpose: TripPurposes.business,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-03-10T08:00`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 70,
        odometer: 50135,
        purpose: TripPurposes.clientMeeting,
        fuelLiters: 35,
        fuelCostEur: 52.5,
        fullTank: true,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-03-20T08:00`,
        origin: SlovakCities.nitra,
        destination: SlovakCities.bratislava,
        distanceKm: 90,
        odometer: 50225,
        purpose: TripPurposes.conference,
      });

      // Make this the exported vehicle and wait for its grid to render
      await setActiveVehicle(vehicle.id as string);
      await waitForTripGrid();

      const exported = await exportPreview();

      // Should contain trip data
      expect(exported.text).toContain(SlovakCities.bratislava);
      expect(exported.text).toContain(SlovakCities.trnava);
      expect(exported.text).toContain(SlovakCities.nitra);

      // Should contain trip purposes
      expect(exported.text).toContain(TripPurposes.business);

      // Should contain vehicle info
      expect(exported.text).toContain('EPV-001'); // License plate

      // Should contain company info
      expect(exported.text).toContain(testCompanySettings.companyName);

      // Should contain the synthetic year-opening row carrying
      // year_start_odometer. Web mode used to drop it, so the printed logbook
      // silently started without the opening odometer reading.
      expect(exported.text).toContain(FIRST_RECORD_PURPOSE);
    });

    it('should show correct totals in export footer', async () => {
      // Seed company settings
      await seedSettings({
        companyName: 'Totals Test Company',
        companyIco: '11111111',
        bufferTripPurpose: TripPurposes.business,
      });

      // Create vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Export Totals Vehicle',
        licensePlate: 'ETV-001',
        initialOdometer: 60000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      const year = new Date().getFullYear();

      // Create trips with known values for easy total verification
      // Total km: 100 + 150 + 200 = 450 km
      // Total fuel: 30 + 45 = 75 L
      // Total fuel cost: 45 + 67.5 = 112.5 EUR
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-01T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 100,
        odometer: 60100,
        purpose: TripPurposes.business,
        fuelLiters: 30,
        fuelCostEur: 45,
        fullTank: true,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-10T08:00`,
        origin: SlovakCities.trnava,
        destination: SlovakCities.nitra,
        distanceKm: 150,
        odometer: 60250,
        purpose: TripPurposes.clientMeeting,
        fuelLiters: 45,
        fuelCostEur: 67.5,
        fullTank: true,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-04-20T08:00`,
        origin: SlovakCities.nitra,
        destination: SlovakCities.kosice,
        distanceKm: 200,
        odometer: 60450,
        purpose: TripPurposes.conference,
      });

      // Set this vehicle as active and wait for the grid to render
      await setActiveVehicle(vehicle.id as string);
      await waitForTripGrid();

      // Verify the backend agrees on what is being exported
      const gridData = await getTripGridData(vehicle.id as string, year);
      expect(gridData.trips.length).toBe(3);

      const exported = await exportPreview();

      // Should contain total km (450)
      // Note: The format might include thousand separators or decimal places
      expect(exported.text).toMatch(/450/);

      // Should contain total fuel (75 or 75.0)
      expect(exported.text).toMatch(/75/);

      // Should contain total fuel cost (112.5 or 112,5 in Slovak format)
      expect(exported.text).toMatch(/112[,.]5/);

      // Should show a consumption rate in the footer
      expect(exported.text).toMatch(/L\/100km|l\/100km/i);
    });

    it('should omit a hidden column from the exported document', async () => {
      await seedSettings({
        companyName: 'Hidden Column Company',
        companyIco: '22222222',
        bufferTripPurpose: TripPurposes.business,
      });

      const vehicle = await seedVehicle({
        name: 'Hidden Column Vehicle',
        licensePlate: 'HCV-001',
        initialOdometer: 70000,
        vehicleType: 'Ice',
        tankSizeLiters: 50,
        tpConsumption: 7.0,
      });

      const year = new Date().getFullYear();

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-06-05T08:00`,
        endDatetime: `${year}-06-05T10:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 65,
        odometer: 70065,
        purpose: TripPurposes.business,
      });

      await setActiveVehicle(vehicle.id as string);
      await waitForTripGrid();

      // Baseline: nothing hidden, so the column prints.
      const withTimeColumn = await exportPreview();
      expect(withTimeColumn.headers).toContain(TIME_COLUMN_HEADER);

      // Hide it the way the column-visibility dropdown does, then export again.
      await setHiddenColumnsViaIpc([TIME_COLUMN_KEY]);
      const withoutTimeColumn = await exportPreview();

      expect(withoutTimeColumn.headers).not.toContain(TIME_COLUMN_HEADER);
      // Sanity: we read a real header row, not an empty one.
      expect(withoutTimeColumn.headers).toContain(ALWAYS_VISIBLE_HEADER);
      expect(withoutTimeColumn.headers.length).toBe(withTimeColumn.headers.length - 1);
    });

    it('should export rows in the sort direction chosen in the grid', async () => {
      await seedSettings({
        companyName: 'Sort Order Company',
        companyIco: '33333333',
        bufferTripPurpose: TripPurposes.business,
      });

      const vehicle = await seedVehicle({
        name: 'Sort Order Vehicle',
        licensePlate: 'SOV-001',
        initialOdometer: 80000,
        vehicleType: 'Ice',
        tankSizeLiters: 50,
        tpConsumption: 7.0,
      });

      const year = new Date().getFullYear();

      // Both trips start in Bratislava and end somewhere that appears nowhere
      // else in the document, so their positions in the text are unambiguous.
      // Same month, so no month-end row can land between them.
      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-05-05T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.trnava,
        distanceKm: 60,
        odometer: 80060,
        purpose: TripPurposes.business,
      });

      await seedTrip({
        vehicleId: vehicle.id as string,
        startDatetime: `${year}-05-20T08:00`,
        origin: SlovakCities.bratislava,
        destination: SlovakCities.kosice,
        distanceKm: 90,
        odometer: 80150,
        purpose: TripPurposes.business,
      });

      await setActiveVehicle(vehicle.id as string);
      await waitForTripGrid();

      // The grid defaults to newest first.
      expect(await currentSortArrow()).toBe(SORT_DESC_ARROW);

      const newestFirst = await exportPreview();
      expect(newestFirst.text).toContain(SlovakCities.kosice);
      expect(newestFirst.text).toContain(SlovakCities.trnava);
      expect(newestFirst.text.indexOf(SlovakCities.kosice)).toBeLessThan(
        newestFirst.text.indexOf(SlovakCities.trnava)
      );

      // Flip the grid's sort control - the export must follow it.
      await flipSortDirection();
      expect(await currentSortArrow()).toBe(SORT_ASC_ARROW);

      const oldestFirst = await exportPreview();
      expect(oldestFirst.text).toContain(SlovakCities.kosice);
      expect(oldestFirst.text).toContain(SlovakCities.trnava);
      expect(oldestFirst.text.indexOf(SlovakCities.trnava)).toBeLessThan(
        oldestFirst.text.indexOf(SlovakCities.kosice)
      );
    });
  });
});
