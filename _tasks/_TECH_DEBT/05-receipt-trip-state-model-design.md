# Receipt-Trip State Model Redesign

**Date:** 2026-02-01
**Status:** Draft v3
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
- Edge cases (toll bought day before, fill-up after trip) are handled gracefully
- The system is **simple to understand** without reading documentation

---

## Use Cases to Support

| Scenario | Expected Behavior |
|----------|-------------------|
| Fuel receipt matches trip exactly | ✅ All good, no action needed |
| Receipt bought 30min after trip ended | ✅⚠ Attached but timing noted |
| Toll bought day before trip | Manual attach → acknowledged, no warnings |
| OCR couldn't read receipt | Needs review → user edits data |
| Trip has fuel but no receipt scanned | ❌ Missing receipt warning |
| Receipt for trip in different year | Show in both years? Or year of receipt? |

---

## Current System Problems

### Problem 1: "Verified" ≠ "Attached"

```
┌─────────────────────────────────────────────────────────────┐
│  Current concept: "Verified" = Receipt + Trip have          │
│                   matching date + liters + price            │
│                                                             │
│  User thinks:     "Verified" = I have a receipt for this    │
│                   trip and it's linked                      │
└─────────────────────────────────────────────────────────────┘
```

A manually attached receipt with slightly different data shows as "Unverified" even though the user explicitly linked them.

### Problem 2: Same Icon, Different Meanings

In TripGrid:
```
  Trip A: 45.2 L ⚠    ← "bez dokladu" (no receipt at all)
  Trip B: 42.0 L ⚠    ← "dátum/čas mimo" (has receipt, timing off)
```

User sees two identical warnings but they require **completely different actions**:
- Trip A: Find/scan the receipt
- Trip B: Maybe nothing, it's just a timing note

### Problem 3: Two Sources of Truth

| Location | What it shows | How calculated |
|----------|---------------|----------------|
| Doklady page | Receipt verification | `verify_receipts()` in receipts_cmd.rs |
| Trip grid | Missing receipts | `calculate_missing_receipts()` in statistics.rs |

These use **similar but not identical** logic, leading to inconsistent results.

### Problem 4: Technical Mismatch Reasons

Current UI shows messages like:
- "DatetimeOutOfRange" vs "DateMismatch"
- "NoFuelTripFound" vs "NoOtherCostMatch"

Users don't care about the internal matching algorithm. They want actionable information.

---

## Proposed Design

### Core Concepts (Only 2!)

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  1. ATTACHMENT                                              │
│     ├── Unattached: receipt.trip_id = null                  │
│     └── Attached:   receipt.trip_id = <uuid>                │
│                                                             │
│  2. DATA QUALITY (only for attached receipts)               │
│     ├── Perfect:    All data matches exactly                │
│     ├── Noted:      Minor discrepancy (timing, rounding)    │
│     └── Overridden: User acknowledged mismatch              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Receipt Lifecycle

```
     ┌─────────────┐
     │   PENDING   │ ← File detected, OCR not run
     └──────┬──────┘
            │ OCR processing
            ▼
   ┌────────┴────────┐
   │                 │
   ▼                 ▼
┌──────────┐   ┌──────────────┐
│  READY   │   │ NEEDS_REVIEW │ ← Low confidence
│          │   │              │
└────┬─────┘   └──────┬───────┘
     │                │ user edits
     │◄───────────────┘
     │
     ├─────────────────────────────────────┐
     │                                     │
     ▼                                     ▼
┌──────────────┐                  ┌──────────────────┐
│  UNATTACHED  │ ──────────────►  │     ATTACHED     │
│              │  user attaches   │                  │
│  Actions:    │                  │  Quality:        │
│  - Attach    │  ◄──────────────  │  - Perfect  ✅   │
│  - Delete    │  user detaches   │  - Noted    ✅ℹ  │
└──────────────┘                  │  - Override ✅✓  │
                                  └──────────────────┘
```

### Attachment Quality Levels

| Quality | When | Visual | Meaning |
|---------|------|--------|---------|
| **Perfect** | Date, liters, price all match within trip time range | ✅ | All good |
| **Noted** | Same day, but time outside trip range | ✅ℹ | Attached, note shown |
| **Override** | Different day (user explicitly acknowledged) | ✅✓ | User says it's correct |

**Timing rule**: Same day = auto-noted, different day = requires explicit acknowledgment.

**Key**: All three are valid attachments. The system trusts the user's decision.

### Trip Receipt Status

From trip perspective, simplified to 3 states:

| Status | Visual | Meaning |
|--------|--------|---------|
| **Has receipt** | ✅ | Attached receipt (any quality) |
| **Missing** | ❌ | Trip has expense, no receipt |
| **N/A** | - | Trip has no expense |

That's it. No separate "datetime warning" in trip grid. The quality details live on the receipt.

---

## Visual Design

### Doklady Page - Grouped by Action Needed

