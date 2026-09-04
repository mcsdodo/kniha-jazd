# Feature: Export System

> Print-ready HTML export of the vehicle trip logbook with vehicle-type-specific templates (ICE/BEV/PHEV), calculated totals, and i18n support.

## User Flow

1. User navigates to the main trips page with an active vehicle selected (button is only shown when a vehicle is active)
2. User clicks the "Export pre tlač" (Export for Print) button in the header
3. The backend returns the complete logbook for the selected year as an HTML **string**
4. The page wraps that string in a `Blob`, takes an object URL and opens it in a new browser tab
5. User prints via Ctrl+P → "Save as PDF" for official record keeping

The export button is disabled when:
- No trips exist for the current year
- An export is already in progress

Export requires company settings (name/IČO) to be configured, otherwise the command fails.

## Technical Implementation

### Export Totals Calculation

`ExportTotals::calculate()` in [export.rs](../../src-tauri/core/src/export.rs) processes trip data to produce footer statistics; the `ExportTotals` struct sits just above it in the same file.

Key fields:
- **Distance:** `total_km` - sum of all trip distances
- **Fuel totals:** `total_fuel_liters`, `total_fuel_cost`, `avg_consumption`, `deviation_percent`
- **Energy totals (BEV/PHEV):** `total_energy_kwh`, `total_energy_cost`, `avg_energy_rate`, `energy_deviation_percent`
- **Other:** `total_other_costs` - tolls, parking, etc.

**Key behavior:**
- **Dummy rows excluded**: Trips with `distance_km = 0` are filtered out before calculations
- **Near-zero normalization**: Values < 0.001 are normalized to 0.0 to avoid "-0.00" display
- **Deviation fallback**: Returns 100% when no fuel data exists (represents "at TP rate")
- **Energy rates/remaining**: Energy rates and battery remaining are not computed here (currently ICE-only calculation paths)

### HTML Generation

The `generate_html()` function builds a complete HTML document with:

1. **Print-optimized CSS**: A4 landscape layout, proper margins, tabular-nums for alignment
2. **Company header**: Company name, IČO (business ID) from settings
3. **Vehicle info section**: Name, license plate, VIN, driver name
4. **Vehicle specs**: Tank size + TP consumption (ICE), battery + baseline (BEV), or all four (PHEV)
5. **Trip data table**: Dynamically generated columns based on vehicle type
6. **Footer summary**: Calculated totals with deviation percentage
7. **Print hint**: Hidden on print, visible on screen ("Ctrl+P → Save as PDF")

### Export Command Flow

`export_html` is the **only** export command. It is async, so it is dispatched from
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) into
`export_html_internal` ([commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs)),
which:

1. Loads vehicle and company settings; fails if either is missing
2. Builds the trip grid via `build_trip_grid_data` (rates, fuel/battery remaining, trip numbers)
3. Appends a synthetic "Prvý záznam" (First Record) row carrying the year-start odometer,
   keyed by `Uuid::nil()` so `generate_html` can special-case it
4. Computes `ExportTotals` (the synthetic 0 km row is skipped by the dummy-row filter)
5. Assembles rows in the caller's `sortDirection` (`"asc"`/`"desc"`) and renders route-map
   attachment pages
6. Returns the finished HTML **as a string** — it writes no file and opens nothing

`handleExport()` in [+page.svelte](../../src/routes/+page.svelte) then does the opening:

```
labels        <- build from the i18n store
hiddenColumns <- re-read from the backend (may have changed since page load)
html          <- exportHtml(vehicleId, year, labels, hiddenColumns, sortDirection)
blob          <- new Blob([html], 'text/html')
window.open(URL.createObjectURL(blob), '_blank')
```

The caller passes the same `sortDirection` the grid header is sorted by, so the printed
record numbers cannot drift from the on-screen order.

### Vehicle-Type Templates

The export system dynamically adjusts columns based on `VehicleType`:

| Vehicle Type | Fuel Columns | Energy Columns | Specs Section |
|--------------|--------------|----------------|---------------|
| **ICE** | ✅ Liters, Cost, Remaining, Rate | ❌ | Tank size, TP consumption |
| **BEV** | ❌ | ✅ kWh, Cost, Battery, Rate | Battery capacity, baseline consumption |
| **PHEV** | ✅ All fuel columns | ✅ All energy columns | All 4 specs |

