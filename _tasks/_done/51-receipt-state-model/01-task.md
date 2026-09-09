# Task: Receipt-Trip State Model Redesign

**Date:** 2026-02-02
**Status:** Complete (shipped in 0.29.0, 2026-02-04)
**Source:** `_TECH_DEBT/05-receipt-trip-state-model.md`, `_TECH_DEBT/05-receipt-trip-state-model-design.md`

---

## Summary

Simplify the receipt-trip relationship model from 7 confusing dimensions to a clear explicit assignment system.

## Problem

Current system has:
- 7 overlapping state dimensions (ReceiptStatus, trip_id, matched, mismatchReason, datetimeWarning, missingReceipts, receiptDatetimeWarnings)
- "Verified" ≠ "Attached" confusion
- Same ⚠ icon with different meanings
- Two sources of truth (verify_receipts vs calculate_missing_receipts)
- Auto-detection of fuel vs other (magic behavior)

## Solution

Explicit assignment model:
- User explicitly assigns invoice to trip
- User picks type: FUEL or OTHER COST
- `trip_id` = NULL means unassigned, SET means assigned
- Data mismatch shows warning, user can override

## Requirements

### Functional
1. Invoice must be explicitly assigned to trip (no auto-matching)
2. User selects assignment type: FUEL or OTHER
3. Show warning when data mismatches (time/liters/price)
4. User can override mismatch warning
5. Trip grid shows inline warning triangles (not separate column)
6. Invoice grid groups by: Unassigned → Assigned

### Visual States
- 🔴⚠ Missing invoice (trip has costs, no invoice)
- 🟡⚠ Data mismatch (assigned but data differs)
- 🟠⚠ User override (mismatch confirmed by user)
- (none) All good (assigned, data matches)

## Out of Scope
- Auto-matching based on data
- Multiple invoices per trip for same cost type
- Batch assignment

## Acceptance Criteria

- [x] User can assign invoice as FUEL or OTHER
- [x] Assignment populates trip data if empty (C1, C2)
- [x] Mismatch warning shows on both grids
- [x] Override suppresses warning
- [x] Trip grid shows inline triangles (not column)
- [x] All scenarios from design doc work (A1-E6)
- [x] Backend tests cover assignment logic
- [x] Integration tests cover UI flow

## Outcome

The task shipped in **0.29.0 on 2026-02-04**. The folder kept the `Planning`
header until 2026-09-09, so read the header of any `_tasks/` file with care.

**Evidence:**

| What | Where |
|---|---|
| The two new columns | [migration 2026-02-03-100000_receipt_assignment_type](../../../src-tauri/core/migrations/2026-02-03-100000_receipt_assignment_type/up.sql) |
| The legacy `Assigned` status removal | [migration 2026-02-04-100000_remove_assigned_status](../../../src-tauri/core/migrations/2026-02-04-100000_remove_assigned_status/up.sql) |
| `assigned` is now only `trip_id.is_some()` | [receipts_cmd.rs](../../../src-tauri/core/src/commands_internal/receipts_cmd.rs), `verify_receipts_with_data` |
| The C1-C7 backend tests | [commands_tests.rs](../../../src-tauri/core/src/commands_internal/commands_tests.rs), block "Task 51: Explicit assignment type tests" |
| The picker UI flow, end to end | [multi-invoice.spec.ts](../../../tests/integration/specs/tier2/multi-invoice.spec.ts) drives `input[name="assignmentType"]` |
| The user-visible list | [CHANGELOG.md](../../../CHANGELOG.md), section `[0.29.0] - 2026-02-04` |
| The override semantics | [ADR-021](../../../DECISIONS.md#adr-021-mismatch_override-is-receipt-only-paperless-path-accepts-and-ignores) |

**Three things differ from the plan above:**

1. **C6 was reversed later.** The plan blocked a second OTHER invoice on a trip
   that already had other costs. A later change allowed it, and
   [task 66](../66-multi-invoice/) made 1 FUEL + N OTHER per trip the model. The
   test is now `test_assign_other_to_trip_with_existing_other_costs_allowed`.
2. **`ReceiptDisplayState` never reached Rust.** Design spec v7 kept the flatter
   `ReceiptVerification` shape instead. The TypeScript type in
   [src/lib/types.ts](../../../src/lib/types.ts) has no producer and no consumer.
3. **Phase 6 cleanup is open.** `models::MismatchReason` has no non-`None`
   producer -- `verify_receipts_with_data` hardcodes `MismatchReason::None` --
   and `ReceiptVerification.matched` only repeats `trip_id.is_some()`. The
   frontend `ReceiptMismatchReason` is fed by that dead enum. (The other
   `MismatchReason` in `src/lib/types.ts` is a different, live type: it drives
   the match indicator in `TripSelectorModal.svelte`.)

## References

- Design document: [../../_TECH_DEBT/05-receipt-trip-state-model-design.md](../../_TECH_DEBT/05-receipt-trip-state-model-design.md)
- Implementation: [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../../src-tauri/core/src/commands_internal/receipts_cmd.rs) (the path moved twice since: `src/commands/` -> `core/src/commands_internal/`)
