# Receipt Verification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace manual receipt assignment with automatic verification matching invoices to trip fill-ups by exact date/liters/price.

**Architecture:** Backend adds `verify_receipts` command and extends trip grid with `has_matching_receipt`. Frontend updates Doklady page with verification summary and Jazdy page with warning indicators.

**Tech Stack:** Rust (Tauri backend), SvelteKit 5 (frontend), SQLite

---

## Task 1: Cleanup - Remove ReceiptPicker

**Files:**
- Delete: `src/lib/components/ReceiptPicker.svelte`
- Modify: `src/lib/components/TripRow.svelte`
- Modify: `src/lib/components/TripGrid.svelte`

**Step 1: Remove ReceiptPicker from TripRow**

In `src/lib/components/TripRow.svelte`:
- Remove import: `import ReceiptPicker from './ReceiptPicker.svelte';`
- Remove prop: `export let onReceiptSelected`
- Remove state: `let showReceiptPicker = false;`
- Remove function: `handleReceiptSelect`
- Remove button: `<button type="button" class="picker-btn">`
- Remove modal: `{#if showReceiptPicker}...{/if}`
- Remove CSS: `.picker-btn` styles

**Step 2: Remove ReceiptPicker wiring from TripGrid**

In `src/lib/components/TripGrid.svelte`:
- Remove `pendingReceiptAssignment` state
- Remove `handleReceiptSelected` function
- Remove `onReceiptSelected` prop from TripRow components
- Remove assignment logic from `handleSaveNew` and `handleUpdate`

**Step 3: Delete ReceiptPicker component**

```bash
rm src/lib/components/ReceiptPicker.svelte
```

**Step 4: Verify app compiles**

```bash
npm run check
```
Expected: No errors related to ReceiptPicker

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove ReceiptPicker component

Manual receipt assignment from trip row removed in favor of
automatic verification system."
```

---

## Task 2: Backend - Add verify_receipts command

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add verification structs to models.rs**

Add to `src-tauri/src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptVerification {
    pub receipt_id: String,
    pub matched: bool,
    pub matched_trip_id: Option<String>,
    pub matched_trip_date: Option<String>,
    pub matched_trip_route: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub total: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub receipts: Vec<ReceiptVerification>,
}
```

**Step 2: Add verify_receipts command to commands.rs**

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn verify_receipts(
    state: tauri::State<'_, AppState>,
    vehicle_id: String,
    year: i32,
) -> Result<VerificationResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all receipts
    let receipts = db.get_receipts().map_err(|e| e.to_string())?;

    // Filter receipts by year
    let year_receipts: Vec<_> = receipts
        .into_iter()
        .filter(|r| {
            r.receipt_date
                .as_ref()
                .map(|d| d.starts_with(&year.to_string()))
                .unwrap_or(false)
        })
        .collect();

    // Get all trips with fuel for this vehicle/year
    let trips = db
        .get_trips_by_vehicle_and_year(&vehicle_id, year)
        .map_err(|e| e.to_string())?;

    let fuel_trips: Vec<_> = trips
        .into_iter()
        .filter(|t| t.fuel_liters.is_some())
        .collect();

    // Match receipts to trips
    let mut verifications = Vec::new();
    let mut matched_count = 0;

    for receipt in &year_receipts {
        let matching_trip = fuel_trips.iter().find(|trip| {
            let date_match = receipt.receipt_date.as_ref() == Some(&trip.date);
            let liters_match = receipt.liters == trip.fuel_liters;
            let price_match = receipt.total_price_eur == trip.fuel_cost_eur;
            date_match && liters_match && price_match
        });

        let verification = if let Some(trip) = matching_trip {
            matched_count += 1;
            ReceiptVerification {
                receipt_id: receipt.id.clone(),
                matched: true,
                matched_trip_id: Some(trip.id.clone()),
                matched_trip_date: Some(trip.date.clone()),
                matched_trip_route: Some(format!("{} → {}", trip.origin, trip.destination)),
            }
        } else {
            ReceiptVerification {
                receipt_id: receipt.id.clone(),
                matched: false,
                matched_trip_id: None,
                matched_trip_date: None,
                matched_trip_route: None,
            }
        };

        verifications.push(verification);
    }

    Ok(VerificationResult {
        total: year_receipts.len(),
        matched: matched_count,
        unmatched: year_receipts.len() - matched_count,
        receipts: verifications,
    })
}
```

**Step 3: Register command in lib.rs**

Add `verify_receipts` to the invoke_handler in `src-tauri/src/lib.rs`.

**Step 4: Verify backend compiles**

```bash
cd src-tauri && cargo check
```
Expected: No errors

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(backend): add verify_receipts command

Matches receipts to trips by exact date/liters/price.
Returns verification status for each receipt."
```

---

## Task 3: Frontend API - Add verification call

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/lib/types.ts`

**Step 1: Add types**

Add to `src/lib/types.ts`:

```typescript
export interface ReceiptVerification {
    receipt_id: string;
    matched: boolean;
    matched_trip_id: string | null;
    matched_trip_date: string | null;
    matched_trip_route: string | null;
}

export interface VerificationResult {
    total: number;
    matched: number;
    unmatched: number;
    receipts: ReceiptVerification[];
}
```

