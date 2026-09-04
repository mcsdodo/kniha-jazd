# Integration Tests

End-to-end tests that drive a real Chrome browser (WebdriverIO) against the
`kniha-jazd-web` HTTP server — the same binary the Docker image ships. The browser
loads the built SvelteKit bundle the server serves, and both the app and the test
helpers talk to the backend over JSON-RPC at `POST /api/rpc`.

## Test Suite Overview

| Folder | Spec files | Purpose |
|--------|-----------|---------|
| [existing](./specs/existing/) | 2 | Original vehicle setup tests (ICE + BEV) |
| [tier1](./specs/tier1/) | 9 | Critical flows: trips, consumption, export |
| [tier2](./specs/tier2/) | 16 | Secondary: backups, receipts, settings, Paperless, route maps |
| [tier3](./specs/tier3/) | 4 | Edge cases: compensation, validation, empty states |
| [env](./specs/env/) | 1 | Settings pinned by environment variables — runs separately (see below) |

> Tier names describe scope, not when they run. CI executes all tiers in parallel on every push to `main` and every PR (see [.github/workflows/test.yml](../../.github/workflows/test.yml)). Tiers exist to let you scope local runs — run [tier1](./specs/tier1/) for a quick check, run the full suite before claiming work done. See [CLAUDE.md](../../CLAUDE.md) → Iteration strategy for the canonical local workflow.

## Prerequisites

### 1. Google Chrome

WebdriverIO downloads a matching chromedriver itself; you only need Chrome installed
and on the default path for your OS.

### 2. Node dependencies

```bash
npm ci
```

### 3. Build the frontend and the server binary

The frontend must be built first: the spawned server serves `build/` as its
`STATIC_DIR`, so a stale or missing bundle means the browser loads nothing.

```bash
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
```

Not needed for Docker mode — the container image already contains both.

## Running Tests

There are two modes. Both use the same config file
([wdio.server.conf.ts](./wdio.server.conf.ts)); `WDIO_EXTERNAL_SERVER` picks
between them.

### Spawned-server mode (default, port 3457)

WDIO starts `src-tauri/target/debug/kniha-jazd-web` itself, pointed at a fresh temp
data directory, and shuts it down afterwards. Port 3457 is deliberately not 3456 so a
running container or app instance does not collide with the test run.

```bash
# All tiers
npx wdio run tests/integration/wdio.server.conf.ts
npm run test:integration        # same thing

# Single spec — use this while iterating on a fix
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/legal-compliance.spec.ts

# Tier 1 only (fast)
npm run test:integration:tier1
```

### External-server mode (Docker, port 3456)

The server must already be listening — WDIO only waits for `/health` and never spawns
or stops anything.

```bash
WDIO_EXTERNAL_SERVER=1 npx wdio run tests/integration/wdio.server.conf.ts
npm run test:integration:docker   # same thing
```

Start the container the way CI does (see the `Start container` step in
[.github/workflows/test.yml](../../.github/workflows/test.yml)) — in particular
[tests/integration/data](./data) must be mounted at `/testdata`, because specs send fixture
paths over RPC and the backend resolves them inside the container. See
[utils/paths.ts](./utils/paths.ts) for the two mount mappings.

### Environment variables

| Variable | Effect |
|----------|--------|
| `TIER` | `1`, `2` or `3` — scope the run to a tier (see below) |
| `PARALLEL_TIERS` | `true` makes `TIER` select *only* that tier instead of it plus everything below |
| `WDIO_EXTERNAL_SERVER` | `1` = connect to an already-running server instead of spawning one |
| `WDIO_ENV_PINNED` | `1` = run only `specs/env/**`, with settings pinned via env vars |
| `WDIO_SERVER_PORT` / `WDIO_SERVER_URL` | Override the port / full URL |
| `KJ_WEB_BINARY` | Path to the server binary to spawn (CI uses this) |

### Tiered Execution

```bash
# Tier 1 only (tier1 + existing)
TIER=1 npx wdio run tests/integration/wdio.server.conf.ts

# Tier 1 + 2 (sequential mode is cumulative)
TIER=2 npx wdio run tests/integration/wdio.server.conf.ts

# Exactly one tier (what CI does)
TIER=2 PARALLEL_TIERS=true npx wdio run tests/integration/wdio.server.conf.ts
```

