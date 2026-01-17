# Receipt Vehicle Filtering Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Filter receipts to show only unassigned + current vehicle's receipts when switching cars, plus auto-select vehicle on app load.

**Architecture:** Backend filtering via new Tauri command `get_receipts_for_vehicle(vehicle_id, year)` that returns receipts where `vehicle_id IS NULL OR vehicle_id = ?`. Frontend components call this instead of `get_receipts`. Vehicle selector removes empty option and auto-selects first vehicle.

**Tech Stack:** Rust (Tauri backend), TypeScript (SvelteKit frontend), SQLite

---

## Task 1: Backend - Database Query + Index

**Files:**
- Modify: `src-tauri/src/db.rs` (implementation + index)
- Test: `src-tauri/src/db_tests.rs` (separate test file per CLAUDE.md convention)

**Step 0: Add database index for vehicle_id**

In `src-tauri/src/db.rs`, find the `create_tables()` function and add index:

```rust
// Add after existing index creation (around line 116)
conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_receipts_vehicle ON receipts(vehicle_id)",
    [],
)?;
```

**Step 1: Write the failing test**

Add to `src-tauri/src/db_tests.rs` (or existing test module):

```rust
#[test]
fn test_get_receipts_for_vehicle_returns_unassigned_and_own() {
    // Use actual codebase patterns
    let db = Database::in_memory().unwrap();

    // Create vehicles using actual helper + db insert
    let vehicle_a = create_test_vehicle("Car A");
    let vehicle_b = create_test_vehicle("Car B");
    db.create_vehicle(&vehicle_a).unwrap();
    db.create_vehicle(&vehicle_b).unwrap();

    // Create receipts using Receipt::new() + manual vehicle_id assignment
    let mut unassigned = Receipt::new("receipt1.jpg".to_string());
    let mut receipt_a = Receipt::new("receipt2.jpg".to_string());
    receipt_a.vehicle_id = Some(vehicle_a.id);
    let mut receipt_b = Receipt::new("receipt3.jpg".to_string());
    receipt_b.vehicle_id = Some(vehicle_b.id);

    db.create_receipt(&unassigned).unwrap();
    db.create_receipt(&receipt_a).unwrap();
    db.create_receipt(&receipt_b).unwrap();

    let results = db.get_receipts_for_vehicle(&vehicle_a.id, None).unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.id == unassigned.id));
    assert!(results.iter().any(|r| r.id == receipt_a.id));
    assert!(!results.iter().any(|r| r.id == receipt_b.id));
}

#[test]
fn test_get_receipts_for_vehicle_excludes_other_vehicles() {
    let db = Database::in_memory().unwrap();

    let vehicle_a = create_test_vehicle("Car A");
    let vehicle_b = create_test_vehicle("Car B");
    db.create_vehicle(&vehicle_a).unwrap();
    db.create_vehicle(&vehicle_b).unwrap();

    let mut receipt_b = Receipt::new("receipt.jpg".to_string());
    receipt_b.vehicle_id = Some(vehicle_b.id);
    db.create_receipt(&receipt_b).unwrap();

    let results = db.get_receipts_for_vehicle(&vehicle_a.id, None).unwrap();

    assert!(results.is_empty() || !results.iter().any(|r| r.id == receipt_b.id));
}

#[test]
fn test_get_receipts_for_vehicle_with_year_filter() {
    let db = Database::in_memory().unwrap();

    let vehicle = create_test_vehicle("Car A");
    db.create_vehicle(&vehicle).unwrap();

    // Receipt with date in 2024
    let mut receipt_2024 = Receipt::new_with_source_year("r1.jpg".to_string(), 2024);
    receipt_2024.receipt_date = Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());

    // Receipt with date in 2025
    let mut receipt_2025 = Receipt::new_with_source_year("r2.jpg".to_string(), 2025);
    receipt_2025.receipt_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());

    db.create_receipt(&receipt_2024).unwrap();
    db.create_receipt(&receipt_2025).unwrap();

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
            Self::row_to_receipt(row)
        }),
        None => stmt.query_map(params![vehicle_id.to_string()], |row| {
            Self::row_to_receipt(row)
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

## Task 2b: Backend - Vehicle Deletion Cleanup

**Problem:** The `receipts` table has `FOREIGN KEY (vehicle_id) REFERENCES vehicles(id)` but NO cascade. Deleting a vehicle with assigned receipts will fail. Also, receipts pointing to deleted vehicles become orphaned and invisible.

**Files:**
- Modify: `src-tauri/src/db.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_delete_vehicle_unassigns_receipts_first() {
    let db = Database::in_memory().unwrap();

    let vehicle = create_test_vehicle("Car A");
    db.create_vehicle(&vehicle).unwrap();

    let mut receipt = Receipt::new("receipt.jpg".to_string());
    receipt.vehicle_id = Some(vehicle.id);
    db.create_receipt(&receipt).unwrap();

    // Should succeed - receipts unassigned before vehicle deleted
    db.delete_vehicle(&vehicle.id).unwrap();

    // Verify receipt still exists but unassigned
    let receipts = db.get_receipts(None).unwrap();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].vehicle_id.is_none());
}
```

**Step 2: Update delete_vehicle to unassign receipts first**

Modify `delete_vehicle()` in `src-tauri/src/db.rs`:

```rust
pub fn delete_vehicle(&self, id: &Uuid) -> Result<(), String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;

    // Unassign all receipts from this vehicle before deletion
    conn.execute(
        "UPDATE receipts SET vehicle_id = NULL WHERE vehicle_id = ?1",
        params![id.to_string()],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM vehicles WHERE id = ?1",
        params![id.to_string()],
    ).map_err(|e| e.to_string())?;

    Ok(())
}
```

**Step 3: Run test**

Run: `cd src-tauri && cargo test test_delete_vehicle_unassigns`
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "fix(db): unassign receipts before vehicle deletion"
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

**Step 1: Update receipt loading with loading state**

Find the `loadReceipts` function (around line 78-82) and change:

```typescript
// Before:
receipts = await api.getReceipts($selectedYearStore);

