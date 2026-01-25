# Feature: Export System

> Print-ready HTML export of the vehicle trip logbook with vehicle-type-specific templates (ICE/BEV/PHEV), calculated totals, and i18n support.

## User Flow

1. User navigates to the main trips page with an active vehicle selected (button is only shown when a vehicle is active)
2. User clicks the "Export pre tlač" (Export for Print) button in the header
3. System generates an HTML file with the complete trip logbook for the selected year
4. HTML file is opened in the default browser
5. User prints via Ctrl+P → "Save as PDF" for official record keeping

The export button is disabled when:
- No trips exist for the current year
- An export is already in progress

Export requires company settings (name/IČO) to be configured, otherwise the command fails.

## Technical Implementation

### Export Totals Calculation

The `ExportTotals::calculate()` function processes trip data to produce footer statistics:

```rust
ExportTotals {
    total_km,           // Sum of all trip distances
    total_fuel_liters,  // Sum of fuel fillups
    total_fuel_cost,    // Sum of fuel costs
    total_other_costs,  // Sum of other costs (tolls, parking, etc.)
    avg_consumption,    // total_fuel / total_km * 100 (l/100km)
    deviation_percent,  // avg_consumption / tp_consumption * 100
    // Energy fields for BEV/PHEV:
    total_energy_kwh,
    total_energy_cost,
    avg_energy_rate,
    energy_deviation_percent
}
```

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

The `export_to_browser` command:
1. Loads vehicle, settings, and trips from database
2. Creates a synthetic "First Record" trip with initial odometer
3. Calculates consumption rates and fuel/battery remaining
4. Applies user's current sort order (date or manual)
5. Writes HTML to temp directory as `kniha-jazd-{license_plate}-{year}.html`
6. Opens file in default browser via `opener::open()`

The `export_html` command:
- Returns the generated HTML string without opening a browser
- Does **not** add the synthetic "First Record" row
- Does **not** apply the user’s sort order (uses chronological order for calculations)

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

**Translation example** (Slovak):
```typescript
export: {
    pageTitle: 'KNIHA JÁZD',
    headerCompany: 'Firma:',
    footerDeviation: 'Odchýlka od TP',
    printHint: 'Pre export do PDF použite Ctrl+P → Uložiť ako PDF',
    // ...
}
```

## Key Files

| File | Purpose |
|------|---------|
| [export.rs](src-tauri/src/export.rs) | Core export logic: `ExportTotals`, `generate_html()`, column rendering |
| [commands.rs](src-tauri/src/commands.rs) | Tauri commands: `export_to_browser`, `export_html` |
| [types.ts](src/lib/types.ts) | TypeScript `ExportLabels` interface |
| [api.ts](src/lib/api.ts) | Frontend API: `openExportPreview()` |
| [+page.svelte](src/routes/+page.svelte) | Export button handler, label construction |
| [sk.ts](src/lib/i18n/sk.ts) | Slovak translation strings |
| [en.ts](src/lib/i18n/en.ts) | English translation strings |

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

A "Prvý záznam" (First Record) trip is auto-generated with initial odometer to establish the year's starting point. This matches the on-screen grid behavior where year-start odometer is displayed.

### Why Deviation as Percentage?

The deviation shows actual consumption as a percentage of TP norm (e.g., 105% = 5% over norm). This is Slovak tax authority convention where < 120% is legally compliant for expense deduction.

### Why Temp File Approach?

Writing to temp directory and opening via `shell::open()` works reliably across Windows/macOS/Linux without requiring a custom print dialog. The browser handles all print UI.
