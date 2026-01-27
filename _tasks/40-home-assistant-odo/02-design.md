# Design: Home Assistant ODO Integration

## User Experience

### Header Display

Add a single stat line in the vehicle info header (main page):

```
Real ODO: 45,230 km (+130 km)  ⏱ 5m
```

**Behavior:**
- Fetch on app startup
- Periodic refresh every ~5 minutes (simple interval)
- If fetch fails → show last cached value with staleness, or hide if no cached value
- If no sensor configured → section not shown

**Visual states:**
- Delta in **warning color** if ≥ 50 km (suggests forgotten trips)
- Staleness shown as simple "⏱ Xm" or "⏱ Xh"

**Delta calculation:**
```
delta = real_odo - last_trip_ending_odo
```

Where `last_trip_ending_odo` = last trip's ODO + km driven (from backend stats)

## Configuration

### Settings Page (Global)

New "Home Assistant" section:

```
┌─ Home Assistant Integration ──────────────────────┐
│                                                   │
│  URL:       [https://my-home.duckdns.org:8123  ]  │
│  API Token: [••••••••••••••••••••••••••••••••• ]  │
│                                                   │
│  Status: ✓ Connected                              │
└───────────────────────────────────────────────────┘
```

- Stored in `local.settings.json` (not synced to DB — security)
- Optional "Test Connection" button to verify credentials

### Vehicle Modal (Per-Vehicle)

Add field to vehicle edit form:

```
┌─ Home Assistant ──────────────────────────────────┐
│                                                   │
│  ODO Sensor: [sensor.skoda_octavia_odometer    ]  │
│              (leave empty to disable)             │
└───────────────────────────────────────────────────┘
```

- Only shown if HA credentials are configured in Settings
- Stored in `vehicles` table: new column `ha_odo_sensor TEXT`

## Technical Architecture

### Where does the HA API call happen?

**Frontend (Svelte)** — not backend (Rust):

- HA API is a simple HTTP GET request
- No business logic or calculations needed
- Keeps Rust backend focused on DB + calculations (per ADR-008)
- Frontend already handles periodic UI updates

### Data Flow

```
┌─────────────────────────────────────────────────────────┐
│                    SvelteKit Frontend                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │ Settings    │    │ HA Service  │    │ Main Page   │  │
│  │ (config)    │───▶│ (fetch+cache│───▶│ (display)   │  │
│  └─────────────┘    └─────────────┘    └─────────────┘  │
│                            │                            │
│                            ▼                            │
│                    Home Assistant API                   │
│               GET /api/states/sensor.xxx                │
└─────────────────────────────────────────────────────────┘
```

### Caching

```typescript
// $lib/stores/homeAssistant.ts
interface HaOdoCache {
  value: number;          // ODO in km
  fetchedAt: Date;        // for staleness display
}

// Map<vehicleId, HaOdoCache>
```

- Cache lives in memory (Svelte store)
- Persisted to `localStorage` for app restart
- Simple 5-minute interval refresh

## Implementation Details

### Home Assistant API

```typescript
// GET https://{ha_url}/api/states/{sensor_entity_id}
// Header: Authorization: Bearer {api_token}

// Response:
{
  "entity_id": "sensor.skoda_octavia_odometer",
  "state": "45230",  // ODO value as string
  "attributes": {
    "unit_of_measurement": "km",
    ...
  }
}
```

### Database Migration

```sql
-- Add HA sensor column to vehicles (backward-compatible)
ALTER TABLE vehicles ADD COLUMN ha_odo_sensor TEXT DEFAULT NULL;
```

### Local Settings Addition

```rust
// settings.rs - LocalSettings struct
pub struct LocalSettings {
    // ... existing fields ...
    pub ha_url: Option<String>,
    pub ha_api_token: Option<String>,  // sensitive - never in DB
}
```

## Files to Modify

| File | Change |
|------|--------|
| `settings.rs` | Add `ha_url`, `ha_api_token` fields |
| `models.rs` | Add `ha_odo_sensor` to Vehicle |
| `schema.rs` | Update after migration |
| `db.rs` | Include new column in vehicle CRUD |
| `src/lib/stores/homeAssistant.ts` | New store for HA data + caching |
| `src/lib/services/homeAssistant.ts` | New service for API calls |
| `src/routes/+page.svelte` | Display real ODO in header |
| `src/routes/settings/+page.svelte` | HA config section |
| Vehicle modal component | Add sensor field |
| `i18n/sk/index.ts` + `en/index.ts` | New translations |

## Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| API location | Frontend | Simple HTTP, no business logic |
| Credentials storage | local.settings.json | Security - never sync to DB |
| Sensor storage | Database | Per-vehicle, needs persistence |
| Refresh strategy | Startup + 5min interval | Simple, no complex retry logic |
| Failure handling | Cache + staleness | Graceful, no error spam |
