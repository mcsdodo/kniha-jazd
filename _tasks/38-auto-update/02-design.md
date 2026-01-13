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

1. App starts → checks `latest.json` from GitHub Releases (background, non-blocking, 5s timeout)
2. Compares `latest.json` version with current app version
3. If newer: shows modal with version number and release notes
4. User clicks "Update Now" → downloads installer, verifies signature, installs, **automatically restarts**
5. App restarts with new version

If user clicks "Later":
- Modal closes
- Subtle indicator appears (badge on Settings)
- Next app startup shows modal again

### Error Handling

*Addresses plan review finding C4: Incomplete Error Handling Strategy*

| Error Type | User Message (SK) | User Message (EN) | Behavior |
|------------|-------------------|-------------------|----------|
| Network timeout | Nepodarilo sa skontrolovať aktualizácie | Failed to check for updates | Silent on startup, show error on manual check |
| No internet | Skontrolujte pripojenie k internetu | Check your internet connection | Same as timeout |
| Download interrupted | Sťahovanie prerušené, skúste znova | Download interrupted, try again | Show retry button |
| Signature verification failed | Neplatná signatúra aktualizácie | Invalid update signature | Block update, log error |
| Insufficient disk space | Nedostatok miesta na disku | Insufficient disk space | Show required space |
| GitHub API error | Server aktualizácií nedostupný | Update server unavailable | Retry later |

**Retry logic:**
- Network failures: No automatic retry on startup (waits for next startup)
- Manual check: User can retry immediately
- Download interruption: Show "Retry" button, attempt up to 3 times

**Logging:**
- All update checks logged to console (dev) and file (production)
- Include: timestamp, current version, result (success/error)
- Use Tauri's logging plugin for production logs

**Recovery:**
- Partial downloads are cleaned up automatically
- Failed updates don't prevent app from running
- User can always manually download from GitHub Releases

## Release Notes Extraction

*Addresses plan review finding C3: Missing Release Notes Fetching Logic*

### Strategy: CHANGELOG.md + GitHub Actions Workflow

**Current CHANGELOG format:** Keep a Changelog standard
- Sections per version: `## [0.15.0] - 2026-01-13`
- Categories: Pridané, Opravené, Zmenené, Odstránené, Testy
- Already well-structured and machine-parsable

**Extraction approach:**
1. **During release build** (GitHub Actions), extract current version's notes from CHANGELOG.md
2. **Embed in `latest.json`** as the `notes` field
3. **Frontend displays** notes from `latest.json` (no additional API calls needed)

**Implementation:**

Add to `.github/workflows/release.yml` before the `tauri-action` step:

```yaml
- name: Extract release notes from CHANGELOG
  id: changelog
  run: |
    # Get version from tauri.conf.json
    VERSION=$(jq -r '.version' src-tauri/tauri.conf.json)
    
    # Extract section for this version from CHANGELOG.md
    # Uses awk to find lines between ## [VERSION] and next ## [ header
    NOTES=$(awk -v ver="$VERSION" '
      /^## \[/ { if (p) exit; if ($0 ~ "\\[" ver "\\]") p=1; next }
      p && /^### / { print $0; next }
      p { print $0 }
    ' CHANGELOG.md | sed 's/^### /\n**/' | sed 's/$/:**/' | head -c 1000)
    
    # Fallback if extraction fails
    if [ -z "$NOTES" ]; then
      NOTES="Verzia $VERSION - podrobnosti v CHANGELOG.md"
    fi
    
    # Output for tauri-action to use
    echo "notes<<EOF" >> $GITHUB_OUTPUT
    echo "$NOTES" >> $GITHUB_OUTPUT
    echo "EOF" >> $GITHUB_OUTPUT
```

Then in `tauri-action` config:
```yaml
releaseBody: ${{ steps.changelog.outputs.notes }}
```

**Format:**
- Plain text (not HTML)
- Slovak language (primary)
- Truncated to ~1000 chars for modal readability
- Full CHANGELOG always available at GitHub Releases page

**Fallback:**
If CHANGELOG extraction fails → use generic message: "Verzia {version} - podrobnosti v CHANGELOG.md"

**Frontend parsing:**
No parsing needed - display `latest.json.notes` as-is in modal (use `white-space: pre-wrap` for line breaks)

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

