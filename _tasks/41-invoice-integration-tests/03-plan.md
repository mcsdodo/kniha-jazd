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

### 3.3 Mismatch Detection Tests (NEW - All 7 Cases)

The receipt-trip matching logic compares **date**, **liters**, and **price**.
When any field differs, `mismatch_reason` is set based on the combination.

**Reference values from `invoice.json`:**
- Date: `2026-01-20`
- Liters: `63.68`
- Price: `91.32 EUR`

#### Test Matrix

| Mismatch Reason | Date Match | Liters Match | Price Match | Trip Data (vs receipt) |
|-----------------|------------|--------------|-------------|------------------------|
| `"date"` | ❌ | ✅ | ✅ | Different date only |
| `"liters"` | ✅ | ❌ | ✅ | Different liters only |
| `"price"` | ✅ | ✅ | ❌ | Different price only |
| `"date_and_liters"` | ❌ | ❌ | ✅ | Different date + liters |
| `"date_and_price"` | ❌ | ✅ | ❌ | Different date + price |
| `"liters_and_price"` | ✅ | ❌ | ❌ | Different liters + price |
| `"all"` | ❌ | ❌ | ❌ | Everything differs |

Plus two success cases:
- `"matches"` - all three fields match → can attach
- `"empty"` - trip has no fuel data → can attach freely

#### 3.3.1 Single Field Mismatches

```typescript
describe('Mismatch Detection - Single Field', () => {
    // Receipt: date=2026-01-20, liters=63.68, price=91.32

    it('should detect "date" mismatch when only date differs', async () => {
        const trip = await seedTrip({
            date: '2026-01-15',      // ❌ Different (receipt: 2026-01-20)
            fuelLiters: 63.68,       // ✅ Matches
            fuelCostEur: 91.32,      // ✅ Matches
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('differs');
        expect(tripMatch.mismatch_reason).toBe('date');
        expect(tripMatch.can_attach).toBe(false);
    });

    it('should detect "liters" mismatch when only liters differs', async () => {
        const trip = await seedTrip({
            date: '2026-01-20',      // ✅ Matches
            fuelLiters: 40.0,        // ❌ Different (receipt: 63.68)
            fuelCostEur: 91.32,      // ✅ Matches
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('differs');
        expect(tripMatch.mismatch_reason).toBe('liters');
        expect(tripMatch.can_attach).toBe(false);
    });

    it('should detect "price" mismatch when only price differs', async () => {
        const trip = await seedTrip({
            date: '2026-01-20',      // ✅ Matches
            fuelLiters: 63.68,       // ✅ Matches
            fuelCostEur: 75.00,      // ❌ Different (receipt: 91.32)
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('differs');
        expect(tripMatch.mismatch_reason).toBe('price');
        expect(tripMatch.can_attach).toBe(false);
    });
});
```

#### 3.3.2 Two Field Mismatches

```typescript
describe('Mismatch Detection - Two Fields', () => {
    // Receipt: date=2026-01-20, liters=63.68, price=91.32

    it('should detect "date_and_liters" when date and liters differ', async () => {
        const trip = await seedTrip({
            date: '2026-01-15',      // ❌ Different
            fuelLiters: 40.0,        // ❌ Different
            fuelCostEur: 91.32,      // ✅ Matches
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.mismatch_reason).toBe('date_and_liters');
    });

    it('should detect "date_and_price" when date and price differ', async () => {
        const trip = await seedTrip({
            date: '2026-01-15',      // ❌ Different
            fuelLiters: 63.68,       // ✅ Matches
            fuelCostEur: 75.00,      // ❌ Different
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.mismatch_reason).toBe('date_and_price');
    });

    it('should detect "liters_and_price" when liters and price differ', async () => {
        const trip = await seedTrip({
            date: '2026-01-20',      // ✅ Matches
            fuelLiters: 40.0,        // ❌ Different
            fuelCostEur: 75.00,      // ❌ Different
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.mismatch_reason).toBe('liters_and_price');
    });
});
```

