**Date:** 2026-01-21
**Subject:** Multi-Currency Receipt Support - Implementation Plan
**Status:** Planning (Revised after review)

# Implementation Plan: Multi-Currency Receipt Support

## Phase 1: Backend - Database & Models

### 1.1 Database migration
- Run: `cd src-tauri && diesel migration generate add_receipt_currency`
- Edit generated `up.sql`:
  ```sql
  ALTER TABLE receipts ADD COLUMN original_amount REAL DEFAULT NULL;
  ALTER TABLE receipts ADD COLUMN original_currency TEXT DEFAULT NULL;
  ```
- Edit generated `down.sql`:
  ```sql
  -- SQLite doesn't support DROP COLUMN, so down.sql will be a no-op comment
  -- Original columns will remain but be unused if rolled back
  ```

### 1.2 Update schema.rs
- File: `src-tauri/src/schema.rs`
- Add to `receipts` table definition:
  ```rust
  original_amount -> Nullable<Double>,
  original_currency -> Nullable<Text>,
  ```
- Run `diesel print-schema` to verify, or add manually

### 1.3 Update models.rs - Receipt struct (line ~377)
- File: `src-tauri/src/models.rs`
- Add to `Receipt` struct after `total_price_eur`:
  ```rust
  pub original_amount: Option<f64>,
  pub original_currency: Option<String>,
  ```

### 1.4 Update models.rs - ReceiptRow struct (line ~686)
- File: `src-tauri/src/models.rs`
- Add to `ReceiptRow` struct (before `created_at`):
  ```rust
  pub original_amount: Option<f64>,
  pub original_currency: Option<String>,
  ```

### 1.5 Update models.rs - NewReceiptRow struct (line ~712)
- File: `src-tauri/src/models.rs`
- Add to `NewReceiptRow` struct:
  ```rust
  pub original_amount: Option<f64>,
  pub original_currency: Option<&'a str>,
  ```

### 1.6 Update models.rs - From<ReceiptRow> conversion (line ~832)
- File: `src-tauri/src/models.rs`
- Add to the `From<ReceiptRow> for Receipt` impl:
  ```rust
  original_amount: row.original_amount,
  original_currency: row.original_currency,
  ```

### 1.7 Update models.rs - Receipt::new_with_source_year (line ~417)
- File: `src-tauri/src/models.rs`
- Add to `Receipt::new_with_source_year()` constructor:
  ```rust
  original_amount: None,
  original_currency: None,
  ```

### 1.8 Write backend tests for model
- File: `src-tauri/src/db_tests.rs`
- Add tests:
  - `test_receipt_with_eur_currency` — EUR receipt has matching original_amount and total_price_eur
  - `test_receipt_with_foreign_currency` — CZK receipt has original_amount but no total_price_eur
  - `test_receipt_backward_compatibility` — Old receipts (NULL currency fields) still work

## Phase 2: Backend - Gemini OCR

### 2.1 Update ExtractedReceipt struct
- File: `src-tauri/src/gemini.rs` (line ~14)
- Replace `total_price_eur` with:
  ```rust
  pub original_amount: Option<f64>,
  pub original_currency: Option<String>,
  ```

### 2.2 Update ExtractionConfidence struct
- File: `src-tauri/src/gemini.rs` (line ~26)
- Add field:
  ```rust
  pub currency: String,  // "high", "medium", "low"
  ```

### 2.3 Update ExtractedReceipt::default()
- File: `src-tauri/src/gemini.rs` (line ~33)
- Update default values:
  ```rust
  original_amount: None,
  original_currency: None,
  // ...
  confidence: ExtractionConfidence {
      // ...
      currency: "low".to_string(),
  }
  ```

### 2.4 Update EXTRACTION_PROMPT
- File: `src-tauri/src/gemini.rs` (line ~139)
- Add currency detection to prompt:
  ```
  For amounts:
  - Look for currency symbols: € (EUR), Kč/CZK (Czech koruna), Ft/HUF (Hungarian forint), zł/PLN (Polish złoty)
  - Return original_amount as the raw number found
  - Return original_currency as ISO code: "EUR", "CZK", "HUF", "PLN"
  - If no currency symbol found, guess based on country/language of receipt
  ```
- Update JSON schema in prompt to show new fields

### 2.5 Update get_response_schema()
- File: `src-tauri/src/gemini.rs` (line ~172)
- Replace `total_price_eur` property with:
  ```rust
  "original_amount": {
      "type": ["number", "null"],
      "description": "Raw amount number from receipt"
  },
  "original_currency": {
      "type": ["string", "null"],
      "enum": ["EUR", "CZK", "HUF", "PLN", null],
      "description": "ISO currency code"
  },
  ```
