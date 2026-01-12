# Implementation Plan: Additional Costs Invoice Recognition

**Date:** 2026-01-12
**Status:** Ready for Review

## Prerequisites

- [ ] User decision on open items (see 02-design.md)
- [ ] Commit planning docs before implementation

---

## Phase 1: Data Model & Backend (Day 1)

### Step 1.1: Add ReceiptType enum
**File:** `src-tauri/src/models.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ReceiptType {
    #[default]
    Fuel,
    CarWash,
    Parking,
    Toll,
    Service,
    Other,
}
```

### Step 1.2: Extend Receipt struct
**File:** `src-tauri/src/models.rs`

Add fields:
- `receipt_type: ReceiptType`
- `cost_amount_eur: Option<f64>`
- `cost_category: Option<String>`
- `cost_description: Option<String>`
- `vendor_name: Option<String>`

### Step 1.3: Database migration
**File:** `src-tauri/migrations/YYYYMMDD_add_receipt_type.sql`

```sql
ALTER TABLE receipts ADD COLUMN receipt_type TEXT NOT NULL DEFAULT 'Fuel';
ALTER TABLE receipts ADD COLUMN cost_amount_eur REAL;
ALTER TABLE receipts ADD COLUMN cost_category TEXT;
ALTER TABLE receipts ADD COLUMN cost_description TEXT;
ALTER TABLE receipts ADD COLUMN vendor_name TEXT;
```

### Step 1.4: Update Diesel schema
**File:** `src-tauri/src/schema.rs`

Add new columns to receipts table definition.

### Step 1.5: Update db.rs
**File:** `src-tauri/src/db.rs`

- Update `ReceiptRow` struct
- Update `NewReceiptRow` struct
- Update CRUD operations

### Step 1.6: Write backend tests
- Test ReceiptType serialization
- Test Receipt with new fields CRUD
- Test migration runs correctly

---

## Phase 2: Gemini Integration (Day 1-2)

### Step 2.1: Update Gemini prompt
**File:** `src-tauri/src/gemini.rs`

Update `RECEIPT_EXTRACTION_PROMPT` to:
- Classify receipt type first
- Extract type-specific fields
- Return unified JSON structure

### Step 2.2: Update response parsing
**File:** `src-tauri/src/gemini.rs`

- Parse `receipt_type` field
- Map string to `ReceiptType` enum
- Extract cost fields for non-fuel receipts

### Step 2.3: Update receipts.rs
**File:** `src-tauri/src/receipts.rs`

- `apply_extraction()` handles both types
- Set appropriate fields based on type

### Step 2.4: Write parsing tests
- Test fuel receipt parsing (existing behavior)
- Test car wash receipt parsing
- Test parking receipt parsing
- Test unknown type defaults to Other

---

## Phase 3: Frontend Types (Day 2)

### Step 3.1: Update TypeScript types
**File:** `src/lib/types.ts`

```typescript
export type ReceiptType = 'Fuel' | 'CarWash' | 'Parking' | 'Toll' | 'Service' | 'Other';

export interface Receipt {
  // ... existing fields ...
  receiptType: ReceiptType;
  costAmountEur: number | null;
  costCategory: string | null;
  costDescription: string | null;
  vendorName: string | null;
}
```

### Step 3.2: Update i18n
**File:** `src/lib/i18n/sk/index.ts`

Add translations for:
- Receipt type names
- Filter labels
- Assignment messages

---

## Phase 4: UI Updates (Day 2-3)

### Step 4.1: Doklady page - type filter
**File:** `src/routes/doklady/+page.svelte`

- Add type filter dropdown
- Filter receipts by selected type
- Update receipt count display

### Step 4.2: ReceiptCard - visual distinction
**File:** `src/lib/components/ReceiptCard.svelte`

- Icon based on receipt type
- Display cost fields for non-fuel
- Different layout for fuel vs other

### Step 4.3: Assignment command update
**File:** `src-tauri/src/commands.rs`

- `assign_receipt_to_trip` checks receipt type
- Fuel → update `fuel_liters`, `fuel_cost_eur`
- Other → update `other_costs_eur`, `other_costs_note`

### Step 4.4: TripRow integration
**File:** `src/lib/components/TripRow.svelte`

- Show other costs indicator
- Click to view assigned receipt

---

## Phase 5: Testing & Polish (Day 3)

### Step 5.1: Integration tests
**File:** `tests/integration/`

- Scan mixed receipts
- Assign fuel receipt
- Assign other cost receipt
- Verify trip fields updated

### Step 5.2: Manual testing
- [ ] Scan folder with various invoice types
- [ ] Verify correct classification
- [ ] Assign each type to trips
- [ ] Check trip displays correctly

### Step 5.3: Update changelog
**File:** `CHANGELOG.md`

Add to [Unreleased]:
- Rozpoznávanie a priradenie ostatných nákladov (umytie, parkovanie, diaľnica, servis)

---

## Estimated Effort

| Phase | Effort | Notes |
|-------|--------|-------|
| 1. Data Model | 2-3h | Straightforward extension |
| 2. Gemini | 2-3h | Prompt tuning may need iteration |
| 3. Frontend Types | 1h | Mechanical changes |
| 4. UI Updates | 3-4h | Most complex part |
| 5. Testing | 2h | Including manual verification |
| **Total** | **10-13h** | ~2 days |

---

## Rollback Plan

If issues arise:
1. Migration is additive (new columns), no data loss
2. Default `receipt_type = 'Fuel'` preserves existing behavior
3. Frontend can hide new types with feature flag if needed

---

## Success Criteria

- [ ] Scan folder with fuel + other receipts → correctly classified
- [ ] Assign fuel receipt → updates fuel fields
- [ ] Assign other cost → updates other_costs fields
- [ ] Doklady page shows both types with visual distinction
- [ ] Trip row shows assigned other costs
- [ ] All existing tests pass
- [ ] New tests for other costs pass