The npm scripts use the Windows `set VAR=x&&` form; on Linux/macOS use the inline
`VAR=x` prefix above.

**CI Behavior:**
- All three tiers run in parallel on both PRs and pushes to `main` (matrix in [.github/workflows/test.yml](../../.github/workflows/test.yml)), each against its own container.
- The env-pinned suite runs as a fourth job against a second container started with the pinning variables.
- The `TIER` env var is for *local* scoping — CI sets `TIER` per matrix job to fan out the suite across runners, not to skip tiers.

### The env-pinned suite

[specs/env/env-managed-settings.spec.ts](./specs/env/env-managed-settings.spec.ts)
checks that settings supplied through
environment variables (`HA_URL`, `PAPERLESS_API_TOKEN`, `KNIHA_JAZD_REVEAL_PIN`, …)
are shown as read-only in the UI. Those variables make settings read-only app-wide,
which would break every spec that edits them — hence a separate run:

```bash
npm run test:integration:docker:env   # WDIO_ENV_PINNED=1, against a pinned container
```

Normal runs blank those variables out before spawning the server, so a developer's
real `.env` cannot pin settings and fail unrelated specs.

## Test Structure

```
tests/integration/
├── wdio.server.conf.ts   # WebdriverIO config: spawns/connects, resets DB, Chrome caps
├── specs/                # Test files
│   ├── _helpers/         # Spec-local helpers (mock Paperless server)
│   ├── env/              # Env-pinned settings (separate run)
│   ├── existing/         # Original tests (vehicle setup, BEV)
│   ├── tier1/            # Critical path: trips, consumption, export, seeding
│   ├── tier2/            # Secondary features: settings, receipts, Paperless, maps
│   └── tier3/            # Edge cases: compensation, validation, multi-vehicle
├── fixtures/             # Test data factories
│   ├── vehicles.ts       # ICE / BEV / PHEV vehicle factories + UI creation helpers
│   ├── trips.ts          # Trip factories, Slovak cities, purposes
│   ├── receipts.ts       # Receipt factories in every status
│   ├── scenarios.ts      # Whole-vehicle scenarios (under limit, over limit, …)
│   └── types.ts          # TS mirrors of the Rust models
├── utils/                # Helper utilities
│   ├── app.ts            # waitForAppReady, navigateTo
│   ├── db.ts             # rpc() + all seed/query helpers
│   ├── forms.ts          # Form filling helpers
│   ├── assertions.ts     # Shared expectations
│   ├── language.ts       # Locale switching
│   └── paths.ts          # Host ↔ container path translation
├── data/                 # Committed fixtures (invoice PDFs, Gemini mock JSON)
└── screenshots/          # Failure screenshots
```

## How It Works

1. **Isolated data dir**: spawned mode creates `%TEMP%\kniha-jazd-server-test-*` and passes it as `KNIHA_JAZD_DATA_DIR`; Docker mode uses the container's `/data` mount.
2. **Fresh database**: `before` deletes every trip and vehicle over RPC, so each spec file starts empty. It does *not* run per test — WDIO's `beforeTest` fires after a spec's own `beforeEach`, so resetting there would wipe data the spec just seeded.
3. **Seeding over RPC**: tests seed through [`rpc()`](./utils/db.ts) → `POST /api/rpc`, the same endpoint the frontend uses, so the backend validates and stores exactly as it would in production. No direct SQLite access.
4. **Mocked externals**: `KNIHA_JAZD_MOCK_GEMINI_DIR` makes receipt scanning read fixture JSON instead of calling Gemini; Paperless specs spin up [a local mock server](./specs/_helpers/mock-paperless-server.ts).
5. **Real browser**: Chrome over the WebDriver protocol — the tests exercise the shipped bundle, not a component harness.

## Writing Tests

### Basic Test Structure

```typescript
import { waitForAppReady } from '../../utils/app';
import { seedVehicle, seedTrip } from '../../utils/db';

describe('My Feature', () => {
  beforeEach(async () => {
    await waitForAppReady();
  });

  it('should do something', async () => {
    // Seed test data
    const vehicle = await seedVehicle({ name: 'Test Car', licensePlate: 'T-001', initialOdometer: 10000 });
    await seedTrip({
      vehicleId: vehicle.id,
      startDatetime: '2026-01-15T08:00',
      origin: 'A',
      destination: 'B',
      distanceKm: 100,
      odometer: 10100,
      purpose: 'Test',
    });

    // Navigate and interact
    const button = await $('button=Click me');
    await button.click();

    // Assert
    await expect($('.result')).toHaveText('Success');
  });
});
```

