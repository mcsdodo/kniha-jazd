# Feature Documentation TODO

Features to document, ranked by complexity and importance.

## Priority Order

| Priority | Feature | Status | File |
|----------|---------|--------|------|
| 🔴 **1** | [Trip Grid Calculation](#1-trip-grid-calculation) | ✅ | [trip-grid-calculation.md](trip-grid-calculation.md) |
| 🔴 **2** | [Backup System](#2-backup-system) | ✅ | [backup-system.md](backup-system.md) |
| 🟠 **3** | [Receipt Scanning & AI OCR](#3-receipt-scanning--ai-ocr) | ✅ | [receipt-scanning.md](receipt-scanning.md) |
| 🟠 **4** | [Read-Only Mode](#4-read-only-mode) | ✅ | [read-only-mode.md](read-only-mode.md) |
| 🟡 **5** | [Multi-Year State](#5-multi-year-state) | ✅ | [multi-year-state.md](multi-year-state.md) |
| 🟡 **6** | [Export System](#6-export-system) | ✅ | [export-system.md](export-system.md) |
| 🟢 **7** | [Magic Fill](#7-magic-fill) | ✅ | [magic-fill.md](magic-fill.md) |
| 🟢 **8** | [Settings Architecture](#8-settings-architecture) | ✅ | [settings-architecture.md](settings-architecture.md) |

✅ = Done | ⬜ = Not started

**All features documented! 🎉**

---

## 1. Trip Grid Calculation

**Why:** Core of the app — 7 modules, 3 vehicle types, period-based logic

**Files involved:**
- `commands.rs:819-985` — `get_trip_grid_data()` (main orchestrator)
- `commands.rs:1076-1230` — `calculate_period_rates()` (ICE)
- `commands.rs:1231-1344` — `calculate_energy_grid_data()` (BEV)
- `commands.rs:1345-1479` — `calculate_phev_grid_data()` (PHEV)
- `calculations.rs` — Core math (consumption, margin, buffer km)
- `calculations_energy.rs` — Battery calculations
- `calculations_phev.rs` — PHEV prioritization logic

**Key concepts to document:**
- 3 different calculation paths (ICE/BEV/PHEV)
- Period-based consumption (closed periods vs open current period)
- PHEV depletes electricity BEFORE fuel (counterintuitive)
- Year carryover (odometer, fuel, battery from previous year)
- "Chronological vs sort_order" trips
- 20% margin limit for legal compliance

---

## 2. Backup System

**Why:** Retention policies, pre-update backups, auto-cleanup

**Files involved:**
- `commands.rs:1616-1775` — Backup creation/listing/cleanup
- `commands.rs:1569-1615` — Backup filename parsing
- `lib.rs:134-150` — Post-update auto-cleanup trigger
- `src/routes/settings/+page.svelte` — Retention UI

**Key concepts to document:**
- Filename encodes type + version: `kniha-jazd-20250125-pre-v0.20.0.db`
- Retention policy: Keep N most recent pre-update backups
- Post-update cleanup runs silently at startup
- Backups move WITH database to custom locations
- Manual vs auto pre-update backups (different cleanup rules)

---

## 3. Receipt Scanning & AI OCR

**Why:** Async pipeline, Gemini AI, confidence scoring, currency handling

**Files involved:**
- `receipts.rs:113-202` — Folder structure detection + scanning
- `receipts.rs:204-285` — Gemini API extraction + confidence mapping
- `gemini.rs:272-358` — API client with mock mode
- `commands.rs:2239-2438` — Scan/sync/process/assign commands
- `src/routes/doklady/+page.svelte` — Receipt UI

**Key concepts to document:**
- Multi-stage flow: scan → Gemini → parse → confidence → assign
- `FolderStructure::YearBased` detection
- Currency handling: EUR auto-fill, foreign needs review
- Confidence scoring: low/medium/high → NeedsReview status
- Mock mode for testing (`KNIHA_JAZD_MOCK_GEMINI_DIR`)
- Receipt status lifecycle: Pending → Parsing → NeedsReview → Verified

---

## 4. Read-Only Mode

**Why:** Lock files, heartbeat thread, multi-PC access

**Files involved:**
- `app_state.rs` — AppMode (Normal/ReadOnly), thread-safe RwLock
- `db_location.rs:103-180` — Lock file creation/checking/staleness
- `lib.rs:62-121` — Startup initialization + lock heartbeat thread
- `commands.rs:33-47` — `check_read_only!()` macro

**Key concepts to document:**
- Lock file staleness: > 2 minutes = stale
- Heartbeat thread keeps lock fresh (every 30 seconds)
- Migration compatibility check (unknown migrations = read-only)
- All 19 write commands need `check_read_only!()` guard
- Read-only triggers: (1) Newer migrations, (2) Lock held by another PC

---

## 5. Multi-Year State

**Why:** Year carryover (odometer, fuel, battery), vehicle switching

**Files involved:**
- `commands.rs:103-235` — Vehicle CRUD
- `commands.rs:242-463` — Trip CRUD with reordering
- `commands.rs:836-905` — Carryover calculations
- `db.rs` — Year filtering queries
- Frontend stores: `activeVehicleStore`, `selectedYearStore`

**Key concepts to document:**
- Year carryover: Previous year's final → current year's start
- Fuel/battery carryover for all vehicle types
- Vehicle type is IMMUTABLE once trips exist
- `sort_order` column for manual trip ordering
- Three separate trip queries for different purposes

---

## 6. Export System

**Why:** Vehicle-type polymorphism, legal compliance format

**Files involved:**
- `export.rs:87-147` — `ExportTotals::calculate()`
- `export.rs:149-671` — `generate_html()` (600+ lines)
- `commands.rs:1967-2238` — `export_to_browser()` and `export_html()`

**Key concepts to document:**
- Three HTML templates: ICE, BEV, PHEV (different columns)
- Column headers & footer vary by vehicle type
- Dummy rows (0 km) excluded from totals
- Deviation % calculation
- i18n through `ExportLabels`

---

## 7. Magic Fill

**Why:** Buffer calculation, helps users stay within legal margin

**Files involved:**
- `commands.rs:1014-1074` — `calculate_magic_fill_liters()`
- `calculations.rs:54-99` — `calculate_buffer_km()`
- Frontend: Trip creation form

**Key concepts to document:**
- Calculates liters needed to hit target margin (e.g., 18%)
- Formula: `target_rate = tp_rate * (1 + target_margin)` → solve for liters
- Only works if current period exists
- Could be merged with Trip Grid doc

---

## 8. Settings Architecture

**Why:** Two separate storage systems — understanding the split prevents confusion

**Files involved:**
- `settings.rs` — `LocalSettings` struct (file-based)
- `models.rs:267-283` — `Settings` struct (database)
- `commands.rs:480-502` — DB settings CRUD
- `commands.rs:1753-1772` — Backup retention settings
- `src/routes/settings/+page.svelte` — Unified Settings UI

**Key concepts to document:**
- **LocalSettings** (file): API keys, paths, theme, backup retention — machine-specific
- **Settings** (database): Company name, IČO, buffer trip purpose — business data
- Why the split: API keys don't travel with shared DB, paths are PC-specific
- Both shown in same UI but saved to different locations
- `local.settings.json` survives app reinstalls

**Storage locations:**
```
LocalSettings → %APPDATA%\com.notavailable.kniha-jazd\local.settings.json
Settings      → kniha-jazd.db (settings table)
```

---

## Completed

- ✅ [Move Database](move-database.md) — Database relocation + multi-PC support
