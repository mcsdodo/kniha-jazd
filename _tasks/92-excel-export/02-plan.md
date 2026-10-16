**Date:** 2026-10-16
**Subject:** Excel export implementation plan
**Status:** Complete

# Excel Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A second export button writes the logbook as `.xlsx`: the same header, rows, totals and settings as the print export, with the route maps on sheet 2.

**Architecture:** One shared table model (columns + typed cells) is built from `ExportData`. `generate_html` renders it to HTML (byte-identical to today), the new `generate_xlsx` renders it to cells with `rust_xlsxwriter`. One shared async function builds `ExportData` for both RPC commands. The new RPC command `export_xlsx` returns `{ fileName, base64 }`; the page decodes it and downloads the file.

**Tech Stack:** Rust (`kniha-jazd-core`), `rust_xlsxwriter` 0.99, `calamine` 0.36 (dev-dependency, reads the file back in tests), SvelteKit, WebdriverIO.

**Spec:** [01-task.md](./01-task.md)

## Global Constraints

- ADR-008: the backend builds the whole file. The frontend only calls `export_xlsx` and starts the download.
- No Excel formulas. Every value is a fixed value.
- A number cell holds the exact backend value. Its number format shows the decimals of the printout: `0` for km, odometer and record number; `0.00` for liters, EUR, l/100km, kWh/100km; `0.0` for fuel remaining, battery remaining and kWh.
- Dates are date cells with the format `dd.mm. hh:mm`. A printout cell with a dash (`"DD.MM. -"`) is a text cell with the same text.
- The print export output does not change (golden test, Task 1).
- The Excel export follows `get_export_normalized` the same as the print export (no hidden columns, `asc`).
- All new UI text goes through i18n (`sk` + `en`), then `npm run i18n`.
- Tests first. Run Rust tests with `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core <filter>`.

## Review Focus

1. A trip with no fuel (`fuel_liters = None`) -> an empty cell, not `0`. Test in Task 3 (`xlsx_none_values_are_empty_cells`).
2. A license plate with a space or a slash (`BA 123/AB`) -> a valid file name `kniha-jazd-BA-123-AB-2026.xlsx`. Test in Task 4.
3. A BEV/PHEV vehicle -> the energy columns and the footer rows of that type. Test in Task 3 (`xlsx_phev_footer_rows`).
4. A year with no maps -> no sheet 2; with maps -> the attachment and record numbers are the same as in the HTML. Test in Task 3.
5. A text with characters that HTML escapes (`&`, `<`) -> the Excel cell holds the raw text, not `&amp;`. Test in Task 3 (`xlsx_text_is_not_html_escaped`).

---

### Task 1: Shared table model, HTML renders from it

**Files:**
- Create: [src-tauri/core/src/export_table.rs](../../src-tauri/core/src/export_table.rs), [src-tauri/core/src/export_table_tests.rs](../../src-tauri/core/src/export_table_tests.rs)
- Create: [src-tauri/core/src/testdata/export_golden_ice.html](../../src-tauri/core/src/testdata/export_golden_ice.html), [src-tauri/core/src/testdata/export_golden_phev.html](../../src-tauri/core/src/testdata/export_golden_phev.html)
- Modify: [src-tauri/core/src/export.rs:243-665](../../src-tauri/core/src/export.rs) (row and header building), [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs) (add `pub mod export_table;`)
- Test: [src-tauri/core/src/export_tests.rs](../../src-tauri/core/src/export_tests.rs)

**Interfaces:**
- Produces (in `export_table.rs`):
  ```rust
  pub enum CellValue { Empty, Text(String), Number { value: f64, decimals: usize }, DateTime(chrono::NaiveDateTime) }
  pub enum RowKind { Trip, FirstRecord, MonthEnd }
  pub struct ExportColumn { pub label: String, pub numeric: bool }
  pub struct ExportTableRow { pub kind: RowKind, pub cells: Vec<CellValue> }
  pub struct ExportTable { pub columns: Vec<ExportColumn>, pub rows: Vec<ExportTableRow> }
  pub fn build_export_table(data: &ExportData) -> ExportTable;
  impl CellValue { pub fn display(&self) -> String } // the printout text
  ```
  `display()`: `Number` -> `format!("{:.*}", decimals, value)`; `DateTime` -> `date_formats::DISPLAY_DATE_TIME`; `Text` -> itself; `Empty` -> `""`.

