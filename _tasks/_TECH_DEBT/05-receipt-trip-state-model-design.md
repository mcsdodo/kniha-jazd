# Receipt-Trip State Model Redesign

**Date:** 2026-02-02
**Status:** Draft v6
**Related:** `05-receipt-trip-state-model.md`

---

## User's Mental Model

From the user's perspective, there are only **two questions**:

1. **"Do I have a receipt for this expense?"** (compliance)
2. **"Is the receipt data correct?"** (accuracy)

The current system conflates these, making it hard to answer either clearly.

---

## Goal

Design a state model where:
- User can immediately see if receipts are complete (no missing)
- User can quickly identify issues that need attention
- Edge cases (toll bought day before) are handled gracefully
- The system is **simple to understand** without reading documentation

---

## The 3-State Model

```
┌─────────────────────────────────────────────────────────────┐
│  🟢 GREEN - Auto-matched (spárovaný/auto-matched)          │
│     Receipt matches trip by ALL criteria:                   │
│     - Same day                                              │
│     - Time within trip range                                │
│     - Liters match                                          │
│     - Price matches                                         │
│     → No trip_id needed (computed match)                    │
│     → No user action needed                                 │
├─────────────────────────────────────────────────────────────┤
│  🔴 RED - Problem to fix (problém/problem)                  │
│     2.1 Partial match - data doesn't align:                 │
│         - Same day but time OUTSIDE trip range → fix trip   │
│         - Same day but liters/price WRONG → fix data        │
│     2.2 Missing invoice:                                    │
│         - Trip has costs but no receipt found → upload      │
│     → No trip_id (nothing to link yet)                      │
│     → User must fix data or upload receipt                  │
├─────────────────────────────────────────────────────────────┤
│  🟠 ORANGE - Exception (výnimka/exception)                  │
│     Receipt intentionally doesn't match:                    │
│     - Different day (toll, parking bought ahead)            │
│     → trip_id IS set (manual assignment)                    │
│     → User explicitly confirmed the mismatch                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### 1. `trip_id` Only for Exceptions

| State | trip_id | Why |
|-------|---------|-----|
| 🟢 Green | **Not set** | System computes match on-the-fly |
| 🔴 Red | **Not set** | No valid match exists |
| 🟠 Orange | **Set** | Only way to link different-day receipt |

**Rationale**: If the system can compute a perfect match, why store it? Only store what the system can't determine automatically.

### 2. Same Day + Time Outside Range = RED (Not "Noted")

```
┌─────────────────────────────────────────────────────────────┐
│  SCENARIO                                                   │
│                                                             │
│  Trip: 15.1. 13:00-17:00 (BA → KE)                         │
│  Receipt: 15.1. 17:15 (gas station stop)                   │
│                                                             │
│  OLD thinking: "Time is slightly off, just note it" ✅ℹ    │
│                                                             │
│  NEW thinking: "If you stopped for gas, you hadn't         │
│  arrived yet. Trip end time is WRONG." 🔴                  │
│  → Fix trip end time to 17:30                              │
│  → Receipt now matches perfectly 🟢                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

This ensures data quality - timing mismatches on the same day indicate incorrect trip times.

### 3. No "Noted" State

The old "Noted" (✅ℹ) state is eliminated. If receipt is same day:
- Time within range + data matches → 🟢 Green
- Time outside range OR data wrong → 🔴 Red (fix it)

---

## Use Cases

| Scenario | State | Action |
|----------|-------|--------|
| Receipt matches trip perfectly | 🟢 Green | None |
| Same day, time outside trip range | 🔴 Red | Extend trip end time |
| Same day, liters don't match | 🔴 Red | Fix receipt or trip data |
| Trip has fuel, no receipt found | 🔴 Red | Scan/upload receipt |
| Toll bought day before trip | 🟠 Orange | Manual assignment |
| OCR couldn't read receipt | 🔴 Red | Edit receipt data |

---

## Current System Problems (Unchanged)

### Problem 1: "Verified" ≠ "Attached"

Current system has `matched` (computed) independent from `trip_id` (stored).
A receipt can be "verified" but unattached, or "assigned" but unverified.

### Problem 2: Same Icon, Different Meanings

In TripGrid, ⚠ means both "no receipt" and "datetime mismatch" - different problems requiring different actions.

### Problem 3: Two Sources of Truth

`verify_receipts()` in receipts_cmd.rs and `calculate_missing_receipts()` in statistics.rs use similar but not identical logic.

### Problem 4: Technical Mismatch Reasons

Users see "DatetimeOutOfRange" instead of actionable "Oprav čas jazdy".

---

## Visual Design

### Doklady Page - Grouped by State

