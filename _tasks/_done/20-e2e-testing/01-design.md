# E2E Integration Testing Design

**Date:** 2026-01-05
**Status:** Approved
**Goal:** Full integration E2E testing with sandboxed database for the Tauri application

## Overview

Add end-to-end integration tests that run the real Tauri application with real Rust backend and SQLite database, in an isolated test environment that doesn't affect development data.

## Technology Choice

**tauri-driver + WebdriverIO** (official Tauri approach)

| Factor | Decision |
|--------|----------|
| Test runner | WebdriverIO v9 |
| Browser automation | tauri-driver (WebDriver protocol) |
| Framework | Mocha (via @wdio/mocha-framework) |
| Platforms | Windows, macOS, Linux |

### Why not Playwright + CDP?

- CDP behavior varies between WebView engines (WebView2 vs WebKit)
- Less official support, may break with Tauri updates
- tauri-driver is maintained by Tauri team

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Test Runner (npm)                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │  WebdriverIO │───►│ tauri-driver │───►│    Tauri App     │  │
│  │   (tests)    │    │  (WebDriver) │    │ (debug build)    │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│         │                                        │              │
│         │            ┌──────────────┐            │              │
│         └───────────►│  Test Utils  │◄───────────┘              │
│                      │ (seed data,  │                           │
│                      │  assertions) │                           │
│                      └──────────────┘                           │
├─────────────────────────────────────────────────────────────────┤
│              Temp Directory (sandboxed per test run)            │
│              %LOCALAPPDATA%\Temp\kniha-jazd-test-{ts}\          │
│              └── kniha-jazd.db                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Database Sandboxing

### Implementation

Add environment variable override in `src-tauri/src/lib.rs`:

```rust
// Allow tests to specify custom data directory
let app_dir = match std::env::var("KNIHA_JAZD_DATA_DIR") {
    Ok(path) => PathBuf::from(path),
    Err(_) => app.path().app_data_dir().expect("Failed to get app data dir"),
};
```

### Test Isolation

