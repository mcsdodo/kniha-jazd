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
4. **Real fuel level** (optional, per vehicle): a second sensor
   (`sensor.car_fuel_level`) reporting a percentage is converted to litres against the
   vehicle's tank size and shown in brackets after the computed zostatok as
   `45.3 L (42.0 L)`. A tooltip on the bracketed value names Home Assistant as its source.
   If the fetch fails, the brackets carry the error text instead. The sensor drives
   display only; it never overwrites the book's own calculation.

**Refresh behavior:**
- ODO and fuel level fetched on app startup and every 5 minutes (in parallel)
- Cached in localStorage for instant display on page load
- Fuel level and ODO track their errors independently (`fuelError` vs `odoError`)

## Technical Implementation

### Frontend

**Main Page:** `src/routes/+page.svelte`
- Subscribes to `haStore` for cached ODO value
- Calculates delta: `haOdoValue - Math.max(...trips.map(t => t.odometer))`
- Displays in header stats row
- Converts the cached fuel-level percentage to litres (`percent × tank_size / 100`) and
  renders it inline after the computed zostatok. The conversion is display formatting,
  deliberately done here (see [ADR-013](../../DECISIONS.md)) — the backend never sees it.

**Settings Page:** `src/routes/settings/+page.svelte`
- `handleSaveHaSettings()` — Saves URL + token to backend
- `handleTestHaConnection()` — Tests connectivity via backend
- Connection status indicator (`connected` / `disconnected` / `testing`)
- Vehicle list shows real ODO for vehicles with configured sensors

**Store:** `src/lib/stores/homeAssistant.ts`
- `haStore.fetchOdo(vehicleId, sensorId)` — Fetches via Rust backend
- `haStore.fetchFuelLevel(vehicleId, sensorId)` — Same `fetch_ha_odo` RPC command
  (a generic sensor fetcher); keeps the fuel reading on the same per-vehicle cache entry
- `haStore.startPeriodicRefresh(vehicleId, odoSensorId, fuelSensorId?)` — 5-minute refresh interval for both sensors, fetched in parallel
- `haStore.getCachedOdo(vehicleId)` — Returns cached value
- LocalStorage persistence for cache

**API Wrapper:** [api.ts](../../src/lib/api.ts)
- `getHaSettings()` — Returns the HA URL, a `hasToken` flag and the env-pinned flags. It
  never returns the token itself; reading that requires `revealSecret()` and a PIN (see
  [settings-architecture.md](./settings-architecture.md))
- `saveHaSettings(url, token)` — Saves URL and/or token (`null` = leave unchanged)
- `testHaConnection()` — Tests HA connectivity using the stored credentials
- `fetchHaOdo(sensorId)` — Fetches the sensor value

### Backend (Rust)

