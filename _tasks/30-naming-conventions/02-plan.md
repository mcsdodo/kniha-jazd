# Implementation Plan: camelCase Standardization

**Date:** 2026-01-09
**Subject:** Step-by-step plan for naming convention migration
**Status:** Planning

## Approach: Test-Driven Development

**Update tests first, then develop against them.**

After migration:
- `TripRaw`, `TripGridDataRaw` types become **obsolete** (removed)
- `Record<string, unknown>` casts become **unnecessary** (removed)
- Tests use proper typed interfaces directly - **no more hacks**

**Why camelCase:**
- JavaScript/TypeScript idiomatic convention
- Consistent with PreviewResult (already uses camelCase)
- Better IDE autocomplete in components
- Industry standard for JSON APIs

---

## Phase 1: Update Integration Test Types (TDD - Tests First)

**File: `tests/integration/fixtures/types.ts`**

### Remove obsolete "Raw" types:
- [ ] Delete `TripRaw` interface (lines 69-90)
- [ ] Delete `TripGridDataRaw` interface (lines 200-216)
- [ ] Remove comments about "snake_case - Rust/Tauri IPC response format"

### Verify camelCase types are correct:
The following already use camelCase (no changes needed):
- `Vehicle` ✓
- `Trip` ✓
- `Route` ✓
- `Settings` ✓
- `TripStats` ✓
- `TripGridData` ✓
- `Receipt` ✓
- `FieldConfidence` ✓
- `ReceiptVerification` ✓
- `VerificationResult` ✓
- `PreviewResult` ✓

## Phase 2: Update Integration Test Utilities

**File: `tests/integration/utils/db.ts`**

- [ ] Remove `TripGridDataRaw` import
- [ ] Change `getTripGridData()` return type from `TripGridDataRaw` to `TripGridData`
- [ ] Remove/update comments about "Returns raw snake_case format"

## Phase 3: Update Integration Test Assertions

**Remove `Record<string, unknown>` hacks and use typed access:**

### `tests/integration/specs/tier1/seeding.spec.ts`
- [ ] Line ~68-70: Remove `as Record<string, unknown>` cast
- [ ] Change `vehicleAny.tank_size_liters` → `vehicle.tankSizeLiters`
- [ ] Change `vehicleAny.tp_consumption` → `vehicle.tpConsumption`
- [ ] Line ~145+: Update trip field assertions to camelCase
- [ ] Line ~181+: Change `fuel_liters` → `fuelLiters`, `fuel_cost_eur` → `fuelCostEur`, `full_tank` → `fullTank`
- [ ] Remove comments about "Tauri returns snake_case fields from Rust"

### `tests/integration/specs/tier2/vehicle-management.spec.ts`
- [ ] Line ~88: Remove comment about "Rust returns snake_case property names"
- [ ] Line ~176: Remove comment about "Rust returns snake_case property names"
- [ ] Update any snake_case field access to camelCase

### Other test files (search for snake_case patterns):
- [ ] `tests/integration/specs/tier1/bev-trips.spec.ts`
- [ ] `tests/integration/specs/tier1/consumption-warnings.spec.ts`
- [ ] `tests/integration/specs/tier1/phev-trips.spec.ts`
- [ ] `tests/integration/specs/tier1/year-handling.spec.ts`
- [ ] `tests/integration/specs/tier2/backup-restore.spec.ts`
- [ ] `tests/integration/specs/tier2/receipts.spec.ts`
- [ ] `tests/integration/specs/tier3/compensation.spec.ts`

---

## Phase 4: Rust Backend (models.rs)

Add `#[serde(rename_all = "camelCase")]` to these structs in `src-tauri/src/models.rs`:

- [ ] `Vehicle`
- [ ] `Trip`
- [ ] `Route`
- [ ] `Settings`
- [ ] `TripStats`
- [ ] `TripGridData`
- [ ] `Receipt`
- [ ] `FieldConfidence`
- [ ] `ReceiptVerification`
- [ ] `VerificationResult`

**Note:** Do NOT add to Row structs (VehicleRow, TripRow, etc.) - these map to database columns.

## Phase 5: Rust Backend (other files)