- Add to confidence properties:
  ```rust
  "currency": {
      "type": "string",
      "enum": ["high", "medium", "low"]
  }
  ```

### 2.6 Write tests for OCR parsing
- File: `src-tauri/src/gemini.rs` (in `mod tests`)
- Update existing tests to use new field names
- Add tests:
  - `test_extracted_receipt_eur` — EUR currency detected
  - `test_extracted_receipt_czk` — CZK currency detected
  - `test_extracted_receipt_unknown_currency` — null currency

## Phase 3: Backend - Processing Logic

### 3.1 Update receipt processing in commands.rs
- File: `src-tauri/src/commands.rs`
- Find `process_receipt_file` or similar function
- After receiving `ExtractedReceipt` from Gemini, compute `total_price_eur`:
  ```rust
  let total_price_eur = match extracted.original_currency.as_deref() {
      Some("EUR") => extracted.original_amount,
      Some(_) => None,  // Foreign currency: user must convert
      None => None,     // Unknown: user must clarify
  };
  ```
- Store `original_amount`, `original_currency`, and computed `total_price_eur`

### 3.2 Update db.rs - create_receipt function
- File: `src-tauri/src/db.rs` (line ~642)
- Ensure new fields are included in the insert:
  ```rust
  original_amount: receipt.original_amount,
  original_currency: receipt.original_currency.as_deref(),
  ```

### 3.3 Update db.rs - update_receipt function
- File: `src-tauri/src/db.rs` (line ~716)
- Ensure new fields are included in the update

### 3.4 Create NEW Tauri command: update_receipt_currency
- File: `src-tauri/src/commands.rs`
- Create new command:
  ```rust
  #[tauri::command]
  pub fn update_receipt_currency(
      app_state: State<'_, AppState>,
      receipt_id: String,
      original_amount: Option<f64>,
      original_currency: Option<String>,
      total_price_eur: Option<f64>,
  ) -> Result<Receipt, String> {
      check_read_only!(app_state);
      // Validate currency is one of: EUR, CZK, HUF, PLN
      // Update receipt in database
      // Return updated receipt
  }
  ```

### 3.5 Register command in lib.rs
- File: `src-tauri/src/lib.rs`
- Add `update_receipt_currency` to `invoke_handler`

### 3.6 Write integration tests
- File: `src-tauri/src/commands.rs` (in test module)
- Add tests:
  - `test_process_receipt_eur` — EUR receipt gets total_price_eur populated
  - `test_process_receipt_czk` — CZK receipt has no total_price_eur
  - `test_update_receipt_currency` — User can set currency and EUR value

## Phase 4: Frontend - Types & API

### 4.1 Update TypeScript types
- File: `src/lib/types.ts`
- Add to `FieldConfidence` interface (line ~123):
  ```typescript
  currency: ConfidenceLevel;
  ```
- Add to `Receipt` interface (line ~129, after `totalPriceEur`):
  ```typescript
  originalAmount: number | null;
  originalCurrency: string | null;
  ```

### 4.2 Add API function
- File: `src/lib/api.ts`
- Add new function:
  ```typescript
  export async function updateReceiptCurrency(
      receiptId: string,
      originalAmount: number | null,
      originalCurrency: string | null,
      totalPriceEur: number | null
  ): Promise<Receipt> {
      return invoke('update_receipt_currency', {
          receiptId,
          originalAmount,
          originalCurrency,
          totalPriceEur
      });
  }
  ```

### 4.3 Add i18n keys
- File: `src/lib/i18n/sk/index.ts`
- Add keys:
  ```typescript
  receipts: {
      // ... existing keys ...
      currency: 'Mena',
      currencyEur: 'EUR (Euro)',
      currencyCzk: 'CZK (Česká koruna)',
      currencyHuf: 'HUF (Maďarský forint)',
      currencyPln: 'PLN (Poľský zlotý)',
      originalAmount: 'Pôvodná suma',
      amountInEur: 'Suma v EUR',
      enterEurAmount: 'Zadajte sumu v EUR',
      convertedFrom: 'Konvertované z',
  }
  ```
- File: `src/lib/i18n/en/index.ts`
- Add equivalent English translations

## Phase 5: Frontend - Doklady View

