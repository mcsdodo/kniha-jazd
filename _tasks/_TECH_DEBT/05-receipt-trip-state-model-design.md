# Receipt-Trip State Model Redesign

**Date:** 2026-02-02
**Status:** Draft v7
**Related:** `05-receipt-trip-state-model.md`

---

## Core Principle: No Magic

**Invoices must be explicitly assigned to trips by the user.**

- No auto-matching
- No computed relationships
- User picks: assign as FUEL or as OTHER COST
- `trip_id` is set when user assigns, NULL otherwise

---

## The Simple Model

```
┌─────────────────────────────────────────────────────────────┐
│  INVOICE STATE                                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  trip_id = NULL                                             │
│    → Nepriradený / Unassigned                               │
│    → Needs user action                                      │
│                                                             │
│  trip_id = SET                                              │
│    → Priradený / Assigned                                   │
│    → Linked to specific trip as FUEL or OTHER               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────┐
│  TRIP RECEIPT STATUS                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Trip has costs (fuel or other) + no invoice assigned       │
│    → Chýba doklad / Missing invoice                         │
│    → Warning shown                                          │
│                                                             │
│  Trip has costs + invoice assigned                          │
│    → Má doklad / Has invoice                                │
│    → May have data mismatch warning (optional)              │
│                                                             │
│  Trip has NO costs                                          │
│    → Bez nákladov / No costs                                │
│    → No invoice needed                                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## All Scenarios

### A. Invoice Scenarios

| # | Scenario | State | Visual | User Action |
|---|----------|-------|--------|-------------|
| A1 | Invoice scanned, OCR pending | Spracováva sa | 🔄 | Wait |
| A2 | Invoice scanned, OCR failed/low confidence | Skontrolovať | 🟡 | Edit data |
| A3 | Invoice ready, not assigned to any trip | Nepriradený | 🔴 | Assign to trip |
| A4 | Invoice assigned as FUEL, data matches | Priradený (palivo) | 🟢 | None |
| A5 | Invoice assigned as FUEL, data mismatch | Priradený (palivo) ⚠ | 🟢⚠ | Fix data or override |
| A6 | Invoice assigned as FUEL, mismatch + override | Priradený (palivo) ✓ | 🟠 | None |
| A7 | Invoice assigned as OTHER COST | Priradený (iné) | 🟢 | None |

### B. Trip Scenarios (from trip grid perspective)

| # | Scenario | State | Visual | User Action |
|---|----------|-------|--------|-------------|
| B1 | Trip with fuel, no invoice | Chýba doklad | 🔴 | Assign invoice |
| B2 | Trip with fuel, invoice assigned, matches | Má doklad | 🟢 | None |
| B3 | Trip with fuel, invoice assigned, mismatch | Má doklad ⚠ | 🟢⚠ | Fix data or override |
| B4 | Trip with fuel, invoice assigned, override | Má doklad ✓ | 🟠 | None |
| B5 | Trip with other costs, no invoice | Chýba doklad | 🔴 | Assign invoice |
| B6 | Trip with other costs, invoice assigned | Má doklad | 🟢 | None |
| B7 | Trip with fuel AND other costs, missing one | Chýba doklad | 🔴 | Assign missing |
| B8 | Trip with fuel AND other costs, both assigned | Má doklady | 🟢 | None |
| B9 | Trip with NO costs | - | - | N/A |

### C. Assignment Scenarios

| # | Scenario | What Happens | Result |
|---|----------|--------------|--------|
| C1 | Assign invoice to trip with NO costs, as FUEL | Trip populated: fuel_liters, fuel_cost_eur from invoice | 🟢 |
| C2 | Assign invoice to trip with NO costs, as OTHER | Trip populated: other_costs_eur from invoice | 🟢 |
| C3 | Assign invoice to trip with matching fuel data, as FUEL | Just link (no data change) | 🟢 |
| C4 | Assign invoice to trip with mismatched fuel data, as FUEL | Link + show warning | 🟢⚠ |
| C5 | User overrides mismatch warning | Warning suppressed | 🟠 |
| C6 | Assign invoice to trip that already has other costs | Block or warn? (decision needed) | ❓ |
| C7 | Assign same invoice to different trip | Reassign (move from old to new) | 🟢 |

### D. Data Mismatch Scenarios (when assigning FUEL invoice)

| # | What Mismatches | Warning Message (SK) | Warning Message (EN) |
|---|-----------------|----------------------|----------------------|
| D1 | Time outside trip range | Čas dokladu mimo jazdy | Invoice time outside trip |
| D2 | Liters differ | Litre: doklad X L ≠ jazda Y L | Liters: invoice X ≠ trip Y |
| D3 | Price differs | Cena: doklad X € ≠ jazda Y € | Price: invoice X € ≠ trip Y € |
| D4 | Multiple fields differ | Show all mismatches | Show all mismatches |

### E. Edge Cases

| # | Scenario | Behavior |
|---|----------|----------|
| E1 | One invoice → multiple trips | NOT allowed (1:1 relationship) |
| E2 | Multiple invoices → one trip | Allowed (fuel + other = 2 invoices) |
| E3 | Invoice date different from trip date | Allowed with warning (toll scenario) |
| E4 | Unassign invoice from trip | Clear trip_id, invoice becomes "unassigned" |
| E5 | Delete trip with assigned invoice | Invoice becomes "unassigned" |
| E6 | Delete invoice assigned to trip | Trip shows "missing invoice" |

---

## Visual States Summary

### Invoice Grid (Doklady)

```
┌─────────────────────────────────────────────────────────────┐
│                     DOKLADY (2026)                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ ▼ Nepriradené / Unassigned (2)                              │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🔴 NEPRIRADENÝ (unassigned)                         │   │
│   │    fuel-jan15.jpg                                   │   │
│   │    📅 15.1. 17:15  •  ⛽ 45.2 L  •  65.80 €         │   │
│   │    [Priradiť ako PALIVO]  [Priradiť ako INÉ]        │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟡 SKONTROLOVAŤ (needs review)                      │   │
│   │    receipt-blurry.jpg                               │   │
│   │    📅 ?.1. ?:??  •  ?? €                            │   │
│   │    ⚠ OCR neistý - skontrolujte údaje               │   │
│   │    [Upraviť]                                        │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│ ▼ Priradené / Assigned (10)                                 │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟢 PRIRADENÝ - PALIVO (assigned as fuel)            │   │
│   │    fuel-jan10.jpg                                   │   │
│   │    📅 10.1. 09:15  •  ⛽ 42.0 L  •  60.50 €         │   │
│   │    🚗 Jazda: 10.1. BA→KE (08:00-12:00)             │   │
│   │    ✓ Údaje súhlasia                                │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟢⚠ PRIRADENÝ - PALIVO (assigned, mismatch)        │   │
│   │    fuel-jan20.jpg                                   │   │
│   │    📅 20.1. 18:30  •  ⛽ 45.2 L  •  65.80 €         │   │
│   │    🚗 Jazda: 20.1. KE→PO (15:00-17:00)             │   │
│   │    ⚠ Čas mimo jazdy: 18:30 vs 15:00-17:00          │   │
│   │    [Opraviť jazdu]  [Opraviť doklad]  [Potvrdiť]    │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟠 PRIRADENÝ - PALIVO ✓ (override)                  │   │
│   │    toll-jan13.jpg                                   │   │
│   │    📅 13.1. 10:00  •  📄 10.00 €                    │   │
│   │    🚗 Jazda: 14.1. BA→ZA (06:00-09:00)             │   │
│   │    ✓ Potvrdené užívateľom (iný dátum)              │   │
│   │    [Zrušiť potvrdenie]                              │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ 🟢 PRIRADENÝ - INÉ NÁKLADY (assigned as other)      │   │
│   │    parking-jan12.jpg                                │   │
│   │    📅 12.1. 08:00  •  📄 5.00 €                     │   │
│   │    🚗 Jazda: 12.1. BA→TT (07:00-10:00)             │   │
│   │    ✓ Parkovanie                                    │   │
│   └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Trip Grid