// After:
async function loadReceipts() {
    loading = true;  // Always set loading state first
    try {
        const vehicle = $activeVehicleStore;
        if (vehicle) {
            receipts = await api.getReceiptsForVehicle(vehicle.id, $selectedYearStore);
        } else {
            receipts = [];
        }
    } finally {
        loading = false;
    }
}
```

**Step 2: Add vehicle change reactivity**

Find the `$effect` block (around lines 58-67) that watches `$selectedYearStore` and add `$activeVehicleStore`:

```typescript
// Before:
$effect(() => {
    if ($selectedYearStore) {
        loadReceipts();
    }
});

// After:
$effect(() => {
    // Re-fetch when year OR vehicle changes
    const year = $selectedYearStore;
    const vehicle = $activeVehicleStore;
    if (year) {
        loadReceipts();
    }
});
```

**Note:** Reading `$activeVehicleStore` inside the effect creates a reactive dependency - receipts will reload when vehicle changes.

**Step 3: Clarify tab filtering interaction**

The existing filter tabs work correctly with backend filtering:
- "All" tab → shows all receipts from backend (unassigned + this car's)
- "Unassigned" tab → filters where `vehicle_id === null` (subset of above)
- "Needs Review" tab → filters by status (subset of above)

No changes needed to `filteredReceipts` logic - it filters within the already-filtered array.

**Step 4: Test manually**

Run: `npm run tauri dev`
- Create 2 vehicles
- Add receipts, assign some to each vehicle
- Switch vehicles and verify:
  - Receipts reload automatically (loading spinner appears)
  - Only relevant receipts show
  - Tab filtering works within filtered set

**Step 5: Commit**

```bash
git add src/routes/doklady/+page.svelte
git commit -m "feat(doklady): filter receipts by active vehicle with reactivity"
```

---

## Task 5: Frontend - ReceiptIndicator Badge

**IMPORTANT:** This is NOT a no-op. The component currently uses `api.getReceipts()` which returns ALL receipts globally. Must change to vehicle-filtered.

**Files:**
- Modify: `src/lib/components/ReceiptIndicator.svelte`

**Step 1: Update receipt fetching to use vehicle filter**

Find where receipts are fetched (around line 39) and change:

```typescript
// Before:
const receipts = await api.getReceipts($selectedYearStore);

// After:
const vehicle = $activeVehicleStore;
if (!vehicle) {
    needsAttentionCount = 0;
    return;
}
const receipts = await api.getReceiptsForVehicle(vehicle.id, $selectedYearStore);
```

**Step 2: Add vehicle reactivity**

Ensure the component re-fetches when vehicle changes:

```typescript
$effect(() => {
    const vehicle = $activeVehicleStore;
    const year = $selectedYearStore;
    if (vehicle && year) {
        loadReceiptStatus();
    }
});
```

**Step 3: Add loading state handling**

```typescript
let loading = $state(true);

