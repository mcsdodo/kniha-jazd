# Plan Review: Fuel Consumed Column

**Date:** 2025-01-26
**Reviewer:** Claude
**Rounds:** 2

---

## Critical Findings

### C1: Missing Preview Mode Support

**Issue:** The plan does not address live preview calculation for the new column.

**Evidence:** TripRow.svelte lines 328-344 show that `consumptionRate` and `fuelRemaining` have preview modes with `~` prefix and special styling when `previewData` is present. The new `fuelConsumed` column should also show a preview value during editing.

**Fix:** Add to Step 3.4:
- In edit mode, calculate preview fuel consumed: `previewData.fuelConsumed` or compute from `km * rate / 100`
- Show with `~` prefix and `.preview` class like other calculated fields

**Impact:** Without this, users won't see how fuel consumed changes as they edit km values.

---

## Important Findings

### I1: Column Width CSS Will Break on Insert

**Issue:** Plan says "adjust CSS to accommodate new 4% column" but CSS uses `nth-child` selectors. Inserting a column between "Cena EUR" (col 8) and "l/100km" (col 9) will shift all subsequent nth-child indices.

**Current widths (lines 691-703):**
```css
th:nth-child(8) { width: 4%; }   /* Cena EUR */
th:nth-child(9) { width: 4%; }   /* l/100km - currently here */
```

**After insert:**
```css
th:nth-child(8) { width: 4%; }   /* Cena EUR */
th:nth-child(9) { width: 4%; }   /* NEW: Spotr. (L) */
th:nth-child(10) { width: 4%; }  /* l/100km - shifted */
```

**Fix:** Step 3.1 should explicitly list:
1. Add new `th:nth-child(9)` with 4% width for "Spotr. (L)"
2. Renumber all subsequent columns (9->10, 10->11, 11->12, 12->13, 13->14)

**Note:** Current total is 96%, adding 4% brings it to 100%.

### I2: New Row Instances Missing `fuelConsumed` Prop

**Issue:** Plan mentions passing prop to TripRow but doesn't clarify which instances need it. There are 3 TripRow usages in TripGrid.svelte:
1. Line 488-507: New row at top (button click)
2. Line 513-532: New row inserted above existing trip
3. Line 560-591: Existing trip rows

**Fix:** Step 3.6 should specify all three instances need the new prop:
- New row at top: pass `fuelConsumed={0}` (new trip, no consumption yet)
- Insert-above row: pass `fuelConsumed={0}`
- Existing trips: pass `fuelConsumed={fuelConsumed.get(trip.id) || 0}`

### I3: First Record Row Missing New Column

**Issue:** Plan Step 3.5 says "Show '0.0' in the new column" but doesn't specify location.

**Location:** TripGrid.svelte lines 536-558, the synthetic "Prvy zaznam" row.

**Fix:** Step 3.5 should specify:
- File: `TripGrid.svelte`
- Location: After line 547 `<td class="number">{tpConsumption.toFixed(2)}</td>`
- Add: `<td class="number calculated">0.0</td>`

### I4: colspan Needs Update for Empty State

**Issue:** Line 597 uses dynamic colspan calculation: `colspan={9 + (showFuelColumns ? 4 : 0) + (showEnergyColumns ? 4 : 0)}`

**Fix:** Update to `colspan={9 + (showFuelColumns ? 5 : 0) + (showEnergyColumns ? 4 : 0)}` when adding fuel consumed column (4->5 for fuel columns).

---

## Minor Findings

### M1: Integration Test May Be YAGNI

**Issue:** Per CLAUDE.md guidelines, integration tests are for "UI correctly invokes backend and displays results" but fuel consumed is a simple calculated display with no new user interaction.

**CLAUDE.md excerpt:**
> "Do NOT write filler tests. No tests for: Trivial CRUD operations, UI rendering (unless behavior-critical)"

**Recommendation:** Consider if integration test adds value beyond backend unit tests. The column is purely display of pre-calculated value from `TripGridData.fuelConsumed` HashMap - if backend tests verify the calculation, integration test may be redundant.

**Decision:** Optional - if user interaction matters (e.g., column appears/hides correctly for different vehicle types), keep it. If just verifying display of a number, skip it.

### M2: Test File Location Unclear

**Issue:** Step 5.1 says "existing trip test file" without specifying which.

**Fix:** Specify exact path: `tests/integration/specs/trip-*.spec.ts` (find the appropriate file for trip display tests).

### M3: TripGridData Struct Location

**Issue:** Plan says struct is in `commands.rs` but it's actually in `models.rs` (line 305).

**Fix:** Step 1.3 should reference `src-tauri/src/models.rs` for the struct definition.

---

## Verification Checklist

- [x] Tasks have file paths (mostly - see I3, M2, M3)
- [x] Verification steps included (Step 6.1)
- [x] Correct task order (tests before implementation)
- [ ] No scope creep (integration test may be optional)

---

## Summary

| Severity | Count | Action Required |
|----------|-------|-----------------|
| Critical | 1 | Must fix: preview mode support |
| Important | 4 | Should fix: CSS renumbering, TripRow instances, first record, colspan |
| Minor | 3 | Nice to fix: test YAGNI, file paths, struct location |

**Recommendation:** Address Critical and Important findings before implementation. Minor findings can be addressed during implementation or skipped.

---

## Resolution

**Date:** 2025-01-26

All Critical and Important findings addressed in updated `03-plan.md`:

- [x] C1: Preview mode support added (Step 3.4)
- [x] I1: CSS nth-child renumbering explicit (Step 3.1)
- [x] I2: All 3 TripRow instances specified (Step 3.6)
- [x] I3: First record row location specified (Step 3.5)
- [x] I4: Colspan update added (Step 3.7)
- [x] M3: TripGridData struct location corrected (Step 1.3)
- [~] M1: Integration test removed (YAGNI per CLAUDE.md)
- [~] M2: Skipped (no integration test needed)