```
┌─────────────────────────────────────────────────────────────┐
│                     DOKLADY (2026)                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ ▼ 🔴 Potrebuje pozornosť / Needs attention (3)              │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🔴 NESPÁROVANÝ (unmatched)              [Upraviť]   │   │
│   │    fuel-jan15.jpg                                   │   │
│   │    📅 15.1. 17:15  •  ⛽ 45.2 L  •  65.80 €         │   │
│   │    ─────────────────────────────────────────────    │   │
│   │    Možná jazda: 15.1. BA→KE (13:00-17:00)          │   │
│   │    ┌─────────────────────────────────────────────┐  │   │
│   │    │  ✓ Dátum súhlasí (date matches)             │  │   │
│   │    │  ✗ Čas mimo jazdy: 17:15 vs 13:00-17:00     │  │   │
│   │    │    (time outside trip range)                 │  │   │
│   │    │  ✓ Litre súhlasia (liters match)            │  │   │
│   │    │  ✓ Cena súhlasí (price matches)             │  │   │
│   │    └─────────────────────────────────────────────┘  │   │
│   │    💡 Oprav koniec jazdy na 17:30                   │   │
│   │       (Fix trip end time to 17:30)                  │   │
│   │    [Upraviť jazdu]  [Upraviť doklad]                │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🔴 SKONTROLOVAŤ (needs review)          [Upraviť]   │   │
│   │    receipt-blurry.jpg                               │   │
│   │    📅 ?.1. ?:??  •  ⛽ ??.? L  •  ?? €              │   │
│   │    ─────────────────────────────────────────────    │   │
│   │    ⚠ Niektoré údaje nemožno prečítať               │   │
│   │      (Some data couldn't be read)                   │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🔴 NESPÁROVANÝ (unmatched)      [Priradiť manuálne] │   │
│   │    toll-receipt.jpg                                 │   │
│   │    📅 14.1. 18:00  •  📄 10.00 €                    │   │
│   │    ─────────────────────────────────────────────    │   │
│   │    ⚠ Žiadna jazda s rovnakým dátumom               │   │
│   │      (No trip on same day)                          │   │
│   │    💡 Pre diaľničnú známku použite manuálne         │   │
│   │       priradenie (For toll, use manual assignment)  │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│ ▼ 🟢 Spárované / Matched (11)                               │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟢 SPÁROVANÝ (auto-matched)                         │   │
│   │    fuel-jan10.jpg                                   │   │
│   │    📅 10.1. 09:15  •  ⛽ 42.0 L  •  60.50 €         │   │
│   │    🚗 10.1. BA→KE (08:00-12:00)                    │   │
│   │    ✓ Všetky údaje súhlasia (all data matches)      │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│ ▼ 🟠 Výnimky / Exceptions (1)                               │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟠 PRIRADENÝ MANUÁLNE (manually assigned)           │   │
│   │    toll-jan13.jpg                       [Zrušiť]    │   │
│   │    📅 13.1. 10:00  •  📄 10.00 €                    │   │
│   │    🚗 14.1. BA→ZA (06:00-09:00)                    │   │
│   │    ✓ Potvrdené užívateľom (confirmed by user)      │   │
│   │      Iný dátum: 13.1. → 14.1. (different day)       │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Trip Grid - Receipt Status Column

```
┌───┬─────────┬────────────────┬──────┬─────────┬────────┬────────┐
│ # │  Dátum  │     Trasa      │  km  │ Palivo  │ Doklad │  Cena  │
├───┼─────────┼────────────────┼──────┼─────────┼────────┼────────┤
│ 1 │ 10.1.   │ BA → KE        │  400 │ 42.0 L  │   🟢   │ 60.50€ │
│ 2 │ 14.1.   │ BA → ZA        │  200 │  -      │   🟠   │ 10.00€ │ ← toll (manual)
│ 3 │ 15.1.   │ BA → KE        │  400 │ 45.2 L  │   🔴   │ 65.80€ │ ← time mismatch
│ 4 │ 16.1.   │ KE → PO        │   80 │ 38.5 L  │   🟢   │ 55.20€ │
│ 5 │ 16.1.   │ PO → KE        │   80 │  -      │   -    │   -    │
└───┴─────────┴────────────────┴──────┴─────────┴────────┴────────┘

Legenda: 🟢 spárovaný (matched) │ 🔴 problém (problem) │ 🟠 manuálne (manual) │ - bez nákladu (no cost)
```

### Hover Tooltip on Trip Grid

When hovering over receipt status icon:
- 🟢: "fuel-jan10.jpg • 10.1. 09:15"
- 🔴: "Čas mimo jazdy - oprav koniec jazdy" / "Time outside trip - fix trip end"
- 🟠: "toll-jan13.jpg • Manuálne priradené" / "Manually assigned"

---

## Manual Assignment Dialog

When user clicks [Priradiť manuálne] for a different-day receipt:

```
┌─────────────────────────────────────────────────────────────┐
│       Manuálne priradenie / Manual Assignment               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Doklad / Receipt:  13.1.2026 10:00  •  10.00 €            │
│  Jazda / Trip:      14.1.2026 BA → ZA (06:00-09:00)        │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ ⚠ Dátum dokladu sa líši od jazdy                      │  │
│  │   (Receipt date differs from trip)                    │  │
│  │                                                       │  │
│  │   Doklad / Receipt: 13.1.2026                         │  │
│  │   Jazda / Trip:     14.1.2026                         │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  Toto je bežné pri / This is common for:                   │
│  • Diaľničná známka kúpená vopred / Toll bought ahead     │
│  • Parkovanie zaplatené deň pred / Parking pre-paid       │
│                                                             │
│                     [Zrušiť]  [Priradiť / Assign]          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Model Changes

