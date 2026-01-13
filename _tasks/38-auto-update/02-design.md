# Auto-Update Feature Design

## Overview

Implement automatic update checking and installation for Kniha Jázd using Tauri's official updater plugin with GitHub Releases as the distribution channel.

## Decisions Made

| Aspect | Decision |
|--------|----------|
| Hosting | GitHub Releases |
| Update check timing | On startup + manual button in Settings |
| UX when update found | Modal with Update/Later buttons |
| "Later" behavior | Remind next startup + subtle indicator |
| Release workflow | Extend GitHub Actions (no changes to local `/release` skill) |
| Key storage | GitHub Secrets |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        GitHub Releases                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ latest.json     │  │ setup.exe       │  │ setup.exe.sig   │ │
│  │ (update manifest)│  │ (installer)     │  │ (signature)     │ │
│  └────────┬────────┘  └─────────────────┘  └─────────────────┘ │
└───────────│─────────────────────────────────────────────────────┘
            │
            │ HTTPS
            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Kniha Jázd App                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │ On Startup   │───▶│ Check Update │───▶│ Show Modal   │      │
│  └──────────────┘    │ (background) │    │ if available │      │
│                      └──────────────┘    └──────┬───────┘      │
│  ┌──────────────┐                               │              │
│  │ Settings     │    ┌──────────────┐           ▼              │
│  │ "Check Now"  │───▶│ Download &   │◀──── [Update Now]        │
│  └──────────────┘    │ Install      │                          │
│                      └──────────────┘                          │
│                             │                                  │
│                             ▼                                  │
│                      ┌──────────────┐                          │
│                      │ Restart App  │                          │
│                      └──────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

## Update Flow

1. App starts → checks `latest.json` from GitHub Releases (background)
2. Compares `latest.json` version with current app version
3. If newer: shows modal with version number and release notes
4. User clicks "Update Now" → downloads installer, verifies signature, installs
5. App restarts with new version

If user clicks "Later":
- Modal closes
- Subtle indicator appears (badge on Settings)
- Next app startup shows modal again

## Configuration

### tauri.conf.json

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "<generated-public-key>",
      "endpoints": [
        "https://github.com/mcsdodo/kniha-jazd/releases/latest/download/latest.json"
      ],
      "windows": {
        "installMode": "passive"
      }
    }
  }
}
```

### GitHub Actions (release.yml)

Add signing key environment variables:

```yaml
env:
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_KEY_PASSWORD }}
```

## Files to Create

| File | Purpose |
|------|---------|
| `.tauri-keys/` | Gitignored folder for keypair |
| `.tauri-keys/private.key` | Signing private key (local backup) |
| `.tauri-keys/public.key` | Public key (also in config) |
| `src/lib/stores/update.ts` | Update state store |
| `src/lib/components/UpdateModal.svelte` | Update notification modal |

## Files to Modify

| File | Changes |
|------|---------|
| `.gitignore` | Add `.tauri-keys/` |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-updater` |
| `src-tauri/tauri.conf.json` | Add updater config + pubkey |
| `src-tauri/src/lib.rs` | Register updater plugin |
| `src-tauri/capabilities/default.json` | Add updater permissions |
| `package.json` | Add `@tauri-apps/plugin-updater` |
| `.github/workflows/release.yml` | Add signing key env vars |
| `src/routes/+layout.svelte` | Startup update check |
| `src/routes/settings/+page.svelte` | Manual check button + version display |
| `src/lib/i18n/sk/index.ts` | Slovak translations |
| `src/lib/i18n/en/index.ts` | English translations |

## One-time Setup

1. Generate keypair: `npx tauri signer generate -w .tauri-keys/private.key`
2. Add to GitHub: `gh secret set TAURI_SIGNING_PRIVATE_KEY < .tauri-keys/private.key`
3. Commit public key in `tauri.conf.json`

## Security

- Private key stored in GitHub Secrets (never in repo)
- Local backup in `.tauri-keys/` (gitignored)
- Public key embedded in app verifies signatures
- Updates only install if signature matches

## UI Components

### UpdateModal.svelte

- Displays version number and release notes
- "Update Now" button with progress bar during download
- "Later" button dismisses until next startup
- Accessible, matches app's design system

### Settings Page Additions

- Current version display
- "Check for Updates" manual button
- Update indicator badge when update was dismissed
