# Design: Receipt Mismatch Reasons

**Date:** 2026-01-20
**Status:** ✅ Complete

## Architecture Decision

**Use tagged enum for mismatch reasons** - This provides:
- Type safety (Rust enforces handling all cases)
- Clean serialization to TypeScript discriminated unions
- i18n-friendly (frontend maps variants to translated strings)
- Extensibility without breaking changes

## Data Model

### Backend: `MismatchReason` enum (models.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MismatchReason {
    /// Receipt is verified - no mismatch
    None,
    /// Receipt missing date, liters, or price
    MissingReceiptData,
    /// No trip with fuel data found for this year
    NoFuelTripFound,
    /// Found trip with matching liters+price but different date
    DateMismatch {
        receipt_date: String,
        closest_trip_date: String
    },
    /// Found trip with matching date+price but different liters
    LitersMismatch {
        receipt_liters: f64,
        trip_liters: f64
    },
    /// Found trip with matching date+liters but different price
    PriceMismatch {
        receipt_price: f64,
        trip_price: f64
    },
    /// Other-cost receipt - no trip with matching price
    NoOtherCostMatch,
}
```

### Updated `ReceiptVerification` struct

```rust
pub struct ReceiptVerification {
    pub receipt_id: String,
    pub matched: bool,
    pub matched_trip_id: Option<String>,
    pub matched_trip_date: Option<String>,
    pub matched_trip_route: Option<String>,
    pub mismatch_reason: MismatchReason,  // NEW FIELD
}
```

### Frontend: TypeScript type (api.ts)

```typescript
type MismatchReason =
  | { type: 'none' }
  | { type: 'missingReceiptData' }
  | { type: 'noFuelTripFound' }
  | { type: 'dateMismatch'; receiptDate: string; closestTripDate: string }
  | { type: 'litersMismatch'; receiptLiters: number; tripLiters: number }
  | { type: 'priceMismatch'; receiptPrice: number; tripPrice: number }
  | { type: 'noOtherCostMatch' };

interface ReceiptVerification {
  receiptId: string;
  matched: boolean;
  matchedTripId: string | null;
  matchedTripDate: string | null;
  matchedTripRoute: string | null;
  mismatchReason: MismatchReason;  // NEW FIELD
}
```

## Matching Logic Priority

When determining mismatch reason, check in this order:

1. **Missing receipt data** - Can't match if we don't have date/liters/price
2. **No fuel trips exist** - Nothing to match against
3. **Find closest match** - Iterate trips, track which fields match:
   - If liters+price match but date differs → `DateMismatch`
   - If date+price match but liters differs → `LitersMismatch`
   - If date+liters match but price differs → `PriceMismatch`
4. **No close match** - Fall back to `NoFuelTripFound`

For other-cost receipts:
1. **Missing price** → `MissingReceiptData`
2. **No matching price** → `NoOtherCostMatch`

## UI Display

**Placement:** Inline text below the badge (not tooltip)

```
┌─────────────────────────────────────────┐
│ 📄 20260120_blok_tankovanie.pdf  [Neoverený] │
│                                         │
│ Dátum: 20. 1. 2026 ●                   │
│ Litre: 63.68 L ●                       │
│ Cena: 91.32 € ●                        │
│                                         │
│ ↳ Dátum 20.1. - jazda je 19.1.         │  ← Mismatch reason
│                                         │
│ [Otvoriť] [Znovu spracovať] [Prideliť] │
└─────────────────────────────────────────┘
```

**Styling:**
- Small text, muted color (secondary text)
- Arrow prefix (↳) to indicate it's explanatory
- Same color as "danger" badge for visual connection

## i18n Strings

### Slovak (sk/index.ts)
```typescript
receipts: {
  // Mismatch reasons
  mismatchMissingData: 'Chýbajú údaje na doklade',
  mismatchNoFuelTrip: 'Žiadna jazda s tankovaním',
  mismatchDate: 'Dátum {receiptDate} - jazda je {tripDate}',
  mismatchLiters: '{receiptLiters} L - jazda má {tripLiters} L',
  mismatchPrice: '{receiptPrice} € - jazda má {tripPrice} €',
  mismatchNoOtherCost: 'Žiadna jazda s touto cenou',
}
```

### English (en/index.ts)
```typescript
receipts: {
  // Mismatch reasons
  mismatchMissingData: 'Receipt data missing',
  mismatchNoFuelTrip: 'No trip with fuel data',
  mismatchDate: 'Date {receiptDate} - trip is {tripDate}',
  mismatchLiters: '{receiptLiters} L - trip has {tripLiters} L',
  mismatchPrice: '{receiptPrice} € - trip has {tripPrice} €',
  mismatchNoOtherCost: 'No trip with this price',
}
```

## Test Cases

1. **Missing receipt data** - Receipt with no liters extracted
2. **No fuel trips** - Receipt exists but no trips with fuel in year
3. **Date mismatch** - Receipt 20.1., trip 19.1. with same liters/price
4. **Liters mismatch** - Same date/price, different liters
5. **Price mismatch** - Same date/liters, different price
6. **Exact match** - Should return `None` (verified)
7. **Other-cost no match** - Receipt with price, no trip with other_costs

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/models.rs` | Add `MismatchReason` enum, update `ReceiptVerification` |
| `src-tauri/src/commands.rs` | Update `verify_receipts_with_data()` logic |
| `src-tauri/src/commands_tests.rs` | Add tests for mismatch reasons |
| `src/lib/api.ts` | Update TypeScript types |
| `src/routes/doklady/+page.svelte` | Display mismatch reason |
| `src/lib/i18n/sk/index.ts` | Add Slovak strings |
| `src/lib/i18n/en/index.ts` | Add English strings |
