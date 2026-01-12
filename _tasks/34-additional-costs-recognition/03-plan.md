# Implementation Plan: Additional Costs Invoice Recognition

**Date:** 2026-01-12
**Status:** Ready for Implementation
**Revised:** Simplified based on user decisions

## Summary of Changes

Original plan: ~13 hours with complex `ReceiptType` enum
Simplified plan: **~6 hours** with binary `liters != null` detection

---

## Phase 1: Backend (Day 1, ~3h)

### Step 1.1: Database Migration
**File:** `src-tauri/migrations/2026-01-12-HHMMSS-add_receipt_cost_fields/up.sql`

```sql
ALTER TABLE receipts ADD COLUMN vendor_name TEXT;
ALTER TABLE receipts ADD COLUMN cost_description TEXT;
```

**File:** `src-tauri/migrations/2026-01-12-HHMMSS-add_receipt_cost_fields/down.sql`
```sql
-- SQLite doesn't support DROP COLUMN, leave columns (additive migration)
```

Run: `cd src-tauri && diesel migration run`

### Step 1.2: Update Diesel Schema
Schema auto-updates when migration runs. Verify `schema.rs` has new columns.

### Step 1.3: Update Receipt Model
**File:** `src-tauri/src/models.rs`

Add to `Receipt` struct:
```rust
pub vendor_name: Option<String>,
pub cost_description: Option<String>,
```

Add to `ReceiptRow` struct:
```rust
pub vendor_name: Option<String>,
pub cost_description: Option<String>,
```

Add to `NewReceiptRow` struct:
```rust
pub vendor_name: Option<&'a str>,
pub cost_description: Option<&'a str>,
```

Update `From<ReceiptRow> for Receipt` implementation.

### Step 1.4: Update db.rs
**File:** `src-tauri/src/db.rs`

Update `create_receipt()` and `update_receipt()` to handle new fields.

### Step 1.5: Update Gemini Prompt
**File:** `src-tauri/src/gemini.rs`

Update `EXTRACTION_PROMPT` to handle both fuel and other costs:
- Extract `vendor_name` and `cost_description` for non-fuel
- Set `liters = null` when not a fuel receipt

Update `ExtractedReceipt` struct:
```rust
pub vendor_name: Option<String>,
pub cost_description: Option<String>,
```

### Step 1.6: Update Assignment Command
**File:** `src-tauri/src/commands.rs`

Modify `assign_receipt_to_trip()`:
```rust
pub fn assign_receipt_to_trip(...) -> Result<Receipt, String> {
    // ... existing receipt lookup ...

    // Check if this is a fuel or other cost receipt
    if receipt.liters.is_some() {
        // FUEL: existing behavior (just link receipt to trip)
        // Trip fuel fields already populated by user
    } else {
        // OTHER COST: populate trip.other_costs_* fields
        let trip = db.get_trip(&trip_id)?;

        // Check for collision
        if trip.other_costs_eur.is_some() {
            return Err("Jazda už má iné náklady".to_string());
        }

        // Build note from receipt data
        let note = match (&receipt.vendor_name, &receipt.cost_description) {
            (Some(v), Some(d)) => format!("{}: {}", v, d),
            (Some(v), None) => v.clone(),
            (None, Some(d)) => d.clone(),
            (None, None) => "Iné náklady".to_string(),
        };

        // Update trip with other costs
        let mut updated_trip = trip.clone();
        updated_trip.other_costs_eur = receipt.total_price_eur;
        updated_trip.other_costs_note = Some(note);
        db.update_trip(&updated_trip)?;
    }

    // Mark receipt as assigned
    receipt.trip_id = Some(trip_uuid);
    receipt.vehicle_id = Some(vehicle_uuid);
    receipt.status = ReceiptStatus::Assigned;
    db.update_receipt(&receipt)?;

    Ok(receipt.clone())
}
```

### Step 1.7: Write Backend Tests
**File:** `src-tauri/src/gemini.rs` (test module)

- Test parsing receipt with liters (fuel)
- Test parsing receipt without liters (other cost)
- Test `vendor_name` and `cost_description` extraction

**File:** `src-tauri/src/commands.rs` (test module)

- Test assignment of other cost to trip
- Test collision rejection when trip already has other_costs

---

## Phase 2: Frontend (~2h)

### Step 2.1: Update TypeScript Types
**File:** `src/lib/types.ts`

```typescript
export interface Receipt {
  // ... existing fields ...
  vendorName: string | null;
  costDescription: string | null;
}
```

### Step 2.2: Update Doklady Page
**File:** `src/routes/doklady/+page.svelte`

Add binary filter:
```svelte
<select bind:value={typeFilter}>
  <option value="all">Všetky</option>
  <option value="fuel">⛽ Tankovanie</option>
  <option value="other">📄 Iné náklady</option>
</select>
```

Filter logic:
```typescript
$: filteredReceipts = receipts.filter(r => {
  if (typeFilter === 'fuel') return r.liters !== null;
  if (typeFilter === 'other') return r.liters === null;
  return true;
});
```

Visual distinction:
```svelte
{#if receipt.liters !== null}
  <span class="icon">⛽</span>
{:else}
  <span class="icon">📄</span>
{/if}
```

### Step 2.3: Update i18n
**File:** `src/lib/i18n/sk/index.ts`

```typescript
receipts: {
  // ... existing ...
  filterAll: 'Všetky',
  filterFuel: 'Tankovanie',
  filterOther: 'Iné náklady',
  otherCost: 'Iné náklady',
  assignmentBlocked: 'Jazda už má iné náklady',
}
```

---

## Phase 3: Polish (~1h)

### Step 3.1: Integration Tests
**File:** `tests/integration/receipts.spec.ts`

- Test: scan folder with mixed receipts (fuel + other)
- Test: assign other cost receipt → verify trip.other_costs populated
- Test: collision rejection

### Step 3.2: Update Changelog
**File:** `CHANGELOG.md`

```markdown
### Pridané
- **Rozpoznávanie iných nákladov** - doklady bez litrov (umytie, parkovanie, servis)
  sa automaticky označia ako iné náklady a pri priradení k jazde vyplnia
  pole "Iné náklady" s popisom z dokladu
```

---

## Verification Checklist

- [ ] Migration runs without error
- [ ] Existing fuel receipts still work
- [ ] New receipts without liters detected as "other cost"
- [ ] Assignment populates `other_costs_eur` and `other_costs_note`
- [ ] Collision blocked with error message
- [ ] Filter works on Doklady page
- [ ] All existing tests pass
- [ ] New tests pass

---

## Rollback Plan

Migration is additive (new columns only). If issues:
1. Deploy fix
2. Columns remain but unused until fix deployed
3. No data loss risk
