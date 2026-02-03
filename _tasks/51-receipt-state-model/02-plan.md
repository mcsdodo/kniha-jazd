# Implementation Plan: Receipt-Trip State Model

**Date:** 2026-02-02
**Updated:** 2026-02-03
**Status:** Ready for Implementation
**Design:** `_TECH_DEBT/05-receipt-trip-state-model-design.md`

---

## Overview

5 phases, each independently testable:
1. Database migration (add fields)
2. Backend logic (assignment with type)
3. Backend verification (new state calculation)
4. Frontend - Invoice grid ⟂
5. Frontend - Trip grid ⟂

**Parallelization:** Phases 4 & 5 can run in parallel (marked with ⟂)

```
Phase 1 → Phase 2 → Phase 3 ─┬─→ Phase 4 (Invoice Grid) ─┬─→ Phase 6
                             │                           │
                             └─→ Phase 5 (Trip Grid) ────┘
```

---

## Key Design Decisions

### Storage vs Display Separation

**DB Storage (scalar fields only):**
- `assignment_type: TEXT` - stores "Fuel" or "Other" (serde default serialization)
- `mismatch_override: INTEGER` - boolean (0/1)
- `trip_id: TEXT` - FK to trips, nullable

**Computed Display State (never stored):**
- `ReceiptDisplayState` - rich type with trip info and mismatch details
- Calculated on-demand by `get_receipt_display_state()`
- Replaces old `ReceiptVerification` struct

### Data Invariant

```
trip_id = NULL  ↔  assignment_type = NULL   (unassigned)
trip_id = SET   ↔  assignment_type = SET    (assigned)
```

### AssignmentType Serialization

Uses serde default - stores as `"Fuel"` or `"Other"` TEXT in SQLite.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssignmentType {
    Fuel,   // stored as "Fuel"
    Other,  // stored as "Other"
}
```

### Mismatch Override Behavior

| `mismatch_override` | Has Mismatch | UI State |
|---------------------|--------------|----------|
| `false` | No | 🟢 Assigned (no indicator) |
| `false` | Yes | 🟡 AssignedMismatch (yellow warning) |
| `true` | Yes | 🟠 AssignedOverride (orange, user confirmed) |

### Trip Already Has Other Costs

**Decision: Block** (keep current behavior)
- Returns error: "Jazda už má iné náklady"
- User must unassign existing invoice first
- Prevents accidental data loss

---

## Phase 1: Database Migration

**Goal:** Add new fields to receipts table

### 1.1 Create migration file
- File: `src-tauri/migrations/YYYYMMDD_receipt_assignment_type.sql`

```sql
-- Add assignment type (Fuel or Other)
ALTER TABLE receipts ADD COLUMN assignment_type TEXT;

-- Add mismatch override flag
ALTER TABLE receipts ADD COLUMN mismatch_override INTEGER DEFAULT 0;

-- Unassign any existing receipts (rare edge case)
-- User can manually reassign with explicit type
UPDATE receipts SET trip_id = NULL WHERE trip_id IS NOT NULL;
```

### 1.2 Update Diesel schema
- Run `diesel print-schema` or manually update `src-tauri/src/schema.rs`

### 1.3 Update Receipt model
- File: `src-tauri/src/models.rs`
- Add fields to `Receipt` struct
- Add `AssignmentType` enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssignmentType {
    Fuel,
    Other,
}

pub struct Receipt {
    // ... existing fields
    pub assignment_type: Option<String>,  // "Fuel" or "Other", stored as TEXT
    pub mismatch_override: bool,
}
```

### 1.4 Update TypeScript types
- File: `src/lib/types.ts`
- Add corresponding fields

```typescript
type AssignmentType = 'Fuel' | 'Other';

interface Receipt {
  // ... existing fields
  assignmentType: AssignmentType | null;
  mismatchOverride: boolean;
}
```

**Tests:** Verify migration runs, fields exist in DB

---

## Phase 2: Backend Assignment Logic

**Goal:** User picks FUEL or OTHER when assigning

### 2.1 Update `assign_receipt_to_trip_internal()`
- File: `src-tauri/src/commands/receipts_cmd.rs`
- Add `assignment_type: String` parameter (accepts "Fuel" or "Other")
- Add `mismatch_override: bool` parameter
- Remove auto-detection logic
- Set both `receipt.trip_id` AND `receipt.assignment_type` together

### 2.2 Add mismatch detection
- When assigning as FUEL, check if data matches (time, liters, price)
- If mismatch and `mismatch_override = false` → assignment succeeds, warning shown in UI
- If mismatch and `mismatch_override = true` → assignment succeeds, warning suppressed