```
┌───┬─────────┬────────────────┬──────┬─────────┬────────┬────────┬────────┐
│ # │  Dátum  │     Trasa      │  km  │ Palivo  │ Iné    │ Doklad │  Cena  │
├───┼─────────┼────────────────┼──────┼─────────┼────────┼────────┼────────┤
│ 1 │ 10.1.   │ BA → KE        │  400 │ 42.0 L  │   -    │   🟢   │ 60.50€ │
│ 2 │ 12.1.   │ BA → TT        │   60 │   -     │  5.00€ │   🟢   │  5.00€ │
│ 3 │ 14.1.   │ BA → ZA        │  200 │   -     │ 10.00€ │   🟠   │ 10.00€ │ ← override
│ 4 │ 15.1.   │ BA → KE        │  400 │ 45.2 L  │   -    │   🔴   │ 65.80€ │ ← missing
│ 5 │ 20.1.   │ KE → PO        │   80 │ 38.5 L  │   -    │  🟢⚠   │ 55.20€ │ ← mismatch
│ 6 │ 20.1.   │ PO → KE        │   80 │   -     │   -    │   -    │   -    │
└───┴─────────┴────────────────┴──────┴─────────┴────────┴────────┴────────┘

Legenda: 🟢 má doklad │ 🟢⚠ nesúlad │ 🟠 potvrdené │ 🔴 chýba │ - bez nákladov
```

---

## Assignment Flow

### User assigns invoice to trip

