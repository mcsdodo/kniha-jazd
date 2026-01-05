# Decisions Log

Architecture Decision Records (ADRs) and business logic decisions. **Newest first.**

---

## 2026-01-05: Fuel Carryover

### BIZ-012: Year-End Fuel Carryover Between Years

**Context:** ADR-009 originally specified "zostatok starts fresh (full tank assumption)" for each new year. However, this didn't reflect reality - fuel doesn't magically reset on January 1st.

**Previous behavior:** Each year started with full tank assumption, ignoring actual fuel state from December 31st.

**Decision:** Fuel (zostatok) now carries over from the previous year's ending state.

**Implementation:**
- `get_year_start_zostatok()` calculates carryover from previous year's last trip
- If no previous year data exists, falls back to full tank assumption
- This also prepares for EV support where battery SoC carries over between years

**Reasoning:**
- Matches real-world behavior (fuel doesn't reset on Jan 1)
- Provides accurate consumption tracking across year boundaries
- Enables proper EV battery state tracking (future feature)

**Note:** This supersedes the "zostatok starts fresh" part of ADR-009. The ODO carryover behavior from ADR-009 remains unchanged.

---

## Template for New Decisions

```markdown
### [ADR|BIZ]-NNN: Title

**Context:** What situation prompted this decision?

**Options considered:** (if applicable)
1. Option A
2. Option B

**Decision:** What was decided?

**Reasoning:** Why this choice?
```

---

## 2025-12-30: Receipt Organization

### ADR-010: Receipt Year Filtering

**Context:** Users may organize receipts in different folder structures - either flat (all files in one folder) or year-based (files in YYYY subfolders like `2024/`, `2025/`). The app needs to handle both cases and filter receipts by year while maintaining clear behavior.

**Decision:**
- **Flat mode:** Files directly in receipts folder → shown in all years (no year filtering)
- **Year-based mode:** Files in YYYY subfolders (e.g., `2024/`) → filtered by selected year
- **Invalid structure:** Mixed content (files + folders) or non-year folders → warning shown, files not loaded
- **Year determination priority:**
  1. Primary: Use `receipt_date.year()` from OCR recognition
  2. Fallback: Use `source_year` from folder name (for unprocessed receipts)
- **Mismatch warning:** When folder year differs from OCR-detected receipt date year, show indicator to user

**Reasoning:**
- Users have different organizational preferences; supporting both flat and year-based is flexible
- OCR date is more accurate than folder placement (user may misfile receipts)
- Folder year serves as fallback for new/unprocessed receipts before OCR runs
- Warning on mismatch helps users identify misfiled receipts without blocking workflow

---

## 2025-12-25: Year Picker

### ADR-009: Year-Scoped Vehicle Logbook

**Context:** Each year is a standalone "kniha jázd" for legal purposes.

**Decision:**
- Year picker in header next to vehicle dropdown
- Stats and trips scoped to selected year
- App starts on current calendar year
- Export only shows years with actual data
- ODO carries over from previous year, zostatok starts fresh (full tank assumption)

**Reasoning:** Slovak legal requirements treat each year as independent logbook. Fresh zostatok per year simplifies accounting.

---

## 2025-12-25: Architecture Refactor

### ADR-008: Remove Frontend Calculation Duplication

**Context:** Frontend (`src/lib/calculations.ts`) duplicated Rust backend calculations (`src-tauri/src/calculations.rs`) "for instant UI responsiveness."

**Problem:**
- ~500 lines of duplicate code
- 21 frontend tests duplicating 41 backend tests
- Risk of logic divergence between frontend and backend
- Double maintenance burden

**Options considered:**
1. Keep duplication - test both implementations
2. Move all to Rust - frontend calls Tauri commands
3. Move all to frontend - backend becomes thin data layer

**Decision:** Move all calculations to Rust backend only.

**Reasoning:**
- Tauri IPC is local and fast (microseconds, not network)
- No other clients will ever exist - single desktop app
- Rust backend already has 41 well-tested calculation functions
- Single source of truth eliminates divergence risk
- Frontend becomes simpler display-only logic

**Implementation:** Add `get_trip_grid_data` Tauri command returning pre-calculated values.

---

## 2025-12-23: UI/UX Decisions

### ADR-007: Database Backup/Restore

**Context:** User needs ability to backup and restore database for data safety.

**Decision:**
- Backups stored in `{app_data_dir}/backups/`
- Manual trigger only (no auto-backup)
- Filename: `kniha-jazd-backup-YYYY-MM-DD-HHmmss.db`
- Restore: Full DB replacement with confirmation showing date, counts, warning
- Keep all backups (no auto-deletion)

**Reasoning:** Simple, transparent backup system. User controls when to backup/restore.

---

### ADR-006: Navigation Header

**Context:** Settings button was buried at bottom of page, requiring scroll.

**Decision:** Top header bar with "Kniha jázd | Nastavenia" navigation links.

**Reasoning:** Always visible, no scrolling needed, clear app structure.

---

### ADR-005: Totals Section Redesign

**Context:** Original single-row totals were cramped and unclear.

**Decision:**
- Two-row layout for totals
- Rename "Km" to "Celkovo najazdené" for clarity
- Show fuel totals and cost summary on separate row

**Reasoning:** Better readability, clearer labels for legal documentation.

---

## 2025-12-23: Calculation Logic Fixes

### BIZ-011: Legal Limit Based on Average Consumption

**Context:** Should the 20% over-limit warning use the last fill-up rate or overall average?

**Decision:** Use **average consumption** (total_fuel / total_km × 100) for legal compliance check.

**Reasoning:** Legal compliance is about the overall picture, not a single fill-up. If average is 6.00 and limit is 6.12 (5.1 × 1.2), we're compliant even if one fill-up was higher.

---

### BIZ-010: Retroactive Consumption Rate Application

**Context:** When a fill-up occurs, which trips should use that rate?

**Decision:** Apply the rate **retroactively** to ALL trips since the previous fill-up.

**Example:** If trips A, B, C happen, then fill-up on C gives rate 6.0 → A, B, and C all show 6.0 l/100km.

**Reasoning:** Matches Excel behavior. The rate represents the consumption for that entire period.

---

### BIZ-009: Same-Day Trip Ordering

**Context:** Multiple trips on the same date need deterministic ordering for correct calculations.

**Decision:** Sort by date, then by **odometer** as tiebreaker.

**Reasoning:** Odometer is sequential and represents actual trip order. Using created_at would fail for imported data.

---

### BIZ-008: ODO Auto-Calculation

**Context:** Manual ODO entry is error-prone and redundant since ODO = previous ODO + km driven.

**Decision:** Auto-calculate ODO when km is entered: `ODO = previousODO + km`. User can still manually override.

**Reasoning:** Reduces data entry errors, matches Excel workflow where this was a formula.

---

## 2024-12-23: Business Logic Decisions

### BIZ-007: Fill-up Detection

**Context:** How to distinguish regular trips from fill-ups?

**Decision:** Auto-detect. If liters field is filled → it's a fill-up. No separate entry types.

**Reasoning:** Simpler UX, matches Excel behavior.

---

### BIZ-006: UI Display Order vs Export Order

**Context:** How to show trips in UI vs PDF export?

**Decision:**
- UI: Newest trips on top (reverse chronological) - easier access
- Export: Oldest first (chronological) - matches Excel/legal format

---

### BIZ-005: Route Distance Memory

**Context:** User often drives same routes.

**Decision:** Store origin→destination pairs with their distances. When user selects a known route, auto-fill the km field.

**Reasoning:** Reduces data entry, fewer errors.

---

### BIZ-004: Compensation Trip Suggestions

**Context:** How to help user plan trips to stay within legal margin?

**Decision:**
1. Calculate km needed to bring margin under limit
2. First, try to find existing route from current location matching needed km (±10%)
3. Fallback: Suggest buffer trip with configurable purpose (e.g., "služobná cesta")
4. Target margin: 16-19% (provides safety buffer below 20% limit)

**Reasoning:** Maintaining a buffer below the 20% limit helps ensure compliance even with measurement variations.

---

### BIZ-003: Legal Margin Limit

**Context:** What's the allowed over-consumption?

**Decision:** Max 20% over the vehicle's TP (technical passport) consumption rate.

**Example:** TP = 5.1 l/100km → Max allowed = 6.12 l/100km

---

### BIZ-002: Pouzita Spotreba (Used Consumption Rate)

**Context:** What rate is used to calculate fuel consumption between fill-ups?

**Decision:**
- Initial value: TP rate from vehicle (e.g., 5.1 l/100km)
- After first fill-up: Use the calculated l/100km from that fill-up
- Rate carries forward until next fill-up recalculates it

**Validation:** Matches Excel pattern - each fill-up sets the rate for subsequent trips.

---

### BIZ-001: Consumption Rate Calculation

**Context:** How is l/100km calculated?

**Decision:** On each fill-up: `l/100km = liters_filled / km_since_last_fillup × 100`

**Validation:** Verified against Excel data - formula matches exactly.

---

## 2024-12-23: Architecture Decisions

### ADR-004: Code in English, UI in Slovak

**Context:** User is Slovak, app is for Slovak legal requirements.

**Decision:**
- All code, variables, comments: English
- UI text: Slovak with i18n support for future translation

**Reasoning:**
- English code is industry standard, easier to maintain
- Slovak UI serves the primary user
- i18n-ready for potential future users

---

### ADR-003: Test-Driven Development

**Context:** Need reliable calculations for legal compliance (20% margin rule).

**Decision:** TDD with focus on business logic tests only

**Reasoning:**
- Calculation errors = legal compliance issues
- Tests must be meaningful, not filler
- Focus: consumption calculations, margin checks, compensation suggestions
- Skip: trivial CRUD, UI rendering, getters/setters

---

### ADR-002: SQLite for Local Storage

**Context:** Need to store trips, vehicles, and calculated data.

**Decision:** SQLite (single local file)

**Reasoning:**
- Simple, portable, robust
- Single file easy to backup/move
- Can still export to Excel/CSV for accountants
- No server needed for personal logbook

---

### ADR-001: Desktop App with Tauri + SvelteKit

**Context:** Need to build a vehicle logbook app to replace Excel spreadsheet.

**Options considered:**
1. Electron + React/Vue - Cross-platform, larger bundle (~150MB+)
2. Tauri + SvelteKit - Cross-platform, Rust backend, small bundle (~10-20MB)
3. Python + PyQt - Good for data apps, simpler
4. C# WPF - Windows-only, excellent Excel interop
5. .NET MAUI + Blazor - Cross-platform, C# everywhere

**Decision:** Tauri + SvelteKit

**Reasoning:**
- User said "don't limit ourselves" - open to learning Rust
- Best end-user experience (small, fast, native)
- Svelte is the simplest modern frontend framework
- No need for Excel interop - reimplementing functionality, not integrating
