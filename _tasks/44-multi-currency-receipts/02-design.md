# Multi-Currency Receipt Support

## Problem

Parking/toll receipts from neighboring countries (CZ, HU, PL) are scanned with amounts in foreign currency. OCR currently assumes EUR, causing:
- 100 CZK interpreted as 100 EUR (25x overstatement)
- Receipt doesn't match trip (wrong amount)
- User cannot correct without manual workarounds

## Solution

Store original amount + currency separately from EUR value. User manually converts foreign currency to EUR.

## Supported Currencies

- EUR (Euro) — default, Slovak receipts
- CZK (Czech koruna) — Kč symbol
- HUF (Hungarian forint) — Ft symbol
- PLN (Polish złoty) — zł symbol

## Data Model

### New fields on `Receipt`

```rust
pub struct Receipt {
    // Existing
    pub total_price_eur: Option<f64>,  // EUR value (for matching + accounting)

    // New
    pub original_amount: Option<f64>,      // Raw number from OCR
    pub original_currency: Option<String>, // "EUR", "CZK", "HUF", "PLN"
}
```

### Behavior

| Scenario | original_amount | original_currency | total_price_eur |
|----------|-----------------|-------------------|-----------------|
| EUR receipt | 50.0 | "EUR" | 50.0 |
| CZK receipt (before conversion) | 100.0 | "CZK" | None |
| CZK receipt (after user converts) | 100.0 | "CZK" | 3.95 |
| Unknown currency | 100.0 | None | None |

### Database Migration

```sql
ALTER TABLE receipts ADD COLUMN original_amount REAL DEFAULT NULL;
ALTER TABLE receipts ADD COLUMN original_currency TEXT DEFAULT NULL;
```

Backward-compatible: adds columns with NULL defaults.

## Gemini OCR Changes

### Updated prompt additions

```
For amounts:
- Look for currency symbols: € (EUR), Kč/CZK (Czech), Ft/HUF (Hungarian), zł/PLN (Polish)
- Return original_amount as the raw number found
- Return original_currency as ISO code: "EUR", "CZK", "HUF", "PLN"
- If no currency symbol found, guess based on country/language of receipt
```

### Updated response schema

```json
{
  "original_amount": 100.0,
  "original_currency": "CZK",
  "confidence": {
    "currency": "high",
    "total_price": "high",
    ...
  }
}
```

### Rust processing logic

```rust
// After receiving ExtractedReceipt from Gemini:
let total_price_eur = match extracted.original_currency.as_deref() {
    Some("EUR") => extracted.original_amount,  // EUR: use directly
    Some(_) => None,  // Foreign currency: user must convert
    None => None,     // Unknown: user must clarify
};
```

Prompt only extracts. Rust decides business logic.

## UI Changes

### Doklady (receipts) view

Display format based on currency:
- EUR: `3,95 €`
- Foreign with conversion: `100 CZK → 3,95 €`
- Foreign without conversion: `100 CZK → ⚠️ zadajte EUR` (warning)

### Receipt edit modal

```
Pôvodná suma:  [100.00] [CZK ▼]
Suma v EUR:    [3.95  ]  €
```

- Currency dropdown: EUR, CZK, HUF, PLN
- When EUR selected, "Suma v EUR" mirrors original (or field hidden)
- When foreign currency, user must enter EUR value manually

### Trip table

No change — shows only `total_price_eur` in "Iné náklady" column.

## Matching Logic

**No changes required.** Matching already uses `total_price_eur`:
- Foreign currency receipts won't match until user provides EUR value
- Once EUR value set, normal matching applies

## Implementation Order

1. Database migration + model changes (backend)
2. Gemini prompt + ExtractedReceipt struct (backend)
3. Processing logic in commands.rs (backend)
4. Backend tests
5. Doklady view updates (frontend)
6. Receipt edit modal (frontend)

## Out of Scope

- Automatic exchange rate lookup (user converts manually)
- Currency conversion suggestions
- Additional currencies beyond EUR/CZK/HUF/PLN
- Parking-specific fields (vendor_name + cost_description sufficient)
