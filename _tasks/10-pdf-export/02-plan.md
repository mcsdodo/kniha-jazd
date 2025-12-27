# PDF Export Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Export official "Kniha jázd" PDF document with all trip data for the selected year.

**Architecture:** Backend generates PDF bytes using `genpdf` crate. Frontend calls Tauri command, receives bytes, uses `tauri-plugin-dialog` for save dialog, writes to user-selected path.

**Tech Stack:** Rust (genpdf), Tauri 2 (dialog plugin), SvelteKit (frontend button)

---

## Task 1: Add Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `src-tauri/capabilities/default.json`

**Step 1: Add genpdf to Cargo.toml**

```toml
# Add after rand = "0.8" line in [dependencies]
genpdf = "0.3"
tauri-plugin-dialog = "2"
```

**Step 2: Add dialog plugin to package.json**

Run:
```bash
npm install @tauri-apps/plugin-dialog
```

**Step 3: Add dialog capabilities**

Edit `src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "enables the default permissions",
  "windows": [
    "main"
  ],
  "permissions": [
    "core:default",
    "dialog:default"
  ]
}
```

**Step 4: Verify dependencies compile**

Run: `cd src-tauri && cargo check`
Expected: Compilation succeeds (may take time for first download)

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml package.json package-lock.json src-tauri/capabilities/default.json
git commit -m "deps: add genpdf and tauri-plugin-dialog for PDF export"
```

---

## Task 2: Embed DejaVu Sans Font

**Files:**
- Create: `src-tauri/assets/fonts/DejaVuSans.ttf`
- Create: `src-tauri/assets/fonts/DejaVuSans-Bold.ttf`

**Step 1: Create assets directory**

```bash
mkdir -p src-tauri/assets/fonts
```

**Step 2: Download DejaVu Sans fonts**

Download from https://dejavu-fonts.github.io/ or use direct links:
- Regular: https://github.com/dejavu-fonts/dejavu-fonts/raw/master/dist/DejaVuSans.ttf
- Bold: https://github.com/dejavu-fonts/dejavu-fonts/raw/master/dist/DejaVuSans-Bold.ttf

Place in `src-tauri/assets/fonts/`

**Step 3: Verify files exist**

```bash
ls -la src-tauri/assets/fonts/
```
Expected: Both .ttf files present

**Step 4: Commit**

```bash
git add src-tauri/assets/fonts/
git commit -m "assets: add DejaVu Sans fonts for PDF export"
```

---

## Task 3: Create PDF Export Module - Data Structures

**Files:**
- Modify: `src-tauri/src/export.rs`

**Step 1: Write the export module with data structures**

Replace contents of `src-tauri/src/export.rs`:

```rust
//! PDF export functionality for Kniha jázd

use crate::models::{Settings, Trip, TripGridData, Vehicle};
use genpdf::fonts;
use genpdf::{elements, style, Document, Element, Margins};
use std::collections::HashMap;

/// Result of PDF generation
pub struct PdfExportData {
    pub vehicle: Vehicle,
    pub settings: Settings,
    pub grid_data: TripGridData,
    pub year: i32,
    pub totals: ExportTotals,
}

/// Calculated totals for the export
pub struct ExportTotals {
    pub total_km: f64,
    pub total_fuel_liters: f64,
    pub total_fuel_cost: f64,
    pub total_other_costs: f64,
    pub avg_consumption: f64,
    pub deviation_percent: f64,
}

