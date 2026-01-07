# Receipt Vehicle Filtering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Filter receipts to show only unassigned + current vehicle's receipts when switching cars, plus auto-select vehicle on app load.

**Architecture:** Backend filtering via new Tauri command `get_receipts_for_vehicle(vehicle_id, year)` that returns receipts where `vehicle_id IS NULL OR vehicle_id = ?`. Frontend components call this instead of `get_receipts`. Vehicle selector removes empty option and auto-selects first vehicle.

**Tech Stack:** Rust (Tauri backend), TypeScript (SvelteKit frontend), SQLite

---

## Task 1: Backend - Database Query

**Files:**
- Modify: `src-tauri/src/db.rs`
- Test: `src-tauri/src/db.rs` (inline tests)

**Step 1: Write the failing test**

Add to `src-tauri/src/db.rs` tests section:

```rust
#[test]
fn test_get_receipts_for_vehicle_returns_unassigned_and_own() {
    let db = setup_test_db();
    let vehicle_a = create_test_vehicle(&db, "Car A");
    let vehicle_b = create_test_vehicle(&db, "Car B");

    // Create receipts: unassigned, assigned to A, assigned to B
    let unassigned = create_test_receipt(&db, None, None);
    let receipt_a = create_test_receipt(&db, Some(&vehicle_a.id), None);
    let receipt_b = create_test_receipt(&db, Some(&vehicle_b.id), None);

    let results = db.get_receipts_for_vehicle(&vehicle_a.id, None).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.id == unassigned.id));
    assert!(results.iter().any(|r| r.id == receipt_a.id));
    assert!(!results.iter().any(|r| r.id == receipt_b.id));
}

#[test]
fn test_get_receipts_for_vehicle_excludes_other_vehicles() {
    let db = setup_test_db();
    let vehicle_a = create_test_vehicle(&db, "Car A");
    let vehicle_b = create_test_vehicle(&db, "Car B");

    let receipt_b = create_test_receipt(&db, Some(&vehicle_b.id), None);

    let results = db.get_receipts_for_vehicle(&vehicle_a.id, None).unwrap();

    assert!(!results.iter().any(|r| r.id == receipt_b.id));
}

#[test]
fn test_get_receipts_for_vehicle_with_year_filter() {
    let db = setup_test_db();
    let vehicle = create_test_vehicle(&db, "Car A");

    // Receipt with date in 2024
    let receipt_2024 = create_test_receipt_with_date(&db, None, "2024-06-15");
    // Receipt with date in 2025
    let receipt_2025 = create_test_receipt_with_date(&db, None, "2025-06-15");

    let results = db.get_receipts_for_vehicle(&vehicle.id, Some(2024)).unwrap();

    assert!(results.iter().any(|r| r.id == receipt_2024.id));
    assert!(!results.iter().any(|r| r.id == receipt_2025.id));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test test_get_receipts_for_vehicle`
Expected: FAIL with "no method named `get_receipts_for_vehicle`"

**Step 3: Write minimal implementation**

Add to `src-tauri/src/db.rs`:

```rust
pub fn get_receipts_for_vehicle(&self, vehicle_id: &Uuid, year: Option<i32>) -> Result<Vec<Receipt>, String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;

    let base_query = match year {
        Some(y) => format!(
            "SELECT * FROM receipts
             WHERE (vehicle_id IS NULL OR vehicle_id = ?1)
               AND (
                 (receipt_date IS NOT NULL AND CAST(strftime('%Y', receipt_date) AS INTEGER) = ?2)
                 OR (receipt_date IS NULL AND source_year = ?2)
                 OR (receipt_date IS NULL AND source_year IS NULL)
               )
             ORDER BY receipt_date DESC, scanned_at DESC"
        ),
        None => "SELECT * FROM receipts
                 WHERE (vehicle_id IS NULL OR vehicle_id = ?1)
                 ORDER BY receipt_date DESC, scanned_at DESC".to_string(),
    };

    let mut stmt = conn.prepare(&base_query).map_err(|e| e.to_string())?;

    let rows = match year {
        Some(y) => stmt.query_map(params![vehicle_id.to_string(), y], |row| {
            Self::map_receipt_row(row)
        }),
        None => stmt.query_map(params![vehicle_id.to_string()], |row| {
            Self::map_receipt_row(row)
        }),
    }.map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test test_get_receipts_for_vehicle`
