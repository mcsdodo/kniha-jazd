# Feature: Home Assistant ODO Integration

> Displays real-time odometer reading from Home Assistant sensor in the app header, enabling detection of unlogged trips.

## User Flow

1. **Configure HA credentials** in Settings → Home Assistant section
   - Enter Home Assistant URL (e.g., `https://my-ha.duckdns.org`)
   - Enter long-lived access token
   - Connection status indicator shows if credentials are valid
2. **Assign sensor to vehicle** in Settings → Vehicles → Edit vehicle
   - Enter sensor entity ID (e.g., `sensor.car_odometer`)
   - Real ODO value appears in vehicle list when configured
3. **View real ODO** in main page header
   - Shows "Reálne ODO: 45,230 km (+130 km)" with delta from last logged trip
   - Positive delta indicates unlogged trips

**Refresh behavior:**
- ODO fetched on app startup and every 5 minutes
- Cached in localStorage for instant display on page load

## Technical Implementation

### Frontend

**Main Page:** `src/routes/+page.svelte`
- Subscribes to `haStore` for cached ODO value
- Calculates delta: `haOdoValue - Math.max(...trips.map(t => t.odometer))`
- Displays in header stats row

**Settings Page:** `src/routes/settings/+page.svelte`
- `handleSaveHaSettings()` — Saves URL + token to backend
- `handleTestHaConnection()` — Tests connectivity via backend
- Connection status indicator (`connected` / `disconnected` / `testing`)
- Vehicle list shows real ODO for vehicles with configured sensors

**Store:** `src/lib/stores/homeAssistant.ts`
- `haStore.fetchOdo(vehicleId, sensorId)` — Fetches via Rust backend
- `haStore.startPeriodicRefresh()` — 5-minute refresh interval
- `haStore.getCachedOdo(vehicleId)` — Returns cached value
- LocalStorage persistence for cache

**API Wrapper:** `src/lib/api.ts:L427-443`
- `getLocalSettingsForHa()` — Retrieves HA URL + token
- `testHaConnection()` — Tests HA connectivity
- `fetchHaOdo(sensorId)` — Fetches sensor value

### Backend (Rust)

**Commands:** `src-tauri/src/commands.rs`
- `test_ha_connection` (L3758) — Tests HA API connectivity using stored credentials
- `fetch_ha_odo` (L3793) — Fetches sensor state from HA API

**Settings:** `src-tauri/src/settings.rs:L36-37`
- `ha_url: Option<String>` — Home Assistant URL
- `ha_api_token: Option<String>` — Long-lived access token

**Vehicle Model:** `src-tauri/src/models.rs:L595`
- `ha_odo_sensor: Option<String>` — Entity ID for ODO sensor

### Data Flow

```
                    ┌─────────────────────────────────────┐
                    │     Home Assistant Instance         │
                    │  sensor.car_odometer: 45230         │
                    └───────────────┬─────────────────────┘
                                    │ HTTPS GET
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Rust Backend                               │
│  fetch_ha_odo(sensor_id) → reqwest → parse JSON → return f64   │
└───────────────────────────────────┬─────────────────────────────┘
                                    │ Tauri IPC
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SvelteKit Frontend                           │
│  haStore → cache → localStorage → +page.svelte → header stats  │
└─────────────────────────────────────────────────────────────────┘
```

**Why Rust backend handles API calls:**
- Browser webview has CORS restrictions
- HA API doesn't allow cross-origin requests from arbitrary domains
- Rust's reqwest client has no CORS limitations

## Key Files

| File | Purpose |
|------|---------|
| `src/routes/+page.svelte` | Main page header with real ODO display |
| `src/routes/settings/+page.svelte` | HA configuration UI |
| `src/lib/stores/homeAssistant.ts` | Svelte store with caching + refresh |
| `src/lib/api.ts:L427-443` | TypeScript API wrappers |
| `src-tauri/src/commands.rs:L3758-3830` | `test_ha_connection`, `fetch_ha_odo` |
| `src-tauri/src/settings.rs:L36-37` | `ha_url`, `ha_api_token` fields |
| `src-tauri/src/models.rs:L595` | `ha_odo_sensor` vehicle field |

## Configuration Storage

**Global HA credentials:** `local.settings.json` (in AppData)
```json
{
  "haUrl": "https://my-ha.duckdns.org",
  "haApiToken": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Per-vehicle sensor:** SQLite `vehicles` table
```sql
ha_odo_sensor TEXT  -- e.g., "sensor.car_odometer"
```

**ODO cache:** LocalStorage (`kniha-jazd-ha-odo-cache`)
```json
{
  "vehicle-uuid-123": {
    "value": 45230,
    "fetchedAt": 1706351234567
  }
}
```

## Design Decisions

- **Why global credentials + per-vehicle sensor?** — Most users have one HA instance but multiple vehicles. Avoids credential duplication.

- **Why Rust backend for API calls?** — CORS restrictions in browser webview block direct HA API calls. Rust's reqwest has no such limitations.

- **Why localStorage cache?** — Instant display on page load. Avoids waiting for HA response on every app start.

- **Why 5-minute refresh?** — Balance between freshness and API load. ODO changes slowly; real-time updates unnecessary.

- **Why delta uses `Math.max()`?** — Trip array order may not match chronological order. Using max ensures correct delta calculation.

## Related

- `_tasks/40-home-assistant-odo/` — Original planning docs
- Migration `2026-01-27-100000_add_vehicle_ha_sensor` — Added `ha_odo_sensor` column