### 2.3 Update Tauri command signature
- File: `src-tauri/src/lib.rs`
- Update command registration

```rust
#[tauri::command]
fn assign_receipt_to_trip(
    receipt_id: String,
    trip_id: String,
    assignment_type: String,      // "Fuel" or "Other"
    mismatch_override: bool,      // true = user confirmed mismatch
    // ... other params
) -> Result<(), String>
```

### 2.4 Write backend tests
- File: `src-tauri/src/commands/commands_tests.rs`

| Test Name | Scenario |
|-----------|----------|
| `test_assign_fuel_to_empty_trip_populates_data` | C1: FUEL to empty trip |
| `test_assign_other_to_empty_trip_populates_data` | C2: OTHER to empty trip |
| `test_assign_fuel_with_matching_data_links_only` | C3: Data matches |
| `test_assign_fuel_with_mismatch_no_override` | C4: Mismatch, no override |
| `test_assign_fuel_with_mismatch_and_override` | C5: Mismatch + override |
| `test_assign_other_to_trip_with_existing_other_costs_blocked` | C6: Block duplicate |
| `test_reassign_invoice_to_different_trip` | C7: Reassignment |

**Tests:** All scenarios pass

---

## Phase 3: Backend Verification Logic

**Goal:** Unified state calculation for both grids

### 3.1 Create `ReceiptDisplayState` (computed, not stored)
- File: `src-tauri/src/models.rs`

```rust
/// Computed display state - NEVER stored in DB
/// Returned by get_receipt_display_state() for UI rendering
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state")]
pub enum ReceiptDisplayState {
    Processing,
    NeedsReview,
    Unassigned,
    Assigned {
        trip_id: String,
        trip_summary: TripSummary,
        assignment_type: AssignmentType,
    },
    AssignedMismatch {
        trip_id: String,
        trip_summary: TripSummary,
        assignment_type: AssignmentType,
        mismatches: Vec<Mismatch>,
    },
    AssignedOverride {
        trip_id: String,
        trip_summary: TripSummary,
        assignment_type: AssignmentType,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TripSummary {
    pub date: String,
    pub route: String,
    pub time_range: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Mismatch {
    TimeOutsideRange { receipt_time: String, trip_range: String },
    LitersDiffer { receipt: f64, trip: f64 },
    PriceDiffers { receipt: f64, trip: f64 },
}
```

### 3.2 Create `get_receipt_display_state()` function
- File: `src-tauri/src/commands/receipts_cmd.rs`
- Single source of truth for state calculation
- Computes state from DB fields + live mismatch check

```rust
fn get_receipt_display_state(receipt: &Receipt, trip: Option<&Trip>) -> ReceiptDisplayState {
    match receipt.status {
        ReceiptStatus::Pending => ReceiptDisplayState::Processing,
        ReceiptStatus::NeedsReview => ReceiptDisplayState::NeedsReview,
        _ if receipt.trip_id.is_none() => ReceiptDisplayState::Unassigned,
        _ => {
            let trip = trip.expect("trip required when trip_id is set");
            let mismatches = calculate_mismatches(receipt, trip);

            if receipt.mismatch_override {
                ReceiptDisplayState::AssignedOverride { ... }
            } else if !mismatches.is_empty() {
                ReceiptDisplayState::AssignedMismatch { mismatches, ... }
            } else {
                ReceiptDisplayState::Assigned { ... }
            }
        }
    }
}
```

### 3.3 Update `verify_receipts()`
- Return `ReceiptDisplayState` instead of old `ReceiptVerification`
- Deprecate old struct

### 3.4 Update `calculate_missing_receipts()`
- File: `src-tauri/src/statistics.rs`
- Use `trip_id` directly instead of computed match
- Trip missing invoice = has costs AND no receipt with `trip_id = this_trip`

### 3.5 Update `get_trip_grid_data()`
- Include receipt state info for each trip
- Return mismatch details for warning display

**Tests:** Verify unified calculation works for all scenarios

---

## Phase 4: Frontend - Invoice Grid (Doklady)

**Goal:** Show assignment type, mismatch warnings, override button

### 4.1 Update `TripSelectorModal.svelte`
- Add radio buttons: "Priradiť ako PALIVO" / "Priradiť ako INÉ"
- Pass `assignmentType` and `mismatchOverride` to backend
- Show mismatch warning dialog when data differs:

```
┌─────────────────────────────────────────────────────────────┐
│  Dialog: "Údaje nesúhlasia"                                 │
├─────────────────────────────────────────────────────────────┤
│  [Zrušiť]  [Priradiť s varovaním]  [Priradiť a potvrdiť]   │
│                                                             │
│     ↓              ↓                        ↓               │
│   Cancel    mismatchOverride=false   mismatchOverride=true  │
│             (shows 🟡 warning)       (shows 🟠 override)    │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Update invoice cards in `doklady/+page.svelte`
- Show assignment type badge (PALIVO / INÉ)
- Show mismatch warning with details
- Add "Potvrdiť" button for mismatch override
- Add "Zrušiť potvrdenie" for overridden invoices

### 4.3 Group invoices by state
- "Nepriradené" section (unassigned + needs review)
- "Priradené" section (assigned)

### 4.4 Add i18n keys
- File: `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

```typescript
receipts: {
  assignAsFuel: 'Priradiť ako PALIVO',
  assignAsOther: 'Priradiť ako INÉ NÁKLADY',
  assignedAsFuel: 'PALIVO',
  assignedAsOther: 'INÉ NÁKLADY',
  mismatchWarning: 'Údaje nesúhlasia',
  confirmOverride: 'Potvrdiť',
  cancelOverride: 'Zrušiť potvrdenie',
  assignWithWarning: 'Priradiť s varovaním',
  assignAndConfirm: 'Priradiť a potvrdiť',
}
```

**Tests:** Integration test for assignment flow with type selection [Tier 2]

---

## Phase 5: Frontend - Trip Grid

**Goal:** Inline warning triangles instead of separate column

### 5.1 Update `TripRow.svelte`
- Remove separate receipt indicator (if exists)
- Add inline triangle next to fuel/other field
- Colors: 🔴 missing, 🟡 mismatch, 🟠 override

### 5.2 Update `TripGrid.svelte`
- Update legend for new indicators
- Pass receipt state to TripRow

### 5.3 Add hover tooltips
- 🔴⚠: "Chýba doklad pre tankovanie"
- 🟡⚠: "Čas mimo jazdy: 18:30 vs 15:00-17:00"
- 🟠⚠: "Potvrdené užívateľom"

### 5.4 Update legend i18n
```typescript
trips: {
  legend: {
    missingInvoice: 'chýba doklad',
    dataMismatch: 'nesúlad údajov',
    userOverride: 'potvrdené',
  }
}
```

**Tests:** Integration test for trip grid indicators [Tier 2]

---

## Phase 6: Cleanup

### 6.1 Remove deprecated code
- Remove old `matched` field usage
- Remove old `datetimeWarning` field usage
- Remove auto-detection in `is_fuel_receipt` check
- Remove/deprecate `ReceiptVerification` struct

### 6.2 Update tests
- Remove tests for old behavior
- Ensure all new scenarios covered

---

## Testing Checklist

### Backend Unit Tests
- [ ] `test_assign_fuel_to_empty_trip_populates_data`
- [ ] `test_assign_other_to_empty_trip_populates_data`
- [ ] `test_assign_fuel_with_matching_data_links_only`
- [ ] `test_assign_fuel_with_mismatch_no_override`
- [ ] `test_assign_fuel_with_mismatch_and_override`
- [ ] `test_assign_other_to_trip_with_existing_other_costs_blocked`
- [ ] `test_reassign_invoice_to_different_trip`

### Integration Tests [Tier 2]
- [ ] Assignment dialog shows FUEL/OTHER selector
- [ ] Mismatch warning dialog appears with correct buttons
- [ ] Override button works (yellow → orange)
- [ ] Trip grid shows correct triangles
- [ ] Hover tooltips show details

---

## Estimated Effort

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1: Migration | 1h | None |
| Phase 2: Backend assignment | 3h | Phase 1 |
| Phase 3: Backend verification | 2h | Phase 2 |
| Phase 4: Frontend invoice ⟂ | 4h | Phase 3 |
| Phase 5: Frontend trip grid ⟂ | 2h | Phase 3 |
| Phase 6: Cleanup | 1h | Phase 4, 5 |
| **Total (sequential)** | **~13h** | |
| **Total (parallel 4∥5)** | **~11h** | Phases 4 & 5 run concurrently |

---

## Rollback Plan

If issues discovered:
1. Migration unassigns existing receipts - safe, user can reassign
2. New columns are nullable - old code can ignore them
3. Frontend can fall back to old UI if backend returns old format

---

## Definition of Done

- [ ] All phases implemented
- [ ] All backend tests pass (`npm run test:backend`)
- [ ] All integration tests pass (`npm run test:integration`)
- [ ] Design doc scenarios (A1-E6) verified
- [ ] No regressions in existing functionality
- [ ] Code reviewed
- [ ] CHANGELOG updated