*Addresses plan review finding I5: Incomplete GitHub Actions Configuration*

**Add signing key environment variables at job level** (applies to all platform builds):

```yaml
jobs:
  build:
    permissions:
      contents: write
    strategy:
      # ... existing matrix config ...
    runs-on: ${{ matrix.platform }}
    
    env:  # <- Add job-level env block here
      TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_KEY_PASSWORD }}
    
    steps:
      # ... existing steps ...
```

**Add release notes extraction** (before \"Build and release\" step):

```yaml
      - name: Extract release notes from CHANGELOG
        id: changelog
        shell: bash
        run: |
          VERSION=$(jq -r '.version' src-tauri/tauri.conf.json)
          NOTES=$(awk -v ver=\"$VERSION\" '
            /^## \\[/ { if (p) exit; if ($0 ~ \"\\\\[\" ver \"\\\\]\") p=1; next }
            p && /^### / { print $0; next }
            p { print $0 }
          ' CHANGELOG.md | head -c 1000)
          
          if [ -z \"$NOTES\" ]; then
            NOTES=\"Verzia $VERSION - podrobnosti v CHANGELOG.md\"
          fi
          
          echo \"notes<<EOF\" >> $GITHUB_OUTPUT
          echo \"$NOTES\" >> $GITHUB_OUTPUT
          echo \"EOF\" >> $GITHUB_OUTPUT

      - name: Build and release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Kniha Jázd ${{ github.ref_name }}'
          releaseBody: ${{ steps.changelog.outputs.notes }}  # <- Use extracted notes
          releaseDraft: false
          prerelease: false
          args: ${{ matrix.args }}
```

**Key points:**
- Env vars at **job level** apply to all steps including `tauri-action`
- Both Windows and macOS use same env vars (Tauri handles platform differences)
- CHANGELOG extraction uses `bash` shell (works on Windows with Git Bash)
- Release notes embedded in GitHub Release, which tauri-action uses for `latest.json`

## Implementation Details

### Version Display API

*Addresses plan review finding C6: Missing Version Display API Command*

**Method:** Use Tauri's built-in `@tauri-apps/api/app` module

```typescript
import { getVersion } from '@tauri-apps/api/app';

const currentVersion = await getVersion(); // Returns version from tauri.conf.json
```

**No custom Tauri command needed** - Tauri provides this out of the box.

### Update Store Interface

*Addresses plan review finding I8: Update Store Pattern Not Aligned with Codebase*

**Following existing patterns** (theme.ts, toast.ts):

```typescript
// src/lib/stores/update.ts
import { writable } from 'svelte/store';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

interface UpdateState {
  checking: boolean;
  available: boolean;
  version: string | null;
  releaseNotes: string | null;
  dismissed: boolean;
  downloading: boolean;
  progress: number;
  error: string | null;
}

function createUpdateStore() {
  const { subscribe, set, update } = writable<UpdateState>({
    checking: false,
    available: false,
    version: null,
    releaseNotes: null,
    dismissed: false,
    downloading: false,
    progress: 0,
    error: null
  });

  let currentUpdate: Update | null = null;

  return {
    subscribe,
    
    check: async () => {
      update(s => ({ ...s, checking: true, error: null }));
      try {
        const result = await check({ timeout: 5000 });
        if (result) {
          currentUpdate = result;
          update(s => ({
            ...s,
            checking: false,
            available: true,
            version: result.version,
            releaseNotes: result.body || null,
            dismissed: false
          }));
        } else {
          update(s => ({ ...s, checking: false, available: false }));
        }
      } catch (error) {
        update(s => ({
          ...s,
          checking: false,
          error: error instanceof Error ? error.message : 'Unknown error'
        }));
      }
    },

    dismiss: () => {
      update(s => ({ ...s, dismissed: true }));
    },

    install: async () => {
      if (!currentUpdate) return;
      
      update(s => ({ ...s, downloading: true, progress: 0 }));
      
      try {
        await currentUpdate.downloadAndInstall((event) => {
          if (event.event === 'Progress') {
            const downloaded = event.data.chunkLength;
            const total = event.data.contentLength || 1;
            update(s => ({ ...s, progress: (downloaded / total) * 100 }));
          }
        });
        
        // Automatically restart after successful install
        await relaunch();
      } catch (error) {
        update(s => ({
          ...s,
          downloading: false,
          error: error instanceof Error ? error.message : 'Installation failed'
        }));
      }
    },

    reset: () => {
      set({
        checking: false,
        available: false,
        version: null,
        releaseNotes: null,
        dismissed: false,
        downloading: false,
        progress: 0,
        error: null
      });
      currentUpdate = null;
    }
  };
}

export const updateStore = createUpdateStore();
```

### Tauri Permissions

*Addresses plan review finding I6: Missing Capabilities Permission Details*

**Required permissions** for `src-tauri/capabilities/default.json`:

```json
{
  "permissions": [
    // ... existing permissions ...
    "updater:default",
    "process:allow-relaunch"
  ]
}
```

**Explanation:**
- `updater:default` - Includes `allow-check`, `allow-download`, `allow-install`, `allow-download-and-install`
- `process:allow-relaunch` - Required for automatic app restart after update

**From Tauri documentation:**
> "updater:default" grants the full workflow from checking for updates to installing them.

### Package Versions

*Addresses plan review findings I3, I7*

**Rust dependencies** (`src-tauri/Cargo.toml`):
```toml
tauri-plugin-updater = "2"  # Latest 2.x compatible with Tauri 2.9.5
```

**JavaScript dependencies** (`package.json`):
```json
"@tauri-apps/plugin-updater": "^2.0.0",
"@tauri-apps/plugin-process": "^2.0.0"  # For relaunch functionality
```

### Windows Install Mode Clarification

*Addresses plan review finding M7: Passive Install Mode Platform Compatibility*

**`"installMode": "passive"`** (Windows only, recommended):
- Shows small progress window with progress bar
- No user interaction required
- Installer can request admin privileges if needed
- Best UX: visible feedback without blocking

**Other options:**
- `"basicUi"`: Full installer UI, requires user clicks
- `"quiet"`: No UI at all, only works for non-admin installs

**macOS:** No installMode config needed - `.app.tar.gz` extraction is automatic and silent.

### Restart Behavior

*Addresses plan review finding M8: No Mention of Restart Behavior*

**Decision:** Automatic restart after successful installation
- Windows: App automatically closes during installation (installer limitation), then relaunches
- macOS: `relaunch()` called after installation completes
- **No user confirmation needed** - user already clicked "Update Now" button
- **No save prompt** - out of scope (may add in future if implementing unsaved changes tracking)

## Bundle Configuration

*Addresses plan review finding M9: Missing Bundle Configuration Change*

**Location in `tauri.conf.json`:**
```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true,  // <- Add this line
    "icon": [
      // ... existing icons ...
    ]
  }
}
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

## Version Synchronization Strategy

*Addresses plan review finding C1: Version Mismatch Across Files*

**Current state:**
- `package.json`: 0.15.0 ✓
- `src-tauri/tauri.conf.json`: 0.15.0 ✓
- `src-tauri/Cargo.toml`: 0.14.0 ⚠️ (out of sync)

**Required before first update-enabled release:**
1. Synchronize all version numbers to match (update Cargo.toml to 0.15.0)
2. Add version sync verification to release skill

**Ongoing version management:**
- Version is the **single source of truth** (from `tauri.conf.json`)
- `package.json` version should match for consistency
- Cargo.toml version should match (Tauri reads from conf.json for bundles)
- Update CONTRIBUTING.md to document: "When bumping version, update all three files"

**Release skill integration:**
The `/release` skill already handles version bumping. Add verification step:
```bash
# After bumping version, verify sync:
tauri_version=$(jq -r '.version' src-tauri/tauri.conf.json)
package_version=$(jq -r '.version' package.json)
cargo_version=$(grep '^version' src-tauri/Cargo.toml | cut -d'"' -f2)

if [ "$tauri_version" != "$package_version" ] || [ "$tauri_version" != "$cargo_version" ]; then
  echo "Error: Version mismatch detected!"
  exit 1
fi
```

## One-time Setup

**Prerequisites:**
1. **Synchronize versions** (see Version Synchronization Strategy above)
2. Verify Cargo.toml version matches package.json and tauri.conf.json

**Key generation:**

*Addresses plan review finding M1: Outdated CLI Command Syntax*

1. Generate keypair: `npm run tauri signer generate -- -w .tauri-keys/private.key`
   - Output: Creates `.tauri-keys/private.key` (private) and `.tauri-keys/private.key.pub` (public)
   - The `--` passes remaining args to tauri CLI (npm script syntax)

*Addresses plan review finding M2: GitHub Secret Name Inconsistency*

2. Add secrets to GitHub:
   ```bash
   # Private key (file content)
   gh secret set TAURI_SIGNING_PRIVATE_KEY < .tauri-keys/private.key
   
   # Password (use empty string if no password set during generation)
   gh secret set TAURI_SIGNING_KEY_PASSWORD
   # (prompts for value, press Enter for empty)
   ```
   
   **Secret names consistency:**
   - `TAURI_SIGNING_PRIVATE_KEY` - contains the private key content
   - `TAURI_SIGNING_KEY_PASSWORD` - contains the password (can be empty)
   - These match the env var names used in GitHub Actions

3. Commit public key in `tauri.conf.json`:
   ```bash
   # Copy public key content
   cat .tauri-keys/private.key.pub
   
   # Paste into tauri.conf.json \"plugins.updater.pubkey\" field
   ```

*Addresses plan review finding M3: Missing .gitignore Verification*

4. Verify `.gitignore` contains `.tauri-keys/`:
   ```bash
   grep -q '.tauri-keys/' .gitignore || echo '.tauri-keys/' >> .gitignore
   ```
   - Use relative path (`.tauri-keys/`) not absolute (`/.tauri-keys/`)
   - Matches directory at any level (though we only use it at root)

## Security

- Private key stored in GitHub Secrets (never in repo)
- Local backup in `.tauri-keys/` (gitignored)
- Public key embedded in app verifies signatures
- Updates only install if signature matches

## UI Components

### UpdateModal.svelte

*Addresses plan review finding M6: Modal Accessibility Not Addressed*

**Features:**
- Displays version number and release notes
- "Update Now" button with progress bar during download
- "Later" button dismisses until next startup
- Matches app's design system (dark theme support)

**Accessibility (following ConfirmModal.svelte patterns):**
- **Keyboard navigation:**
  - Tab cycles through buttons
  - Enter activates focused button
  - Escape closes modal (calls "Later" action)
- **Screen reader support:**
  - Modal has `role="dialog"` and `aria-modal="true"`
  - Title has unique `id` referenced by `aria-labelledby`
  - Release notes have `aria-label="Release notes"`
- **Focus management:**
  - Focus trapped within modal while open
  - First button (Update Now) receives focus on open
  - Focus returns to trigger element (Settings button) on close
- **Visual indicators:**
  - Clear focus outlines on all interactive elements
  - High contrast text for readability

### Settings Page Additions

- Current version display
- "Check for Updates" manual button
- Update indicator badge when update was dismissed

## "Later" Indicator Design

*Addresses plan review finding I1: Unclear "Later" Indicator Placement*

### Visual Design

**Navigation Dot:**
- Small blue dot (8×8px) using `--accent-primary` color
- Positioned after "Settings" text in navigation
- Appears when: `updateStore.available && !updateStore.dismissed`

```css
.update-dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  background-color: var(--accent-primary);
  border-radius: 50%;
  margin-left: 6px;
  vertical-align: middle;
}
```

**Settings Page Section** (placed after Appearance, before Vehicles):

```
┌─────────────────────────────────────────────────────────────┐
│  Aktualizácie                                               │
├─────────────────────────────────────────────────────────────┤
│  Verzia: 0.15.0                                             │
│                                                             │
│  ┌──────────────────────┐                                   │
│  │  Skontrolovať        │    ✓ Aplikácia je aktuálna       │
│  │  aktualizácie        │    (or)                          │
│  └──────────────────────┘    ⬆ Dostupná verzia 0.16.0      │
└─────────────────────────────────────────────────────────────┘
```

### State Management

**Persistence:** Memory only (Svelte store). Resets on app restart.

**Store Interface:**
```typescript
interface UpdateState {
  checking: boolean;       // Currently checking for updates
  available: boolean;      // Update is available
  version: string | null;  // Available version number
  releaseNotes: string | null;
  dismissed: boolean;      // User clicked "Later"
  downloading: boolean;    // Download in progress
  progress: number;        // Download progress 0-100
  error: string | null;    // Error message if any
}
```

### State Flow

```
App Start → Check update → Update available?
                              ↓ Yes
                         Show modal + nav dot
                              ↓
              User clicks "Later" → dismissed: true
                              ↓
                    Modal closes, dot hides
                              ↓
                   (Manual check still works)
                              ↓
              App restart → store resets → modal again