**Commands:** [commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
- `test_ha_connection_internal` — Tests HA API connectivity using stored credentials
- `fetch_ha_odo_internal` — Fetches sensor state from HA API

Both are async, so they are dispatched from
[dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs).

**Settings:** [settings.rs](../../src-tauri/core/src/settings.rs)
- `ha_url: Option<String>` — Home Assistant URL (env override: `HA_URL`)
- `ha_api_token: Option<String>` — Long-lived access token (env override: `HA_API_TOKEN`)

**Vehicle Model:** [models.rs](../../src-tauri/core/src/models.rs)
- `ha_odo_sensor: Option<String>` — Entity ID for ODO sensor
- `ha_fuel_level_sensor: Option<String>` — Entity ID for a fuel-level percentage sensor
  (migration `2026-02-12-100000_add_vehicle_ha_fuel_level_sensor`; display-only)
- `ha_fillup_sensor: Option<String>` -- `input_text.*` helper the app pushes the fillup
  recommendation to (migration `2026-02-11-100000_add_vehicle_ha_fillup_sensor`; see
  [Outbound: Suggested-Fillup Push](#outbound-suggested-fillup-push))

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
                                    │ POST /api/rpc
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SvelteKit Frontend                           │
│  haStore → cache → localStorage → +page.svelte → header stats  │
└─────────────────────────────────────────────────────────────────┘
```

**Why the Rust backend handles API calls:**
- The browser has CORS restrictions
- HA API doesn't allow cross-origin requests from arbitrary domains
- Rust's reqwest client has no CORS limitations
- The long-lived token never has to leave the server

## Key Files

| File | Purpose |
|------|---------|
| `src/routes/+page.svelte` | Main page header with real ODO display |
| `src/routes/settings/+page.svelte` | HA configuration UI |
| `src/lib/stores/homeAssistant.ts` | Svelte store with caching + refresh |
| [src/lib/api.ts](../../src/lib/api.ts) | `getHaSettings`, `saveHaSettings`, `testHaConnection`, `fetchHaOdo` |
| [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs) | `test_ha_connection_internal`, `fetch_ha_odo_internal`, fillup push |
| [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs) | `ha_url`, `ha_api_token` fields |
| [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs) | `ha_odo_sensor`, `ha_fuel_level_sensor`, `ha_fillup_sensor` vehicle fields |

## Configuration Storage

**Global HA credentials:** `local.settings.json` in the data directory
(`/data/local.settings.json` in the container). Keys are the Rust field names, snake_case:

```json
{
  "ha_url": "https://my-ha.duckdns.org",
  "ha_api_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

Both can be overridden by the `HA_URL` / `HA_API_TOKEN` environment variables, which win
over the file and are never written back to it.

**Per-vehicle sensor:** SQLite `vehicles` table
```sql
ha_odo_sensor        TEXT  -- e.g., "sensor.car_odometer"
ha_fuel_level_sensor TEXT  -- e.g., "sensor.car_fuel_level" (percentage)
ha_fillup_sensor     TEXT  -- e.g., "input_text.car_fillup" (push target, see below)
```

**ODO cache:** LocalStorage (`kniha-jazd-ha-odo-cache`)
```json
{
  "vehicle-uuid-123": {
    "value": 45230,
    "fetchedAt": 1706351234567,
    "fuelLevelPercent": 42.0,
    "fuelFetchedAt": 1706351234567
  }
}
```

## Outbound: Suggested-Fillup Push

The integration also writes *to* HA. On every `get_trip_grid_data` the app pushes
the current fillup recommendation into an HA `input_text` helper, so automations
and dashboards can show it.

- **Per-vehicle target:** `vehicles.ha_fillup_sensor`, set in Settings → Vehicles →
  Edit ("Návrh tankovania"). Must be an **`input_text.*` helper**, not a `sensor.*`
  — the push calls the `input_text/set_value` service, which only accepts helpers.
  Create it in HA under Settings → Devices → Helpers → Create → Text.
- **Value format:** `"20.39 L → 5.66 l/100km"`, or `"Plná nádrž"` when the current
  period needs no fillup.
- **Delivery:** fire-and-forget. Errors are logged, never surfaced, and never block
  the grid — a wrong entity id or an unreachable HA looks like silence.
- **Where it runs:** `ha_fillup_push_payload` (the "should we push, and what" rule) and
  `push_ha_input_text` live in
  [core's integrations module](../../src-tauri/core/src/commands_internal/integrations.rs),
  and the server's
  [async dispatcher](../../src-tauri/core/src/server/dispatcher_async.rs) calls them from
  the `get_trip_grid_data` arm. The push once lived only in the desktop wrapper and
  silently stopped when the server became the canonical deployment — see ADR-026 in
  [DECISIONS.md](../../DECISIONS.md).

Because delivery is silent, check these first when nothing arrives: the vehicle has
a helper configured, the entity really is an `input_text`, and `HA_URL` /
`HA_API_TOKEN` (or their `local.settings.json` equivalents) are set on whichever
instance is serving the UI.

## Design Decisions

- **Why global credentials + per-vehicle sensor?** — Most users have one HA instance but multiple vehicles. Avoids credential duplication.

- **Why Rust backend for API calls?** — the browser cannot call an arbitrary HA instance
  cross-origin, and the token must not reach the browser at all. Rust's reqwest has no CORS
  constraint and keeps the credential server-side.

- **Why localStorage cache?** — Instant display on page load. Avoids waiting for HA response on every app start.

- **Why 5-minute refresh?** — Balance between freshness and API load. ODO changes slowly; real-time updates unnecessary.

- **Why delta uses `Math.max()`?** — Trip array order may not match chronological order. Using max ensures correct delta calculation.

- **Why is the percentage→litres conversion in the frontend?** — See
  [ADR-013](../../DECISIONS.md). It is display formatting, not business logic: the sensor
  reports a percentage, the vehicle knows its tank, and the book's own computed zostatok
  is never touched by the HA reading.

## Related

- `_tasks/_done/40-home-assistant-odo/` — Original planning docs
- `_tasks/_done/53-ha-real-fuel-level/` — the fuel-level sensor display
- Migration `2026-01-27-100000_add_vehicle_ha_sensor` — Added `ha_odo_sensor` column
- Migration `2026-02-12-100000_add_vehicle_ha_fuel_level_sensor` — Added `ha_fuel_level_sensor` column
- [ADR-013](../../DECISIONS.md) — HA sensor percentage-to-litres conversion lives in the frontend