- [ ] **Step 1: Write the golden test and record the current HTML**

  In `export_tests.rs` add `golden_html_ice` and `golden_html_phev`. Each builds a fixed `ExportData`. Use fixed UUIDs and fixed `created_at`, because `generate_html` does not print them, but keep the data deterministic. The ICE fixture has: a "Prvý záznam" row (`Uuid::nil()`), 3 trips over 2 months with fuel, a trip with `end_datetime = None`, one month-end row, `hidden_columns = ["fuelConsumed"]`, `sort_direction = "desc"`, and one `RouteMapPage` with a short base64 string. The PHEV fixture has fuel and energy values, no hidden columns, `asc`. The test compares `generate_html(..)` with `include_str!("testdata/export_golden_<x>.html")`. If the env var `UPDATE_GOLDEN=1` is set, the test writes the file instead.

- [ ] **Step 2: Record the golden files on the unchanged code**

  Run: `UPDATE_GOLDEN=1 cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core golden_html`, then the same command without `UPDATE_GOLDEN`.
  Expected: PASS. Commit the test and the two files before the refactor.

- [ ] **Step 3: Write the failing model tests in `export_table_tests.rs`**

  - `table_columns_ice_all_visible`: the labels are, in order, `col_trip_number, col_start_datetime, col_end_datetime, col_driver, col_origin, col_destination, col_purpose, col_km, col_odo_start, col_odo, col_fuel_liters, col_fuel_cost, col_fuel_consumed, col_remaining, col_consumption, col_other_costs, col_note`.
  - `table_columns_honour_hidden_columns`: hide `time`, `fuelConsumed`, `fuelRemaining`, `otherCosts`, `otherCostsNote` -> those 5 labels are absent; for PHEV `fuelRemaining` also removes `col_battery_remaining`.
  - `table_columns_bev`: no fuel columns; `col_energy_kwh, col_energy_cost, col_battery_remaining, col_energy_rate` after `col_odo`.
  - `table_rows_kinds_and_order`: with `desc`, the kinds are in the HTML order; the nil trip is `FirstRecord`, its record-number cell is `Empty`.
  - `table_cells_typed`: a trip with `fuel_liters = Some(12.3456)` -> `Number { value: 12.3456, decimals: 2 }`; `fuel_liters = None` -> `Empty`; start -> `DateTime`; `end_datetime = None` -> `Text("05.03. -")`; month-end start -> `Text("31.03. -")`; month-end odometer cells -> `Number { decimals: 0 }`.
  - `every_row_has_one_cell_per_column` for ICE, BEV, PHEV with all hide combinations of one column.

- [ ] **Step 4: Run them to see them fail**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core export_table`
  Expected: FAIL (does not compile: `build_export_table` not defined).

- [ ] **Step 5: Implement `build_export_table` and render the HTML table from it**

  Move the sort/interleave code and the per-column rules from `generate_html` into `build_export_table`. `generate_html` then renders `columns` as `<th>` and each row as `<tr>`. Keep the exact whitespace of today. The `class="num"` rule that keeps the HTML identical:
  - `Trip` and `FirstRecord` rows: `class="num"` when `column.numeric`, also on an empty cell.
  - `MonthEnd` rows: `class="num"` only on a `Number` cell.
  - `FirstRecord` and `MonthEnd` rows get `class="month-end-synthetic"` on `<tr>`.
  - Cell text goes through `html_escape(display())`.

- [ ] **Step 6: Run all export tests**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core export`
  Expected: PASS, including `golden_html_ice`, `golden_html_phev` and the old `export_tests.rs` tests.

- [ ] **Step 7: Commit**

  `git commit -m "refactor(export): build the printed table from one shared model"`

### Task 2: One function builds the export data

**Files:**
- Modify: [src-tauri/core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)

