# Task: Receipt-Trip State Model Redesign

**Date:** 2026-02-02
**Status:** Planning
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

- [ ] User can assign invoice as FUEL or OTHER
- [ ] Assignment populates trip data if empty (C1, C2)
- [ ] Mismatch warning shows on both grids
- [ ] Override suppresses warning
- [ ] Trip grid shows inline triangles (not column)
- [ ] All scenarios from design doc work (A1-E6)
- [ ] Backend tests cover assignment logic
- [ ] Integration tests cover UI flow

## References

- Design document: `_TECH_DEBT/05-receipt-trip-state-model-design.md`
- Current implementation: `src-tauri/src/commands/receipts_cmd.rs`
