# E2E Integration Testing - Implementation Plan

## Overview

Implement full Tauri integration testing using tauri-driver + WebdriverIO with sandboxed database.

See `01-design.md` for detailed architecture and decisions.

## Implementation Tasks

### Phase 1: Foundation

- [ ] **1.1** Add `KNIHA_JAZD_DATA_DIR` env var support to `src-tauri/src/lib.rs`
  - Single line change to check env var before using default app_data_dir
  - Add test to verify env var is respected

- [ ] **1.2** Install WebdriverIO dependencies
  ```bash
  npm install -D @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter
  ```

- [ ] **1.3** Install tauri-driver
  ```bash
  cargo install tauri-driver
  ```

- [ ] **1.4** Create directory structure
  ```
  tests/integration/
  ├── wdio.conf.ts
  ├── specs/
  ├── fixtures/
  └── utils/
  ```

### Phase 2: Configuration

- [ ] **2.1** Create `tests/integration/wdio.conf.ts`
  - Cross-platform binary path detection
  - Temp directory lifecycle (create/cleanup)
  - tauri-driver spawn/kill
  - Fresh DB before each test

- [ ] **2.2** Create `tests/integration/utils/app.ts`
  - Helper to wait for app ready state
  - Screenshot on failure helper

- [ ] **2.3** Create `tests/integration/utils/db.ts`
  - SQLite seeding utilities
  - Direct database connection for test setup

- [ ] **2.4** Create `tests/integration/fixtures/seed-data.ts`
  - `seedVehicle()` - create test vehicle
  - `seedTrip()` - create test trip
  - `seedRoute()` - create saved route
  - `seedReceipt()` - create test receipt

### Phase 3: First Test (Proof of Concept)

- [ ] **3.1** Create `tests/integration/specs/vehicle-setup.spec.ts`
  - Test: App loads successfully
  - Test: Create new vehicle with TP consumption
  - Verify vehicle appears in list

- [ ] **3.2** Add npm scripts to `package.json`
  ```json
  "test:integration": "wdio run tests/integration/wdio.conf.ts",
  "test:integration:build": "npm run tauri build -- --debug && npm run test:integration"
  ```

- [ ] **3.3** Verify first test runs successfully on local machine

### Phase 4: Core Test Cases

- [ ] **4.1** `trip-crud.spec.ts`
  - Create trip with all fields
  - Edit existing trip
  - Delete trip
  - Verify grid updates correctly

- [ ] **4.2** `fuel-calculation.spec.ts`
  - Add trip with refueling (full tank)
  - Verify consumption rate calculated (l/100km)
  - Verify zostatok (fuel remaining) calculated
  - Verify spotreba displayed correctly

- [ ] **4.3** `margin-warning.spec.ts`
  - Seed vehicle with TP rate 7.0
  - Create trips resulting in >120% consumption
  - Verify warning indicator appears
  - Verify margin percentage shown

- [ ] **4.4** `compensation.spec.ts`
  - Seed over-margin scenario
  - Seed saved routes
  - Open compensation dialog
  - Verify suggestion appears
  - Accept suggestion
  - Verify margin reduced

- [ ] **4.5** `export.spec.ts`
  - Seed vehicle with trips
  - Trigger export
  - Verify export command completes (file created or browser opens)

### Phase 5: CI Integration

- [ ] **5.1** Create `.github/workflows/integration-tests.yml`
  - Matrix: windows-latest, macos-latest, ubuntu-latest
  - Install dependencies (Rust, Node, platform-specific)
  - Build debug app
  - Run integration tests
  - Upload artifacts on failure

- [ ] **5.2** Update existing test workflow to include integration tests
  - Add `needs: backend-tests` dependency
  - Only run integration if backend passes

### Phase 6: Documentation

- [ ] **6.1** Update `CLAUDE.md` with new test commands
  - Add `test:integration` to common commands
  - Document test structure

- [ ] **6.2** Update `CHANGELOG.md` via `/changelog`

## Verification

After each phase, verify:
- [ ] Tests pass locally
- [ ] No interference with development database
- [ ] Temp directories cleaned up properly

## Notes

- Build debug app once, reuse for all tests (faster)
- Keep integration tests focused on user flows, not edge cases
- Edge cases belong in Rust unit tests (faster feedback)