async function loadReceiptStatus() {
    loading = true;
    try {
        const vehicle = $activeVehicleStore;
        if (!vehicle) {
            needsAttentionCount = 0;
            return;
        }
        const receipts = await api.getReceiptsForVehicle(vehicle.id, $selectedYearStore);
        // ... existing count logic
    } finally {
        loading = false;
    }
}
```

**Step 4: Test manually**

Run: `npm run tauri dev`
- Create 2 vehicles with different receipts
- Verify badge count changes when switching vehicles
- Verify badge shows correct count (unassigned + current vehicle's needing attention)

**Step 5: Commit**

```bash
git add src/lib/components/ReceiptIndicator.svelte
git commit -m "fix(receipt-indicator): filter badge count by active vehicle"
```

---

## Task 6: UX Fix - Remove Empty Vehicle Option

**Files:**
- Modify: `src/routes/+layout.svelte`
- Modify: `src/lib/i18n/sk/index.ts` (add new translation key if needed)
- Modify: `src/lib/i18n/en/index.ts` (add new translation key if needed)

**Step 1: Find vehicle selector dropdown**

Locate the `<select>` element for vehicle selection (around lines 128-142).

**Step 2: Remove or conditionally hide empty option (using i18n)**

```svelte
<!-- Before: -->
<select on:change={handleVehicleChange}>
    <option value="">{$LL.app.vehiclePlaceholder()}</option>
    {#each $vehiclesStore as vehicle}
        <option value={vehicle.id}>{vehicle.name}</option>
    {/each}
</select>

<!-- After: -->
<select on:change={handleVehicleChange}>
    {#if $vehiclesStore.length === 0}
        <option value="">{$LL.app.noVehicles()}</option>
    {/if}
    {#each $vehiclesStore as vehicle}
        <option value={vehicle.id}>{vehicle.name} ({vehicle.license_plate})</option>
    {/each}
</select>
```

**Step 3: Add i18n key if not exists**

Check if `noVehicles` key exists in i18n. If not, add to both language files:

```typescript
// src/lib/i18n/sk/index.ts
app: {
    // ... existing keys
    noVehicles: 'Žiadne vozidlá',
}

// src/lib/i18n/en/index.ts
app: {
    // ... existing keys
    noVehicles: 'No vehicles',
}
```

**Step 4: Test manually**

Run: `npm run tauri dev`
- With vehicles: no empty option visible
- Without vehicles: shows translated "No vehicles" message

**Step 5: Commit**

```bash
git add src/routes/+layout.svelte src/lib/i18n/
git commit -m "fix(layout): remove empty vehicle option when vehicles exist"
```

---

## Task 7: UX Fix - Auto-Select Vehicle on Load

**Files:**
- Modify: `src/routes/+layout.svelte`

**Step 1: Find onMount initialization**

Locate the `onMount` block (around lines 65-87) where vehicles are loaded. Note: current code uses `Promise.all` for parallel loading - preserve this pattern.

**Step 2: Add auto-select logic (preserving parallel loading)**

```typescript
onMount(async () => {
    // PRESERVE parallel loading for performance
    const [vehicles, persistedActiveVehicle] = await Promise.all([
        api.getVehicles(),
        api.getActiveVehicle()
    ]);

    vehiclesStore.set(vehicles);

    let activeVehicle = persistedActiveVehicle;

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

    // Reset year to current after auto-select to avoid stale year
    if (activeVehicle) {
        selectedYearStore.set(new Date().getFullYear());
    }

    // ... rest of initialization (loadYears, etc.)
});
```

**Key changes:**
1. Preserved `Promise.all` for parallel vehicle + activeVehicle fetch
2. Added year reset after auto-select to avoid stale year data
3. Sequential `setActiveVehicle` only runs when needed (not in happy path)

**Step 3: Test manually**

Run: `npm run tauri dev`
- Clear active vehicle in backend, reload → first vehicle auto-selected
- Delete active vehicle, reload → next vehicle auto-selected
- Verify app loads quickly (no sequential API calls on happy path)

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
| 1 | Database query + index + tests | Medium |
| 2 | Tauri command | Simple |
| 2b | Vehicle deletion cleanup | Simple |
| 3 | API wrapper | Simple |
| 4 | Doklady page + vehicle reactivity | Medium |
| 5 | ReceiptIndicator vehicle filtering | Medium |
| 6 | Remove empty option + i18n | Simple |
| 7 | Auto-select vehicle (parallel load) | Medium |
| 8 | Verification + changelog | Simple |

**Total: 9 tasks, TDD approach, frequent commits**

### Review Addressed
- [x] Fixed test helper patterns to match codebase (`Database::in_memory()`, `Receipt::new()`)
- [x] Fixed method name `map_receipt_row` → `row_to_receipt`
- [x] Added vehicle deletion cleanup (Task 2b)
- [x] Added database index on `vehicle_id`
- [x] Fixed test file location to use separate file
- [x] Added vehicle change reactivity to Doklady page
- [x] Added loading state during vehicle switch
- [x] ReceiptIndicator now uses vehicle-filtered receipts
- [x] Task 6 uses i18n keys
- [x] Task 7 preserves `Promise.all` parallel loading pattern
- [x] Added year reset after auto-select