**`src-tauri/src/commands.rs`:**
- [ ] `BackupInfo`
- [ ] `ScanResult`
- [ ] `SyncResult`
- [ ] `SyncError`
- [ ] `ReceiptSettings`
- [ ] `WindowSize`

**`src-tauri/src/suggestions.rs`:**
- [ ] `CompensationSuggestion`

## Phase 6: Frontend Types

**File: `src/lib/types.ts`**

Convert all interfaces to camelCase:

### Vehicle
| Before | After |
|--------|-------|
| `license_plate` | `licensePlate` |
| `vehicle_type` | `vehicleType` |
| `tank_size_liters` | `tankSizeLiters` |
| `tp_consumption` | `tpConsumption` |
| `battery_capacity_kwh` | `batteryCapacityKwh` |
| `baseline_consumption_kwh` | `baselineConsumptionKwh` |
| `initial_battery_percent` | `initialBatteryPercent` |
| `initial_odometer` | `initialOdometer` |
| `is_active` | `isActive` |
| `driver_name` | `driverName` |
| `created_at` | `createdAt` |
| `updated_at` | `updatedAt` |

### Trip
| Before | After |
|--------|-------|
| `vehicle_id` | `vehicleId` |
| `distance_km` | `distanceKm` |
| `fuel_liters` | `fuelLiters` |
| `fuel_cost_eur` | `fuelCostEur` |
| `full_tank` | `fullTank` |
| `energy_kwh` | `energyKwh` |
| `energy_cost_eur` | `energyCostEur` |
| `full_charge` | `fullCharge` |
| `soc_override_percent` | `socOverridePercent` |
| `other_costs_eur` | `otherCostsEur` |
| `other_costs_note` | `otherCostsNote` |
| `sort_order` | `sortOrder` |
| `created_at` | `createdAt` |
| `updated_at` | `updatedAt` |

### Route
| Before | After |
|--------|-------|
| `vehicle_id` | `vehicleId` |
| `distance_km` | `distanceKm` |
| `usage_count` | `usageCount` |
| `last_used` | `lastUsed` |

### CompensationSuggestion
| Before | After |
|--------|-------|
| `distance_km` | `distanceKm` |
| `is_buffer` | `isBuffer` |

### Settings
| Before | After |
|--------|-------|
| `company_name` | `companyName` |
| `company_ico` | `companyIco` |
| `buffer_trip_purpose` | `bufferTripPurpose` |
| `updated_at` | `updatedAt` |

### TripStats
| Before | After |
|--------|-------|
| `fuel_remaining_liters` | `fuelRemainingLiters` |
| `avg_consumption_rate` | `avgConsumptionRate` |
| `last_consumption_rate` | `lastConsumptionRate` |
| `margin_percent` | `marginPercent` |
| `is_over_limit` | `isOverLimit` |
| `total_km` | `totalKm` |
| `total_fuel_liters` | `totalFuelLiters` |
| `total_fuel_cost_eur` | `totalFuelCostEur` |

### BackupInfo
| Before | After |
|--------|-------|
| `created_at` | `createdAt` |
| `size_bytes` | `sizeBytes` |
| `vehicle_count` | `vehicleCount` |
| `trip_count` | `tripCount` |

### TripGridData
| Before | After |
|--------|-------|
| `estimated_rates` | `estimatedRates` |
| `fuel_remaining` | `fuelRemaining` |
| `consumption_warnings` | `consumptionWarnings` |
| `energy_rates` | `energyRates` |
| `estimated_energy_rates` | `estimatedEnergyRates` |
| `battery_remaining_kwh` | `batteryRemainingKwh` |
| `battery_remaining_percent` | `batteryRemainingPercent` |
| `soc_override_trips` | `socOverrideTrips` |
| `date_warnings` | `dateWarnings` |
| `missing_receipts` | `missingReceipts` |

### FieldConfidence
| Before | After |
|--------|-------|
| `total_price` | `totalPrice` |

### Receipt
| Before | After |
|--------|-------|
| `vehicle_id` | `vehicleId` |
| `trip_id` | `tripId` |
| `file_path` | `filePath` |
| `file_name` | `fileName` |
| `scanned_at` | `scannedAt` |
| `total_price_eur` | `totalPriceEur` |
| `receipt_date` | `receiptDate` |
| `station_name` | `stationName` |
| `station_address` | `stationAddress` |
| `source_year` | `sourceYear` |
| `raw_ocr_text` | `rawOcrText` |
| `error_message` | `errorMessage` |
| `created_at` | `createdAt` |
| `updated_at` | `updatedAt` |

