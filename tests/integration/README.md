# Integration Tests

Full end-to-end tests for the Tauri application using tauri-driver + WebdriverIO.

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
# Run integration tests
npm run test:integration

# Build and run (if you haven't built recently)
npm run test:integration:build
```

## Test Structure

```
tests/integration/
├── wdio.conf.ts          # WebdriverIO configuration
├── specs/                # Test files
│   └── vehicle-setup.spec.ts
├── fixtures/             # Test data factories
│   └── seed-data.ts
├── utils/                # Helper utilities
│   ├── app.ts            # App interaction helpers
│   └── db.ts             # Database utilities
└── screenshots/          # Failure screenshots
```

## How It Works

1. **Sandboxed environment**: Tests use a temp directory (`%TEMP%\kniha-jazd-test-*`) for the database
2. **Fresh database**: Each test starts with an empty database
3. **Real app**: Tests run against the actual Tauri application with full Rust backend
4. **WebDriver protocol**: tauri-driver provides WebDriver API to control the app

## Writing Tests

```typescript
import { waitForAppReady } from '../utils/app';

describe('My Feature', () => {
  beforeEach(async () => {
    await waitForAppReady();
  });

  it('should do something', async () => {
    const button = await $('button=Click me');
    await button.click();
    await expect($('.result')).toHaveText('Success');
  });
});
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
