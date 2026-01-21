# Implementation Plan: Receipt Mismatch Reasons

**Date:** 2026-01-20
**Status:** ✅ Complete

## Phase 1: Backend - Data Model

### Step 1.1: Add MismatchReason enum to models.rs

Add new enum after `ReceiptVerification` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MismatchReason {
    None,
    MissingReceiptData,
    NoFuelTripFound,
    DateMismatch { receipt_date: String, closest_trip_date: String },
    LitersMismatch { receipt_liters: f64, trip_liters: f64 },
    PriceMismatch { receipt_price: f64, trip_price: f64 },
    NoOtherCostMatch,
}

impl Default for MismatchReason {
    fn default() -> Self {
        MismatchReason::None
    }
}
```

### Step 1.2: Update ReceiptVerification struct

Add `mismatch_reason` field:

```rust
pub struct ReceiptVerification {
    pub receipt_id: String,
    pub matched: bool,
    pub matched_trip_id: Option<String>,
    pub matched_trip_date: Option<String>,
    pub matched_trip_route: Option<String>,
    pub mismatch_reason: MismatchReason,
}
```

---

## Phase 2: Backend - Verification Logic

### Step 2.1: Write tests first (TDD)

Add tests to `commands_tests.rs`:

```rust
#[test]
fn test_verify_receipts_mismatch_reason_missing_data() { ... }

#[test]
fn test_verify_receipts_mismatch_reason_no_fuel_trip() { ... }

#[test]
fn test_verify_receipts_mismatch_reason_date_mismatch() { ... }

#[test]
fn test_verify_receipts_mismatch_reason_liters_mismatch() { ... }

#[test]
fn test_verify_receipts_mismatch_reason_price_mismatch() { ... }

#[test]
fn test_verify_receipts_mismatch_reason_none_when_matched() { ... }
```

### Step 2.2: Update verify_receipts_with_data() in commands.rs

Refactor the function to:
1. Track closest match while iterating
2. Determine specific mismatch reason when no exact match
3. Populate `mismatch_reason` field

Key logic changes:

```rust
fn verify_receipts_with_data(...) -> Result<VerificationResult, String> {
    // ... existing setup ...

    for receipt in &receipts_for_year {
        let mut matched = false;
        let mut mismatch_reason = MismatchReason::None;
        // ... other fields ...

        // Check for missing receipt data first
        let has_receipt_data = receipt.receipt_date.is_some()
            && receipt.liters.is_some()
            && receipt.total_price_eur.is_some();

        if !has_receipt_data {
            mismatch_reason = MismatchReason::MissingReceiptData;
        } else if trips_with_fuel.is_empty() {
            mismatch_reason = MismatchReason::NoFuelTripFound;
        } else {
            // Try to find exact match, track closest match for reason
            let mut closest_match: Option<ClosestMatch> = None;

            for trip in &trips_with_fuel {
                // ... matching logic ...
                // Track which fields match for closest match
            }

            if !matched {
                mismatch_reason = determine_mismatch_reason(receipt, closest_match);
            }
        }

        verifications.push(ReceiptVerification {
            // ... existing fields ...
            mismatch_reason,
        });
    }
}
```

### Step 2.3: Run backend tests

```bash
cd src-tauri && cargo test verify_receipts_mismatch
```

---

## Phase 3: Frontend - TypeScript Types

### Step 3.1: Update api.ts

Add TypeScript type for MismatchReason:

```typescript
export type MismatchReason =
  | { type: 'none' }
  | { type: 'missingReceiptData' }
  | { type: 'noFuelTripFound' }
  | { type: 'dateMismatch'; receiptDate: string; closestTripDate: string }
  | { type: 'litersMismatch'; receiptLiters: number; tripLiters: number }
  | { type: 'priceMismatch'; receiptPrice: number; tripPrice: number }
  | { type: 'noOtherCostMatch' };

