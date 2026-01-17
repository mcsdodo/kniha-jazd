# PDF Export - Design

**Status:** Designed
**Date:** 2025-12-28

## Purpose

Export official "Kniha jázd" PDF document for Slovak tax authorities and audits. Contains all trip data for a selected year with company/vehicle info and consumption statistics.

## Document Layout

**Format:** Landscape A4 (297 × 210 mm)

```
┌─────────────────────────────────────────────────────────────────────┐
│ KNIHA JÁZD                                                          │
│ Firma: [company_name] | IČO: [company_ico]                          │
│ Vozidlo: [name] | ŠPZ: [plate] | Nádrž: [L] | TP spotreba: [l/100km]│
│ Rok: [year]                                                         │
├─────────────────────────────────────────────────────────────────────┤
│ Dátum | Odkiaľ | Kam | Účel | Km | ODO | PHM(L) | €PHM | €Iné |     │
│       |        |     |      |    |     |        |      |      | ... │
│  ... trip rows ...                                                  │
├─────────────────────────────────────────────────────────────────────┤
│ SPOLU: [km] km | PHM: [L] L / [€] € | Iné náklady: [€] €            │
│ Priemerná spotreba: [x.xx] l/100km | Odchýlka oproti TP: [xxx]%     │
└─────────────────────────────────────────────────────────────────────┘
```

## Table Columns (12)

| # | Slovak Label | Field | Notes |
|---|--------------|-------|-------|
| 1 | Dátum | date | dd.mm.yyyy format |
| 2 | Odkiaľ | origin | |
| 3 | Kam | destination | |
| 4 | Účel | purpose | |
| 5 | Km | distance_km | |
| 6 | ODO | odometer | |
| 7 | PHM (L) | fuel_liters | Empty if no fillup |
| 8 | € PHM | fuel_cost_eur | |
| 9 | € Iné | other_costs_eur | |
| 10 | Poznámka | other_costs_note | Note for other costs |
| 11 | Zostatok | fuel_remaining | Calculated |
| 12 | Spotreba | rate | l/100km for period |

## Footer Totals

- Celkom km: sum of distance_km
- Celkom PHM: sum of fuel_liters (L) + sum of fuel_cost_eur (€)
- Celkom iné náklady: sum of other_costs_eur (€)
- Priemerná spotreba: total_fuel / total_km * 100 (l/100km)
- Odchýlka spotreby oproti TP: (avg_consumption / tp_consumption) * 100 (%)

## Technical Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| PDF library | `genpdf` crate | Built-in table layout helpers |
| Font | DejaVu Sans (embedded) | Slovak character support (ľščťžýáíéúäôň) |
| Scope | Selected year only | Aligns with year-scoped architecture (ADR-009) |
| Save method | Native save dialog | Standard UX, user picks location |

## Architecture

```
src-tauri/src/export.rs
├── generate_pdf(vehicle_id, year, settings) -> Result<Vec<u8>>
│   ├── build_header(settings, vehicle, year)
│   ├── build_trip_table(grid_data)
│   └── build_footer(stats)
```

**Tauri command:**
```rust
#[tauri::command]
async fn export_pdf(vehicle_id: String, year: i32) -> Result<Vec<u8>, String>
```

**Frontend flow:**
1. User clicks "Exportovať PDF" button (in header near year picker)
2. Call `invoke("export_pdf", { vehicleId, year })`
3. On success, open Tauri save dialog with suggested filename: `kniha-jazd-{year}-{plate}.pdf`
4. Write returned bytes to selected path
5. Show success toast

## Error Handling

| Scenario | Behavior |
|----------|----------|
| No trips for year | Show message: "Žiadne záznamy na export" |
| Save cancelled | Silent, no error |
| Write failed | Show error message with details |

## Dependencies to Add

```toml
# src-tauri/Cargo.toml
[dependencies]
genpdf = "0.3"
```

Font file: `src-tauri/assets/fonts/DejaVuSans.ttf`