- Each test run creates fresh temp directory
- Location: `%LOCALAPPDATA%\Temp\kniha-jazd-test-{timestamp}\` (Windows)
- Database deleted before each test (clean slate)
- Temp directory removed after test suite completes

## Directory Structure

```
tests/
├── e2e/                    # Existing Playwright smoke tests
│   ├── example.spec.ts
│   ├── doklady.spec.ts
│   └── receipt-assignment.spec.ts
├── integration/            # New: Full Tauri integration tests
│   ├── wdio.conf.ts        # WebdriverIO configuration
│   ├── specs/
│   │   ├── vehicle-setup.spec.ts
│   │   ├── trip-crud.spec.ts
│   │   ├── fuel-calculation.spec.ts
│   │   ├── compensation.spec.ts
│   │   └── export.spec.ts
│   ├── fixtures/
│   │   └── seed-data.ts    # Test data factories
│   └── utils/
│       ├── app.ts          # App launch/teardown helpers
│       └── db.ts           # Direct SQLite seeding
└── tsconfig.json
```

## Dependencies

```json
{
  "devDependencies": {
    "@wdio/cli": "^9.0.0",
    "@wdio/local-runner": "^9.0.0",
    "@wdio/mocha-framework": "^9.0.0",
    "@wdio/spec-reporter": "^9.0.0"
  }
}
```

Plus `cargo install tauri-driver` for the WebDriver server.

## Test Lifecycle

### Before All Tests
1. Create temp directory: `/tmp/kniha-jazd-test-{timestamp}` (or platform equivalent)
2. Set `KNIHA_JAZD_DATA_DIR` environment variable
3. Start tauri-driver (WebDriver server on port 4444)
4. Build app if needed: `cargo build --debug`

### Before Each Test
1. Delete existing test DB (clean slate)
2. Seed test data via SQLite (optional, per-test)
3. Launch Tauri app via tauri-driver
4. Wait for app window to be ready

### After Each Test
1. Close Tauri app
2. Optionally: snapshot DB state for debugging failed tests

### After All Tests
1. Stop tauri-driver
2. Delete temp directory
3. Generate test report

## WebdriverIO Configuration

```typescript
// tests/integration/wdio.conf.ts
import { spawn, ChildProcess } from 'child_process';
import { mkdtempSync, rmSync, existsSync, unlinkSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

let tauriDriver: ChildProcess;
let testDataDir: string;

const getBinaryPath = () => {
  if (process.env.TAURI_BINARY) {
    return process.env.TAURI_BINARY;
  }

  const platform = process.platform;
  const base = '../../src-tauri/target/debug';

  switch (platform) {
    case 'win32':
      return `${base}/kniha-jazd.exe`;
    case 'darwin':
      return `${base}/bundle/macos/Kniha Jázd.app`;
    case 'linux':
      return `${base}/kniha-jazd`;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
};

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./specs/**/*.spec.ts'],
  maxInstances: 1, // Tauri apps run one at a time

  capabilities: [{
    'tauri:options': {
      application: getBinaryPath(),
    }
  }],

  framework: 'mocha',
  reporters: ['spec'],

  mochaOpts: {
    timeout: 60000,
  },

  onPrepare: async () => {
    // Create sandboxed directory
    testDataDir = mkdtempSync(join(tmpdir(), 'kniha-jazd-test-'));
    process.env.KNIHA_JAZD_DATA_DIR = testDataDir;

    // Start tauri-driver
    tauriDriver = spawn('tauri-driver', [], { stdio: 'pipe' });

    // Wait for tauri-driver to be ready
    await new Promise(resolve => setTimeout(resolve, 2000));
  },

  onComplete: async () => {
    tauriDriver?.kill();
    if (testDataDir) {
      rmSync(testDataDir, { recursive: true, force: true });
    }
  },

  beforeTest: async () => {
    // Fresh DB for each test
    const dbPath = join(testDataDir, 'kniha-jazd.db');
    if (existsSync(dbPath)) {
      unlinkSync(dbPath);
    }
  }
};
```

## npm Scripts

```json
{
  "scripts": {
    "test": "vitest",
    "test:run": "vitest run",
    "test:e2e": "playwright test",
    "test:e2e:ui": "playwright test --ui",
    "test:backend": "cd src-tauri && cargo test",
    "test:integration": "wdio run tests/integration/wdio.conf.ts",
    "test:integration:build": "npm run tauri build -- --debug && npm run test:integration",
    "test:all": "npm run test:backend && npm run test:run && npm run test:integration"
  }
}
```

## CI Pipeline (GitHub Actions)

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  backend-tests:
    strategy:
      matrix:
        os: [windows-latest, macos-latest, ubuntu-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-action@stable
      - name: Run backend tests
        run: cd src-tauri && cargo test

  integration-tests:
    needs: backend-tests
    strategy:
      matrix:
        include:
          - os: windows-latest
            binary: src-tauri/target/debug/kniha-jazd.exe
          - os: macos-latest
            binary: src-tauri/target/debug/bundle/macos/Kniha Jázd.app
          - os: ubuntu-latest
            binary: src-tauri/target/debug/kniha-jazd
    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install Linux dependencies
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev

      - name: Install dependencies
        run: npm ci

      - name: Install tauri-driver
        run: cargo install tauri-driver

      - name: Build Tauri app (debug)
        run: npm run tauri build -- --debug

      - name: Run integration tests
        env:
          TAURI_BINARY: ${{ matrix.binary }}
        run: npm run test:integration

      - name: Upload test artifacts
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: test-results-${{ matrix.os }}
          path: |
            tests/integration/results/
            tests/integration/screenshots/
```

## Test Cases to Implement

### Priority 1: Core User Flows (5 tests)
1. **Vehicle setup** - Create vehicle with TP consumption rate
2. **Trip with refueling** - Add trip, verify consumption rate calculated
3. **Over-margin warning** - Verify warning appears when >120% TP rate
4. **Compensation suggestion** - Verify suggestion flow reduces margin
5. **Export** - Verify HTML export generates correctly

### Priority 2: CRUD Operations (3 tests)
6. **Trip editing** - Edit existing trip, verify recalculation
7. **Trip deletion** - Delete trip, verify totals update
8. **Year switching** - Change year, verify correct data loads

### Priority 3: Edge Cases (2 tests)
9. **Receipt assignment** - Assign receipt to trip
10. **Empty state** - App loads correctly with no data

## Test Pyramid

```
                    ┌─────────────────────────┐
                    │     Integration         │  5-10 tests × 3 platforms
                    │  (Tauri + WebdriverIO)  │  ~2-3 min per platform
                    ├─────────────────────────┤
                    │         E2E             │  ~15 tests (web only)
                    │     (Playwright)        │  ~10s
                    ├─────────────────────────┤
                    │       Backend           │  72 tests × 3 platforms
                    │       (Rust)            │  ~5s per platform
                    └─────────────────────────┘
```

## Implementation Steps

1. **Rust change** - Add `KNIHA_JAZD_DATA_DIR` env var support to `lib.rs`
2. **Dependencies** - Install WebdriverIO packages
3. **Configuration** - Create `wdio.conf.ts`
4. **Utilities** - Create seed data and app launch helpers
5. **First test** - Implement vehicle setup test
6. **Expand** - Add remaining priority 1 tests
7. **CI** - Add GitHub Actions workflow
8. **Documentation** - Update CLAUDE.md with new test commands