impl ExportTotals {
    pub fn calculate(trips: &[Trip], tp_consumption: f64) -> Self {
        let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();
        let total_fuel_liters: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
        let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
        let total_other_costs: f64 = trips.iter().filter_map(|t| t.other_costs_eur).sum();

        let avg_consumption = if total_km > 0.0 {
            (total_fuel_liters / total_km) * 100.0
        } else {
            0.0
        };

        let deviation_percent = if tp_consumption > 0.0 && total_fuel_liters > 0.0 {
            (avg_consumption / tp_consumption) * 100.0
        } else {
            100.0 // No deviation if no data
        };

        Self {
            total_km,
            total_fuel_liters,
            total_fuel_cost,
            total_other_costs,
            avg_consumption,
            deviation_percent,
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src-tauri/src/export.rs
git commit -m "feat(export): add PDF export data structures"
```

---

## Task 4: Create PDF Export Module - Generation Logic

**Files:**
- Modify: `src-tauri/src/export.rs`

**Step 1: Add the PDF generation function**

Add to `src-tauri/src/export.rs` after the existing code:

```rust
/// Generate PDF bytes for the logbook export
pub fn generate_pdf(data: PdfExportData) -> Result<Vec<u8>, String> {
    // Load fonts from embedded bytes
    let regular_bytes = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    let bold_bytes = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");

    let regular = fonts::FontData::new(regular_bytes.to_vec(), None)
        .map_err(|e| format!("Failed to load regular font: {}", e))?;
    let bold = fonts::FontData::new(bold_bytes.to_vec(), None)
        .map_err(|e| format!("Failed to load bold font: {}", e))?;

    let font_family = fonts::FontFamily {
        regular,
        bold,
        italic: fonts::FontData::new(regular_bytes.to_vec(), None)
            .map_err(|e| format!("Failed to load italic font: {}", e))?,
        bold_italic: fonts::FontData::new(bold_bytes.to_vec(), None)
            .map_err(|e| format!("Failed to load bold-italic font: {}", e))?,
    };

    // Create document with landscape A4
    let mut doc = Document::new(font_family);
    doc.set_paper_size(genpdf::PaperSize::A4);
    doc.set_landscape();
    doc.set_margins(Margins::trbl(10, 15, 10, 15)); // top, right, bottom, left in mm

    // Add title
    doc.push(
        elements::Paragraph::new("KNIHA JÁZD")
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(0.5));

    // Add company info
    let company_line = format!(
        "Firma: {} | IČO: {}",
        data.settings.company_name,
        data.settings.company_ico
    );
    doc.push(elements::Paragraph::new(company_line));

    // Add vehicle info
    let vehicle_line = format!(
        "Vozidlo: {} | ŠPZ: {} | Nádrž: {} L | TP spotreba: {} l/100km",
        data.vehicle.name,
        data.vehicle.license_plate,
        data.vehicle.tank_size_liters,
        data.vehicle.tp_consumption
    );
    doc.push(elements::Paragraph::new(vehicle_line));

    // Add year
    doc.push(elements::Paragraph::new(format!("Rok: {}", data.year)));
    doc.push(elements::Break::new(1.0));

    // Build trip table
    let table = build_trip_table(&data);
    doc.push(table);

    doc.push(elements::Break::new(1.0));

    // Add footer with totals
    let footer = build_footer(&data.totals, data.vehicle.tp_consumption);
    doc.push(footer);

    // Render to bytes
    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Failed to render PDF: {}", e))?;

    Ok(buffer)
}

fn build_trip_table(data: &PdfExportData) -> elements::TableLayout {
    let mut table = elements::TableLayout::new(vec![
        1, // Dátum
        2, // Odkiaľ
        2, // Kam
        2, // Účel
        1, // Km
        1, // ODO
        1, // PHM (L)
        1, // € PHM
        1, // € Iné
        2, // Poznámka
        1, // Zostatok
        1, // Spotreba
    ]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

    // Header row
    let headers = vec![
        "Dátum",
        "Odkiaľ",
        "Kam",
        "Účel",
        "Km",
        "ODO",
        "PHM (L)",
        "€ PHM",
        "€ Iné",
        "Poznámka",
        "Zostatok",
        "Spotreba",
    ];

    let mut header_row = table.row();
    for h in headers {
        header_row.push_element(
            elements::Paragraph::new(h)
                .styled(style::Style::new().bold().with_font_size(8)),
        );
    }
    header_row.push().expect("Failed to push header row");

    // Data rows
    for trip in &data.grid_data.trips {
        let trip_id = trip.id.to_string();
        let rate = data.grid_data.rates.get(&trip_id).copied().unwrap_or(0.0);
        let zostatok = data
            .grid_data
            .fuel_remaining
            .get(&trip_id)
            .copied()
            .unwrap_or(0.0);

        let mut row = table.row();
        row.push_element(cell(&trip.date.format("%d.%m.%Y").to_string()));
        row.push_element(cell(&trip.origin));
        row.push_element(cell(&trip.destination));
        row.push_element(cell(&trip.purpose));
        row.push_element(cell(&format!("{:.0}", trip.distance_km)));
        row.push_element(cell(&format!("{:.0}", trip.odometer)));
        row.push_element(cell(
            &trip
                .fuel_liters
                .map(|f| format!("{:.2}", f))
                .unwrap_or_default(),
        ));
        row.push_element(cell(
            &trip
                .fuel_cost_eur
                .map(|f| format!("{:.2}", f))
                .unwrap_or_default(),
        ));
        row.push_element(cell(
            &trip
                .other_costs_eur
                .map(|f| format!("{:.2}", f))
                .unwrap_or_default(),
        ));
        row.push_element(cell(
            trip.other_costs_note.as_deref().unwrap_or(""),
        ));
        row.push_element(cell(&format!("{:.1}", zostatok)));
        row.push_element(cell(&format!("{:.2}", rate)));
        row.push().expect("Failed to push data row");
    }

    table
}

fn cell(text: &str) -> elements::Paragraph {
    elements::Paragraph::new(text).styled(style::Style::new().with_font_size(7))
}

fn build_footer(totals: &ExportTotals, tp_consumption: f64) -> elements::Paragraph {
    let footer_text = format!(
        "SPOLU: {:.0} km | PHM: {:.2} L / {:.2} € | Iné náklady: {:.2} € | \
         Priemerná spotreba: {:.2} l/100km | Odchýlka oproti TP: {:.1}%",
        totals.total_km,
        totals.total_fuel_liters,
        totals.total_fuel_cost,
        totals.total_other_costs,
        totals.avg_consumption,
        totals.deviation_percent
    );

    elements::Paragraph::new(footer_text).styled(style::Style::new().bold().with_font_size(9))
}
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src-tauri/src/export.rs
git commit -m "feat(export): add PDF generation logic with genpdf"
```

---

## Task 5: Add Export Tauri Command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add export_pdf command to commands.rs**

Add at the end of `src-tauri/src/commands.rs`, before the closing of the file:

```rust
// ============================================================================
// PDF Export Commands
// ============================================================================

#[tauri::command]
pub fn export_pdf(
    db: State<Database>,
    vehicle_id: String,
    year: i32,
) -> Result<Vec<u8>, String> {
    use crate::export::{generate_pdf, ExportTotals, PdfExportData};

    // Get vehicle
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    // Get settings
    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    // Get trip grid data (includes trips and calculated values)
    let grid_data = {
        let trips = db
            .get_trips_for_vehicle_in_year(&vehicle_id, year)
            .map_err(|e| e.to_string())?;

        if trips.is_empty() {
            return Err("Žiadne záznamy na export".to_string());
        }

        // Sort chronologically for calculations
        let mut chronological = trips.clone();
        chronological.sort_by(|a, b| {
            a.date.cmp(&b.date).then_with(|| {
                a.odometer
                    .partial_cmp(&b.odometer)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        let (rates, estimated_rates) =
            calculate_period_rates(&chronological, vehicle.tp_consumption);
        let fuel_remaining =
            calculate_fuel_remaining(&chronological, &rates, vehicle.tank_size_liters);

        crate::models::TripGridData {
            trips,
            rates,
            estimated_rates,
            fuel_remaining,
            date_warnings: HashSet::new(),
            consumption_warnings: HashSet::new(),
        }
    };

    // Calculate totals
    let totals = ExportTotals::calculate(&grid_data.trips, vehicle.tp_consumption);

    // Generate PDF
    let pdf_data = PdfExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
    };

    generate_pdf(pdf_data)
}
```

**Step 2: Register the command in lib.rs**

In `src-tauri/src/lib.rs`, add `commands::export_pdf` to the invoke_handler list:

```rust
.invoke_handler(tauri::generate_handler![
    commands::get_vehicles,
    // ... existing commands ...
    commands::get_trip_grid_data,
    commands::export_pdf,  // Add this line
])
```

**Step 3: Add dialog plugin registration in lib.rs**

In `src-tauri/src/lib.rs`, add the dialog plugin in the setup:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())  // Add this line after default()
        .setup(|app| {
            // ... existing setup code ...
```

**Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(export): add export_pdf Tauri command"
```

---

## Task 6: Add Frontend API and Export Function

**Files:**
- Modify: `src/lib/api.ts`

**Step 1: Add exportPdf function to api.ts**

Add at the end of `src/lib/api.ts`:

```typescript
// PDF Export
export async function exportPdf(vehicleId: string, year: number): Promise<Uint8Array> {
	const bytes: number[] = await invoke('export_pdf', { vehicleId, year });
	return new Uint8Array(bytes);
}
```

**Step 2: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add exportPdf function"
```

---

## Task 7: Add Export Button to UI

**Files:**
- Modify: `src/routes/+page.svelte`

**Step 1: Add import and export function**

At the top of the `<script>` section in `src/routes/+page.svelte`, add:

```typescript
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';
import { exportPdf } from '$lib/api';
```

Add the export handler function after `handleTripsChanged`:

```typescript
async function handleExportPdf() {
    if (!$activeVehicleStore) return;

    try {
        // Generate PDF bytes from backend
        const pdfBytes = await exportPdf($activeVehicleStore.id, $selectedYearStore);

        // Show save dialog
        const suggestedName = `kniha-jazd-${$selectedYearStore}-${$activeVehicleStore.license_plate}.pdf`;
        const filePath = await save({
            defaultPath: suggestedName,
            filters: [{ name: 'PDF', extensions: ['pdf'] }]
        });

        if (filePath) {
            // Write file
            await writeFile(filePath, pdfBytes);
            // Could add a success toast here
        }
    } catch (error) {
        console.error('Export failed:', error);
        alert(error instanceof Error ? error.message : 'Export zlyhal');
    }
}
```

**Step 2: Add the export button to the UI**

In the template section, find the `vehicle-header` div and add an export button. Modify:

```svelte
<div class="vehicle-header">
    <div class="header-title-row">
        <h2>Aktívne vozidlo</h2>
        <button class="export-btn" on:click={handleExportPdf} title="Exportovať PDF">
            Exportovať PDF
        </button>
    </div>
    {#if stats}
        <!-- existing stats-container -->
    {/if}
</div>
```

**Step 3: Add button styles**

Add to the `<style>` section:

```css
.header-title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.export-btn {
    padding: 0.5rem 1rem;
    background-color: #27ae60;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 0.875rem;
    cursor: pointer;
    transition: background-color 0.2s;
}

.export-btn:hover {
    background-color: #219a52;
}
```

**Step 4: Add fs plugin dependency**

Run:
```bash
npm install @tauri-apps/plugin-fs
```

**Step 5: Add fs capability**

Modify `src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "enables the default permissions",
  "windows": [
    "main"
  ],
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default"
  ]
}
```

**Step 6: Add fs plugin to Cargo.toml and lib.rs**

Add to `src-tauri/Cargo.toml`:
```toml
tauri-plugin-fs = "2"
```

Add to `src-tauri/src/lib.rs` after dialog plugin:
```rust
.plugin(tauri_plugin_fs::init())
```

**Step 7: Verify the app compiles and runs**

Run: `npm run tauri dev`
Expected: App starts, export button visible

**Step 8: Commit**

```bash
git add src/routes/+page.svelte src-tauri/capabilities/default.json package.json package-lock.json src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "feat(ui): add PDF export button"
```

---

## Task 8: Test PDF Export End-to-End

**Step 1: Run the app**

```bash
npm run tauri dev
```

**Step 2: Test with existing data**

1. Select a vehicle with trips
2. Select a year with data
3. Click "Exportovať PDF"
4. Choose save location
5. Open the saved PDF

**Step 3: Verify PDF content**

- Header shows company name, IČO
- Vehicle info (name, plate, tank, TP consumption)
- Year displayed correctly
- All trips in table with all 12 columns
- Totals in footer match app display
- Slovak characters render correctly

**Step 4: Test edge case - no trips**

1. Select year with no trips
2. Click export
3. Should show error message "Žiadne záznamy na export"

**Step 5: Commit final state**

```bash
git add .
git commit -m "feat: complete PDF export implementation"
```

---

## Summary Checklist

| Task | Description | Status |
|------|-------------|--------|
| 1 | Add dependencies (genpdf, dialog plugin) | |
| 2 | Embed DejaVu Sans fonts | |
| 3 | Create export data structures | |
| 4 | Implement PDF generation logic | |
| 5 | Add export_pdf Tauri command | |
| 6 | Add frontend API function | |
| 7 | Add export button to UI | |
| 8 | End-to-end testing | |

---

**Plan complete and saved to `_tasks/10-pdf-export/02-plan.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
