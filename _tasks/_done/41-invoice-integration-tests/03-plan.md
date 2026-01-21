# Implementation Plan: Invoice Integration Tests

**Date:** 2026-01-20
**Updated:** 2026-01-21
**Related:** Task 42 (Receipt Mismatch Reasons) - ✅ Complete

## Context: What Task 42 Already Implemented

Task 42 added mismatch reason display for **receipt verification**:
- ✅ `MismatchReason` enum (`DateMismatch`, `LitersMismatch`, `PriceMismatch`, etc.)
- ✅ `ReceiptVerification.mismatch_reason` field
- ✅ Frontend displays mismatch in receipt card
- ✅ i18n strings (Slovak + English)
- ✅ Unit tests in `commands_tests.rs`

**This task focuses on:** Integration tests for the full receipt workflow, including the **trip assignment** mismatch UI (different from verification).

---

## Phase 1: Backend Mock Infrastructure

### 1.1 Add Mock Gemini Support
**File:** `src-tauri/src/gemini.rs`

> **Note:** `wdio.conf.ts` line 166 currently sets `KNIHA_JAZD_MOCK_GEMINI = 'true'` but this is dead code (nothing checks it). We'll implement the DIR-based approach and update wdio.conf.ts accordingly.

- Add `KNIHA_JAZD_MOCK_GEMINI_DIR` environment variable check
- When set, load mock JSON instead of calling Gemini API
- Match mock file by receipt filename stem (e.g., `invoice.pdf` → `invoice.json`)

```rust
// In gemini.rs extract_from_image() or receipts.rs process_receipt_with_gemini()
if let Ok(mock_dir) = std::env::var("KNIHA_JAZD_MOCK_GEMINI_DIR") {
    return load_mock_extraction(&mock_dir, file_path);
}

fn load_mock_extraction(mock_dir: &str, file_path: &Path) -> Result<ExtractedReceipt> {
    let stem = file_path.file_stem().unwrap();
    let mock_file = Path::new(mock_dir).join(format!("{}.json", stem.to_string_lossy()));

    if mock_file.exists() {
        let json = std::fs::read_to_string(&mock_file)?;
        return serde_json::from_str(&json).map_err(|e| e.to_string());
    }

    // No mock found - return empty/pending result
    Ok(ExtractedReceipt::default())
}
```

**Also update:** `wdio.conf.ts` - replace boolean flag with DIR path (see Phase 2.2)

### 1.2 Create Mock JSON Schema
**File:** `tests/integration/data/mocks/invoice.json`

> **Current format is WRONG.** The existing `invoice.json` uses:
> ```json
> { "price": 91.32, "date": "2026-01-20", "litres": 63.680, "station": "Slovnaft, a.s." }
> ```
> This will cause deserialization errors. Must match `ExtractedReceipt` struct (`gemini.rs:8-18`).

**Correct format** (matches `ExtractedReceipt` struct):
```json
{
    "liters": 63.68,
    "total_price_eur": 91.32,
    "receipt_date": "2026-01-20",
    "station_name": "Slovnaft, a.s.",
    "station_address": "Prístavna ulica, Bratislava",
    "vendor_name": null,
    "cost_description": null,
    "raw_text": null,
    "confidence": {
        "liters": "High",
        "total_price": "High",
        "date": "High"
    }
}
```

**Field mapping from original:**
| Original | Correct | Notes |
|----------|---------|-------|
| `price` | `total_price_eur` | Renamed |
| `date` | `receipt_date` | Renamed |
| `litres` | `liters` | Renamed (US spelling) |
| `station` | `station_name` | Renamed |
| - | `confidence` | **Must add** (required by struct) |

### 1.3 Add Backend Unit Tests
**File:** `src-tauri/src/gemini.rs` or `gemini_tests.rs`

- Test mock file loading
- Test fallback when no mock exists
- Test malformed mock JSON handling

### 1.4 Receipt Seeding Strategy (IMPORTANT)

> **No `create_receipt` command exists.** Receipts can ONLY be created via folder scanning (`scan_receipts`). This means:

**Option A: Scan Workflow (Recommended)**
1. Place test PDF in receipts folder
2. Call `scan_receipts` → creates Pending receipt
3. Call `sync_receipts` or `reprocess_receipt` → mock Gemini returns test data
4. Receipt is now ready for testing

**Option B: Add Test-Only Command**
Add `seed_receipt` Tauri command that directly inserts parsed receipts (bypasses scan+OCR):
```rust
#[tauri::command]
#[cfg(debug_assertions)]  // Only in debug builds
pub fn seed_receipt(receipt: Receipt) -> Result<Receipt, String> {
    // Direct insert for testing
}
```

