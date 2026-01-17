# Local Update Testing

This folder contains a mock release server for testing the auto-update flow without publishing to GitHub Releases.

## Quick Start

### 1. Build a test release

```powershell
.\scripts\test-release.ps1
```

This will:
- Temporarily bump version (0.17.2 → 0.18.0)
- Build the release
- Copy installer to `releases/v0.18.0/`
- Update `latest.json` (if signing key is set)
- Revert version back to 0.17.2

Options:
```powershell
.\scripts\test-release.ps1 -BumpType patch   # 0.17.2 → 0.17.3
.\scripts\test-release.ps1 -BumpType minor   # 0.17.2 → 0.18.0 (default)
```

### 2. Start mock server

```bash
node _test-releases/serve.js
```

### 3. Run app with test endpoint

In a new terminal:

```bash
# Windows
set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json && npm run tauri dev
```

The app will run at the current version and detect the test release as an available update.

## Signing

For auto-update to work, the installer must be signed. Set the environment variable before building:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "your-key-here"
.\scripts\test-release.ps1
```

Without signing, you can still test the installer manually, but the auto-updater will reject unsigned updates.

## Test Scenarios

| # | Scenario | How to Test |
|---|----------|-------------|
| 1 | Happy path | Follow steps above, click "Update Now" |
| 2 | No update | Edit `latest.json` version to match current |
| 3 | Network failure | Stop the server before check |
| 4 | Bad signature | Modify the .exe after signing |
| 5 | "Later" dismissal | Click "Later", verify dot appears |
| 6 | Manual check | Use Settings > Check for Updates |

## Files

- `serve.js` - Node.js HTTP server
- `latest.json` - Update manifest (auto-updated by script)
- `releases/` - Test installer storage (gitignored)

## Manual Process

If you prefer not to use the script:

1. Bump version in `src-tauri/tauri.conf.json` and `package.json`
2. Run `npm run tauri build`
3. Copy `.exe` and `.sig` to `releases/v{version}/`
4. Update `latest.json` with version, URL, and signature
5. Revert version in config files
6. Start server and run app