```

### Indicator Lifecycle

| Event | Nav Dot | Settings Status | Modal |
|-------|---------|-----------------|-------|
| App start, no update | Hidden | "Up to date" | — |
| App start, update found | Visible | "v0.16.0 available" | Shows |
| User clicks "Later" | Hidden | "v0.16.0 available" | Closes |
| User clicks "Check" (update) | Visible | "v0.16.0 available" | Shows |
| User clicks "Update Now" | — | Downloading... | Progress |
| App restart after dismiss | Visible | "v0.16.0 available" | Shows |

### Manual Check Behavior

When user clicks "Check for Updates" in Settings:
1. Button shows spinner, status shows "Checking..."
2. If update found → show UpdateModal (same as startup)
3. If no update → status shows "✓ App is up to date"
4. If error → status shows error message

### i18n Translation Keys

*Addresses plan review finding I2: Missing i18n Translation Keys*

**Complete list of required keys:**

#### Slovak (`src/lib/i18n/sk/index.ts`):
```typescript
update: {
  // Section header
  sectionTitle: 'Aktualizácie',
  
  // Version display
  currentVersion: 'Verzia',
  
  // Button text
  checkForUpdates: 'Skontrolovať aktualizácie',
  updateNow: 'Aktualizovať',
  later: 'Neskôr',
  
  // Status messages
  checking: 'Kontrolujem...',
  upToDate: 'Aplikácia je aktuálna',
  available: 'Dostupná aktualizácia',
  availableVersion: 'Dostupná verzia {version}',
  downloading: 'Sťahujem...',
  downloadProgress: 'Sťahovanie: {percent}%',
  installing: 'Inštalujem...',
  
  // Modal
  modalTitle: 'Dostupná aktualizácia',
  modalBody: 'Verzia {version} je pripravená na inštaláciu.',
  releaseNotes: 'Čo je nové:',
  
  // Errors (from error handling table)
  errorChecking: 'Nepodarilo sa skontrolovať aktualizácie',
  errorNetwork: 'Skontrolujte pripojenie k internetu',
  errorDownloadInterrupted: 'Sťahovanie prerušené, skúste znova',
  errorSignature: 'Neplatná signatúra aktualizácie',
  errorDiskSpace: 'Nedostatok miesta na disku',
  errorServerUnavailable: 'Server aktualizácií nedostupný',
  retry: 'Skúsiť znova'
}
```

#### English (`src/lib/i18n/en/index.ts`):
```typescript
update: {
  // Section header
  sectionTitle: 'Updates',
  
  // Version display
  currentVersion: 'Version',
  
  // Button text
  checkForUpdates: 'Check for updates',
  updateNow: 'Update Now',
  later: 'Later',
  
  // Status messages
  checking: 'Checking...',
  upToDate: 'App is up to date',
  available: 'Update available',
  availableVersion: 'Version {version} available',
  downloading: 'Downloading...',
  downloadProgress: 'Downloading: {percent}%',
  installing: 'Installing...',
  
  // Modal
  modalTitle: 'Update Available',
  modalBody: 'Version {version} is ready to install.',
  releaseNotes: 'What\\'s new:',
  
  // Errors
  errorChecking: 'Failed to check for updates',
  errorNetwork: 'Check your internet connection',
  errorDownloadInterrupted: 'Download interrupted, try again',
  errorSignature: 'Invalid update signature',
  errorDiskSpace: 'Insufficient disk space',
  errorServerUnavailable: 'Update server unavailable',
  retry: 'Retry'
}
```

### Accessibility

- Nav dot has `aria-label` for screen readers
- Settings section is fully keyboard navigable
- Focus management: modal traps focus, returns to trigger on close

## Verification Strategy

Testing the update flow requires simulating the complete update cycle without publishing to production GitHub Releases. We use a **local mock release server** approach.

### Local Mock Server Setup

#### Directory Structure

```
_test-releases/
├── serve.js                              # Local HTTP server script
├── latest.json                           # Update manifest (editable)
└── releases/
    └── v0.16.0/
        ├── Kniha-jazd_0.16.0_x64-setup.exe
        └── Kniha-jazd_0.16.0_x64-setup.exe.sig
