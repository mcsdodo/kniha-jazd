# Implementation Plan: Home Assistant ODO Integration

## Phase 1: Backend - Database & Settings

### Step 1.1: Add HA fields to LocalSettings
**Files:** `src-tauri/src/settings.rs`

- Add `ha_url: Option<String>` field
- Add `ha_api_token: Option<String>` field
- Update tests for new fields

### Step 1.2: Database Migration
**Files:** `src-tauri/migrations/`, `src-tauri/src/schema.rs`

- Create migration: `ALTER TABLE vehicles ADD COLUMN ha_odo_sensor TEXT DEFAULT NULL`
- Run `diesel migration run` to update schema.rs

### Step 1.3: Update Vehicle Model
**Files:** `src-tauri/src/models.rs`, `src-tauri/src/db.rs`

- Add `ha_odo_sensor: Option<String>` to Vehicle struct
- Update vehicle CRUD operations to include new field

### Step 1.4: Expose Settings Commands
**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

- Add `get_ha_settings` command (returns url + whether token is set, NOT the token itself)
- Add `save_ha_settings` command (saves url + token)
- Register commands in invoke_handler

---

## Phase 2: Frontend - Settings UI

### Step 2.1: Add i18n Translations
**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

- Add translations for HA settings section
- Add translations for vehicle modal HA field
- Add translations for header display

### Step 2.2: Settings Page - HA Section
**Files:** `src/routes/settings/+page.svelte`

- Add "Home Assistant" collapsible section
- URL input field
- API Token input field (password type)
- "Test Connection" button (optional nice-to-have)
- Save button triggers `save_ha_settings` command

### Step 2.3: Vehicle Modal - Sensor Field
**Files:** `src/lib/components/VehicleModal.svelte` (or similar)

- Add "ODO Sensor" input field
- Only show if HA is configured (check settings)
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

- HTTP fetch to HA API
- Parse response, extract state value
- Handle errors gracefully

### Step 3.2: Create HA Store
**Files:** `src/lib/stores/homeAssistant.ts` (new)

```typescript
// Store structure:
// - cache: Map<vehicleId, { value: number, fetchedAt: Date }>
// - loading: boolean
// - error: string | null

// Functions:
// - fetchOdo(vehicleId, sensorId)
// - startPeriodicRefresh()
// - stopPeriodicRefresh()
```

- Svelte writable store for cache
- localStorage persistence for cache
- 5-minute interval refresh logic

---

## Phase 4: Frontend - Header Display

### Step 4.1: Display Real ODO in Header
**Files:** `src/routes/+page.svelte`

- Import HA store
- On mount: start periodic refresh if vehicle has sensor configured
- Display: `Real ODO: {value} km ({delta} km) ⏱ {staleness}`
- Warning style if delta ≥ 50 km
- Hide section if no sensor or no data

### Step 4.2: Calculate Delta
**Files:** `src/routes/+page.svelte`

- Get last trip ending ODO from stats (already available)
- Delta = realOdo - lastTripEndingOdo
- Handle edge case: no trips yet (show just real ODO, no delta)

---

## Phase 5: Frontend Types

### Step 5.1: Update TypeScript Types
**Files:** `src/lib/types.ts`

- Add `haOdoSensor?: string` to Vehicle type
- Add `HaOdoCache` interface
- Add `HaSettings` interface

---

## Phase 6: Testing

### Step 6.1: Backend Unit Tests
**Files:** `src-tauri/src/settings.rs`, `src-tauri/src/commands_tests.rs`

- Test LocalSettings serialization with new HA fields
- Test vehicle CRUD with ha_odo_sensor field

### Step 6.2: Integration Tests
**Files:** `tests/integration/`

- Test HA settings save/load flow
- Test vehicle with sensor field save/load
- (Note: actual HA API calls cannot be tested without mock server)

---

## Implementation Order

```
Phase 1 (Backend)     → Phase 2 (Settings UI) → Phase 3 (Service/Store)
       ↓                                                    ↓
Phase 5 (Types)  ←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←  Phase 4 (Header Display)
       ↓
Phase 6 (Testing)
```

**Recommended execution:**
1. Phase 1 → commit "feat(backend): add Home Assistant settings and vehicle sensor field"
2. Phase 2 + 5 → commit "feat(settings): add Home Assistant configuration UI"
3. Phase 3 + 4 → commit "feat(ui): display real ODO from Home Assistant in header"
4. Phase 6 → commit "test: add tests for Home Assistant integration"

---

## Checklist

- [ ] Phase 1: Backend settings + DB migration
- [ ] Phase 2: Settings UI
- [ ] Phase 3: HA service + store
- [ ] Phase 4: Header display
- [ ] Phase 5: TypeScript types
- [ ] Phase 6: Tests
- [ ] Update CHANGELOG.md
- [ ] Update DECISIONS.md (if architectural choices needed)