**Decision:** Use **Option A** for this task (no code changes needed). Phase 1 mock infrastructure enables this workflow.

**Dependency:** Phase 3 tests CANNOT run until Phase 1 + Phase 2 are complete.

---

## Phase 2: Test Infrastructure Setup

### 2.1 Reorganize Test Data

**Current structure:**
```
tests/integration/data/
├── invoice.pdf     # At root
└── invoice.json    # At root (wrong format)
```

**Target structure:**
```
tests/integration/data/
├── invoices/                    # Files to be scanned
│   └── invoice.pdf
├── mocks/                       # Mock LLM responses
│   └── invoice.json             # Updated to correct format
└── README.md                    # Document usage
```

**Commands to execute:**
```bash
# Create directories
mkdir -p tests/integration/data/invoices
mkdir -p tests/integration/data/mocks

# Move files
mv tests/integration/data/invoice.pdf tests/integration/data/invoices/
mv tests/integration/data/invoice.json tests/integration/data/mocks/

# Update invoice.json content (see Phase 1.2 for correct format)
```

**Important:** Update `invoice.json` content AFTER moving (Phase 1.2 defines the correct schema).

### 2.2 Update WebDriverIO Config
**File:** `wdio.conf.ts` or test setup

> **Note:** `wdio.conf.ts` line 166 has dead code `KNIHA_JAZD_MOCK_GEMINI = 'true'`. Replace with DIR-based approach.

**Environment variables:**
```typescript
// In wdio.conf.ts beforeSession or env block
process.env.KNIHA_JAZD_MOCK_GEMINI_DIR = path.join(__dirname, 'data/mocks');
```

**Receipts folder configuration:**
> **IMPORTANT:** The env var `KNIHA_JAZD_RECEIPTS_FOLDER` is NOT implemented in Rust code.
> Receipt scanning reads from `LocalSettings.receipts_folder_path`.

**Option A: Set via settings in test setup** (Recommended)
```typescript
// In beforeEach or test setup
await browser.execute(async (folderPath) => {
    await window.__TAURI__.core.invoke('set_receipts_folder_path', {
        path: folderPath
    });
}, path.join(__dirname, 'data/invoices'));
```

**Option B: Implement env var check in Rust** (More work)
Add env var check in `scan_receipts` command to override `LocalSettings`.

**Test isolation:**
- Clean receipts table between tests via `delete_receipt` or test DB reset
- Each test should seed its own data

### 2.3 Add Receipt Seeding via IPC
**File:** `tests/integration/utils/db.ts`

Add helpers to trigger receipt scanning and processing:
```typescript
/**
 * Scan receipts folder for new files (creates Pending receipts)
 */
async function triggerReceiptScan(): Promise<void> {
    await browser.execute(async () => {
        return await window.__TAURI__.core.invoke('scan_receipts');
    });
}

/**
 * Process all pending receipts (uses mock Gemini when KNIHA_JAZD_MOCK_GEMINI_DIR is set)
 */
async function syncReceipts(): Promise<void> {
    await browser.execute(async () => {
        return await window.__TAURI__.core.invoke('sync_receipts');
    });
}

/**
 * Reprocess a single receipt by ID
 * NOTE: Command is 'reprocess_receipt', not 'process_receipt'
 */
async function reprocessReceipt(receiptId: string): Promise<void> {
    await browser.execute(async (id) => {
        return await window.__TAURI__.core.invoke('reprocess_receipt', { id });
    }, receiptId);
}
```

**Typical test workflow:**
1. `triggerReceiptScan()` - finds invoice.pdf, creates Pending receipt
2. `syncReceipts()` - processes all pending (mock returns invoice.json data)
3. Receipt is now `Parsed` and ready for assignment tests

## Phase 3: Enable Skipped Tests

### 3.1 Receipt Display Test
**File:** `tests/integration/specs/tier2/receipts.spec.ts`

```typescript
it('should display pre-seeded receipts in list', async () => {
    // 1. Trigger scan (picks up invoice.pdf)
    await triggerReceiptScan();

    // 2. Process with mock (reads invoice.json)
    const receipts = await getReceipts(2026);
    await processReceiptWithMock(receipts[0].id);

    // 3. Navigate and verify
    await navigateTo('doklady');
    const receiptCards = await $$(Doklady.receiptCard);
    expect(receiptCards.length).toBeGreaterThan(0);

    // 4. Verify mock data displayed
    const stationName = await $('*=Slovnaft');
    expect(await stationName.isDisplayed()).toBe(true);
});
```