**Step 2: Add API function**

Add to `src/lib/api.ts`:

```typescript
export async function verifyReceipts(vehicleId: string, year: number): Promise<VerificationResult> {
    return invoke('verify_receipts', { vehicleId, year });
}
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat(api): add verifyReceipts function"
```

---

## Task 4: Doklady Page - Verification UI

**Files:**
- Modify: `src/routes/doklady/+page.svelte`

**Step 1: Add verification state and load**

Add state variables:
```typescript
let verification = $state<VerificationResult | null>(null);
let verifying = $state(false);
```

Add verification function:
```typescript
async function loadVerification() {
    const vehicle = $activeVehicleStore;
    if (!vehicle) return;

    verifying = true;
    try {
        verification = await api.verifyReceipts(vehicle.id, $selectedYearStore);
    } catch (error) {
        console.error('Failed to verify receipts:', error);
    } finally {
        verifying = false;
    }
}
```

Call in onMount after loadReceipts:
```typescript
onMount(async () => {
    await loadSettings();
    await loadReceipts();
    await loadVerification();
});
```

Also call after handleSync completes.

**Step 2: Add summary bar**

Add after filters div:
```svelte
{#if verification}
    <div class="verification-summary" class:all-matched={verification.unmatched === 0}>
        {#if verification.unmatched === 0}
            <span class="status-ok">✓ {verification.matched}/{verification.total} dokladov overených</span>
        {:else}
            <span class="status-ok">✓ {verification.matched}/{verification.total} overených</span>
            <span class="status-warning">⚠ {verification.unmatched} neoverených</span>
        {/if}
    </div>
{/if}
```

**Step 3: Add helper function to get verification for receipt**

```typescript
function getVerificationForReceipt(receiptId: string): ReceiptVerification | null {
    return verification?.receipts.find(v => v.receipt_id === receiptId) ?? null;
}
```

**Step 4: Update receipt card badge**

Replace status badge logic with verification-aware version:
```svelte
{@const verif = getVerificationForReceipt(receipt.id)}
{#if verif?.matched}
    <span class="badge success">Overený</span>
{:else if receipt.status === 'NeedsReview'}
    <span class="badge warning">Na kontrolu</span>
{:else}
    <span class="badge danger">Neoverený</span>
{/if}
```

**Step 5: Show matched trip info**

Add below receipt details for matched receipts:
```svelte
{#if verif?.matched}
    <div class="matched-trip">
        Jazda: {verif.matched_trip_date} | {verif.matched_trip_route}
    </div>
{/if}
```

**Step 6: Update actions - only show assign for unmatched**

```svelte
{#if !verif?.matched && receipt.status !== 'Assigned'}
    <button class="button-small" onclick={() => handleAssignClick(receipt)}>
        Prideliť k jazde
    </button>
{/if}
```

**Step 7: Add CSS**

```css
.verification-summary {
    display: flex;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background: #f8f9fa;
    border-radius: 4px;
    margin-bottom: 1rem;
}

.verification-summary.all-matched {
    background: #d4edda;
}

.status-ok {
    color: #155724;
    font-weight: 500;
}

.status-warning {
    color: #856404;
    font-weight: 500;
}

.badge.danger {
    background: #f8d7da;
    color: #721c24;
}

.matched-trip {
    font-size: 0.875rem;
    color: #28a745;
    margin-top: 0.5rem;
}
```

**Step 8: Update filter to use verification**

Change "Nepridelené" filter label to "Neoverené" and update logic:
```typescript
let filteredReceipts = $derived(
    receipts.filter((r) => {
        if (filter === 'unassigned') {
            const verif = getVerificationForReceipt(r.id);
            return !verif?.matched;
        }
        if (filter === 'needs_review') return r.status === 'NeedsReview';
        return true;
    })
);
```

Update filter button label:
```svelte
<button ... onclick={() => (filter = 'unassigned')}>
    Neoverené ({verification?.unmatched ?? 0})
</button>
```

**Step 9: Verify app works**

```bash
npm run tauri dev
```
Test: Navigate to Doklady, verify summary shows, matched receipts show trip info.

**Step 10: Commit**

```bash
git add -A
git commit -m "feat(doklady): add verification UI

- Summary bar showing matched/unmatched counts
- Green badge + trip info for matched receipts
- Red badge + assign button for unmatched
- Updated filter to show unverified receipts"
```

---