```
┌─────────────────────────────────────────────────────────────┐
│              Priradiť doklad k jazde                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Doklad: fuel-jan15.jpg                                     │
│  📅 15.1. 17:15  •  ⛽ 45.2 L  •  65.80 €                   │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Vybrať jazdu:                                              │
│                                                             │
│  ○ 15.1. BA → KE (13:00-17:00)  │ 45.2 L │ 65.80 € │ ✓     │
│  ○ 15.1. KE → BA (18:00-22:00)  │   -    │    -    │       │
│  ○ 16.1. BA → TT (08:00-10:00)  │ 30.0 L │ 45.00 € │ ⚠     │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Priradiť ako:                                              │
│  ● Palivo (FUEL)                                            │
│  ○ Iné náklady (OTHER)                                      │
│                                                             │
│                              [Zrušiť]  [Priradiť]           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### When data mismatches (assigning as FUEL)

```
┌─────────────────────────────────────────────────────────────┐
│              ⚠ Údaje nesúhlasia                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Doklad                    Jazda                            │
│  ────────                  ────────                         │
│  📅 15.1. 17:15           📅 15.1. 13:00-17:00             │
│  ⛽ 45.2 L                 ⛽ 45.2 L              ✓          │
│  💰 65.80 €                💰 64.50 €              ✗          │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Možnosti:                                                  │
│  • Opraviť údaje na doklade alebo jazde                    │
│  • Priradiť aj tak (zobrazí sa varovanie)                  │
│  • Priradiť a potvrdiť (varovanie sa nezobrazí)            │
│                                                             │
│  [Zrušiť]  [Priradiť s varovaním]  [Priradiť a potvrdiť]   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Model

### Receipt fields

```rust
pub struct Receipt {
    pub id: Uuid,
    pub trip_id: Option<Uuid>,        // NULL = unassigned, SET = assigned
    pub assignment_type: Option<AssignmentType>,  // Fuel or Other
    pub mismatch_override: bool,      // True = user confirmed mismatch
    // ... other fields
}

pub enum AssignmentType {
    Fuel,
    Other,
}
```

### Validation logic

```rust
pub enum ReceiptState {
    /// OCR not complete
    Processing,

    /// OCR low confidence, needs review
    NeedsReview,

    /// Ready but not assigned
    Unassigned,

    /// Assigned, data matches (or N/A for "other")
    Assigned { trip: Trip, assignment_type: AssignmentType },

    /// Assigned as fuel, data mismatch, no override
    AssignedWithMismatch { trip: Trip, mismatches: Vec<Mismatch> },

    /// Assigned as fuel, data mismatch, user confirmed
    AssignedWithOverride { trip: Trip },
}

pub enum Mismatch {
    TimeOutsideRange { receipt: String, trip_range: String },
    LitersDiffer { receipt: f64, trip: f64 },
    PriceDiffers { receipt: f64, trip: f64 },
    DateDiffers { receipt: String, trip: String },
}
```

---

## Visual States Mapping

| State | Invoice Grid | Trip Grid | Color |
|-------|--------------|-----------|-------|
| Processing | 🔄 Spracováva sa | - | Gray |
| NeedsReview | 🟡 Skontrolovať | - | Yellow |
| Unassigned | 🔴 Nepriradený | 🔴 Chýba doklad | Red |
| Assigned (match) | 🟢 Priradený | 🟢 Má doklad | Green |
| Assigned (mismatch) | 🟢⚠ Priradený | 🟢⚠ Má doklad | Green+Warning |
| Assigned (override) | 🟠 Potvrdený | 🟠 Potvrdený | Orange |

---

## Decisions Made

1. **No auto-matching**: User must explicitly assign invoices to trips

2. **User picks type**: FUEL or OTHER COST during assignment

3. **trip_id meaning**: NULL = unassigned, SET = assigned

4. **Mismatch handling**:
   - Show warning on both grids
   - User can fix data OR override
   - Override suppresses warning

5. **One-to-one**: One invoice → one trip (but trip can have multiple invoices: fuel + other)

6. **Assignment populates trip**: If trip has no costs, assignment fills them from invoice

---

## Open Questions

1. **Block or warn when trip already has other costs?**
   - Current: Blocks with "Jazda už má iné náklady"
   - Alternative: Warn and allow (replace old value)

2. **Show suggestions for likely matches?**
   - Even without auto-matching, we can highlight trips with matching date/data
   - Helps user find the right trip faster

---

## Migration

### Phase 1: Add fields
- `assignment_type: TEXT` (nullable, 'Fuel' or 'Other')
- `mismatch_override: BOOLEAN DEFAULT false`

### Phase 2: Migrate existing data
- Existing `trip_id` assignments: determine type from context (has liters? → Fuel)
- Existing `status = 'Assigned'` → set appropriate `assignment_type`

### Phase 3: Update UI
- Invoice grid: show assignment type badge
- Trip grid: unified receipt column
- Assignment dialog: type selector

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| v1-v5 | 2026-02-01 | Various iterations |
| v6 | 2026-02-02 | 3-state model (auto-match concept) |
| v7 | 2026-02-02 | **Simplified**: No magic, explicit assignment, user picks type |