Expected: All 3 tests PASS

**Step 5: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add get_receipts_for_vehicle with vehicle filtering"
```

---

## Task 2: Backend - Tauri Command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Add the command**

Add to `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn get_receipts_for_vehicle(
    db: State<Database>,
    vehicle_id: String,
    year: Option<i32>,
) -> Result<Vec<Receipt>, String> {
    let vehicle_uuid = Uuid::parse_str(&vehicle_id)
        .map_err(|e| format!("Invalid vehicle ID: {}", e))?;
    db.get_receipts_for_vehicle(&vehicle_uuid, year)
}
```

**Step 2: Register in main.rs**

Find the `invoke_handler` in `src-tauri/src/main.rs` and add `get_receipts_for_vehicle` to the list.

**Step 3: Run backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS, no compilation errors

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat(commands): expose get_receipts_for_vehicle via Tauri IPC"
```

---

## Task 3: Frontend - API Wrapper

**Files:**
- Modify: `src/lib/api.ts`

**Step 1: Add the API function**

Add to `src/lib/api.ts`:

```typescript
export async function getReceiptsForVehicle(
    vehicleId: string,
    year?: number
): Promise<Receipt[]> {
    return invoke('get_receipts_for_vehicle', {
        vehicleId,
        year: year ?? null
    });
}
```

**Step 2: Verify TypeScript compiles**

Run: `npm run check`
Expected: No TypeScript errors

**Step 3: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): add getReceiptsForVehicle wrapper"
```

---

## Task 4: Frontend - Doklady Page

**Files:**
- Modify: `src/routes/doklady/+page.svelte`

**Step 1: Update receipt loading**

Find the `loadReceipts` function (around line 78-82) and change:

```typescript
// Before:
receipts = await api.getReceipts($selectedYearStore);