export interface ReceiptVerification {
  receiptId: string;
  matched: boolean;
  matchedTripId: string | null;
  matchedTripDate: string | null;
  matchedTripRoute: string | null;
  mismatchReason: MismatchReason;
}
```

---

## Phase 4: Frontend - i18n

### Step 4.1: Add Slovak strings (sk/index.ts)

```typescript
receipts: {
  // ... existing ...
  mismatchMissingData: 'Chýbajú údaje na doklade',
  mismatchNoFuelTrip: 'Žiadna jazda s tankovaním',
  mismatchDate: 'Dátum {receiptDate:string} - jazda je {tripDate:string}',
  mismatchLiters: '{receiptLiters:number} L - jazda má {tripLiters:number} L',
  mismatchPrice: '{receiptPrice:number} € - jazda má {tripPrice:number} €',
  mismatchNoOtherCost: 'Žiadna jazda s touto cenou',
}
```

### Step 4.2: Add English strings (en/index.ts)

```typescript
receipts: {
  // ... existing ...
  mismatchMissingData: 'Receipt data missing',
  mismatchNoFuelTrip: 'No trip with fuel data',
  mismatchDate: 'Date {receiptDate:string} - trip is {tripDate:string}',
  mismatchLiters: '{receiptLiters:number} L - trip has {tripLiters:number} L',
  mismatchPrice: '{receiptPrice:number} € - trip has {tripPrice:number} €',
  mismatchNoOtherCost: 'No trip with this price',
}
```

### Step 4.3: Regenerate i18n types

```bash
npm run typesafe-i18n
```

---

## Phase 5: Frontend - UI Display

### Step 5.1: Add formatMismatchReason helper in +page.svelte

```typescript
function formatMismatchReason(reason: MismatchReason): string {
  switch (reason.type) {
    case 'none':
      return '';
    case 'missingReceiptData':
      return $LL.receipts.mismatchMissingData();
    case 'noFuelTripFound':
      return $LL.receipts.mismatchNoFuelTrip();
    case 'dateMismatch':
      return $LL.receipts.mismatchDate({
        receiptDate: reason.receiptDate,
        tripDate: reason.closestTripDate
      });
    case 'litersMismatch':
      return $LL.receipts.mismatchLiters({
        receiptLiters: reason.receiptLiters,
        tripLiters: reason.tripLiters
      });
    case 'priceMismatch':
      return $LL.receipts.mismatchPrice({
        receiptPrice: reason.receiptPrice,
        tripPrice: reason.tripPrice
      });
    case 'noOtherCostMatch':
      return $LL.receipts.mismatchNoOtherCost();
  }
}
```

### Step 5.2: Update receipt card template

After the badge, add mismatch reason display:

```svelte
{#if verif?.matched}
  <span class="badge success">{$LL.receipts.statusVerified()}</span>
{:else if receipt.status === 'NeedsReview'}
  <span class="badge warning">{$LL.receipts.statusNeedsReview()}</span>
{:else}
  <span class="badge danger">{$LL.receipts.statusUnverified()}</span>
{/if}

{#if !verif?.matched && verif?.mismatchReason?.type !== 'none'}
  <span class="mismatch-reason">↳ {formatMismatchReason(verif.mismatchReason)}</span>
{/if}
```

### Step 5.3: Add CSS styling

```css
.mismatch-reason {
  display: block;
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 0.25rem;
}
```

---

## Phase 6: Testing & Verification

### Step 6.1: Run all backend tests

```bash
npm run test:backend
```

### Step 6.2: Manual testing scenarios

1. Create receipt without matching trip → should show "Žiadna jazda s tankovaním"
2. Create trip on 19.1., receipt on 20.1. with same liters/price → should show date mismatch
3. Create receipt with pending OCR → should show "Chýbajú údaje"
4. Create matching trip/receipt → should show "Overený"

### Step 6.3: Run integration tests

```bash
npm run test:integration:tier1
```

---

## Summary

| Phase | Estimated Complexity |
|-------|---------------------|
| 1. Backend data model | Low |
| 2. Backend verification logic | Medium |
| 3. Frontend TypeScript | Low |
| 4. Frontend i18n | Low |
| 5. Frontend UI | Low |
| 6. Testing | Medium |

**Total files to modify:** 7
**New tests to add:** 6-7 test cases
