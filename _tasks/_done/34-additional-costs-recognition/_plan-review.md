# Plan Review: Additional Costs Invoice Recognition

**Target:** `_tasks/34-additional-costs-recognition/03-plan.md`
**Date:** 2026-01-12
**Reviewer:** Independent assessment
**Status:** Complete

---

## Review Summary

**Verdict:** READY FOR IMPLEMENTATION with 1 Important finding

| Category | Count | Status |
|----------|-------|--------|
| Critical | 0 | - |
| Important | 1 | Needs attention |
| Minor | 3 | Can fix during implementation |

---

## Iteration 1: Initial Review

### Critical Findings

None. The plan is well-structured and addresses all open questions from the task definition.

### Important Findings

#### 1. `assign_receipt_to_trip` command does not have access to `trip_id` for lookup

**Issue:** The plan (Step 1.6) shows code that calls `db.get_trip(&trip_id)` but the current `assign_receipt_to_trip` function signature (commands.rs:1989-2008) receives `trip_id` as a String and does not perform any trip lookup - it only updates the receipt.

```rust
// Current implementation (commands.rs:1989-2008)
pub fn assign_receipt_to_trip(
    db: State<Database>,
    receipt_id: String,
    trip_id: String,
    vehicle_id: String,
) -> Result<Receipt, String> {
    // Only receipt update - no trip lookup or update
    receipt.trip_id = Some(Uuid::parse_str(&trip_id)?);
    receipt.status = ReceiptStatus::Assigned;
    db.update_receipt(receipt)?;
    Ok(receipt.clone())
}
```

**Impact:** The plan assumes we can call `db.get_trip()` and `db.update_trip()` inside this function, which is correct since `db.get_trip()` exists (db.rs:220-229). However, the multi-stage matching logic requires:
1. Trip lookup (exists)
2. Trip update with `other_costs_eur` and `other_costs_note` (need to verify `update_trip` exists)

**Verification needed:** Confirm `db.update_trip()` function exists and accepts the modified trip.

**Resolution:** Checked db.rs - `update_trip()` function exists (line 169-214). The plan is implementable.

### Minor Findings

#### 1. Missing `From<ReceiptRow> for Receipt` update in plan

**Issue:** Step 1.3 mentions updating the `From<ReceiptRow> for Receipt` implementation but does not show the code changes needed to map `vendor_name` and `cost_description` from the row to the domain model.

**Impact:** Low - implementer should know to update this.

**Resolution:** Add a note or the implementation will naturally require it.

#### 2. `db.rs` update details incomplete

**Issue:** Step 1.4 says "Update `create_receipt()` and `update_receipt()` to handle new fields" but doesn't specify that:
- `NewReceiptRow` needs new fields (mentioned in Step 1.3)
- The actual `create_receipt()` and `update_receipt()` functions need to map these fields

Looking at current implementation (db.rs:649-750), both functions use `NewReceiptRow` struct and manual field mapping. The new fields will need to be added to both the struct AND the function bodies.

**Impact:** Low - straightforward addition following existing patterns.

#### 3. Integration test tier not specified

**Issue:** Step 3.1 mentions `tests/integration/receipts.spec.ts` but this file is actually at `tests/integration/specs/tier2/receipts.spec.ts`. Also, the plan doesn't specify which tier the new tests should be in.

**Impact:** Low - tests can be added to Tier 2 (where receipts tests already exist).

---

## Iteration 2: Verify No New Findings

After reviewing:
- Current `assign_receipt_to_trip` implementation
- `db.rs` CRUD functions for receipts and trips
- TypeScript types
- Gemini prompt structure
- Integration test structure

No additional findings. The plan addresses all requirements from `01-task.md` and `02-design.md`.

---

## Completeness Checklist

| Requirement | Addressed | Notes |
|-------------|-----------|-------|
| Folder-based input | Yes | Same folder, AI distinguishes |
| Automatic recognition | Yes | Binary: `liters != null` |
| Extract key fields | Yes | vendor_name, cost_description added |
| Assignment to trips | Yes | Multi-stage matching logic |
| Collision handling | Yes | Block if trip has other_costs_eur |
| Management UI | Yes | Binary filter on Doklady page |
| Consistent UX | Yes | Same workflow as fuel |
| Distinguish from fuel | Yes | Icons and filter |

---

## Feasibility Assessment

| Aspect | Rating | Notes |
|--------|--------|-------|
| Backend changes | Feasible | Additive migration, clear file paths |
| Frontend changes | Feasible | Straightforward filter and icon addition |
| Testing | Feasible | Follows existing patterns |
| Migration risk | Low | Additive columns only, backward compatible |
| Gemini prompt | Medium | Prompt change needs real-world testing |

---

## Task Order Verification

| Step | Dependencies | Order Correct |
|------|--------------|---------------|
| 1.1 Migration | None | Yes |
| 1.2 Diesel schema | 1.1 (auto) | Yes |
| 1.3 Models | 1.2 | Yes |
| 1.4 db.rs | 1.3 | Yes |
| 1.5 Gemini | 1.3 | Yes |
| 1.6 Commands | 1.4, 1.5 | Yes |
| 1.7 Tests | 1.3-1.6 | Yes |
| 2.1 TS types | 1.3 | Yes |
| 2.2 Doklady page | 2.1 | Yes |
| 2.3 i18n | None | Yes |
| 3.1 Integration | 1.*, 2.* | Yes |
| 3.2 Changelog | All | Yes |

---

## Recommendations

1. **During implementation:** When updating `assign_receipt_to_trip`, ensure the trip lookup uses `db.get_trip()` which returns `QueryResult<Option<Trip>>` - handle the `None` case with appropriate error.

2. **Testing Gemini changes:** Test the updated prompt with real receipts (fuel and non-fuel) before merging to ensure AI correctly distinguishes receipt types.

3. **Integration tests:** Add tests to `tests/integration/specs/tier2/receipts.spec.ts` - this aligns with existing receipt test location.

---

## Final Verdict

**READY FOR IMPLEMENTATION**

The plan is complete, feasible, and well-structured. The Important finding (trip lookup) is already addressed by existing code. Minor findings can be resolved during implementation without plan changes.

Estimated effort: ~6 hours as planned.