// After:
const vehicle = $activeVehicleStore;
if (vehicle) {
    receipts = await api.getReceiptsForVehicle(vehicle.id, $selectedYearStore);
} else {
    receipts = [];
}
```

**Step 2: Update filtered receipts derivation**

The existing filter tabs should work as-is since they filter within the already-filtered `receipts` array:
- "All" shows all (which is now unassigned + this car's)
- "Unassigned" filters where `vehicle_id === null`
- "Needs Review" filters by status

Verify the `filteredReceipts` derived store logic handles this correctly.

**Step 3: Test manually**

Run: `npm run tauri dev`
- Create 2 vehicles
- Add receipts, assign some to each vehicle
- Switch vehicles and verify only relevant receipts show

**Step 4: Commit**

```bash
git add src/routes/doklady/+page.svelte
git commit -m "feat(doklady): filter receipts by active vehicle"
```

---

## Task 5: Frontend - ReceiptIndicator Badge

**Files:**
- Modify: `src/lib/components/ReceiptIndicator.svelte`

**Step 1: Analyze current behavior**

Read the file to understand how it currently counts receipts. It uses `verifyReceipts` which IS vehicle-aware. Check if it needs changes or if it already works correctly.

**Step 2: Update if needed**

If the badge count includes receipts from other vehicles, update to use `getReceiptsForVehicle` for the count calculation.

**Step 3: Test manually**

Run: `npm run tauri dev`
- Verify badge count only reflects unassigned + current vehicle's receipts

**Step 4: Commit (if changes made)**

```bash
git add src/lib/components/ReceiptIndicator.svelte
git commit -m "fix(receipt-indicator): filter badge count by vehicle"
```

---

## Task 6: UX Fix - Remove Empty Vehicle Option

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Find vehicle selector dropdown**

Locate the `<select>` element for vehicle selection (around lines 128-142).

**Step 2: Remove or conditionally hide empty option**

```svelte
<!-- Before: -->
<select on:change={handleVehicleChange}>
    <option value="">Select vehicle</option>
    {#each $vehiclesStore as vehicle}
        <option value={vehicle.id}>{vehicle.name}</option>
    {/each}
</select>

<!-- After: -->
<select on:change={handleVehicleChange}>
    {#if $vehiclesStore.length === 0}
        <option value="">No vehicles</option>
    {/if}
    {#each $vehiclesStore as vehicle}
        <option value={vehicle.id}>{vehicle.name} ({vehicle.license_plate})</option>
    {/each}
</select>
```

**Step 3: Test manually**

Run: `npm run tauri dev`
- With vehicles: no empty option visible
- Without vehicles: shows "No vehicles" or appropriate message

**Step 4: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "fix(layout): remove empty vehicle option when vehicles exist"
```

---

## Task 7: UX Fix - Auto-Select Vehicle on Load

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Find onMount initialization**

Locate the `onMount` block (around lines 65-87) where vehicles are loaded.

**Step 2: Add auto-select logic**

```typescript
onMount(async () => {
    const vehicles = await api.getVehicles();
    vehiclesStore.set(vehicles);

    let activeVehicle = await api.getActiveVehicle();

    // Auto-select first vehicle if none set but vehicles exist
    if (!activeVehicle && vehicles.length > 0) {
        activeVehicle = vehicles[0];
        await api.setActiveVehicle(activeVehicle.id);
    }

    // Handle deleted vehicle: if persisted ID not in list, select first
    if (activeVehicle && !vehicles.find(v => v.id === activeVehicle.id)) {
        if (vehicles.length > 0) {
            activeVehicle = vehicles[0];
            await api.setActiveVehicle(activeVehicle.id);
        } else {
            activeVehicle = null;
        }
    }

    activeVehicleStore.set(activeVehicle);
    // ... rest of initialization
});
```

**Step 3: Test manually**

Run: `npm run tauri dev`
- Clear active vehicle in backend, reload → first vehicle auto-selected
- Delete active vehicle, reload → next vehicle auto-selected

**Step 4: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "fix(layout): auto-select vehicle on app load"
```

---

## Task 8: Final Verification & Changelog

**Step 1: Run all backend tests**

Run: `cd src-tauri && cargo test`
Expected: All tests PASS

**Step 2: Run full app test**

Run: `npm run tauri dev`
Manual verification:
- [ ] Switching vehicles filters receipts correctly
- [ ] Unassigned receipts visible for all vehicles
- [ ] Other vehicle's receipts hidden
- [ ] Badge count reflects filtered receipts
- [ ] No empty vehicle option when vehicles exist
- [ ] Auto-selects vehicle on fresh load

**Step 3: Update changelog**

Run: `/changelog`

Add to [Unreleased]:
```markdown
### Changed
- Receipts page now filters by active vehicle (shows unassigned + current vehicle's receipts)
- Receipt badge count reflects only relevant receipts

### Fixed
- Vehicle selector no longer allows deselecting to empty when vehicles exist
- App auto-selects first vehicle on load if none persisted
```

**Step 4: Final commit**

```bash
git add CHANGELOG.md
git commit -m "docs: update changelog with receipt vehicle filtering"
```

---

## Summary

| Task | Description | Estimated Complexity |
|------|-------------|---------------------|
| 1 | Database query + tests | Medium |
| 2 | Tauri command | Simple |
| 3 | API wrapper | Simple |
| 4 | Doklady page update | Simple |
| 5 | ReceiptIndicator update | Simple (may be no-op) |
| 6 | Remove empty option | Simple |
| 7 | Auto-select vehicle | Medium |
| 8 | Verification + changelog | Simple |

**Total: 8 tasks, TDD approach, frequent commits**
