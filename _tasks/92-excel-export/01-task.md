**Date:** 2026-10-16
**Subject:** Export the logbook to Excel (`.xlsx`) next to the print export
**Status:** Planning

## Goal

The tax office (daňová kontrola) can ask for the vehicle logbook as a legal
record. Today the app gives only the print export (HTML -> Ctrl+P -> PDF). Add an
Excel export that shows the same record as the printout, with real number cells.
The auditor can then sort, filter and sum the data, and also print the file.

## Why

- The logbook is legal evidence. The Excel file and the printout must show the
  same record: the same rows, the same values, the same totals.
- An auditor works faster with a spreadsheet than with a PDF.

## Goals

1. A new button "Export do Excelu" next to "Export pre tlač" on the trips page.
   The browser downloads an `.xlsx` file.
2. **Sheet 1** (the logbook) looks like the printout:
   - The same header: company, IČO, vehicle, license plate, VIN, driver, year,
     and the specs for the vehicle type (ICE / BEV / PHEV).
   - The same trip table: the same columns, the same column order, the
     "Prvý záznam" row, and the month-end rows.
   - The same footer totals.
3. **Sheet 2** ("Prílohy") holds the route-map attachments. Each attachment has
   the same heading as in the printout ("Príloha č. N -- záznam č. M"), the map
   image, and the line "© OpenStreetMap contributors".
4. The Excel export follows the same settings as the print export:
   - The setting "Always print the logbook in a standard layout"
     (`get_export_normalized`). If it is on: no hidden columns, ascending order.
   - If it is off: the hidden columns and the sort direction of the grid.
5. Excel helpers on sheet 1 (option 2, the user chose it):
   - A frozen table header row.
   - An autofilter on the trip table.
   - A print setup: A4 landscape, fit to page width, the table header row
     repeats on each printed page.

## Non-goals

- No Excel formulas. All values, also the totals, are fixed values that the
  backend calculated (ADR-008). Reason: the backend calculates the consumption
  per fill-up period. A formula that gives a different number than the app shows
  two values for one fact in a legal record.
- No import from Excel.
- No CSV export.
- No change to the print export output.
- No new export settings. The Excel export uses the print settings.

## Anti-goals (a failure even if it works)

- The Excel file and the printout show a different value, row, row number,
  column set or total for the same vehicle, year and settings.
- An attachment on sheet 2 refers to a different record number than the same
  attachment in the printout.
- A number is stored as text, so the auditor cannot sum or filter it.
- A number cell shows a different text than the printout. (The cell can hold
  more decimals than it shows. See "Exact values" below.)

## Constraints

- **ADR-008:** the backend (Rust, `kniha-jazd-core`) builds the whole file. The
  frontend only calls the RPC command and starts the download.
- **Transport:** the server has only JSON-RPC on `POST /api/rpc`
  ([server/mod.rs](../../src-tauri/core/src/server/mod.rs) line 162). It has no
  route for binary downloads.
- **i18n:** all text in the file comes from the labels that the frontend sends,
  the same as `ExportLabels` for the print export. New strings go into
  [sk/index.ts](../../src/lib/i18n/sk/index.ts) and
  [en/index.ts](../../src/lib/i18n/en/index.ts), then run `npm run i18n`.
- **Test-first** (project CLAUDE.md): backend unit tests first, then the code.
- One map image is about 700 KB. A year with many maps gives a large file. The
  user accepted this.

## Decided by the user

| Topic | Decision |
|---|---|
| Who uses the file | The tax office in an audit (legal requirement) |
| Look | The same as the printout |
| Level | Option 2: plain values + frozen header + autofilter + print setup. No formulas |
| Attachments | On sheet 2 |
| Header and settings | The same as the print export, the same settings |
| Exact values | A number cell holds the exact backend value. Its number format shows it rounded, the same as the printout. SUM over a column adds the exact values (a 0 km row with fuel is in the SUM but not in the footer total, see ADR-062). The auditor can see more decimals in the formula bar; the user accepted this |

## Calls made by the assistant (check these)

1. **Library:** `rust_xlsxwriter` in `kniha-jazd-core`. It writes `.xlsx`,
   number formats, frozen panes, autofilter, print setup and PNG images.
2. **One data path for both exports.** Split `export_html_internal`
   ([export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)):
   one shared function builds the `ExportData` (settings, synthetic first
   record, totals, row assembly, route-map pages). `export_html_internal` and
   the new `export_xlsx_internal` both call it. Do not copy that code.
