# Feature: Receipt Scanning & AI OCR

> Automated receipt scanning and AI-powered OCR extraction using Google Gemini for vehicle expense tracking, supporting fuel receipts and other costs across multiple currencies.

## User Flow

1. **Configure Settings**: User sets Gemini API key and receipts folder path in Settings
2. **Organize Receipts**: Place receipt images/PDFs in configured folder (flat or year-based structure)
3. **Scan**: Click "Skenovať" to detect new receipt files without OCR processing
4. **Process with AI**: Click "Spracovať" to send pending receipts to Gemini for OCR extraction
5. **Review**: Receipts with low confidence or foreign currency are flagged for manual review
  - Filters: unassigned/needs review, fuel vs other
  - Per-receipt actions: reprocess, edit fields, delete, open file
  - Year mismatch warning (folder year vs OCR date)
6. **Assign**: Match receipts to trips manually or let verification system check matches automatically

**Read-only mode**: scanning, processing, editing, deleting, reprocessing, and assignment are blocked in read-only mode.

## Technical Implementation

### Folder Structure Detection

The system supports two folder organization modes, detected automatically:

| Structure | Description | Example |
|-----------|-------------|---------|
| **Flat** | All receipt images at root level | `receipts/invoice1.pdf` |
| **YearBased** | Receipts organized in year folders | `receipts/2024/invoice1.pdf`, `receipts/2025/invoice2.jpg` |

Detection rules:
- **Flat**: Only image files at root, no subfolders
- **YearBased**: Only folders named as 4-digit years (2024, 2025, etc.)
- **Invalid**: Mixed structures (files + folders, year + non-year folders)

When using YearBased structure, each receipt's `source_year` is set from its folder name, enabling year-filtered queries.

**Supported file formats**: `jpg`, `jpeg`, `png`, `webp`, `pdf`

### Scanning Process

The scanning flow is split into two phases:

**Phase 1: File Discovery** (`scan_receipts` command)
- Reads configured folder path from local settings
- Calls `scan_folder_for_new_receipts()` which:
  - Detects folder structure (Flat/YearBased)
  - Iterates through supported image files
  - Checks each file against database by file path (deduplication)
  - Creates new `Receipt` records with `Pending` status
- Returns count of new files and any folder structure warnings

**Phase 2: AI Processing** (`process_pending_receipts` command)
- Fetches all receipts with `Pending` status
- For each pending receipt:
  - Emits progress event for UI updates
  - Calls `process_receipt_with_gemini()` async
  - Updates receipt in database with extracted data
  - On error: returns an error entry and keeps the receipt `Pending` in the DB for retry (no DB update on failure)

**Alternative one-shot path** (`sync_receipts` command)
- Scans for new files and immediately processes only those newly discovered receipts
- Returns both `processed` receipts and `errors`
- Errors are returned to the caller; failed receipts remain `Pending` (no DB update on failure)

### Gemini AI Extraction

The Gemini client handles OCR extraction:

**Model**: `gemini-2.5-flash`

**Request Flow**:
1. Read image file and encode as Base64
2. Determine MIME type from extension
3. Send structured prompt with JSON schema for response
4. Parse response into `ExtractedReceipt` struct

**Extraction Prompt** handles:
- **Fuel receipts**: liters, station name/address, total amount
- **Other costs**: vendor name, cost description (car wash, parking, toll, etc.)
- **Multi-currency**: EUR, CZK (Czech), HUF (Hungarian), PLN (Polish)
- **Multi-language**: Slovak, Czech, Hungarian, Polish receipts

**Response Schema:** See `ExtractedReceipt` struct in `gemini.rs:L14` for full field definitions.

Key fields:
- `liters`, `station_name`, `station_address` — Fuel receipts only
- `vendor_name`, `cost_description` — Other costs only
- `original_amount`, `original_currency` — Raw OCR values (EUR/CZK/HUF/PLN)
- `receipt_date` — YYYY-MM-DD format
- `confidence` — Per-field extraction confidence levels

### Confidence Scoring

Each field has an associated confidence level from Gemini:

| Level | Meaning |
|-------|---------|
| `high` | AI is confident in extracted value |
| `medium` | Likely correct but uncertain |
| `low` | Poor image quality or ambiguous data |
| `not_applicable` | Field doesn't apply (e.g., liters for non-fuel receipt) |

**Confidence determines status**:

Receipt is marked `NeedsReview` if:
- Any of `liters`, `station_name`, `station_address` has `Low` or `Unknown` confidence
- Missing critical data (`liters` AND `vendor_name` both null)
- Missing `original_amount` or `receipt_date`
- **Foreign currency** (CZK/HUF/PLN) — user must manually convert to EUR

`not_applicable` is mapped to `Unknown`, which typically triggers `NeedsReview` for non-fuel receipts unless manually edited.

Otherwise, receipt gets `Parsed` status.

### Receipt Status Lifecycle

```
┌─────────┐     scan_folder      ┌─────────┐
│  File   │ ──────────────────►  │ Pending │
└─────────┘                      └────┬────┘
                                      │
                    process_with_gemini()
                                      │
                    ┌─────────────────┴─────────────────┐
                    ▼                                   ▼
            ┌───────────────┐                   ┌─────────────┐
            │  NeedsReview  │                   │   Parsed    │
            │ (low conf /   │                   │ (high conf) │
            │  foreign EUR) │                   └──────┬──────┘
            └───────┬───────┘                          │
                    │                                  │
                    │      user edits / converts       │
                    └───────────────┬──────────────────┘
                                    │
                      assign_receipt_to_trip()
                                    │
                                    ▼
                            ┌────────────┐
                            │  Assigned  │
                            └────────────┘
```