```
┌─────────────────────────────────────────────────────────────┐
│                     DOKLADY (2026)                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ ▼ Potrebuje pozornosť (3)                                   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🔴 NESPÁROVANÝ                     [Priradiť]       │   │
│   │    fuel-jan15.jpg                                   │   │
│   │    📅 15.1. 14:30  •  ⛽ 45.2 L  •  65.80 €         │   │
│   │    ─────────────────────────────────────────────    │   │
│   │    💡 Možná jazda: 15.1. BA→KE (13:00-17:00)       │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟡 SKONTROLOVAŤ                    [Upraviť]        │   │
│   │    receipt-blurry.jpg                               │   │
│   │    📅 ?.1. ?:??  •  ⛽ ??.? L  •  ?? €              │   │
│   │    ─────────────────────────────────────────────    │   │
│   │    ⚠ Niektoré údaje nemožno prečítať               │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟡 SKONTROLOVAŤ                    [Upraviť]        │   │
│   │    toll-receipt.jpg                                 │   │
│   │    📅 14.1. 18:00  •  📄 10.00 €                    │   │
│   │    ─────────────────────────────────────────────    │   │
│   │    ⚠ Nebola nájdená jazda s rovnakou cenou         │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│ ▼ Spárované (12)                                            │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ ✅ SPÁROVANÝ                                        │   │
│   │    fuel-jan10.jpg                                   │   │
│   │    📅 10.1. 09:15  •  ⛽ 42.0 L  •  60.50 €         │   │
│   │    🚗 10.1. BA→KE (08:00-12:00)                    │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ ✅ℹ SPÁROVANÝ                                       │   │
│   │    fuel-jan20.jpg                                   │   │
│   │    📅 20.1. 18:30  •  ⛽ 38.5 L  •  55.20 €         │   │
│   │    🚗 20.1. KE→PO (15:00-17:00)                    │   │
│   │    ℹ Tankovanie bolo po skončení jazdy             │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ ✅✓ SPÁROVANÝ (manuálne)                            │   │
│   │    toll-jan13.jpg                                   │   │
│   │    📅 13.1. 10:00  •  📄 10.00 €                    │   │
│   │    🚗 14.1. BA→ZA (06:00-09:00)                    │   │
│   │    ✓ Priradené užívateľom                          │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Trip Grid - Clean Receipt Column

```
┌───┬─────────┬────────────────┬──────┬─────────┬────────┬────────┐
│ # │  Dátum  │     Trasa      │  km  │ Palivo  │ Doklad │  Cena  │
├───┼─────────┼────────────────┼──────┼─────────┼────────┼────────┤
│ 1 │ 10.1.   │ BA → KE        │  400 │ 42.0 L  │   ✅   │ 60.50€ │
│ 2 │ 14.1.   │ BA → ZA        │  200 │  -      │   ✅✓  │ 10.00€ │ ← toll
│ 3 │ 15.1.   │ BA → KE        │  400 │ 45.2 L  │   ❌   │ 65.80€ │
│ 4 │ 20.1.   │ KE → PO        │   80 │ 38.5 L  │   ✅ℹ  │ 55.20€ │
│ 5 │ 20.1.   │ PO → KE        │   80 │  -      │   -    │   -    │
└───┴─────────┴────────────────┴──────┴─────────┴────────┴────────┘

Legenda: ✅ spárovaný │ ✅ℹ s poznámkou │ ✅✓ manuálne │ ❌ chýba │ - bez nákladu
```

### Attachment Dialog - Acknowledge Override

When user attaches receipt with mismatched data:

```
┌─────────────────────────────────────────────────────────────┐
│              Priradiť doklad k jazde                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Doklad:  14.1.2026 18:00  •  10.00 €                      │
│  Jazda:   15.1.2026 BA → ZA (06:00-09:00)                  │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ ⚠ Dátum dokladu sa líši od jazdy                      │  │
│  │                                                       │  │
│  │   Doklad: 14.1.2026                                   │  │
│  │   Jazda:  15.1.2026                                   │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  Toto je bežné napríklad pri:                               │
│  • Diaľničnej známke kúpenej deň vopred                    │
│  • Parkovaní zaplatenom večer pred odchodom                │
│                                                             │
│                        [Zrušiť]  [Priradiť aj tak]          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Model Changes

### Option A: Add `mismatch_acknowledged` field

```rust
pub struct Receipt {
    // ... existing fields ...

    /// True if user explicitly acknowledged data mismatch when attaching
    pub mismatch_acknowledged: bool,
}
```

**Pros**: Explicit, queryable
**Cons**: New field to maintain

### Option B: Derive from `status = Assigned`

Use existing `Assigned` status to mean "user explicitly attached":
- `Assigned` + data matches = shown as ✅
- `Assigned` + data mismatch = shown as ✅✓

**Pros**: No schema change
**Cons**: Overloads meaning of `Assigned`

### Recommendation: Option A

Clearer semantics. The `status` field tracks OCR pipeline, `mismatch_acknowledged` tracks user intent.

---

## Validation Logic (Single Source)

Currently there are two calculation paths. Consolidate to one:

