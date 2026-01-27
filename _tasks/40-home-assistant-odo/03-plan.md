# Implementation Plan: Home Assistant ODO Integration

## Phase 1: Backend - Database & Settings

### Step 1.1: Add HA fields to LocalSettings
**Files:** `src-tauri/src/settings.rs`

- Add `ha_url: Option<String>` field
- Add `ha_api_token: Option<String>` field
- Update tests for new fields

**Verification:** Run `cargo test settings` - all tests pass

### Step 1.2: Database Migration
**Files:** `src-tauri/migrations/`, `src-tauri/src/schema.rs`

- Create migration: `ALTER TABLE vehicles ADD COLUMN ha_odo_sensor TEXT DEFAULT NULL`
- Run `diesel migration run` to update schema.rs

**Verification:** Check `schema.rs` contains `ha_odo_sensor` column in vehicles table

### Step 1.3: Update Vehicle Model
**Files:** `src-tauri/src/models.rs`, `src-tauri/src/db.rs`

Update all vehicle-related structs:
- Add `ha_odo_sensor: Option<String>` to `Vehicle` struct
- Add `ha_odo_sensor: Option<String>` to `VehicleRow` struct
- Add `ha_odo_sensor: Option<String>` to `NewVehicleRow` struct
- Update `From<VehicleRow> for Vehicle` impl to map the field
- Update vehicle CRUD operations in `db.rs` to include new field

**Verification:** Run `cargo test` - all vehicle tests pass

### Step 1.4: Expose Settings Commands
**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

- Add `get_ha_settings` command returning `HaSettingsResponse { url: Option<String>, has_token: bool }`
- Add `save_ha_settings` command (saves url + token to local.settings.json)
- Register commands in invoke_handler

**Verification:** Run `npm run tauri dev`, test commands via browser console

---

## Phase 2: Frontend - Types & Settings UI

### Step 2.1: Update TypeScript Types
**Files:** `src/lib/types.ts`

- Add `haOdoSensor?: string` to Vehicle type
- Add `HaOdoCache` interface:
  ```typescript
  interface HaOdoCache {
    value: number;
    fetchedAt: number; // timestamp ms
  }
  ```
- Add `HaSettings` interface:
  ```typescript
  interface HaSettings {
    url: string | null;
    hasToken: boolean;
  }
  ```

### Step 2.2: Add i18n Translations
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

- Add translations for HA settings section
- Add translations for vehicle modal HA field
- Add translations for header display (real ODO, staleness)

### Step 2.3: Settings Page - HA Section
**Files:** `src/routes/settings/+page.svelte`

- Add "Home Assistant" collapsible section
- URL input field with validation:
  - Must start with `http://` or `https://`
  - Must be valid URL format
  - Show error message if invalid
- API Token input field (password type)
- "Test Connection" button (optional nice-to-have)
- Save button triggers `save_ha_settings` command

### Step 2.4: Vehicle Modal - Sensor Field
**Files:** `src/lib/components/VehicleModal.svelte`

- Add "ODO Sensor" input field (e.g., `sensor.skoda_octavia_odometer`)
- Only show if HA is configured (check settings via `get_ha_settings`)
- Placeholder text: "sensor.car_odometer"
- Save sensor entity ID with vehicle

---

## Phase 3: Frontend - HA Service & Store

### Step 3.1: Create HA Service
**Files:** `src/lib/services/homeAssistant.ts` (new)

```typescript
// Functions:
// - fetchOdometer(url, token, sensorId): Promise<number>
// - testConnection(url, token): Promise<boolean>
```

- HTTP fetch to HA API with 5-second timeout
- Parse response, extract state value as number
- **Error handling:**
  - Network timeout (5s) → throw `TimeoutError`
  - HTTP 401 → throw `AuthError` (invalid token)
  - HTTP 404 or empty state → throw `SensorNotFoundError`
  - Non-numeric state → throw `InvalidResponseError`
  - All errors include user-friendly message for display

### Step 3.2: Create HA Store
**Files:** `src/lib/stores/homeAssistant.ts` (new)

```typescript
// Store structure:
// - cache: Map<vehicleId, HaOdoCache>
// - loading: boolean
// - error: string | null

// Functions:
// - fetchOdo(vehicleId, sensorId)
// - startPeriodicRefresh(vehicleId, sensorId)
// - stopPeriodicRefresh()
// - getCachedOdo(vehicleId): HaOdoCache | null
```

- Svelte writable store for cache
- localStorage persistence with key `kniha-jazd-ha-odo-cache`
- 5-minute interval refresh logic
- Handle localStorage errors gracefully (corrupt data → clear and continue)

---

## Phase 4: Frontend - Header Display

### Step 4.1: Display Real ODO in Header
**Files:** `src/routes/+page.svelte`

- Import HA store
- On mount: start periodic refresh if vehicle has `haOdoSensor` configured
- Display in stats section: `Real ODO: {value} km ({delta} km) ⏱ {staleness}`
- Warning style (orange) if delta ≥ 50 km
- Hide section entirely if:
  - No sensor configured for vehicle
  - No cached data and fetch failed

### Step 4.2: Calculate Delta
**Files:** `src/routes/+page.svelte`

**Delta calculation:**
```typescript
// Last trip's odometer field already represents ending ODO
// (per app convention: odometer = ending ODO after trip)
const lastTripEndingOdo = trips.length > 0
  ? trips[trips.length - 1].odometer
  : null;

const delta = lastTripEndingOdo !== null
  ? realOdo - lastTripEndingOdo
  : null;
```

- If no trips yet: show just real ODO, no delta
- Handle edge case: trips sorted by date descending → get first item instead

### Step 4.3: Staleness Display
**Files:** `src/routes/+page.svelte`

Format staleness indicator:
- < 60 minutes: show minutes (e.g., "5m")
- >= 60 minutes: show hours (e.g., "2h")
- If data older than 24h, consider showing "1d+" or hiding

---

## Phase 5: Testing

### Step 5.1: Backend Unit Tests
**Files:** `src-tauri/src/settings.rs`, `src-tauri/src/commands_tests.rs`

- Test LocalSettings serialization with new HA fields
- Test vehicle CRUD with ha_odo_sensor field

### Step 5.2: Integration Tests
**Files:** `tests/integration/`

- Test HA settings save/load flow
- Test vehicle with sensor field save/load
- (Note: actual HA API calls cannot be tested without mock server)

---

## Implementation Order

```
Phase 1 (Backend) → Phase 2 (Types + Settings UI) → Phase 3 (Service/Store) → Phase 4 (Header)
                                                                                      ↓
                                                                              Phase 5 (Testing)
```

**Recommended execution:**
1. Phase 1 → commit "feat(backend): add Home Assistant settings and vehicle sensor field"
2. Phase 2 → commit "feat(settings): add Home Assistant configuration UI"
3. Phase 3 + 4 → commit "feat(ui): display real ODO from Home Assistant in header"
4. Phase 5 → commit "test: add tests for Home Assistant integration"

---

## Checklist

- [ ] Phase 1: Backend settings + DB migration
- [ ] Phase 2: Types + Settings UI
- [ ] Phase 3: HA service + store
- [ ] Phase 4: Header display
- [ ] Phase 5: Tests
- [ ] Update CHANGELOG.md
- [ ] Update DECISIONS.md (if architectural choices needed)
