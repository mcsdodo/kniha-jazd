# Implementation Plan: Receipt-Trip State Model

**Date:** 2026-02-02
**Status:** Ready for Implementation
**Design:** `_TECH_DEBT/05-receipt-trip-state-model-design.md`

---

## Overview

5 phases, each independently testable:
1. Database migration (add fields)
2. Backend logic (assignment with type)
3. Backend verification (new state calculation)
4. Frontend - Invoice grid
5. Frontend - Trip grid

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
    pub assignment_type: Option<AssignmentType>,
    pub mismatch_override: bool,
}
```

### 1.4 Update TypeScript types
- File: `src/lib/types.ts`
- Add corresponding fields

**Tests:** Verify migration runs, fields exist in DB

---

## Phase 2: Backend Assignment Logic

**Goal:** User picks FUEL or OTHER when assigning

### 2.1 Update `assign_receipt_to_trip_internal()`
- File: `src-tauri/src/commands/receipts_cmd.rs`
- Add `assignment_type: AssignmentType` parameter
- Remove auto-detection logic
- Set `receipt.assignment_type` on assignment

### 2.2 Add mismatch detection
- When assigning as FUEL, check if data matches
- If mismatch and no override → set warning state
- If mismatch and override → set `mismatch_override = true`

### 2.3 Update Tauri command signature
- File: `src-tauri/src/lib.rs`
- Update command registration

### 2.4 Write backend tests
- File: `src-tauri/src/commands/commands_tests.rs`
- Test: assign as FUEL populates trip (C1)
- Test: assign as OTHER populates trip (C2)
- Test: assign with matching data (C3)
- Test: assign with mismatch shows warning (C4)
- Test: assign with override (C5)

**Tests:** All C1-C7 scenarios pass

---

## Phase 3: Backend Verification Logic

**Goal:** Unified state calculation for both grids

### 3.1 Create new `ReceiptState` enum
- File: `src-tauri/src/models.rs`

```rust
pub enum ReceiptState {
    Processing,
    NeedsReview,
    Unassigned,
    Assigned { trip_id: Uuid, assignment_type: AssignmentType },
    AssignedWithMismatch { trip_id: Uuid, mismatches: Vec<Mismatch> },
    AssignedWithOverride { trip_id: Uuid },
}
```

### 3.2 Create `calculate_receipt_state()` function
- File: `src-tauri/src/commands/receipts_cmd.rs`
- Single source of truth for state calculation
- Used by both verify_receipts and trip grid

### 3.3 Update `verify_receipts()`
- Return `ReceiptState` instead of old `ReceiptVerification`
- Remove `matched`, `datetimeWarning` fields

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
- Pass `assignmentType` to backend
- Show mismatch warning dialog when data differs
- Add "Priradiť s varovaním" / "Priradiť a potvrdiť" buttons

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

**Tests:** Integration test for assignment flow with type selection

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

**Tests:** Integration test for trip grid indicators

---

## Phase 6: Cleanup & Migration

### 6.1 Migrate existing data
- Existing `trip_id` assignments: set `assignment_type` based on context
- If receipt has liters → Fuel, else → Other
- Existing `status = 'Assigned'` → keep, determine type

### 6.2 Remove deprecated code
- Remove old `matched` field usage
- Remove old `datetimeWarning` field usage
- Remove auto-detection in `is_fuel_receipt` check

### 6.3 Update tests
- Remove tests for old behavior
- Ensure all new scenarios covered

---

## Testing Checklist

### Backend Unit Tests
- [ ] C1: Assign to empty trip as FUEL → populates fuel fields
- [ ] C2: Assign to empty trip as OTHER → populates other fields
- [ ] C3: Assign to trip with matching fuel → links only
- [ ] C4: Assign to trip with mismatched fuel → shows warning
- [ ] C5: Override mismatch → warning suppressed
- [ ] C6: Trip already has other costs → block/warn
- [ ] C7: Reassign invoice to different trip

### Integration Tests
- [ ] Assignment dialog shows FUEL/OTHER selector
- [ ] Mismatch warning dialog appears
- [ ] Override button works
- [ ] Trip grid shows correct triangles
- [ ] Hover tooltips show details

---

## Estimated Effort

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| Phase 1: Migration | 1h | None |
| Phase 2: Backend assignment | 3h | Phase 1 |
| Phase 3: Backend verification | 2h | Phase 2 |
| Phase 4: Frontend invoice | 4h | Phase 3 |
| Phase 5: Frontend trip grid | 2h | Phase 3 |
| Phase 6: Cleanup | 2h | Phase 4, 5 |
| **Total** | **~14h** | |

---

## Rollback Plan

If issues discovered:
1. Migration is additive (new columns), can be ignored
2. Backend can fall back to old logic if `assignment_type` is NULL
3. Frontend can show old UI if backend returns old format

---

## Definition of Done

- [ ] All phases implemented
- [ ] All backend tests pass (`npm run test:backend`)
- [ ] All integration tests pass (`npm run test:integration`)
- [ ] Design doc scenarios (A1-E6) verified
- [ ] No regressions in existing functionality
- [ ] Code reviewed
- [ ] CHANGELOG updated