```rust
/// Single source of truth for receipt-trip matching
pub struct ReceiptTripMatch {
    pub receipt_id: Uuid,
    pub trip_id: Option<Uuid>,      // None = unattached
    pub quality: MatchQuality,      // Perfect | Noted | Override | NotApplicable
    pub note: Option<String>,       // Human-readable note if quality != Perfect
}

pub enum MatchQuality {
    Perfect,        // All data matches
    Noted(Note),    // Minor issue, shown as info
    Override,       // User acknowledged mismatch
    NotApplicable,  // Not attached
}

pub enum Note {
    TimingOff { receipt_time: String, trip_range: String },
    PriceRounded { receipt: f64, trip: f64 },
}
```

Both doklady page and trip grid use the same calculation.

---

## Migration Plan

### Phase 1: Data Model (migration)
1. Add `mismatch_acknowledged BOOLEAN DEFAULT false` to receipts table
2. Set `mismatch_acknowledged = true` where `status = 'Assigned'` AND verification would fail

### Phase 2: Backend Logic
1. Create unified `ReceiptTripMatch` calculation
2. Replace `verify_receipts()` to use new logic
3. Replace `calculate_missing_receipts()` to use new logic
4. Update `calculate_receipt_datetime_warnings()` to use new logic

### Phase 3: Frontend - Doklady Page
1. Update receipt cards to show new visual states
2. Group receipts by "needs attention" vs "paired"
3. Update attachment dialog with mismatch acknowledgment

### Phase 4: Frontend - Trip Grid
1. Add dedicated "Doklad" column
2. Remove inline ⚠ indicators from fuel column
3. Update legend

### Phase 5: Cleanup
1. Remove redundant verification endpoints
2. Update tests

---

## Summary

| Aspect | Current | Proposed |
|--------|---------|----------|
| User questions | "Is it verified?" | "Is it attached? Is data OK?" |
| State dimensions | 7 | 2 (Attachment + Quality) |
| Calculation sources | 2 (receipts_cmd, statistics) | 1 (unified) |
| Missing receipt icon | ⚠ | ❌ |
| Timing note icon | ⚠ | ✅ℹ (info, not warning) |
| Manual override | Implicit (Assigned status) | Explicit (acknowledged flag) |
| Trip grid | Inline indicators | Dedicated column |

---

## Decisions Made

1. **Timing tolerance for "Noted" vs "Override"**: ✅ Decided
   - **Same day** = Noted (auto, no prompt)
   - **Different day** = Override (requires explicit acknowledgment)

2. **"Noted" in "needs attention"?**: ✅ Decided
   - **No** - only unattached and NeedsReview need attention
   - Noted is informational, not actionable

3. **Toggle override state**: ✅ Decided
   - Multi-state button instead of detach/re-attach
   - States: **Potvrdené** (confirmed) ↔ **Skontrolovať** (to review)
   - See "Override Toggle" section below

4. **Hover tooltips on trip grid**: ✅ Decided
   - **Yes** - show receipt filename, datetime on hover over status icon

## Override Toggle

For attached receipts with data mismatch (different day), user can toggle:

```
┌─────────────────────────────────────────────────────────────┐
│  OVERRIDE STATES                                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ✅✓ POTVRDENÉ (Confirmed)                                  │
│      User explicitly says "this is correct"                 │
│      → No warnings shown                                    │
│      → Button: [Skontrolovať]                               │
│                                                             │
│  ⚠ SKONTROLOVAŤ (To review)                                │
│      System flags mismatch for attention                    │
│      → Warning shown in "needs attention"                   │
│      → Button: [Potvrdiť]                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**UI in receipt card:**

```
┌─────────────────────────────────────────────────────────────┐
│ ✅✓ SPÁROVANÝ                              [Skontrolovať]   │
│    toll-jan13.jpg                                           │
│    📅 13.1. 10:00  •  📄 10.00 €                           │
│    🚗 14.1. BA→ZA (06:00-09:00)                            │
│    ✓ Priradené užívateľom                                  │
└─────────────────────────────────────────────────────────────┘

        ↓ user clicks [Skontrolovať] ↓

┌─────────────────────────────────────────────────────────────┐
│ ⚠ SPÁROVANÝ - skontrolovať                    [Potvrdiť]   │
│    toll-jan13.jpg                                           │
│    📅 13.1. 10:00  •  📄 10.00 €                           │
│    🚗 14.1. BA→ZA (06:00-09:00)                            │
│    ⚠ Dátum dokladu (13.1.) ≠ dátum jazdy (14.1.)          │
└─────────────────────────────────────────────────────────────┘
```

## Open Questions

*(All major questions resolved)*

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| v1 | 2026-02-01 | Initial draft |
| v2 | 2026-02-01 | Added edge cases, migration path, state diagram |
| v3 | 2026-02-01 | Refocused on user mental model, simplified to 2 concepts |
| v3.1 | 2026-02-01 | Decision: same day = Noted, different day = Override |
| v4 | 2026-02-01 | Decisions: Noted not in "needs attention", toggle button for override, hover tooltips |