**Interfaces:**
- Produces: `pub async fn build_export_data(db: &Database, app_dir: &Path, vehicle_id: String, year: i32, labels: ExportLabels, hidden_columns: Vec<String>, sort_direction: String) -> Result<ExportData, String>` (the body of today's `export_html_internal` without the `generate_html` call). `export_html_internal` becomes `generate_html(build_export_data(..).await?)`.

- [ ] **Step 1: Move the code**

  No new test. The existing tests prove the refactor: `export_html_honours_hidden_columns_and_sort_direction` in `dispatcher_async.rs` and the normalized-setting tests (grep `export_normalized` in the tests).

- [ ] **Step 2: Run the tests**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core export`
  Expected: PASS.

- [ ] **Step 3: Commit**

  `git commit -m "refactor(export): share the export data builder"`

### Task 3: The xlsx writer

**Files:**
- Modify: [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml) (`rust_xlsxwriter = "0.99"`; dev: `calamine = { version = "0.36", features = ["dates", "picture"] }`)
- Create: [src-tauri/core/src/export_xlsx.rs](../../src-tauri/core/src/export_xlsx.rs), [src-tauri/core/src/export_xlsx_tests.rs](../../src-tauri/core/src/export_xlsx_tests.rs)
- Modify: [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs), [src-tauri/core/src/export.rs](../../src-tauri/core/src/export.rs) (`ExportLabels` + `sample_export_labels`)

**Interfaces:**
- Consumes: `build_export_table`, `CellValue::display`, `ExportData`, `ExportTotals`.
- Produces: `pub fn generate_xlsx(data: &ExportData) -> Result<Vec<u8>, String>`.
- New `ExportLabels` fields, each with `#[serde(default = "...")]` so old callers of `export_html` still parse: `sheet_logbook` (default `"Kniha jázd"`), `sheet_attachments` (default `"Prílohy"`).

**Layout of sheet 1 (`sheet_logbook`):**
- Row 0: `page_title`, bold.
- From row 2: one header item per row, label in column A, value in column B, in the HTML order: company, IČO, vehicle, license plate, the specs of the vehicle type, year, VIN, driver. A spec value is a number cell with the format `General" L"`, `General" l/100km"`, `General" kWh"`, `General" kWh/100km"` (the printout prints them with `{}`).
- One empty row, then the table header row: bold, grey fill (`#f0f0f0`), border. Then the rows of `ExportTable`. `FirstRecord` and `MonthEnd` rows have a light grey fill (the same color as `.month-end-synthetic` in the HTML CSS).
- Cell writing: `Number` -> `write_number_with_format`, format `"0"`, `"0.0"` or `"0.00"` from `decimals`; `DateTime` -> `write_datetime_with_format`, format `"dd.mm. hh:mm"`; `Text` -> `write_string`; `Empty` -> nothing (only the row fill).
- Autofilter over the header row and all table rows. Freeze panes below the table header row.
- One empty row, then the footer rows. Label in column A, number cell in column B. Formats: km `0" km"`, liters `0.00" L"`, EUR `0.00" €"`, kWh `0.00" kWh"`, rates `0.00" l/100km"` / `0.00" kWh/100km"`, deviation `0.0"%"` (value 112.3, not 1.123), norm `General" l/100km"`.
  - ICE: total km; total fuel (L); total fuel (€); other costs; avg consumption; deviation; TP norm. The fuel total label is `footer_total_fuel` on both rows.
  - BEV: total km; total energy (kWh); total energy (€); other costs; avg energy rate; deviation (`energy_deviation_percent`); baseline norm.
  - PHEV: total km; fuel L; fuel €; energy kWh; energy €; other costs; avg consumption (`footer_avg_consumption`); avg energy rate (`footer_avg_energy_rate`); deviation (`deviation_percent`). No norm row, the same as the printout.
- Print setup: A4 (`set_paper_size(9)`), landscape, fit to 1 page wide and any height (`set_print_fit_to_pages(1, 0)`), repeat the table header row (`set_repeat_rows`).
- Column widths: about 8 for numeric columns, 22 for text columns, 12 for date columns.

**Layout of sheet 2 (`sheet_attachments`), only if `route_maps` is not empty:**
- For each page: a bold heading row with the same text as the HTML heading (`"{attachment_heading} {n} — {record_reference} {row}"`, without HTML escape), the PNG under it (`Image::new_from_buffer`, scaled with `set_scale_to_size(1000, 640, true)`), the line `© OpenStreetMap contributors` under the image (34 rows below the heading), then a horizontal page break before the next heading.
- Print setup: A4, landscape, fit to 1 page wide.
- A sheet name is cut to 31 characters, and `[]:*?/\` become `-`.

- [ ] **Step 1: Write the failing tests in `export_xlsx_tests.rs`**

  Read the bytes back with `calamine::open_workbook_from_rs::<Xlsx<_>, _>(Cursor::new(bytes))`. Reuse a fixture builder that has the shape of the Task 1 ICE fixture (share it from `export_tests.rs` with `pub(crate)` if needed).
  - `xlsx_sheet_names`: no maps -> `["Kniha jázd"]`; one map -> `["Kniha jázd", "Prílohy"]`.
  - `xlsx_header_block`: cell A3 = `header_company`, B3 = company name; the license plate row has the plate; the TP row has `Float(6.5)`.
  - `xlsx_table_matches_model`: the table header row = `build_export_table(..).columns` labels; each table row cell = the model cell (`Number` -> `Data::Float(value)` exact, for example `12.3456`; `DateTime` -> a date-time cell with the same date and time; `Text` -> `Data::String`; `Empty` -> `Data::Empty`).
  - `xlsx_none_values_are_empty_cells`: the fuel cells of a trip without fuel are `Data::Empty`.
  - `xlsx_text_is_not_html_escaped`: a purpose `"A & <B>"` is the cell text `"A & <B>"`.
  - `xlsx_ice_footer_rows`: the labels and the `Float` values after the table are the ICE list above, with the values of `ExportTotals` (exact).
  - `xlsx_phev_footer_rows`: the same for PHEV; no norm row.
  - `xlsx_attachments_sheet`: one map with `attachment_no = 1, row_number = 3` -> A1 of sheet 2 = `"Príloha č. 1 — záznam č. 3"` (with the sample labels), the sheet has 1 picture (calamine `pictures()`), and the attribution text is in the sheet.
  - `xlsx_number_formats`: a unit test of the private helper `num_format(decimals: usize) -> &'static str`: `0 -> "0"`, `1 -> "0.0"`, `2 -> "0.00"`.

- [ ] **Step 2: Run them to see them fail**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core xlsx`
  Expected: FAIL (does not compile).

- [ ] **Step 3: Implement `generate_xlsx` with the layout above**

- [ ] **Step 4: Run the tests**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core xlsx`
  Expected: PASS. Then open one generated file with `python3 -c` + `openpyxl` (if installed) or `unzip -l` to check that it is a valid zip with `xl/worksheets/sheet1.xml`.

- [ ] **Step 5: Commit**

  `git commit -m "feat(export): write the logbook as xlsx"`

### Task 4: The `export_xlsx` RPC command

**Files:**
- Modify: [src-tauri/core/src/commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs), [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)

**Interfaces:**
- Consumes: `build_export_data` (Task 2), `generate_xlsx` (Task 3).
- Produces:
  ```rust
  #[derive(Serialize)] #[serde(rename_all = "camelCase")]
  pub struct ExportFile { pub file_name: String, pub base64: String }
  pub async fn export_xlsx_internal(/* same args as export_html_internal */) -> Result<ExportFile, String>;
  fn xlsx_file_name(license_plate: &str, year: i32) -> String;
  ```
  RPC: `"export_xlsx"`, the same `Args` as `export_html`. JSON result: `{ "fileName": "...", "base64": "..." }`.
  File name: `kniha-jazd-<plate>-<year>.xlsx`. In the plate, each character that is not ASCII alphanumeric, `-` or `_` becomes `-`.

- [ ] **Step 1: Write the failing tests**

  - In `dispatcher_async.rs` tests: `export_xlsx_returns_a_named_xlsx_file`. Seed like `export_html_honours_hidden_columns_and_sort_direction`. Call `export_xlsx` with `hiddenColumns: ["time"]` and `labels.col_end_datetime = "CAS-MARKER"`. Assert `fileName == "kniha-jazd-BA-123AB-2026.xlsx"`, the decoded bytes start with `PK`, and the header row of sheet 1 (calamine) has no `"CAS-MARKER"`.
  - `xlsx_file_name_replaces_unsafe_characters`: `("BA 123/AB", 2026) -> "kniha-jazd-BA-123-AB-2026.xlsx"`.

- [ ] **Step 2: Run them to see them fail**

  Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core export_xlsx xlsx_file_name`
  Expected: FAIL.

- [ ] **Step 3: Implement `export_xlsx_internal`, `xlsx_file_name` and the dispatcher arm**

  Encode with `base64::engine::general_purpose::STANDARD`.

- [ ] **Step 4: Run the tests**

  Expected: PASS.

- [ ] **Step 5: Commit**

  `git commit -m "feat(export): add the export_xlsx RPC command"`

### Task 5: The button

**Files:**
- Modify: [src/lib/api.ts](../../src/lib/api.ts), [src/lib/types.ts](../../src/lib/types.ts) (`ExportLabels` + `ExportFile`), [src/routes/+page.svelte](../../src/routes/+page.svelte), [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts), [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)
- Test: [tests/integration/specs/tier1/export.spec.ts](../../tests/integration/specs/tier1/export.spec.ts)

**Interfaces:**
- Produces: `exportXlsx(vehicleId, year, labels, hiddenColumns, sortDirection): Promise<ExportFile>`; `interface ExportFile { fileName: string; base64: string }`.
- i18n keys: `home.exportToExcel` (sk `"Export do Excelu"`, en `"Export to Excel"`); `export.sheetLogbook` (sk `"Kniha jázd"`, en `"Logbook"`); `export.sheetAttachments` (sk `"Prílohy"`, en `"Attachments"`).

- [ ] **Step 1: Write the failing integration test**

  In `export.spec.ts` add `it('downloads the logbook as an xlsx file')`. Seed like the other tests. Before the click, run in the page:
  ```js
  window.__downloads = [];
  const orig = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function () {
    if (this.download) { window.__downloads.push({ name: this.download, href: this.href }); return; }
    return orig.call(this);
  };
  ```
  Click `button.export-xlsx-btn`. Wait until `window.__downloads.length === 1`. Assert `name` matches `/^kniha-jazd-.+-\d{4}\.xlsx$/`. Fetch the blob URL in the page (`executeAsync` + `fetch(href).then(r => r.arrayBuffer())`) and assert the first two bytes are `P`,`K` and the size is > 1000.

- [ ] **Step 2: Build and run it to see it fail**

  Run: `npm run build && cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web && npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier1/export.spec.ts`
  Expected: FAIL (no `button.export-xlsx-btn`).

- [ ] **Step 3: Implement**

  - Move the label object of `handleExport` into `function buildExportLabels(): ExportLabels` and use it in both handlers. Add `sheet_logbook` and `sheet_attachments`.
  - `handleExportXlsx()`: the same guards, the same `exporting` flag, the same error toast. Decode `base64` with `atob` into a `Uint8Array`, make a `Blob` with type `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`, create an `<a>` with `href = URL.createObjectURL(blob)` and `download = fileName`, call `click()`, then `URL.revokeObjectURL` after the click.
  - The button: `<button class="export-btn export-xlsx-btn" ...>` next to the print button, `disabled={exporting || trips.length === 0}`, text `$LL.home.exportToExcel()` (or `$LL.home.exporting()` while it runs).
  - Run `npm run i18n`, then `npm run check`.

- [ ] **Step 4: Run the spec**

  Run the Step 2 command.
  Expected: PASS for the whole `export.spec.ts`.

- [ ] **Step 5: Commit**

  `git commit -m "feat(export): add the Export to Excel button"`

### Task 6: Documents, full check, local deploy

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) (via `/changelog`), [DECISIONS.md](../../DECISIONS.md) (via `/decision`), [docs/features/export-system.md](../../docs/features/export-system.md), [README.md](../../README.md), [README.en.md](../../README.en.md), [.claude/rules/rust-backend.md](../../.claude/rules/rust-backend.md) (key files table: `export_table.rs`, `export_xlsx.rs`), [_tasks/index.md](../../_tasks/index.md)

- [ ] **Step 1:** `/decision`: ADR "Excel export: fixed values, no formulas; one shared table model; base64 over JSON-RPC; exact values shown rounded".
- [ ] **Step 2:** `/changelog`: a user-visible `Pridané` entry. The `Pokyny k aktualizácii` block: no env var, no migration, no image change.
- [ ] **Step 3:** Update the feature doc (a section "Excel Export") and both READMEs.
- [ ] **Step 4:** Full check: `npm run test:backend`, `npm run typecheck:tests`, `npm run check`, and the full integration suite `npm run test:integration`. Expected: all PASS.
- [ ] **Step 5:** Commit the documents.
- [ ] **Step 6:** Local deploy: `docker compose -f docker-compose.web.yml up -d --build`. Check `curl -s localhost:3456/health` = `ok`, then call `export_xlsx` with `curl` on `/api/rpc` for an existing vehicle and year, and check that the decoded file opens (zip with `xl/worksheets/sheet1.xml`).
