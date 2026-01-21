**Date:** 2026-01-21
**Subject:** Multi-Currency Receipt Support - Implementation Plan
**Status:** Planning

# Implementation Plan: Multi-Currency Receipt Support

## Phase 1: Backend - Database & Models

### 1.1 Database migration
- New file: `src-tauri/migrations/2026-01-21-100000-add_receipt_currency/up.sql`
- Add columns (backward-compatible with NULL defaults):
  ```sql
  ALTER TABLE receipts ADD COLUMN original_amount REAL DEFAULT NULL;
  ALTER TABLE receipts ADD COLUMN original_currency TEXT DEFAULT NULL;
  ```
- Create `down.sql` for rollback

### 1.2 Update schema.rs
- Run `diesel print-schema` or manually add:
  - `original_amount -> Nullable<Double>`
  - `original_currency -> Nullable<Text>`

### 1.3 Update models.rs
- Add to `Receipt` struct:
  - `original_amount: Option<f64>`
  - `original_currency: Option<String>`
- Add to `ReceiptRow` struct (same fields)
- Add to `NewReceiptRow` struct (same fields)
- Update `From<ReceiptRow> for Receipt` conversion

### 1.4 Write backend tests for model
- Test: Receipt with EUR currency
- Test: Receipt with CZK currency (no EUR value)
- Test: Receipt with CZK currency (with EUR conversion)
- Test: Backward compatibility (NULL currency fields)

## Phase 2: Backend - Gemini OCR

### 2.1 Update ExtractedReceipt struct (gemini.rs)
- Add `original_amount: Option<f64>`
- Add `original_currency: Option<String>`
- Add `currency: String` to `ExtractionConfidence`

### 2.2 Update Gemini prompt (EXTRACTION_PROMPT)
- Add currency detection instructions:
  ```
  For amounts:
  - Look for currency symbols: € (EUR), Kč/CZK (Czech), Ft/HUF (Hungarian), zł/PLN (Polish)
  - Return original_amount as the raw number found
  - Return original_currency as ISO code: "EUR", "CZK", "HUF", "PLN"
  - If no currency symbol found, guess based on country/language of receipt
  ```

### 2.3 Update response schema (get_response_schema)
- Add `original_amount` property
- Add `original_currency` property (enum: EUR, CZK, HUF, PLN, null)
- Add `currency` to confidence object

### 2.4 Write tests for OCR parsing
- Test: Parse EUR receipt JSON
- Test: Parse CZK receipt JSON
- Test: Parse receipt with unknown currency (null)
- Test: Confidence levels for currency detection

## Phase 3: Backend - Processing Logic

### 3.1 Update receipt processing in commands.rs
- When saving receipt from OCR:
  ```rust
  let total_price_eur = match extracted.original_currency.as_deref() {
      Some("EUR") => extracted.original_amount,
      Some(_) => None,  // Foreign: user must convert
      None => None,     // Unknown: user must clarify
  };
  ```
- Store `original_amount` and `original_currency` from extraction

### 3.2 Update db.rs receipt functions
- `create_receipt`: Include new fields
- `update_receipt`: Include new fields
- Ensure backward compatibility for receipts without currency

### 3.3 Add/update receipt edit command
- Allow updating `original_currency` and `total_price_eur`
- When currency changed to EUR: auto-copy `original_amount` to `total_price_eur`
- When currency changed to foreign: clear `total_price_eur` if user hasn't set it

### 3.4 Write integration tests
- Test: Create receipt with EUR → total_price_eur populated
- Test: Create receipt with CZK → total_price_eur is None
- Test: Update receipt with CZK conversion → total_price_eur set
- Test: Receipt matching still works with converted EUR value

## Phase 4: Frontend - Doklady View

### 4.1 Add i18n keys (sk/index.ts, en/index.ts)
- `receipts.currency` labels
- `receipts.currencyEur`, `receipts.currencyCzk`, etc.
- `receipts.enterEurAmount` (warning text)
- `receipts.convertedFrom` (e.g., "100 CZK →")

### 4.2 Update receipt display component
- If `original_currency == "EUR"` or null: show `{total_price_eur} €`
- If foreign currency with conversion: show `{original_amount} {currency} → {total_price_eur} €`
- If foreign currency without conversion: show `{original_amount} {currency} → ⚠️` with warning style

### 4.3 Update receipt card/row styling
- Warning state for unconverted foreign receipts
- Visual indicator that conversion is needed

## Phase 5: Frontend - Receipt Edit Modal

### 5.1 Update receipt edit form
- Add currency dropdown: EUR, CZK, HUF, PLN
- Add "Pôvodná suma" (original amount) field
- Add "Suma v EUR" field (editable for foreign currency)
- When EUR selected: hide or disable EUR field (mirrors original)
- When foreign selected: EUR field required

### 5.2 Add form validation
- Foreign currency: require EUR amount before save
- EUR amount must be positive number
- Original amount must be positive number

### 5.3 Wire up save logic
- Call updated backend command with all fields
- Refresh receipt list after save
- Show success/error toast

## Phase 6: Finalization

### 6.1 Manual testing
- Scan actual CZK parking receipt
- Verify currency detected
- Convert to EUR manually
- Verify matching works after conversion
- Test with EUR receipt (unchanged behavior)

### 6.2 Update CHANGELOG.md
- Add entry under [Unreleased]

### 6.3 Run full test suite
- `npm run test:backend`
- `npm run test:integration:tier1`

## Files to Modify

| File | Changes |
|------|---------|
| `migrations/.../up.sql` | NEW: Add currency columns |
| `schema.rs` | Add new columns |
| `models.rs` | Add fields to Receipt, ReceiptRow, NewReceiptRow |
| `gemini.rs` | Update ExtractedReceipt, prompt, schema |
| `commands.rs` | Update receipt processing logic |
| `db.rs` | Update receipt CRUD |
| `i18n/sk/index.ts` | Add currency translations |
| `i18n/en/index.ts` | Add currency translations |
| Receipt display component | Show currency format |
| Receipt edit modal | Add currency dropdown + EUR field |

## Dependencies Graph

```
Phase 1 (DB + Models)
       │
       ▼
Phase 2 (Gemini OCR) ──→ Phase 3 (Processing)
                                │
                                ▼
                    Phase 4 (Doklady View)
                                │
                                ▼
                    Phase 5 (Edit Modal)
                                │
                                ▼
                    Phase 6 (Finalize)
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| OCR misdetects currency | Confidence field + user can override |
| Existing receipts break | NULL defaults, backward-compatible migration |
| Matching stops working | Matching unchanged, uses total_price_eur |
| User forgets to convert | Warning UI state for unconverted receipts |