### 3.2 Receipt Assignment Test
```typescript
it('should assign receipt to trip with matching data', async () => {
    // Seed trip with MATCHING data (63.68L, 91.32 EUR, 2026-01-20)
    const trip = await seedTrip({
        vehicleId: vehicle.id,
        date: '2026-01-20',
        fuelLiters: 63.68,
        fuelCostEur: 91.32,
        // ...
    });

    // Scan + process receipt
    await triggerReceiptScan();
    await processReceiptWithMock(receiptId);

    // Assign via UI
    await navigateTo('doklady');
    // ... click assign, verify success
});
```

### 3.3 Mismatch Detection Tests

Per **ADR-008 (Backend-Only Calculations)**, mismatch detection logic lives in Rust.

**Testing Strategy:**
- **Unit tests (Rust):** All 7 mismatch combinations + 2 success cases
- **Integration test (WebDriverIO):** ONE test to verify IPC returns `mismatch_reason` and UI displays it

#### 3.3.1 Backend Unit Tests (in `commands.rs` or `commands_tests.rs`)

> **CHECK EXISTING TESTS FIRST.** The following tests already exist in `commands.rs` (lines 4656-4803):
> - `test_get_trips_for_receipt_assignment_empty_trip_returns_can_attach_true` ✅
> - `test_get_trips_for_receipt_assignment_matching_fuel_returns_can_attach_true` ✅
> - `test_get_trips_for_receipt_assignment_different_liters_returns_can_attach_false` ✅
> - `test_get_trips_for_receipt_assignment_different_price_returns_can_attach_false` ✅
> - `test_get_trips_for_receipt_assignment_different_date_returns_can_attach_false` ✅

**Before adding tests, verify coverage:**
```bash
cd src-tauri && cargo test get_trips_for_receipt_assignment --no-run -- --list
```

| # | Mismatch Reason | Status | Notes |
|---|-----------------|--------|-------|
| 1 | `"date"` | ✅ Exists | `different_date` test |
| 2 | `"liters"` | ✅ Exists | `different_liters` test |
| 3 | `"price"` | ✅ Exists | `different_price` test |
| 4 | `"date_and_liters"` | ⬜ Check | May need to add |
| 5 | `"date_and_price"` | ⬜ Check | May need to add |
| 6 | `"liters_and_price"` | ⬜ Check | May need to add |
| 7 | `"all"` | ⬜ Check | May need to add |
| 8 | `"matches"` | ✅ Exists | `matching_fuel` test |
| 9 | `"empty"` | ✅ Exists | `empty_trip` test |

**Only add tests for missing combination cases (4-7) if not already covered.**

```rust
// Example Rust unit test structure (commands_tests.rs)
#[test]
fn test_mismatch_reason_liters_only() {
    let receipt = Receipt { liters: 63.68, price: 91.32, date: "2026-01-20" };
    let trip = Trip { fuel_liters: 40.0, fuel_cost: 91.32, date: "2026-01-20" };

    let result = check_receipt_trip_compatibility(&receipt, &trip);

    assert_eq!(result.status, "differs");
    assert_eq!(result.mismatch_reason, Some("liters".to_string()));
    assert!(!result.can_attach);
}
```

#### 3.3.2 Integration Test (ONE test for E2E verification)

```typescript
describe('Mismatch Detection E2E', () => {
    it('should return mismatch_reason via IPC and display in UI', async () => {
        // Seed trip with liters mismatch (any mismatch type works)
        const trip = await seedTrip({
            date: '2026-01-20',
            fuelLiters: 40.0,        // Different from receipt's 63.68
            fuelCostEur: 91.32,
        });

        await triggerReceiptScan();
        await processReceiptWithMock(receiptId);

        // 1. Verify IPC returns mismatch_reason
        const result = await getTripsForReceiptAssignment(receiptId, vehicleId, 2026);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('differs');
        expect(tripMatch.mismatch_reason).toBe('liters');  // Backend logic
        expect(tripMatch.can_attach).toBe(false);

        // 2. Verify UI displays mismatch indicator
        await navigateTo('doklady');
        const assignBtn = await $('[data-testid="assign-receipt-btn"]');
        await assignBtn.click();
        await browser.pause(300);

        const mismatchIndicator = await $('[data-testid="mismatch-reason"]');
        expect(await mismatchIndicator.isDisplayed()).toBe(true);
    });
});
```

