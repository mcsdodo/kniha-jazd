# Implementation Plan: Invoice Integration Tests

## Phase 1: Backend Mock Infrastructure

### 1.1 Add Mock Gemini Support
**File:** `src-tauri/src/gemini.rs`

- Add `KNIHA_JAZD_MOCK_GEMINI_DIR` environment variable check
- When set, load mock JSON instead of calling Gemini API
- Match mock file by receipt filename stem (e.g., `invoice.pdf` → `invoice.json`)

```rust
// Pseudocode
if let Ok(mock_dir) = std::env::var("KNIHA_JAZD_MOCK_GEMINI_DIR") {
    return load_mock_extraction(&mock_dir, file_path);
}
```

### 1.2 Create Mock JSON Schema
**File:** `tests/integration/data/mocks/invoice.json`

Update existing `invoice.json` to match `ExtractedReceipt` struct format:
```json
{
    "liters": 63.68,
    "total_price_eur": 91.32,
    "receipt_date": "2026-01-20",
    "station_name": "Slovnaft, a.s.",
    "station_address": "Prístavna ulica, Bratislava",
    "raw_text": null,
    "confidence": {
        "liters": "High",
        "totalPrice": "High",
        "date": "High"
    }
}
```

### 1.3 Add Backend Unit Tests
**File:** `src-tauri/src/gemini.rs` or `gemini_tests.rs`

- Test mock file loading
- Test fallback when no mock exists
- Test malformed mock JSON handling

## Phase 2: Test Infrastructure Setup

### 2.1 Reorganize Test Data
```
tests/integration/data/
├── invoices/                    # Files to be scanned
│   └── invoice.pdf              # Existing - move here
├── mocks/                       # Mock LLM responses
│   └── invoice.json             # Existing - move & update format
└── README.md                    # Document usage
```

### 2.2 Update WebDriverIO Config
**File:** `wdio.conf.ts` or test setup

- Set `KNIHA_JAZD_MOCK_GEMINI_DIR` env var
- Set receipts folder to `tests/integration/data/invoices`
- Ensure test isolation (clean receipts table between tests)

### 2.3 Add Receipt Seeding via IPC
**File:** `tests/integration/utils/db.ts`

Add helper to trigger receipt scanning:
```typescript
async function triggerReceiptScan(): Promise<void> {
    await browser.execute(async () => {
        return await window.__TAURI__.core.invoke('scan_receipts');
    });
}

async function processReceiptWithMock(receiptId: string): Promise<void> {
    await browser.execute(async (id) => {
        return await window.__TAURI__.core.invoke('process_receipt', { id });
    }, receiptId);
}
```

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

These tests already exist or should be added to the Rust test suite:

| # | Mismatch Reason | Test Scenario |
|---|-----------------|---------------|
| 1 | `"date"` | Date differs, liters + price match |
| 2 | `"liters"` | Liters differs, date + price match |
| 3 | `"price"` | Price differs, date + liters match |
| 4 | `"date_and_liters"` | Date + liters differ, price matches |
| 5 | `"date_and_price"` | Date + price differ, liters matches |
| 6 | `"liters_and_price"` | Liters + price differ, date matches |
| 7 | `"all"` | All three fields differ |
| 8 | `"matches"` | All fields match → `can_attach: true` |
| 9 | `"empty"` | Trip has no fuel → `can_attach: true` |

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

**Rust Unit Tests** (9 tests in `commands_tests.rs`):
- [ ] `"date"` mismatch
- [ ] `"liters"` mismatch
- [ ] `"price"` mismatch
- [ ] `"date_and_liters"` mismatch
- [ ] `"date_and_price"` mismatch
- [ ] `"liters_and_price"` mismatch
- [ ] `"all"` mismatch
- [ ] `"matches"` (exact match → can_attach: true)
- [ ] `"empty"` (no fuel → can_attach: true)

**Integration Test** (1 test - E2E verification):
- [ ] Verify IPC returns `mismatch_reason` and UI displays it

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
