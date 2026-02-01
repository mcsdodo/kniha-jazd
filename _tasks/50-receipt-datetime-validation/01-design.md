# Receipt Datetime Validation

## Summary

Upgrade receipts from storing date-only (`receipt_date`) to full datetime (`receipt_datetime`), and add validation that warns when an assigned receipt's datetime falls outside the trip's `[start_datetime, end_datetime]` range.

## Motivation

- Most fuel receipts print both date and time
- Trips now have mandatory `start_datetime` and `end_datetime`
- Validating receipt datetime against trip range catches user errors (e.g., assigning January 15 receipt to January 20 trip)
- Warning-only approach: show red asterisk, don't block assignment

## Design

### 1. Database Migration

Single migration that adds new column, backfills, and drops legacy column:

```sql
-- Migration: replace_receipt_date_with_datetime

-- Add new datetime column
ALTER TABLE receipts ADD COLUMN receipt_datetime TEXT DEFAULT NULL;

-- Backfill from existing receipt_date (assume midnight for existing data)
UPDATE receipts
SET receipt_datetime = receipt_date || 'T00:00:00'
WHERE receipt_date IS NOT NULL;

-- Drop legacy column
ALTER TABLE receipts DROP COLUMN receipt_date;
```

### 2. Model Changes

**models.rs - Receipt struct:**
```rust
pub struct Receipt {
    // ...
    pub receipt_datetime: Option<NaiveDateTime>,  // Replaces receipt_date
    // ...
}
```

**schema.rs:**
```rust
receipts {
    // ...
    receipt_datetime -> Nullable<Text>,  // Was: receipt_date
    // ...
}
```

**TripGridData - new warning field:**
```rust
pub struct TripGridData {
    // ... existing fields ...

    /// Trip IDs where assigned receipt datetime is outside trip's [start, end] range
    pub receipt_datetime_warnings: HashSet<String>,
}
```

### 3. OCR Changes (gemini.rs)

**Prompt update:**
```
"receipt_datetime": "YYYY-MM-DDTHH:MM:SS" or "YYYY-MM-DD" (if time not found),

Rules:
- Date formats: DD.MM.YYYY or DD.MM.YY (European format)
- Time formats: HH:MM or HH:MM:SS (24-hour, common on receipts)
- If time found: return "YYYY-MM-DDTHH:MM:SS"
- If only date found: return "YYYY-MM-DD" (triggers NeedsReview for manual time entry)
```

**Response schema:**
```rust
"receipt_datetime": {
    "type": ["string", "null"],
    "description": "DateTime in YYYY-MM-DDTHH:MM:SS format, or YYYY-MM-DD if time unavailable"
},
```

**Post-processing (receipts.rs):**
```rust
let receipt_datetime = match extracted.receipt_datetime {
    Some(s) if s.len() == 10 => {
        // Date only (YYYY-MM-DD) → needs review for time
        receipt.status = ReceiptStatus::NeedsReview;
        NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
    },
    Some(s) => {
        // Full datetime
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").ok()
    },
    None => None, // NeedsReview - no date at all
};
```

### 4. Backend Validation (commands/mod.rs)

**In `get_trip_grid_data`:**
```rust
let mut receipt_datetime_warnings: HashSet<String> = HashSet::new();

for trip in &trips {
    if let Some(receipt) = receipts.iter().find(|r| r.trip_id == Some(trip.id)) {
        if let Some(receipt_dt) = receipt.receipt_datetime {
            let in_range = receipt_dt >= trip.start_datetime
                && receipt_dt <= trip.end_datetime;

            if !in_range {
                receipt_datetime_warnings.insert(trip.id.to_string());
            }
        }
    }
}
```

**Update existing matching logic (multiple locations):**

Change:
```rust
let date_match = receipt.receipt_date == Some(trip.start_datetime.date());
```

To:
```rust
let datetime_in_range = receipt.receipt_datetime
    .map(|dt| dt >= trip.start_datetime && dt <= trip.end_datetime)
    .unwrap_or(false);
```

### 5. Frontend Changes

**TripGrid.svelte:**
```svelte
<TripRow
    ...
    hasReceiptDatetimeWarning={gridData?.receiptDatetimeWarnings.includes(trip.id)}
    ...
/>
```