3. **One column definition for both exports.** `generate_html` in
   [export.rs](../../src-tauri/core/src/export.rs) decides the column set, the
   order and the cell values for each row. The Excel writer must not decide
   this a second time. Extract a shared model (the column list, then the rows
   as typed cells: text, number with a fixed number of decimals, empty).
   `generate_html` renders that model to HTML, the Excel writer renders it to
   cells. The HTML output must not change (the existing export tests prove it).
4. **Transport:** a new RPC command `export_xlsx` returns the file as a base64
   string in the JSON result. The frontend decodes it to a `Blob` with the type
   `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` and
   starts a download. Reason: no new HTTP route. The HTML export already sends
   about 700 KB of base64 for each map through the same path.
5. **Async:** `export_xlsx` goes into
   [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs),
   the same as `export_html`, because the map tiles are fetched.
6. **File name:** `kniha-jazd-<license plate>-<year>.xlsx`. Remove characters
   that are not allowed in a file name (for example a space becomes `-`).
7. **Sheet names** (labels, Slovak text): sheet 1 "Kniha jázd", sheet 2
   "Prílohy". If the year has no maps, the file has no sheet 2.
8. **Number formats:** each number cell holds the exact backend value (the user
   decided this). The number format shows the same number of decimals as the
   printout for each
   column (`{:.0}` km and odometer, `{:.2}` liters, EUR and l/100km, `{:.1}`
   fuel remaining and kWh, see [export.rs](../../src-tauri/core/src/export.rs)).
   **Dates** are real Excel date cells. Their number format shows the same text
   as the printout, `dd.mm. hh:mm` (`DISPLAY_DATE_TIME = "%d.%m. %H:%M"`,
   [constants.rs](../../src-tauri/core/src/constants.rs) line 44). The cell
   still holds the full date with the year, so the auditor can sort and filter.
   A cell that the printout shows with a dash (`"DD.MM. -"`: a trip with no
   end time, the start cell of a month-end row) is a text cell with the same
   text as the printout.
9. **Month-end rows and "Prvý záznam" row:** the same cells as in the printout.
   They use a light grey background, the same as the printout style.
10. **Footer totals:** below the table, with one empty row between. Each
    value has its own row: the label in one cell, the value as a number cell
    in the next cell. The unit goes into the number format (for example
    `0.00 "L"`, `0.00 "€"`). A printed item with two values (`"X L / Y €"`,
    the PHEV `"avg l/100km / avg kWh/100km"`) becomes two rows with the same
    label, so no new label is necessary. The deviation is the number that the
    printout shows (112.3) with the format `0.0"%"`, not 1.123. The footer
    items per vehicle type are the same as in the printout (the PHEV footer
    has no norm item, the same as the printout).
11. **Sheet 2 layout:** one attachment after the other in column A. The heading
    in one row, the image under it (scaled to about A4 landscape width), the
    attribution line under the image, then a page break before the next
    attachment. Print setup: A4 landscape.
12. **Button state:** the same rules as "Export pre tlač". The button is shown
    only with an active vehicle. It is disabled when the year has no trips or
    when an export runs.
13. **Errors:** the same errors as the print export (no company settings, no
    trips). The UI shows them the same way as for the print export.

## Tests

**Backend unit tests** (read the file back with a reader crate, for example
`calamine`, as a dev-dependency):

- The column set and order are the same as the print export for ICE, BEV and
  PHEV, and with hidden columns.
- The normalized setting on: no hidden columns, ascending order.
- The row order for `asc` and `desc`; the "Prvý záznam" row; a month-end row
  directly after the last trip of its month.
- The number columns are number cells, not text.
- Each number cell holds the exact backend value, and its formatted text is
  the same as the printout text for that cell.
- The footer totals are the same values as `ExportTotals`.
- Sheet 2 exists only if maps exist. The attachment number and record number
  are the same as in the HTML export.
- The HTML export does not change after the refactor (the existing tests in
  [export_tests.rs](../../src-tauri/core/src/export_tests.rs) stay green).

**Integration test** (WebdriverIO): click "Export do Excelu" -> the RPC command
runs and the browser receives a file. Do not test the values again.

## Documentation

- [CHANGELOG.md](../../CHANGELOG.md) (via `/changelog`): a user-visible feature. The
  "Pokyny k aktualizácii" block: no env var, no migration, no image change.
- [DECISIONS.md](../../DECISIONS.md) (via `/decision`): an ADR for "Excel export has no formulas,
  one shared row model for HTML and Excel, base64 over JSON-RPC".
- [export-system.md](../../docs/features/export-system.md): add the Excel export.
- [README.md](../../README.md) and [README.en.md](../../README.en.md): mention the Excel export.

## Left to the builder

- The exact shape of the shared row model and where it lives.
- The column widths and the cell styles (keep them close to the printout).
- The exact image scale on sheet 2.
