# HA Real Fuel Level Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show real fuel level from HA sensor inline on zostatok line in yellow: `45.3 L (HA: 42.0 L)`

**Architecture:** Reuse existing HA fetch + store pattern. New DB field on vehicle, extend haStore for dual caching, display inline on +page.svelte.

---

## Task 1: DB migration + Rust model — add `ha_fuel_level_sensor`

**Files:**
- Create: `src-tauri/migrations/2026-02-12-100000_add_vehicle_ha_fuel_level_sensor/up.sql`
- Create: `src-tauri/migrations/2026-02-12-100000_add_vehicle_ha_fuel_level_sensor/down.sql`
- Modify: `src-tauri/src/schema.rs` — add `ha_fuel_level_sensor` column
- Modify: `src-tauri/src/models.rs` — add field to `Vehicle`, `VehicleRow`, `NewVehicle`
- Modify: `src-tauri/src/db.rs` — include field in insert/update queries
- Modify: `src-tauri/src/commands/vehicles.rs` — include in create vehicle defaults

**Steps:**
1. Create migration: `ALTER TABLE vehicles ADD COLUMN ha_fuel_level_sensor TEXT;`
2. Add `ha_fuel_level_sensor: Option<String>` to `Vehicle` model
3. Add to `VehicleRow` and `NewVehicle` structs
4. Update `db.rs` insert/update to include the field
5. Update `vehicles.rs` create_vehicle defaults (`ha_fuel_level_sensor: None`)
6. Add test for persistence (follow `test_vehicle_ha_fillup_sensor_persistence` pattern)

**Verification:** `cargo test` passes, new field round-trips through DB

---

## Task 2: TypeScript types + Vehicle Modal — add sensor config UI

**Files:**
- Modify: `src/lib/types.ts` — add `haFuelLevelSensor` to `Vehicle` interface
- Modify: `src/lib/components/VehicleModal.svelte` — add input field
- Modify: `src/lib/i18n/sk/index.ts` — add Slovak i18n keys
- Modify: `src/lib/i18n/en/index.ts` — add English i18n keys

**Steps:**
1. Add `haFuelLevelSensor?: string | null` to `Vehicle` interface in types.ts
2. Add i18n keys: `fuelLevelSensorLabel`, `fuelLevelSensorPlaceholder`, `fuelLevelSensorHint`, `realFuel`
3. In VehicleModal: add `haFuelLevelSensor` variable, bind to input, include in save payload
4. Follow existing pattern from `haFillupSensor` input (lines 150-159)

**Verification:** Vehicle modal shows third HA sensor field, saves/loads correctly

---

## Task 3: Extend haStore — dual caching for ODO + fuel level

**Files:**
- Modify: `src/lib/stores/homeAssistant.ts` — add fuel level cache
- Modify: `src/lib/types.ts` — add `HaFuelLevelCache` type (or reuse `HaOdoCache`)

**Steps:**
1. Add second cache map for fuel level (or extend cache entry to hold both)
2. Add `fetchFuelLevel(vehicleId, sensorId)` method — reuses `fetchHaOdo` backend command
3. Extend `startPeriodicRefresh` to accept optional `fuelLevelSensorId` parameter
4. Fetch both ODO and fuel level on same interval (parallel calls)
5. Add `getCachedFuelLevel(vehicleId)` convenience method
6. Use separate localStorage key `kniha-jazd-ha-fuel-cache` for fuel level cache

**Verification:** Store fetches and caches fuel level value, periodic refresh works for both

---

## Task 4: Display inline on zostatok line in +page.svelte

**Files:**
- Modify: `src/routes/+page.svelte` — add HA fuel level display

**Steps:**
1. Add reactive variable: `$: haFuelCache = $activeVehicleStore ? $haStore.fuelCache.get($activeVehicleStore.id) : null`
2. Compute liters from percentage: `$: haFuelLiters = haFuelCache && $activeVehicleStore?.tankSizeLiters ? (haFuelCache.value * $activeVehicleStore.tankSizeLiters / 100) : null`
3. On the zostatok value line (line 243), append after `{stats.fuelRemainingLiters.toFixed(1)} L`:
   ```svelte
   {#if haFuelLiters !== null}
     <span class="ha-fuel">(HA: {haFuelLiters.toFixed(1)} L)</span>
   {/if}
   ```
4. Pass `fuelLevelSensorId` to `haStore.startPeriodicRefresh()`
5. Add CSS for `.ha-fuel`: yellow color via `var(--accent-warning)`, slightly smaller font
6. Handle error state: show `(HA: chyba)` in red if sensor configured but fetch fails

**Verification:** Zostatok line shows `45.3 L (HA: 42.0 L)` in yellow when HA fuel sensor is configured

---

## Task 5: Backend test for new field + cleanup

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs` — add persistence test

**Steps:**
1. Add `test_vehicle_ha_fuel_level_sensor_persistence` — save vehicle with sensor, reload, verify
2. Add `test_vehicle_ha_fuel_level_sensor_null_by_default` — verify new vehicles have null
3. Run full test suite: `npm run test:backend`

**Verification:** `cargo test` — all tests pass including new ones
