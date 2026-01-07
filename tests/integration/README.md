# Integration Tests

Full end-to-end tests for the Tauri application using tauri-driver + WebdriverIO.

## Test Suite Overview

**Total: 61 tests across 4 tiers**

| Tier | Tests | Purpose | When Run |
|------|-------|---------|----------|
| existing | 10 | Original vehicle setup tests | Always |
| tier1 | 29 | Critical flows: trips, consumption, export | PRs + main |
| tier2 | 13 | Secondary: backups, receipts, settings | main only |
| tier3 | 9 | Edge cases: compensation, validation, empty states | main only |

## Prerequisites

### 1. Install tauri-driver

```bash
cargo install tauri-driver
```

### 2. Install Microsoft Edge WebDriver (Windows only)

Tauri on Windows uses WebView2 (Edge-based), so you need the Edge WebDriver:

1. Check your Edge version: `edge://version` in Edge browser
2. Download matching driver from: https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/
3. Extract `msedgedriver.exe` to a location in your PATH, or set the `MSEDGEDRIVER_PATH` environment variable

```powershell
# Example: Add to PATH
$env:PATH += ";C:\path\to\msedgedriver"

# Or set specific path
$env:MSEDGEDRIVER_PATH = "C:\path\to\msedgedriver.exe"
```

### 3. Build the Tauri app (debug)

```bash
npm run tauri build -- --debug
```

## Running Tests

```bash
# Run all integration tests
npm run test:integration

# Run only Tier 1 (fast, for PRs)
npm run test:integration:tier1

# Build and run (if you haven't built recently)
npm run test:integration:build
```

### Tiered Execution

Set the `TIER` environment variable to run specific tiers:

```bash
# Tier 1 only (existing + tier1)
set TIER=1 && npm run test:integration

# Tier 1 + 2
set TIER=2 && npm run test:integration

# All tiers (default)
npm run test:integration
```

**CI Behavior:**
- Pull Requests: Run Tier 1 only (fast feedback)
- Main branch: Run all tiers (comprehensive)

## Test Structure

```
tests/integration/
├── wdio.conf.ts          # WebdriverIO configuration
├── specs/                # Test files
│   ├── existing/         # Original tests (vehicle setup, BEV)
│   ├── tier1/            # Critical path tests
│   │   ├── seeding.spec.ts          # DB seeding verification
│   │   ├── trip-management.spec.ts  # CRUD, calculations
│   │   ├── consumption-warnings.spec.ts
│   │   ├── year-handling.spec.ts
│   │   ├── bev-trips.spec.ts
│   │   ├── phev-trips.spec.ts
│   │   └── export.spec.ts
│   ├── tier2/            # Secondary features
│   │   ├── vehicle-management.spec.ts
│   │   ├── backup-restore.spec.ts
│   │   ├── receipts.spec.ts
│   │   └── settings.spec.ts
│   └── tier3/            # Edge cases
│       ├── compensation.spec.ts
│       ├── validation.spec.ts
│       ├── multi-vehicle.spec.ts
│       └── empty-states.spec.ts
├── fixtures/             # Test data factories
│   └── seed-data.ts      # Vehicle/trip factories
├── utils/                # Helper utilities
│   ├── app.ts            # App interaction helpers
│   ├── db.ts             # DB seeding via Tauri IPC
│   └── navigation.ts     # Page navigation helpers
└── screenshots/          # Failure screenshots
```

## How It Works

1. **Sandboxed environment**: Tests use a temp directory (`%TEMP%\kniha-jazd-test-*`) for the database
2. **Fresh database**: Each test starts with an empty database
3. **DB seeding via IPC**: Tests seed data using Tauri's `invoke()` - no direct DB access needed
4. **Real app**: Tests run against the actual Tauri application with full Rust backend
5. **WebDriver protocol**: tauri-driver provides WebDriver API to control the app

## Writing Tests

### Basic Test Structure

```typescript
import { waitForAppReady } from '../utils/app';
import { seedVehicle, seedTrip } from '../utils/db';

describe('My Feature', () => {
  beforeEach(async () => {
    await waitForAppReady();
  });

  it('should do something', async () => {
    // Seed test data
    const vehicle = await seedVehicle({ name: 'Test Car' });
    await seedTrip(vehicle.id, { km: 100 });

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
import { createICEVehicle, createBEVVehicle, createTrip } from '../fixtures/seed-data';

// Pre-defined test data factories
const vehicle = createICEVehicle({ name: 'Custom Name' });
const trip = createTrip({ km: 150, liters_filled: 10 });
```

### DB Seeding

Tests seed data via Tauri IPC commands:

```typescript
import { seedVehicle, seedTrip, seedReceipt } from '../utils/db';

// Creates vehicle and returns with ID
const vehicle = await seedVehicle({ name: 'Test', tp_consumption: 7.5 });

// Creates trip linked to vehicle
const trip = await seedTrip(vehicle.id, { date: '2025-01-15', km: 100 });
```

## Troubleshooting

### "msedgedriver.exe not found"

Download from https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/ and add to PATH.

### "Timeout waiting for port 4444"

tauri-driver failed to start. Check:
- Is tauri-driver installed? (`cargo install tauri-driver`)
- Is msedgedriver available? (Windows only)
- Check console output for errors

### Tests pass locally but fail in CI

- Ensure CI installs all prerequisites
- Check platform-specific paths in `wdio.conf.ts`
- Verify Edge WebDriver version matches CI Edge version

### Test timeout (30s default)

Most tests should complete in under 10s. If a test times out:
- Check for missing `await` statements
- Verify selectors are correct
- Add debug screenshots: `await browser.saveScreenshot('./debug.png')`
