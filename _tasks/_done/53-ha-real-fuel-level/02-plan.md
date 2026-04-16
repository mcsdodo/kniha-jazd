# HA Real Fuel Level Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show real fuel level from HA sensor inline on zostatok line in yellow: `45.3 L (HA: 42.0 L)`

**Architecture:** Reuse existing HA fetch + store pattern. New DB field on vehicle, extend haStore cache entry for dual values, display inline on +page.svelte. Percentage-to-liters conversion in frontend per ADR-013.

---

## Task 1: Backend — DB migration, Rust model, tests (TDD)

**Files:**
- Modify: `src-tauri/src/commands/commands_tests.rs` — add persistence tests FIRST
- Create: `src-tauri/migrations/2026-02-12-100000_add_vehicle_ha_fuel_level_sensor/up.sql`
- Create: `src-tauri/migrations/2026-02-12-100000_add_vehicle_ha_fuel_level_sensor/down.sql`
- Modify: `src-tauri/src/schema.rs` — add `ha_fuel_level_sensor` column
- Modify: `src-tauri/src/models.rs` — add field to `Vehicle`, `VehicleRow`, `NewVehicle`
- Modify: `src-tauri/src/db.rs` — include field in insert/update queries
- Modify: `src-tauri/src/commands/vehicles.rs` — include in create vehicle defaults

**Steps:**
1. Write failing tests first (follow `test_vehicle_ha_fillup_sensor_persistence` pattern):
   - `test_vehicle_ha_fuel_level_sensor_persistence` — save vehicle with sensor, reload, verify
   - `test_vehicle_ha_fuel_level_sensor_null_by_default` — verify new vehicles have null
2. Create migration: `ALTER TABLE vehicles ADD COLUMN ha_fuel_level_sensor TEXT;`
3. Add `ha_fuel_level_sensor: Option<String>` to `Vehicle`, `VehicleRow`, `NewVehicle`
4. Update `db.rs` insert/update to include the field
5. Update `vehicles.rs` create_vehicle defaults (`ha_fuel_level_sensor: None`)
6. Run tests — they should pass now

**Verification:** `cargo test` passes, new field round-trips through DB

---

## Task 2: TypeScript types + Vehicle Modal — add sensor config UI

**Files:**
- Modify: `src/lib/types.ts` — add `haFuelLevelSensor` to `Vehicle` interface
- Modify: `src/lib/components/VehicleModal.svelte` — add input field, include in save payload
- Modify: `src/lib/i18n/sk/index.ts` — add Slovak i18n keys
- Modify: `src/lib/i18n/en/index.ts` — add English i18n keys

**Steps:**
1. Add `haFuelLevelSensor?: string | null` to `Vehicle` interface in types.ts
2. Add i18n keys to `homeAssistant` namespace:
   - SK: `fuelLevelSensorLabel: 'Senzor hladiny paliva'`
   - SK: `fuelLevelSensorPlaceholder: 'sensor.auto_fuel_level'`
   - SK: `fuelLevelSensorHint: 'Entity ID senzora hladiny paliva (v %) z Home Assistant'`
   - SK: `realFuel: 'HA'` (prefix for inline display)
   - EN: `fuelLevelSensorLabel: 'Fuel level sensor'`
   - EN: `fuelLevelSensorPlaceholder: 'sensor.car_fuel_level'`
   - EN: `fuelLevelSensorHint: 'Entity ID of fuel level sensor (%) from Home Assistant'`
   - EN: `realFuel: 'HA'`
3. In VehicleModal: add `haFuelLevelSensor` variable, bind to new input, include in save payload
4. Follow existing pattern from `haFillupSensor` input

**Verification:** Vehicle modal shows third HA sensor field, saves/loads correctly

---

## Task 3: Extend haStore — extend cache entry for ODO + fuel level

**Files:**
- Modify: `src/lib/stores/homeAssistant.ts` — extend cache entry, add fuel level fetching
- Modify: `src/lib/types.ts` — extend `HaOdoCache` type to include optional fuel level

**Steps:**
1. Extend `HaOdoCache` type to include optional fuel level: `fuelLevelPercent?: number; fuelFetchedAt?: number`
2. Add separate error tracking: replace single `error: string | null` with `odoError: string | null; fuelError: string | null`
3. Add `fetchFuelLevel(vehicleId, sensorId)` method — reuses `fetchHaOdo` backend command
4. Extend `startPeriodicRefresh` signature to accept optional `fuelLevelSensorId` parameter
5. Fetch both ODO and fuel level on same 5-min interval (parallel Promise.all)
6. Cache both values in the same entry, same localStorage key

**Verification:** Store fetches and caches fuel level alongside ODO, errors are independent

---

## Task 4: Display inline on zostatok line in +page.svelte

**Files:**
- Modify: `src/routes/+page.svelte` — add HA fuel level display, pass fuel sensor to store

**Steps:**
1. Add reactive variable for fuel level from cache:
   ```
   haFuelPercent = haOdoCache?.fuelLevelPercent
   haFuelLiters = haFuelPercent != null && tankSize ? (haFuelPercent * tankSize / 100) : null
   ```
2. Pass `fuelLevelSensorId` to `haStore.startPeriodicRefresh()` alongside ODO sensor
3. On the zostatok value span, append inline:
   ```svelte
   {#if haFuelLiters !== null}
     <span class="ha-fuel">({$LL.homeAssistant.realFuel()}: {haFuelLiters.toFixed(1)} L)</span>
   {/if}
   ```
4. Add CSS for `.ha-fuel`: `color: var(--accent-warning); font-size: 0.85em; margin-left: 0.25rem`
5. Handle fuel-specific error: show `(HA: chyba)` in red if fuel sensor configured but `fuelError` is set

**Verification:** Zostatok line shows `45.3 L (HA: 42.0 L)` in yellow when HA fuel sensor is configured
