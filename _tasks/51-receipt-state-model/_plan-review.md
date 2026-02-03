# Plan Review: Receipt-Trip State Model Redesign

**Date:** 2026-02-03
**Reviewer:** Claude (Haiku)
**Status:** Needs Revisions
**Summary:** 2 Critical, 3 Important, 2 Minor findings

---

## Findings

### CRITICAL

#### C1: Phase 3 Enum Definition Conflicts with Current Codebase
**Location:** Phase 3, Section 3.1
**Issue:** Plan proposes new `ReceiptState` enum with complex variants:
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

**Problem:**
- Current code uses `ReceiptStatus` enum (Pending, Parsed, NeedsReview, Assigned) defined in `models.rs:411`
- `ReceiptStatus::Processing` doesn't exist; current code uses `ReceiptStatus::Pending`
- Variants contain complex data (Trip, Vec<Mismatch>) unsuitable for DB persistence
- Plan doesn't specify how this enum maps to DB storage
- Existing `ReceiptVerification` struct (line 701) already contains matching/mismatch logic

**Resolution Needed:**
1. Clarify if `ReceiptState` is:
   - A transient type returned by `verify_receipts()` (not stored in DB)?
   - Or a replacement for `ReceiptStatus` (stored in DB)?
2. If transient: update step 3.2 to clarify it's used for calculations only
3. If DB-backed: redesign to store only scalar values (use `trip_id` to fetch Trip data on demand)
4. Reconcile with existing `ReceiptVerification` struct

**Impact:** Implementation will fail or duplicate logic if not clarified

---

#### C2: AssignmentType Storage in DB Not Specified
**Location:** Phase 1, Section 1.3
**Issue:** Plan shows:
```rust
pub assignment_type: Option<AssignmentType>,
```
Where `AssignmentType` is an enum (Fuel, Other).

**Problem:**
- How is this stored in SQLite? TEXT column with enum serialization?
- Migration (Section 1.1) shows:
  ```sql
  ALTER TABLE receipts ADD COLUMN assignment_type TEXT;
  ```
- But no serialization pattern specified (e.g., "Fuel" vs "FUEL" vs 1/0?)
- Need to match Diesel ORM expectations

**Resolution Needed:**
1. Specify exact string values for enum variants ("Fuel", "Other", or alternatives)
2. Confirm Diesel `#[derive]` attributes needed (likely `#[derive(DbEnum)]` or custom SQL type)
3. Add serialization example to migration section
4. Check existing enum serialization patterns in codebase (e.g., `ConfidenceLevel` at `models.rs:426` uses default serde)

**Impact:** Migration runs, but ORM deserialization will fail at runtime

---

### IMPORTANT

#### I1: Phase 2 Requires Specification of Mismatch Behavior
**Location:** Phase 2, Section 2.2 & Design Doc Section C6
**Issue:** Open question in design doc (line 362):
> "Block or warn when trip already has other costs?"

Plan says "Block" but doesn't clarify interaction with explicit type selection:

**Problem:**
- User selects "Assign as OTHER" on a trip that already has other_costs
- Should this be rejected? Allowed to replace? Warn with override?
- Current implementation blocks (line 436): `return Err("Jazda už má iné náklady")`
- Plan doesn't state if this behavior changes or stays

**Resolution Needed:**
1. Explicitly state: "Block (current behavior) / Allow with replace / Warn with override"
2. If changing: add test case to Phase 2.4
3. If keeping: explicitly document in acceptance criteria

**Related:** C6 scenario in design doc needs explicit answer

**Impact:** User flow in Phase 4 (invoice grid) undefined; integration tests incomplete

---

#### I2: Phase 6 Migration Strategy Too Vague
**Location:** Phase 6, Section 6.1
**Issue:** Migration of existing data states:
```
If receipt has liters → Fuel, else → Other
```

**Problem:**
- Current `trip_id` assignments may have mixed history
- No SQL script provided to execute this migration
- No rollback plan if migration categorizes incorrectly
- What if receipt has NULL liters but user clearly assigned as fuel? (manual assignment today)