```

#### Server Script (`_test-releases/serve.js`)

```javascript
const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 3456;
const BASE_DIR = __dirname;

http.createServer((req, res) => {
  const filePath = path.join(BASE_DIR, req.url === '/' ? 'latest.json' : req.url);

  if (!fs.existsSync(filePath)) {
    res.writeHead(404);
    return res.end('Not found');
  }

  const ext = path.extname(filePath);
  const contentType = ext === '.json' ? 'application/json' : 'application/octet-stream';

  res.writeHead(200, { 'Content-Type': contentType });
  fs.createReadStream(filePath).pipe(res);
}).listen(PORT, () => {
  console.log(`Mock release server running at http://localhost:${PORT}`);
});
```

#### Test Manifest (`_test-releases/latest.json`)

```json
{
  "version": "0.16.0",
  "notes": "Test release for update verification:\n- Feature A\n- Fix B",
  "pub_date": "2026-01-13T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "url": "http://localhost:3456/releases/v0.16.0/Kniha-jazd_0.16.0_x64-setup.exe",
      "signature": "<paste-signature-from-.sig-file>"
    }
  }
}
```

### Development Endpoint Override

For testing, override the updater endpoint at compile time:

**Option A: Environment variable (recommended)**
```bash
# In tauri.conf.json, use placeholder:
# "endpoints": ["$TAURI_UPDATER_ENDPOINT"]
# Then set env var for dev builds:
set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json
npm run tauri dev
```

**Option B: Separate dev config**
Create `tauri.dev.conf.json` with localhost endpoint for development builds.

### Creating Test Artifacts

1. **Build a signed release locally:**
   ```bash
   # Bump version in tauri.conf.json to test version (e.g., 0.16.0)
   npm run tauri build
   ```

2. **Copy artifacts to test folder:**
   ```bash
   cp src-tauri/target/release/bundle/nsis/*.exe _test-releases/releases/v0.16.0/
   cp src-tauri/target/release/bundle/nsis/*.exe.sig _test-releases/releases/v0.16.0/
   ```

3. **Extract signature for manifest:**
   ```bash
   # The .sig file contains the signature - paste into latest.json
   cat _test-releases/releases/v0.16.0/*.exe.sig
   ```

### Manual Test Scenarios

| # | Scenario | Setup | Expected Result |
|---|----------|-------|-----------------|
| 1 | **Happy path** | Server returns v0.16.0, app is v0.15.0 | Modal shows version + notes, "Update Now" downloads and installs |
| 2 | **No update available** | Server returns v0.15.0 (same as app) | No modal, silent completion |
| 3 | **Network failure** | Stop mock server before check | Graceful error handling, app continues working |
| 4 | **Bad signature** | Modify .exe file after signing | Update rejected with signature error |
| 5 | **"Later" dismissal** | Click "Later" on modal | Modal closes, badge appears in Settings |
| 6 | **Reminder on restart** | Dismiss, then restart app | Modal appears again on startup |
| 7 | **Manual check** | Click "Check for Updates" in Settings | Same behavior as startup check |
| 8 | **Slow download** | Add delay in server response | Progress bar shows, cancel works |
| 9 | **Download interruption** | Stop server mid-download | Graceful error, can retry |

### Rollback Procedure

If an update causes issues in production:

1. **User-side:** Previous version installers remain available in GitHub Releases history
2. **Developer-side:**
   - Publish hotfix release with higher version number
   - Or: Remove broken release from GitHub, users won't see it as "latest"
3. **Emergency:** Users can manually download any previous release from GitHub Releases page

### Verification Checklist

Before marking auto-update feature as complete:

- [ ] Local mock server test passes (scenarios 1-9)
- [ ] Signature verification rejects tampered files
- [ ] Error states show user-friendly messages (not technical errors)
- [ ] "Later" state persists across app restarts
- [ ] Settings page shows correct version number
- [ ] Manual "Check for Updates" works
- [ ] First real GitHub Release with signing works end-to-end
