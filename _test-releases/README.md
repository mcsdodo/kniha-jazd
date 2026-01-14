# Local Update Testing

This folder contains a mock release server for testing the auto-update flow without publishing to GitHub Releases.

## Quick Start

### 1. Build a test release

First, bump the version in `src-tauri/tauri.conf.json` to a higher version (e.g., `0.16.0`):

```bash
npm run tauri build
```

### 2. Copy artifacts

```bash
# Windows
cp src-tauri/target/release/bundle/nsis/*.exe _test-releases/releases/v0.16.0/
cp src-tauri/target/release/bundle/nsis/*.exe.sig _test-releases/releases/v0.16.0/
```

### 3. Update latest.json

Copy the signature from the `.sig` file into `latest.json`:

```bash
cat _test-releases/releases/v0.16.0/*.exe.sig
```

Paste this value into the `signature` field in `latest.json`.

### 4. Revert version and start server

Revert `tauri.conf.json` back to `0.15.0` (so the app thinks there's an update available).

Start the mock server:

```bash
node _test-releases/serve.js
```

### 5. Run app with test endpoint

In a new terminal:

```bash
# Windows
set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json && npm run tauri dev
```

The app should detect version `0.16.0` as available and show the update modal.

## Test Scenarios

| # | Scenario | How to Test |
|---|----------|-------------|
| 1 | Happy path | Follow steps above, click "Update Now" |
| 2 | No update | Set `latest.json` version to `0.15.0` |
| 3 | Network failure | Stop the server before check |
| 4 | Bad signature | Modify the .exe after signing |
| 5 | "Later" dismissal | Click "Later", verify dot appears |
| 6 | Manual check | Use Settings > Check for Updates |

## Files

- `serve.js` - Node.js HTTP server
- `latest.json` - Update manifest template
- `releases/` - Test installer storage (gitignored)