### 5.1 Update receipt display in Doklady page
- File: `src/routes/doklady/+page.svelte`
- Update price display logic:
  ```svelte
  {#if receipt.originalCurrency === 'EUR' || !receipt.originalCurrency}
      {receipt.totalPriceEur?.toFixed(2)} €
  {:else if receipt.totalPriceEur}
      {receipt.originalAmount} {receipt.originalCurrency} → {receipt.totalPriceEur.toFixed(2)} €
  {:else}
      <span class="warning">{receipt.originalAmount} {receipt.originalCurrency} → ⚠️</span>
  {/if}
  ```

### 5.2 Add warning styling for unconverted receipts
- Add CSS class for warning state (yellow/orange indicator)
- Show tooltip explaining conversion needed

## Phase 6: Frontend - Receipt Edit Modal (NEW)

### 6.1 Create ReceiptEditModal.svelte component
- File: `src/lib/components/ReceiptEditModal.svelte` (NEW FILE)
- Props: `receipt: Receipt`, `onSave`, `onClose`
- Form fields:
  - Original amount (number input)
  - Currency (dropdown: EUR, CZK, HUF, PLN)
  - Amount in EUR (number input, disabled if EUR selected)
  - Date (existing field)
  - Vendor name / Station name (existing fields)
  - Cost description (existing field)

### 6.2 Add currency dropdown component
- Currency options: EUR, CZK, HUF, PLN
- When EUR selected: hide or auto-fill "Amount in EUR" from original
- When foreign selected: "Amount in EUR" is required

### 6.3 Add form validation
- Original amount: required, positive number
- Currency: required, one of EUR/CZK/HUF/PLN
- Amount in EUR: required if currency ≠ EUR, positive number

### 6.4 Wire up save logic
- Call `updateReceiptCurrency()` API function
- Handle success: close modal, refresh receipt list, show toast
- Handle error: show error message

### 6.5 Add edit button to Doklady page
- File: `src/routes/doklady/+page.svelte`
- Add edit (pencil) icon button to receipt card actions
- Wire up to open `ReceiptEditModal`

### 6.6 Add modal state management
- Add state: `editingReceipt: Receipt | null`
- Open modal: `editingReceipt = receipt`
- Close modal: `editingReceipt = null`

## Phase 7: Finalization

### 7.1 Manual testing checklist
- [ ] Scan EUR receipt → currency detected, total_price_eur populated
- [ ] Scan CZK receipt → currency detected, total_price_eur is null
- [ ] Edit CZK receipt → set EUR value → saves correctly
- [ ] Doklady view shows "100 CZK → 3,95 €" format
- [ ] Receipt matching works after EUR conversion
- [ ] Old receipts (no currency fields) still display correctly

### 7.2 Run test suite
```bash
cd src-tauri && cargo test
npm run test:integration:tier1
```

### 7.3 Update CHANGELOG.md
- Add entry under [Unreleased]:
  ```markdown
  ### Added
  - Multi-currency receipt support (EUR, CZK, HUF, PLN)
  - Receipt edit modal for currency conversion
  ```

## Files Summary

| File | Action | Changes |
|------|--------|---------|
| `migrations/.../up.sql` | CREATE | Add currency columns |
| `migrations/.../down.sql` | CREATE | No-op (SQLite limitation) |
| `schema.rs` | MODIFY | Add new columns |
| `models.rs` | MODIFY | Add fields to Receipt, ReceiptRow, NewReceiptRow |
| `gemini.rs` | MODIFY | Update ExtractedReceipt, prompt, schema |
| `commands.rs` | MODIFY | Add update_receipt_currency command |
| `lib.rs` | MODIFY | Register new command |
| `db.rs` | MODIFY | Update create/update receipt |
| `db_tests.rs` | MODIFY | Add currency tests |
| `types.ts` | MODIFY | Add Receipt currency fields |
| `api.ts` | MODIFY | Add updateReceiptCurrency function |
| `i18n/sk/index.ts` | MODIFY | Add currency translations |
| `i18n/en/index.ts` | MODIFY | Add currency translations |
| `doklady/+page.svelte` | MODIFY | Update display, add edit button |
| `ReceiptEditModal.svelte` | CREATE | New modal component |

## Dependencies Graph

```
Phase 1 (DB + Models)
       │
       ▼
Phase 2 (Gemini OCR)
       │
       ▼
Phase 3 (Processing + Command)
       │
       ▼
Phase 4 (Types + API)
       │
       ├──→ Phase 5 (Doklady View)
       │
       └──→ Phase 6 (Edit Modal)
                │
                ▼
          Phase 7 (Finalize)
```

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| OCR misdetects currency | Confidence field + user can edit |
| Existing receipts break | NULL defaults, backward-compatible |
| Matching stops working | Unchanged, uses total_price_eur |
| User forgets to convert | Warning UI + unconverted won't match |