### ReceiptSettings
| Before | After |
|--------|-------|
| `gemini_api_key` | `geminiApiKey` |
| `receipts_folder_path` | `receiptsFolderPath` |
| `gemini_api_key_from_override` | `geminiApiKeyFromOverride` |
| `receipts_folder_from_override` | `receiptsFolderFromOverride` |

### SyncError
| Before | After |
|--------|-------|
| `file_name` | `fileName` |

### ScanResult
| Before | After |
|--------|-------|
| `new_count` | `newCount` |

### ReceiptVerification
| Before | After |
|--------|-------|
| `receipt_id` | `receiptId` |
| `matched_trip_id` | `matchedTripId` |
| `matched_trip_date` | `matchedTripDate` |
| `matched_trip_route` | `matchedTripRoute` |

**Note:** `ExportLabels` KEEPS snake_case - passed TO Rust for HTML template.

## Phase 7: Component Updates

Update field access in Svelte components (TypeScript will guide us with errors):

### `src/lib/components/TripGrid.svelte`
- [ ] All TripGridData field access (`fuel_remaining` → `fuelRemaining`, etc.)

### `src/lib/components/TripRow.svelte`
- [ ] formData field names (`distance_km` → `distanceKm`, etc.)
- [ ] trip field access

### `src/lib/components/TripSelectorModal.svelte`
- [ ] trip field access

### `src/lib/components/VehicleModal.svelte`
- [ ] All vehicle field access (`license_plate` → `licensePlate`, etc.)

### `src/lib/components/CompensationBanner.svelte`
- [ ] suggestion field access

### `src/routes/+page.svelte`
- [ ] stats field access
- [ ] vehicle field access

### `src/routes/doklady/+page.svelte`
- [ ] receipt field access

### `src/routes/settings/+page.svelte`
- [ ] vehicle/settings field access

---

## Phase 8: Verification

- [ ] `npm run check` - TypeScript/Svelte errors
- [ ] `cd src-tauri && cargo test` - Rust tests
- [ ] `npm run tauri build` - Full build
- [ ] `npm run test:integration` - Integration tests pass
- [ ] Manual: Create/edit vehicle
- [ ] Manual: Create/edit trips
- [ ] Manual: Export to HTML
- [ ] Manual: Receipt scanning

## Phase 9: Finalize

- [ ] Run `/changelog`
- [ ] Commit all changes

---

## Files Summary

### Integration Tests (update first - TDD)
1. `tests/integration/fixtures/types.ts` - Remove Raw types
2. `tests/integration/utils/db.ts` - Update return types
3. `tests/integration/specs/**/*.ts` - Fix assertions

### Rust (3-4 files)
1. `src-tauri/src/models.rs`
2. `src-tauri/src/commands.rs`
3. `src-tauri/src/suggestions.rs`

### Frontend Types (1 file)
1. `src/lib/types.ts`

### Svelte Components (7+ files)
1. `src/lib/components/TripGrid.svelte`
2. `src/lib/components/TripRow.svelte`
3. `src/lib/components/TripSelectorModal.svelte`
4. `src/lib/components/VehicleModal.svelte`
5. `src/lib/components/CompensationBanner.svelte`
6. `src/routes/+page.svelte`
7. `src/routes/doklady/+page.svelte`
8. `src/routes/settings/+page.svelte`

---

## Post-Migration Benefits

After this change:
- ✅ **No more dual type systems** - One set of types for everything
- ✅ **No more `Record<string, unknown>` hacks** - Proper typed access
- ✅ **No more "Raw" types** - `TripRaw`, `TripGridDataRaw` deleted
- ✅ **Full IDE support** - Autocomplete works correctly
- ✅ **Consistent codebase** - All TypeScript uses camelCase

## Notes

- **TDD approach:** Tests updated first, then implementation
- **Atomic change:** Do all files in one session
- **No database changes:** Row structs stay snake_case for Diesel
- **No breaking external changes:** Internal code only
