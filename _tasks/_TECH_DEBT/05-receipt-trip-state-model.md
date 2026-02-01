# Tech Debt: Receipt & Trip State Model Documentation

**Date:** 2026-02-01
**Priority:** Medium
**Effort:** Medium (2-8h)
**Component:** `src-tauri/src/models.rs`, `src/lib/types.ts`, `src/routes/doklady/+page.svelte`
**Status:** Open

## Problem

The receipt (invoice/doklad) and trip verification system has grown organically with multiple overlapping state fields. The current state model is implicit in the code rather than explicitly documented, making it difficult to:

1. Understand all possible states at a glance
2. Ensure UI handles all states consistently
3. Add new states without missing edge cases

## Current State Model

### Receipt Processing Status (`ReceiptStatus`)

```typescript
'Pending'      // Scanned but not OCR'd yet
'Parsed'       // OCR complete, data extracted
'NeedsReview'  // OCR low confidence, needs manual check
'Assigned'     // Manually assigned to a trip by user
```

### Receipt Verification Status (via `ReceiptVerification`)

| State | `matched` | `mismatchReason` | `datetimeWarning` | Visual |
|-------|-----------|------------------|-------------------|--------|
| **Matched perfectly** | `true` | `none` | `false` | Green checkmark |
| **Matched with datetime warning** | `true` | `none` | `true` | Checkmark + triangle |
| **Unmatched - no data** | `false` | `missingReceiptData` | - | Red border |
| **Unmatched - no fuel trip** | `false` | `noFuelTripFound` | - | Red border |
| **Unmatched - date mismatch** | `false` | `dateMismatch` | - | Red border + warning |
| **Unmatched - datetime out of range** | `false` | `datetimeOutOfRange` | - | Red border + warning |
| **Unmatched - liters mismatch** | `false` | `litersMismatch` | - | Red border + warning |
| **Unmatched - price mismatch** | `false` | `priceMismatch` | - | Red border + warning |
| **Unmatched - no other cost match** | `false` | `noOtherCostMatch` | - | Red border |
| **Manually assigned** | - | - | - | `status: 'Assigned'` |

### Attachment Status (for trip assignment dialog)

```typescript
'empty'    // Trip has no receipt attached
'matches'  // Receipt data matches trip data
'differs'  // Receipt attached but data differs
```

### Trip States Related to Receipts (in `TripGridData`)

| Array | Meaning |
|-------|---------|
| `missingReceipts: string[]` | Trip IDs that have fuel/energy but no receipt attached |
| `receiptDatetimeWarnings: string[]` | Trip IDs where attached receipt's datetime is outside trip's time range |

### Trip Receipt Attachment Flow

```
┌─────────────────────────────────────────────────────────┐
│ Trip has fuel (fuelLiters > 0) or other costs?          │
├─────────────────────────────────────────────────────────┤
│ YES → Does receipt exist with tripId = this trip?       │
│       ├─ NO  → "bez dokladu" (missing receipt)          │
│       └─ YES → Is receipt datetime within trip range?   │
│                ├─ YES → All good                        │
│                └─ NO  → "dátum/čas mimo jazdy"          │
├─────────────────────────────────────────────────────────┤
│ NO → No receipt expected                                │
└─────────────────────────────────────────────────────────┘
```

## Impact

- **Maintenance burden**: Adding new states requires changes in multiple places
- **Consistency risk**: Different parts of UI may not handle all states
- **Onboarding friction**: New developers must trace code to understand states

## Root Cause

Feature evolved incrementally:
1. Initial receipt matching (matched/unmatched)
2. Added mismatch reasons for better UX
3. Added datetime validation (Task 50)
4. States now span multiple files without central documentation

## Recommended Solution

### Phase 1: Documentation (This Item)
- Document all states in one place (this file serves as living documentation)
- Add state diagram to `docs/features/`

### Phase 2: State Machine Refactor (Future)
- Consider explicit state machine pattern
- Single source of truth for state transitions
- Type-safe state handling

### Phase 3: UI State Mapping (Future)
- Create mapping table: State → Visual Treatment
- Ensure all states have explicit UI handling
- Add exhaustive switch/match statements

## Alternative Options

1. **Keep as-is**: Document implicitly in code comments
   - Pro: No refactoring needed
   - Con: Documentation drifts from implementation

2. **Full state machine**: Implement Rust enum for all states
   - Pro: Type-safe, exhaustive
   - Con: Significant refactor, may be overengineered

## Related

- Task 50: Receipt datetime validation (`_tasks/50-receipt-datetime-validation/`)
- `src-tauri/src/models.rs` - Rust types
- `src/lib/types.ts` - TypeScript types
- `src/routes/doklady/+page.svelte` - Receipt list UI

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-01 | Created documentation | After Task 50 completion, state model needs explicit documentation for future iteration |