### Only Change: Track Manual Assignments

```rust
pub struct Receipt {
    // ... existing fields ...

    // trip_id is ONLY set for manual assignments (🟠 orange)
    // For auto-matched receipts (🟢 green), trip_id stays NULL
    pub trip_id: Option<Uuid>,
}
```

**No new fields needed!** The existing `trip_id` field now has clearer semantics:
- `trip_id = NULL` → Not manually assigned (could be auto-matched or unmatched)
- `trip_id = Some(uuid)` → Manually assigned to this trip (exception)

### Matching Logic

```rust
pub enum ReceiptState {
    /// 🟢 All criteria match - computed, no trip_id needed
    AutoMatched { trip: Trip },

    /// 🔴 Problem - needs user action
    Problem(ProblemKind),

    /// 🟠 Manually assigned exception - trip_id is set
    ManualException { trip: Trip },
}

pub enum ProblemKind {
    /// Same day, but time outside trip range
    TimeOutsideRange {
        receipt_time: String,
        trip_range: String,
        suggestion: String,  // "Oprav koniec jazdy na 17:30"
    },
    /// Same day, but liters don't match
    LitersMismatch { receipt: f64, trip: f64 },
    /// Same day, but price doesn't match
    PriceMismatch { receipt: f64, trip: f64 },
    /// No trip found on same day
    NoTripOnDay,
    /// OCR data incomplete
    IncompleteData,
    /// Trip has costs but no receipt
    MissingReceipt,
}
```

---

## Migration Plan

### Phase 1: Simplify Semantics
1. Document that `trip_id` = manual assignment only
2. Existing `trip_id` values for perfect matches can be cleared (optional)

### Phase 2: Backend Logic
1. Create unified `ReceiptState` calculation
2. Remove `ReceiptVerification.matched` - replaced by state enum
3. Remove `ReceiptVerification.datetimeWarning` - absorbed into `Problem`
4. Single source: both doklady and trip grid use same calculation

### Phase 3: Frontend - Doklady Page
1. Group by state: 🔴 Needs attention → 🟢 Matched → 🟠 Exceptions
2. Show progressive match details for problems
3. Add [Priradiť manuálne] button for different-day receipts

### Phase 4: Frontend - Trip Grid
1. Replace inline ⚠ with dedicated column showing 🟢/🔴/🟠/-
2. Add hover tooltips
3. Simplify legend

### Phase 5: Cleanup
1. Remove redundant verification fields
2. Update tests

---

## Summary: Old vs New

| Aspect | Old (v5) | New (v6) |
|--------|----------|----------|
| States | 4+ (Perfect, Noted, Override, Unmatched...) | **3** (Green, Red, Orange) |
| Same day + time off | ✅ℹ Noted (OK, just info) | 🔴 **Problem** (fix trip time) |
| `trip_id` for perfect match | Set by user | **Not set** (computed) |
| `trip_id` meaning | "User attached" | **"Manual exception"** |
| User action for perfect match | Click [Priradiť] | **None** |
| Tolerance for timing | Built-in (same day = OK) | **None** (must be in range) |

---

## Decisions Made

1. **3-State Model**: 🟢 Green (auto-matched), 🔴 Red (problem), 🟠 Orange (exception)

2. **trip_id only for exceptions**: Auto-matched receipts don't need stored link

3. **Same day + time outside = RED**: Trip time is wrong, fix it (not "noted")

4. **No "Noted" state**: Eliminated - either it matches or it's a problem

5. **Hover tooltips on trip grid**: Yes - show receipt filename and details

---

## Open Questions

1. **Should we clear existing trip_id for perfect matches during migration?**
   - Pro: Cleaner data model
   - Con: Loses historical "who attached this" info

2. **Progressive match details - how much to show?**
   - Current mockup shows all criteria (✓/✗)
   - Maybe collapse by default, expand on click?

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| v1 | 2026-02-01 | Initial draft |
| v2 | 2026-02-01 | Added edge cases, migration path |
| v3 | 2026-02-01 | Refocused on user mental model |
| v4 | 2026-02-01 | Added decisions: timing tolerance, toggle button |
| v5 | 2026-02-02 | Clarified auto-verified vs user-confirmed |
| v6 | 2026-02-02 | **Major rewrite**: 3-state model (Green/Red/Orange), removed "Noted" |