**TripRow.svelte - new prop:**
```svelte
export let hasReceiptDatetimeWarning: boolean = false;
```

**TripRow.svelte - indicator display (combined tooltip for yellow + red):**
```svelte
{#if trip.fuelLiters}
    {trip.fuelLiters.toFixed(2)}

    {#if !trip.fullTank || hasReceiptDatetimeWarning}
        {@const tooltipLines = [
            !trip.fullTank ? $LL.trips.partialFillup() : null,
            hasReceiptDatetimeWarning ? $LL.trips.receiptDatetimeMismatch() : null
        ].filter(Boolean)}

        <span class="indicator-group" title={tooltipLines.join('\n')}>
            {#if !trip.fullTank}
                <span class="partial-indicator">*</span>
            {/if}
            {#if hasReceiptDatetimeWarning}
                <span class="datetime-warning-indicator">*</span>
            {/if}
        </span>
    {/if}

    {#if !hasMatchingReceipt}
        <span class="no-receipt-indicator" title={$LL.trips.noReceipt()}>⚠</span>
    {/if}
{/if}
```

**New CSS:**
```css
.datetime-warning-indicator {
    color: var(--accent-error);  /* Red */
    font-weight: bold;
    margin-left: 0.1rem;
}
```

**Legend (TripGrid.svelte):**
```svelte
{#if receiptDatetimeWarningCount > 0}
    <span class="legend-item">
        <span class="datetime-warning-indicator">*</span>
        {$LL.trips.legend.receiptDatetimeMismatch()} ({receiptDatetimeWarningCount})
    </span>
{/if}
```

### 6. Receipts Page

**Datetime input (replaces date-only):**
```svelte
<input type="datetime-local" bind:value={receipt.receiptDatetime} />
```

**Warning when time not extracted:**
```svelte
{#if !hasTimeComponent(receipt.receiptDatetime)}
    <span class="needs-review-badge">{$LL.receipts.timeNotExtracted()}</span>
{/if}
```

### 7. i18n Strings

**Slovak (sk/index.ts):**
```typescript
receiptDatetimeMismatch: 'Dátum/čas dokladu mimo jazdy',
legend: {
    receiptDatetimeMismatch: 'dátum/čas dokladu mimo jazdy',
},
timeNotExtracted: 'Čas nerozpoznaný',
```

**English (en/index.ts):**
```typescript
receiptDatetimeMismatch: 'Receipt datetime outside trip range',
legend: {
    receiptDatetimeMismatch: 'receipt datetime outside trip range',
},
timeNotExtracted: 'Time not extracted',
```

## Files to Modify

### Backend (Rust)
- `src-tauri/migrations/YYYY-MM-DD-NNNNNN_replace_receipt_date_with_datetime/up.sql` (new)
- `src-tauri/migrations/YYYY-MM-DD-NNNNNN_replace_receipt_date_with_datetime/down.sql` (new)
- `src-tauri/src/schema.rs` - update receipts table
- `src-tauri/src/models.rs` - Receipt struct, ReceiptRow, NewReceiptRow, TripGridData
- `src-tauri/src/gemini.rs` - prompt and schema update
- `src-tauri/src/receipts.rs` - datetime parsing
- `src-tauri/src/commands/mod.rs` - validation logic, matching logic updates
- `src-tauri/src/db.rs` - receipt queries

### Frontend (Svelte)
- `src/lib/components/TripGrid.svelte` - pass warning, legend
- `src/lib/components/TripRow.svelte` - display red asterisk, combined tooltip
- `src/lib/components/ReceiptCard.svelte` (or similar) - datetime input
- `src/lib/i18n/sk/index.ts` - Slovak strings
- `src/lib/i18n/en/index.ts` - English strings

### Tests
- `src-tauri/src/commands/commands_tests.rs` - receipt matching tests
- `src-tauri/src/gemini_tests.rs` - OCR parsing tests
- `tests/integration/` - E2E tests for warning display

## Out of Scope

- Re-OCR existing receipts (user action, not automated)
- Blocking assignment when datetime mismatches (warning only)
- Receipt time confidence tracking (just use NeedsReview status)