**Status Values**:
- `Pending` — File detected, awaiting OCR processing
- `Parsed` — Successfully extracted with high confidence
- `NeedsReview` — Needs manual verification (low confidence or foreign currency)
- `Assigned` — Linked to a specific trip

### Multi-Currency Handling

Central European receipts often use local currencies:

| Currency | Symbol | Country |
|----------|--------|---------|
| EUR | € | Slovakia, Germany, Austria |
| CZK | Kč | Czech Republic |
| HUF | Ft | Hungary |
| PLN | zł | Poland |

**Processing logic**:
- `original_amount` + `original_currency` store raw OCR values
- EUR receipts: `total_price_eur` auto-populated from `original_amount`
- Foreign currency: `total_price_eur` left as `null` → triggers `NeedsReview`
- User manually converts in edit modal → updates `total_price_eur`

### Trip Assignment

Receipt-to-trip matching supports two paths:

**1. Fuel Receipt Assignment**:
- Compares `receipt_date`, `liters`, and `total_price_eur` with trip's fuel fields
- Exact match (no tolerance) → receipt verifies trip's fuel entry
- Mismatch reasons tracked: `date`, `liters`, `price`, or combinations

**2. Other Cost Assignment**:
- Receipts without liters (car wash, parking, etc.)
- Populates `trip.other_costs_eur` and `trip.other_costs_note`
- Note built from `vendor_name` + `cost_description`

**Assignment Logic** (`commands.rs:L2652`):

| Trip State | Receipt Type | Result |
|------------|--------------|--------|
| No fuel data | Fuel receipt | **Populates** fuel fields (liters, cost, full_tank=true) |
| Has fuel data | Matching fuel receipt | Links receipt (verification) |
| Has fuel data | Non-matching fuel receipt | Attaches as other-cost (if empty) |
| Any | Other-cost receipt | Populates other_costs fields |

A fuel receipt is one with both `liters > 0` and `total_price_eur` set.

### Receipt Verification

The verification system checks receipts against trips for a vehicle/year, filtering receipts by `receipt_date` year only (receipts without date are excluded, even if `source_year` is set):

- Counts `matched` vs `unmatched` receipts
- For each receipt, identifies mismatch reason:
  - `MissingReceiptData` — OCR incomplete
  - `NoFuelTripFound` — No trips have fuel data this year
  - `DateMismatch` — Found matching liters+price but wrong date
  - `LitersMismatch` — Found matching date+price but wrong liters
  - `PriceMismatch` — Found matching date+liters but wrong price
  - `NoOtherCostMatch` — Other-cost receipt with no matching trip

### Mock Mode for Testing

Environment variable `KNIHA_JAZD_MOCK_GEMINI_DIR` enables deterministic testing:

```bash
KNIHA_JAZD_MOCK_GEMINI_DIR=/path/to/mocks npm run test
```

When set:
- API key validation is skipped
- Loads JSON from mock directory instead of calling Gemini
- Mock file naming: `{filename}.json` (e.g., `invoice.pdf` → `invoice.json`)
- Missing mock file returns default response with low confidence

## Key Files

| File | Purpose |
|------|---------|
| [receipts.rs](src-tauri/src/receipts.rs) | Folder scanning, structure detection, Gemini integration |
| [gemini.rs](src-tauri/src/gemini.rs) | Gemini API client, extraction prompt, mock mode |
| [commands.rs](src-tauri/src/commands.rs) | Tauri commands: `scan_receipts`, `process_pending_receipts`, `sync_receipts`, `reprocess_receipt`, `assign_receipt_to_trip`, `get_trips_for_receipt_assignment`, `verify_receipts` |
| [models.rs](src-tauri/src/models.rs) | `Receipt`, `ReceiptStatus`, `ExtractionConfidence` |
| [+page.svelte](src/routes/doklady/+page.svelte) | Receipt list UI, scan/process buttons, assignment flow |
| [api.ts](src/lib/api.ts) | Frontend API wrappers for receipt commands |
| [types.ts](src/lib/types.ts) | TypeScript types for `Receipt`, `ScanResult`, etc. |

## Design Decisions

### Two-Phase Processing (Scan → Process)
Scanning and OCR are separated to allow:
- Quick detection of new files without API costs
- Batch processing with progress feedback
- Retry failed OCR without re-scanning

### Foreign Currency Requires Manual Conversion
Rather than auto-converting currencies (which would need exchange rate API + date matching), the system:
- Stores original amount/currency from OCR
- Flags receipt as `NeedsReview`
- User enters EUR value manually (they know the actual rate used)

### Year-Based Folder Structure
Users often organize receipts by year for archival. Supporting `2024/`, `2025/` folders:
- Sets `source_year` on receipts for filtering
- Validates year mismatches (receipt date vs folder year) in UI
- Handles multi-year scanning efficiently

### Confidence-Based Review Workflow
Not all OCR extractions are reliable. The system:
- Returns per-field confidence from Gemini
- Maps to typed `ExtractionConfidence` enum (not strings)
- Auto-routes uncertain receipts to review queue
- Allows manual verification and editing

### Fuel vs Other Cost Detection
Gemini prompt explicitly distinguishes:
- Fuel receipts: have liters, station info, fuel type indicators
- Other costs: have vendor/description, no fuel markers
- Enables proper trip field population during assignment

## Related

- `_tasks/35-receipt-improvements/` — Receipt UX improvements planning
- `receipts_tests.rs` — Backend unit tests for folder detection, extraction
- `commands_tests.rs` — Tests for assignment logic, verification