### Using Fixtures

```typescript
import { createTestIceVehicle, createSkodaOctavia } from '../../fixtures/vehicles';
import { createTrip, createTripWithFuel } from '../../fixtures/trips';
import { createOverLimitScenario } from '../../fixtures/scenarios';

const vehicle = createTestIceVehicle({ name: 'Custom Name' });
const trip = createTrip({ distanceKm: 150 });
const scenario = createOverLimitScenario();  // vehicle + trips, seed with seedScenario()
```

### DB Seeding

Tests seed data over JSON-RPC:

```typescript
import { seedVehicle, seedTrip, seedReceipt } from '../../utils/db';

// Creates vehicle and returns with ID
const vehicle = await seedVehicle({ name: 'Test', licensePlate: 'T-001', initialOdometer: 10000, tpConsumption: 7.5 });

// Creates trip linked to vehicle
const trip = await seedTrip({ vehicleId: vehicle.id, startDatetime: '2026-01-15T08:00', origin: 'A', destination: 'B', distanceKm: 100, odometer: 10100, purpose: 'Test' });

// Creates a processed (Parsed) unassigned receipt. There is no create_receipt
// command, so this writes a placeholder file into the sandboxed data dir,
// scans it, then fills in the parsed fields via update_receipt.
// Requires a filesystem shared with the backend — skip in Docker mode.
const receipt = await seedReceipt({ assignmentType: 'Other', totalPriceEur: 10.0, receiptDatetime: '2026-01-15T09:00' });
```

Reach for `rpc()` directly rather than any other route to the backend — it is the
single point of backend communication for the whole test suite.

## Troubleshooting

### "Timed out waiting for http://localhost:3457/health"

The spawned server never came up. Check:
- Was the binary built? (`cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web`, from [src-tauri/web](../../src-tauri/web))
- Is something else already on 3457? Set `WDIO_SERVER_PORT` to a free port.
- Run the binary by hand with `KNIHA_JAZD_DATA_DIR` set and read its output — WDIO
  spawns it with `stdio: 'ignore'`, so startup errors are invisible in the test log.

### "Timed out waiting for http://localhost:3456/health" (Docker mode)

The container is not running or not healthy: `docker ps`, then `docker logs kniha-jazd-web`.

### A Docker-mode spec fails on a connection the *backend* could not make

Most likely the container cannot reach a mock server running in the test process.
Some specs start one on the host's `127.0.0.1` and hand the backend its URL, so the
backend has to call back out to the host.

CI starts the container with `--network=host`, which on Linux shares the host's
network stack and makes that work. Docker Desktop for Windows/macOS does not support
that, so locally you publish a port instead (`-p 3456:3456`) — and then the
container's `127.0.0.1` is the container, not your machine.

`paperless-integration.spec.ts` is the one that hits this. It fails locally under
published ports and passes in CI, and the failure looks like an application bug
rather than a networking one. Run that spec in spawned mode instead, where the
backend is a host process:

```bash
npx wdio run tests/integration/wdio.server.conf.ts   --spec tests/integration/specs/tier2/paperless-integration.spec.ts
```

### App loads but every page is blank

`STATIC_DIR` points at the repo's `build/` directory, which is missing or stale.
Re-run `npm run build` ([package.json](../../package.json)).

### "managed by the ... environment variable" in a settings spec

A real value in the repo's `.env` leaked into the spawned server.
[wdio.server.conf.ts](./wdio.server.conf.ts) blanks
the known variables (`SCRUBBED_ENV`); if you added a new overridable setting, add its
variable there too.

### Tests pass locally but fail in CI

CI runs Docker mode on Linux. Differences that bite: fixture paths must go through
[`utils/paths.ts`](./utils/paths.ts), and specs that write files for the backend to
read need a shared filesystem (skip them in Docker mode).

### Test timeout (30s default)

Most tests should complete in under 10s. If a test times out:
- Check for missing `await` statements
- Verify selectors are correct
- Add debug screenshots: `await browser.saveScreenshot('./debug.png')`