**Why only one integration test?**
- Mismatch logic is pure Rust function → fast unit tests cover all cases
- Integration test verifies the "glue": IPC serialization + UI rendering
- Avoids slow, redundant E2E tests for backend logic

## Phase 4: CI Integration

### 4.1 Update Test Workflow
**File:** `.github/workflows/test.yml`

Ensure env vars set before integration tests:
```yaml
- name: Run integration tests
  env:
    KNIHA_JAZD_MOCK_GEMINI_DIR: ${{ github.workspace }}/tests/integration/data/mocks
    KNIHA_JAZD_RECEIPTS_FOLDER: ${{ github.workspace }}/tests/integration/data/invoices
```

### 4.2 Add Helper Functions
**File:** `tests/integration/utils/db.ts`

```typescript
/**
 * Get trips available for receipt assignment with compatibility info
 */
async function getTripsForReceiptAssignment(
    receiptId: string,
    vehicleId: string,
    year: number
): Promise<Array<{
    trip: Trip;
    can_attach: boolean;
    attachment_status: string;  // "matches" | "differs" | "empty"
    mismatch_reason: string | null;  // "date" | "liters" | "price" | "date_and_*" | "all"
}>> {
    return await browser.execute(
        async (rId, vId, y) => {
            return await window.__TAURI__.core.invoke('get_trips_for_receipt_assignment', {
                receiptId: rId,
                vehicleId: vId,
                year: y,
            });
        },
        receiptId,
        vehicleId,
        year
    );
}
```

### 4.3 Add More Test Fixtures

Create additional mock scenarios in `tests/integration/data/mocks/`:

| File | Purpose | Key Fields |
|------|---------|------------|
| `invoice.json` | Standard fuel receipt (existing) | 63.68L, €91.32, 2026-01-20 |
| `blurry-receipt.json` | Low confidence extraction | All fields with `"Low"` confidence |
| `car-wash.json` | Other cost (no fuel) | `liters: null`, `cost_description: "Car wash"` |
| `partial.json` | Missing fields | Only `total_price_eur` set |
| `different-date.json` | For testing date-only mismatch | Same L + €, different date |

## Deliverables Checklist

### Phase 1: Backend Infrastructure
- [ ] Mock Gemini support in `gemini.rs` (`KNIHA_JAZD_MOCK_GEMINI_DIR`)
- [ ] Mock JSON format matches `ExtractedReceipt` struct
- [ ] Backend unit tests for mock loading
- [ ] Fallback behavior when no mock exists

### Phase 2: Test Infrastructure
- [ ] Reorganize `tests/integration/data/` structure
- [ ] Add `getTripsForReceiptAssignment()` helper
- [ ] Update `wdio.conf.ts` with env vars
- [ ] Document mock format in README

### Phase 3: Mismatch Detection Tests

**Verification Mismatch (Task 42 - ✅ Already Done):**
- [x] `MismatchReason` enum in `models.rs`
- [x] `ReceiptVerification.mismatch_reason` field
- [x] Unit tests for verification mismatch reasons
- [x] Frontend displays mismatch in receipt card
- [x] i18n strings

**Trip Assignment Mismatch (Task 41 - Check/Add):**

Before adding tests, check `commands_tests.rs` for existing coverage of `TripForAssignment.mismatch_reason`:
- [ ] Check if assignment mismatch tests exist
- [ ] Add missing tests for `"date"`, `"liters"`, `"price"` etc. if needed
- [ ] Add `"matches"` test (exact match → `can_attach: true`)
- [ ] Add `"empty"` test (no fuel → `can_attach: true`)

**Integration Test** (1 test - E2E verification):
- [ ] Verify `get_trips_for_receipt_assignment` returns `mismatch_reason`
- [ ] Verify assignment modal UI displays the reason

### Phase 4: Other Receipt Integration Tests
- [ ] Enable "display receipts" test
- [ ] Enable "assign receipt" test
- [ ] Enable "filter by status" test
- [ ] Enable "delete receipt" test

### Phase 5: CI & Documentation
- [ ] CI workflow updated with env vars
- [ ] Documentation in `tests/CLAUDE.md`
- [ ] Add additional mock fixtures

## Estimated Effort

| Phase | Complexity | Dependencies |
|-------|------------|--------------|
| Phase 1 | Medium | None |
| Phase 2 | Low | Phase 1 |
| Phase 3 | Medium | Phase 2 |
| Phase 4 | Low | Phase 3 |

**Total:** ~4 hours of focused work