#### 3.3.3 All Fields Mismatch

```typescript
describe('Mismatch Detection - All Fields', () => {
    it('should detect "all" when date, liters, and price all differ', async () => {
        const trip = await seedTrip({
            date: '2026-01-15',      // ❌ Different
            fuelLiters: 40.0,        // ❌ Different
            fuelCostEur: 75.00,      // ❌ Different
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('differs');
        expect(tripMatch.mismatch_reason).toBe('all');
        expect(tripMatch.can_attach).toBe(false);
    });
});
```

#### 3.3.4 Success Cases

```typescript
describe('Mismatch Detection - Success Cases', () => {
    it('should return "matches" when all fields match exactly', async () => {
        const trip = await seedTrip({
            date: '2026-01-20',      // ✅ Matches
            fuelLiters: 63.68,       // ✅ Matches
            fuelCostEur: 91.32,      // ✅ Matches
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('matches');
        expect(tripMatch.mismatch_reason).toBeNull();
        expect(tripMatch.can_attach).toBe(true);
    });

    it('should return "empty" when trip has no fuel data', async () => {
        const trip = await seedTrip({
            date: '2026-01-20',
            fuelLiters: null,        // No fuel
            fuelCostEur: null,       // No cost
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('empty');
        expect(tripMatch.mismatch_reason).toBeNull();
        expect(tripMatch.can_attach).toBe(true);
    });

    it('should allow matching with tolerance (±0.01)', async () => {
        // Backend uses approximate matching for floating point
        const trip = await seedTrip({
            date: '2026-01-20',
            fuelLiters: 63.679,      // Within ±0.01 of 63.68
            fuelCostEur: 91.319,     // Within ±0.01 of 91.32
        });

        const result = await getTripsForReceiptAssignment(receiptId);
        const tripMatch = result.find(t => t.trip.id === trip.id);

        expect(tripMatch.attachment_status).toBe('matches');
        expect(tripMatch.can_attach).toBe(true);
    });
});
```

#### 3.3.5 UI Mismatch Display Test

```typescript
describe('Mismatch UI Display', () => {
    it('should display specific mismatch reason in assignment modal', async () => {
        // Seed trip with liters mismatch
        await seedTrip({
            date: '2026-01-20',
            fuelLiters: 40.0,        // Different from receipt's 63.68
            fuelCostEur: 91.32,
        });

        await triggerReceiptScan();
        await processReceiptWithMock(receiptId);

        await navigateTo('doklady');
        await browser.pause(500);

        // Open assignment modal
        const assignBtn = await $('[data-testid="assign-receipt-btn"]');
        await assignBtn.click();
        await browser.pause(300);

        // Verify mismatch indicator shown
        const mismatchIndicator = await $('[data-testid="mismatch-reason"]');
        expect(await mismatchIndicator.isDisplayed()).toBe(true);

        // Should show "liters" mismatch
        const mismatchText = await mismatchIndicator.getText();
        expect(mismatchText.toLowerCase()).toContain('liter');
    });
});
```

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

### Phase 3: Mismatch Detection Tests (9 tests)
- [ ] Single field: `"date"` mismatch
- [ ] Single field: `"liters"` mismatch
- [ ] Single field: `"price"` mismatch
- [ ] Two fields: `"date_and_liters"` mismatch
- [ ] Two fields: `"date_and_price"` mismatch
- [ ] Two fields: `"liters_and_price"` mismatch
- [ ] All fields: `"all"` mismatch
- [ ] Success: `"matches"` (exact match)
- [ ] Success: `"empty"` (trip has no fuel)

### Phase 4: Other Receipt Tests
- [ ] Enable "display receipts" test
- [ ] Enable "assign receipt" test
- [ ] Enable "filter by status" test
- [ ] Enable "delete receipt" test
- [ ] Tolerance test (±0.01 matching)
- [ ] UI mismatch display test

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