Column visibility is controlled by `VehicleType` methods:
- `has_fuel()` → Shows fuel columns (ICE + PHEV)
- `has_battery()` → Shows energy columns (BEV + PHEV)

Footer sections similarly adapt:
- ICE: Total fuel (L/€), avg consumption, deviation from TP
- BEV: Total energy (kWh/€), avg energy rate, deviation from baseline
- PHEV: Combined fuel + energy stats

### Internationalization

Export labels are passed from the frontend to ensure proper translation:

1. **Frontend** (`+page.svelte`): Builds `ExportLabels` object from i18n store
2. **TypeScript interface** (`types.ts`): Defines all label fields (snake_case for Rust compatibility)
3. **Rust struct** (`export.rs`): Mirrors the TypeScript interface
4. **HTML template**: Uses labels directly in generated HTML

Labels include:
- Page title and header labels (company, vehicle, specs)
- Column headers (date, origin, destination, km, fuel, energy, etc.)
- Footer labels (totals, averages, deviation)
- BEV-specific labels (battery capacity, energy rate, baseline)
- Print hint text

**Translation location:** the `export:` key in [sk/index.ts](../../src/lib/i18n/sk/index.ts) and [en/index.ts](../../src/lib/i18n/en/index.ts). Labels cover page title, headers, column names, footer labels, and print hints.

## Key Files

| File | Purpose |
|------|---------|
| [export.rs](../../src-tauri/core/src/export.rs) | Core export logic: `ExportTotals`, `generate_html()`, column rendering |
| [commands_internal/export_cmd.rs](../../src-tauri/core/src/commands_internal/export_cmd.rs) | `export_html_internal`: synthetic first record, totals, hidden columns, sort direction, route-map pages |
| [server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs) | The `export_html` RPC arm (async — it awaits route-map tile fetches) |
| [types.ts](../../src/lib/types.ts) | TypeScript `ExportLabels` interface |
| [api.ts](../../src/lib/api.ts) | Frontend API: `exportHtml()` |
| [+page.svelte](../../src/routes/+page.svelte) | Export button handler, label construction |
| [sk/index.ts](../../src/lib/i18n/sk/index.ts) | Slovak translation strings |
| [en.ts](../../src/lib/i18n/en/index.ts) | English translation strings |

## Design Decisions

### Why HTML Over PDF Library?

Using HTML with print CSS allows browser-native PDF generation without heavyweight PDF libraries. Users get a reliable cross-platform solution via "Print → Save as PDF".

**Benefits**:
- No binary PDF dependencies
- Browser handles fonts, margins, page breaks
- User can customize print settings
- Works identically on Windows/macOS/Linux

### Why Labels Passed from Frontend?

Rather than embedding translations in Rust, labels are passed at export time.

**Benefits**:
- i18n centralized in TypeScript codebase
- Dynamic language switching
- No translation files in Rust
- Single source of truth for UI strings

### Why Snake_case for Labels?

The `ExportLabels` interface uses snake_case (unusual for TypeScript) because these values are passed directly to Rust and used in the HTML template. This avoids serde rename attributes and keeps the Rust code clean.

### Why Exclude Dummy Rows from Totals?

Trips with 0 km (used for recording other costs like parking without actual travel) are excluded from totals to prevent distorting consumption averages.

**Example**: A parking receipt recorded as a 0 km trip shouldn't affect average l/100km.

### Why Synthetic First Record?

A "Prvý záznam" (First Record) trip is auto-generated with the year-start odometer to
establish the year's starting point. This matches the on-screen grid, which shows the same
baseline row (`TripGrid.svelte`'s `FIRST_RECORD_ID`).

It is added inside `export_html_internal`, not by the caller, so there is exactly one place
that can produce it — the printed logbook and the grid cannot disagree about the opening
odometer.

### Why Deviation as Percentage?

The deviation shows actual consumption as a percentage of TP norm (e.g., 105% = 5% over norm). This is Slovak tax authority convention where < 120% is legally compliant for expense deduction.

### Why a Blob URL Instead of a Temp File?

The app is a server the user reaches over the LAN, so the backend has no way to open a
window on the viewer's machine — and a file it wrote would land on the server's disk, not
the user's. Returning the HTML as a string and letting the page open it as a `Blob` object
URL puts the document in the tab that asked for it, on whichever device that is, with the
browser still handling all print UI.