## Task 5: Backend - Extend trip grid with has_matching_receipt

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`

**Step 1: Add field to TripWithCalculations**

In `src-tauri/src/models.rs`, add to `TripWithCalculations`:
```rust
pub has_matching_receipt: bool,
```

**Step 2: Update get_trip_grid_data to compute match status**

In `src-tauri/src/commands.rs`, in `get_trip_grid_data`:

After getting trips, get receipts and build a lookup:
```rust
let receipts = db.get_receipts().map_err(|e| e.to_string())?;
```

When building TripWithCalculations, compute has_matching_receipt:
```rust
let has_matching_receipt = if trip.fuel_liters.is_some() {
    receipts.iter().any(|r| {
        let date_match = r.receipt_date.as_ref() == Some(&trip.date);
        let liters_match = r.liters == trip.fuel_liters;
        let price_match = r.total_price_eur == trip.fuel_cost_eur;
        date_match && liters_match && price_match
    })
} else {
    true // No fuel = no receipt needed
};
```

Add to the TripWithCalculations struct initialization:
```rust
has_matching_receipt,
```

**Step 3: Verify backend compiles**

```bash
cd src-tauri && cargo check
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(backend): add has_matching_receipt to trip grid data"
```

---

## Task 6: Frontend - Add types for has_matching_receipt

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: Update TripWithCalculations type**

Add to `TripWithCalculations` interface:
```typescript
has_matching_receipt: boolean;
```

**Step 2: Commit**

```bash
git add -A
git commit -m "feat(types): add has_matching_receipt to TripWithCalculations"
```

---

## Task 7: Jazdy Page - Warning indicator and legend

**Files:**
- Modify: `src/lib/components/TripRow.svelte`
- Modify: `src/lib/components/TripGrid.svelte`

**Step 1: Add hasMatchingReceipt prop to TripRow**

In `src/lib/components/TripRow.svelte`:
```typescript
export let hasMatchingReceipt: boolean = true;
```

**Step 2: Add warning indicator in display mode**

In the display mode (non-editing) fuel cell:
```svelte
<td class="number">
    {#if trip.fuel_liters}
        {trip.fuel_liters.toFixed(2)}
        {#if !trip.full_tank}
            <span class="partial-indicator" title="Čiastočné tankovanie">*</span>
        {/if}
        {#if !hasMatchingReceipt}
            <span class="no-receipt-indicator" title="Bez dokladu">⚠</span>
        {/if}
    {/if}
</td>
```

**Step 3: Add CSS for warning indicator**

```css
.no-receipt-indicator {
    color: #e67e22;
    margin-left: 0.25rem;
    cursor: help;
}
```

**Step 4: Pass prop from TripGrid**

In `src/lib/components/TripGrid.svelte`, pass the prop:
```svelte
<TripRow
    ...
    hasMatchingReceipt={tripData.has_matching_receipt}
/>
```

**Step 5: Add legend below table**

In `src/lib/components/TripGrid.svelte`, after the table:
```svelte
<div class="table-legend">
    <span class="legend-item"><span class="partial-indicator">*</span> čiastočné tankovanie</span>
    <span class="legend-item"><span class="no-receipt-indicator">⚠</span> bez dokladu</span>
    <span class="legend-item"><span class="consumption-warning-sample"></span> vysoká spotreba</span>
</div>
```

**Step 6: Add legend CSS**

```css
.table-legend {
    display: flex;
    gap: 1.5rem;
    padding: 0.75rem 1rem;
    background: #f8f9fa;
    border-radius: 4px;
    margin-top: 1rem;
    font-size: 0.875rem;
    color: #666;
}

.legend-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
}

.partial-indicator {
    color: #ff9800;
    font-weight: bold;
}

.no-receipt-indicator {
    color: #e67e22;
}

.consumption-warning-sample {
    display: inline-block;
    width: 12px;
    height: 12px;
    background: #fff3e0;
    border: 1px solid #ffe0b2;
    border-radius: 2px;
}
```

**Step 7: Verify app works**

```bash
npm run tauri dev
```
Test: Check that trips with fuel but no matching receipt show ⚠ icon. Legend visible below table.

**Step 8: Commit**

```bash
git add -A
git commit -m "feat(jazdy): add no-receipt warning and legend

- Warning icon (⚠) on trips with fuel but no matching receipt
- Legend explaining all indicators below the table"
```

---

## Task 8: Final cleanup and testing

**Step 1: Run type check**

```bash
npm run check
```
Expected: No errors

**Step 2: Run backend tests**

```bash
cd src-tauri && cargo test
```
Expected: All tests pass

**Step 3: Manual testing checklist**

- [ ] Doklady page shows verification summary
- [ ] Matched receipts show green badge + trip info
- [ ] Unmatched receipts show red badge + assign button
- [ ] Filter "Neoverené" works correctly
- [ ] Jazdy page shows ⚠ on trips without matching receipt
- [ ] Legend displays correctly
- [ ] Manual assignment via TripSelectorModal still works

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: receipt verification system complete

- Auto-matches invoices to trips by exact date/liters/price
- Doklady: verification summary + match status per receipt
- Jazdy: warning indicator for trips without receipts
- Removed manual ReceiptPicker in favor of verification"
```

---

## Summary

| Task | Description | Estimated |
|------|-------------|-----------|
| 1 | Cleanup - Remove ReceiptPicker | 5 min |
| 2 | Backend - verify_receipts command | 10 min |
| 3 | Frontend API - verification call | 3 min |
| 4 | Doklady Page - Verification UI | 15 min |
| 5 | Backend - has_matching_receipt | 10 min |
| 6 | Frontend types update | 2 min |
| 7 | Jazdy Page - Warning + legend | 10 min |
| 8 | Final cleanup and testing | 10 min |

**Total: ~65 minutes**