**Resolution Needed:**
1. Write actual SQL migration script for Phase 6.1 (INSERT/UPDATE with logic)
2. Specify source-of-truth: if `trip_id != NULL` + has liters → set FUEL
3. Add data validation step: count of auto-categorized receipts, flag outliers
4. Document rollback: can `assignment_type` be reset to NULL safely?

**Impact:** Data corruption risk; no way to verify migration correctness

---

#### I3: Frontend Changes in Phase 4 Incomplete
**Location:** Phase 4, Section 4.1-4.2
**Issue:** Plan mentions UI elements but doesn't specify:

**Problem:**
- "Show mismatch warning dialog when data differs" - dialog design not specified
- Three buttons mentioned: "Priradiť s varovaním" / "Priradiť a potvrdiť" - but backend logic to handle both states not in Phase 2
- Plan calls them "warning" and "confirm" but backend only has binary `mismatch_override` flag
- How does UI distinguish between "assign with warning shown" vs "assign and suppress warning"?

**Resolution Needed:**
1. Clarify backend behavior: Does `mismatch_override=true` mean "suppress UI warning" or "user confirmed mismatch"? (Design suggests both)
2. Specify UI flow:
   - User sees mismatch → two options: (a) Assign+Show Warning, (b) Assign+Suppress?
   - Or: Cancel, Assign (warning), Assign and Confirm (no warning)?
3. Add state diagram: receipt states + user actions + resulting DB values

**Impact:** Integration tests in Phase 4 unclear; backend tests may not cover all paths

---

### MINOR

#### M1: Test Names Don't Match Scenario Labels
**Location:** Phase 2.4 & Phase 3.5
**Issue:** Tests reference C1-C7 scenarios but don't align:
- C1/C2 appear in Phase 2.4 (assignment tests)
- C1/C2 also appear in Phase 3 design (data population scenarios)
- Same label, different meaning?

**Resolution Needed:** Rename tests for clarity (e.g., `test_assign_fuel_empty_trip` instead of `test_c1`)

**Impact:** Low - maintainability only

---

#### M2: Integration Test Tier 1 Criteria Not Specified
**Location:** Testing Checklist, Integration Tests
**Issue:** Plan lists 5 integration tests but doesn't specify:
- Which are Tier 1 (fast PR checks)?
- Which require full debug build?
- Execution time estimates

**Resolution Needed:** Mark each integration test with `[Tier 1]` or `[Tier 2]` label

**Impact:** Low - affects CI/CD optimization only

---

## Acceptance Checklist

- [ ] C1: Clarify ReceiptState enum purpose (transient vs DB-backed)
- [ ] C2: Specify AssignmentType serialization to SQLite
- [ ] I1: Decide behavior for "trip already has other_costs" scenario
- [ ] I2: Write actual SQL migration script for Phase 6.1
- [ ] I3: Document backend state machine for mismatch_override behavior
- [ ] M1: Rename tests for clarity (optional but recommended)
- [ ] M2: Tier tests 1 vs 2 (optional, affects CI only)

---

## Recommendation

**Status: NEEDS REVISIONS**

Plan has solid structure and correctly identifies phases, but:
1. **C1 & C2 must be resolved** - implementation will fail without clear enum design
2. **I1 & I2 should be resolved** - affects data correctness and testability
3. **I3 should be resolved** - affects integration test implementation

Estimated effort to fix: 2-3 hours planning + clarification.
Then proceed with implementation as structured.

---

## Quick Wins (Can Start Immediately)

These don't block critical path:
1. Phase 1: Migration SQL is clear (adding two TEXT columns with defaults)
2. Phase 5: Trip grid UI changes are well-specified
3. Backend unit tests (Phase 2.4) can be written once C1/C2 resolved

---

## Notes for Implementation

**Strengths:**
- Clear phase decomposition (migration → logic → state → UI)
- Good test-first approach with explicit scenarios (A1-E6)
- Comprehensive design doc with visual mockups
- Backward compatibility via nullable fields

**Watch-outs:**
- `trip_id` is UNIQUE when not null (schema constraint needed)
- Existing code does auto-detection (lines 394-417); explicit type will be breaking change for any API clients
- ReceiptVerification struct duplication risk
