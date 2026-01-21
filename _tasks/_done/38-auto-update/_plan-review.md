# Plan Review: Auto-Update Feature

**Date:** 2026-01-13  
**Reviewer:** AI Assistant  
**Iteration:** Round 2 (Final)

---

## Executive Summary

The auto-update design is well-structured and comprehensive, covering the main workflow and configuration requirements. However, several **critical** issues need resolution before implementation, primarily around version synchronization, platform-specific configuration, and missing implementation details.

**Round 2 Update:** Additional findings identified related to API imports, version display implementation, and passive install mode clarification.

---

## Round 1 Findings

### Critical Issues

#### C1: Version Mismatch Across Files
**Finding:** Current codebase has version inconsistencies that will break updates:
- `package.json`: 0.15.0
- `src-tauri/tauri.conf.json`: 0.15.0
- `src-tauri/Cargo.toml`: 0.14.0

**Impact:** The updater compares versions from `tauri.conf.json`, but Cargo.toml being out of sync will cause build inconsistencies and potential update failures.

**Required Action:** Implementation plan must include a step to synchronize all version numbers before first update-enabled release. Consider documenting version management strategy in CONTRIBUTING.md.

---

#### C2: Missing Platform-Specific Endpoint Configuration
**Finding:** Design shows single endpoint but GitHub Releases have different installers per platform:
- Windows: `.msi` or `.exe`
- macOS Intel: `.dmg` (x86_64-apple-darwin)
- macOS Apple Silicon: `.dmg` (aarch64-apple-darwin)

**Impact:** The `latest.json` endpoint is correct, but design doesn't clarify how Tauri's updater handles multi-platform scenarios from a single manifest.

**Required Action:** Verify and document that Tauri's updater automatically selects the correct platform artifact from `latest.json`. If manual platform targeting is needed, update the configuration example.

---

#### C3: Missing Release Notes Fetching Logic
**Finding:** Design states "shows modal with version number and release notes" but provides no implementation details for:
- Where release notes come from (CHANGELOG.md? GitHub Release body?)
- How they're fetched (API call? Embedded in latest.json?)
- Format handling (Markdown? Plain text?)

**Impact:** Cannot implement UpdateModal.svelte without knowing the data source and format.

**Required Action:** Specify explicitly:
1. Release notes source (recommended: GitHub Releases API or embed in latest.json)
2. Fetching mechanism (JS API call or Rust command?)
3. Data format and parsing requirements
4. Fallback if release notes unavailable

---

#### C4: Incomplete Error Handling Strategy
**Finding:** Design covers happy path but doesn't address:
- Network failures during update check
- Download interruption/corruption
- Signature verification failure
- Insufficient disk space
- User cancels during download

**Impact:** Poor UX and potential data loss without proper error states.

**Required Action:** Add error handling section covering:
- Error states and user-facing messages
- Retry logic for network failures
- Recovery from partial downloads
- Logging strategy for debugging

---

#### C5: Missing Verification Steps ✅ RESOLVED
**Finding:** No specific verification steps for validating the implementation works correctly.

**Impact:** Cannot confirm feature is production-ready without test criteria.

**Required Action:** Add verification section with:
- Manual test scenarios (happy path, network failure, dismiss/later)
- How to test update flow without publishing to production GitHub Releases (test repository?)
- Rollback procedure if update fails in production

**Resolution:** Added comprehensive "Verification Strategy" section to `02-design.md` including:
- Local mock release server setup (directory structure, Node.js server script, test manifest)
- Development endpoint override options (env var and separate config)
- Creating test artifacts workflow
- 9 manual test scenarios covering happy path, errors, and edge cases
- Rollback procedure for production issues
- Verification checklist for feature sign-off

---

### Important Issues

#### I1: Unclear "Later" Indicator Placement ✅ RESOLVED
**Finding:** Design mentions "subtle indicator" and "badge on Settings" but doesn't specify:
- Visual design (color, icon, position)
- State persistence (store? localStorage?)
- When indicator disappears (after manual check? After update installed?)

**Resolution:** Added comprehensive ""Later" Indicator Design" section to `02-design.md` covering:
- Blue 8×8px dot on Settings nav link (`--accent-primary` color)
- Memory-only persistence (Svelte store, resets on app restart)
- Full state lifecycle table showing when dot appears/disappears
- Settings page UI mockup with status row
- Complete i18n keys for all update-related strings
- Accessibility notes (aria-label, keyboard nav, focus management)

---

#### I2: Missing i18n Translation Keys ✅ RESOLVED
**Finding:** Design lists files to modify (`src/lib/i18n/sk/index.ts` and `en/index.ts`) but doesn't enumerate required translation keys.

**Resolution:** Complete i18n keys added in ""Later" Indicator Design" section including:
- `sectionTitle`, `currentVersion`, `checkForUpdates`, `checking`
- `upToDate`, `available`, `availableVersion`
- `updateNow`, `later`, `downloading`, `downloadProgress`
- `installRestart`, `errorChecking`

---

#### I3: Tauri Plugin Version Not Specified
**Finding:** Design says "Add `tauri-plugin-updater`" to Cargo.toml but doesn't specify version.

**Current Tauri version:** 2.9.5  
**Expected plugin version:** Should match (2.x family)

**Recommendation:** Specify exact version (e.g., `tauri-plugin-updater = "2"` for latest 2.x compatible).

---

#### I4: Missing Update Store State Definition ✅ RESOLVED
**Finding:** Design lists creating `src/lib/stores/update.ts` but doesn't define its interface.

**Resolution:** Complete `UpdateState` interface added in ""Later" Indicator Design" section:
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

---

#### I5: Incomplete GitHub Actions Configuration
**Finding:** Design shows adding env vars to release.yml but doesn't address:
- Where in the workflow they should be added (job-level? step-level?)
- Whether they apply to all platforms or just Windows (macOS may need different signing approach)

**Recommendation:** Provide complete workflow diff showing exact placement of env vars.

---

#### I6: Missing Capabilities Permission Details
**Finding:** Design says "Add updater permissions" to `src-tauri/capabilities/default.json` but doesn't specify which permissions.

**Expected permissions:**
- `updater:default`
- Possibly `updater:allow-check` and `updater:allow-install`

**Recommendation:** List exact permissions to add based on Tauri 2.x updater documentation.

---

### Minor Issues

#### M1: Outdated CLI Command Syntax
**Finding:** Setup step 1 uses `npx tauri signer generate -w .tauri-keys/private.key`

**Potential Issue:** Tauri 2.x may have different CLI syntax. Should verify current command format.

**Recommendation:** Test command before including in docs. Consider adding output example.

---

#### M2: GitHub Secret Name Inconsistency
**Finding:** Setup step 2 uses `TAURI_SIGNING_PRIVATE_KEY` but workflow env example uses same name twice for key and password.

**Clarification Needed:** Design shows:
```yaml
TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_KEY_PASSWORD }}
```

Note the inconsistency: `TAURI_SIGNING_KEY_PASSWORD` (secret name) vs `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (env var name).

**Recommendation:** Use consistent naming convention across setup steps and workflow config.

---

#### M3: Missing .gitignore Verification
**Finding:** Design says "Add `.tauri-keys/` to .gitignore" but current .gitignore already has `/target/` pattern.

**Note:** Good practice, but should verify current .gitignore doesn't have conflicting patterns or if `.tauri-keys/` should be absolute path.

**Recommendation:** Confirm `.tauri-keys/` (relative) vs `/.tauri-keys/` (absolute from repo root).

---

#### M4: No Mention of Development/Testing Strategy
**Finding:** No guidance on testing update mechanism without polluting production GitHub Releases.

**Recommendation:** Add note about using test repository or draft releases for testing update flow before production deployment.

---

#### M5: Startup Check Performance Impact Not Addressed
**Finding:** Design shows update check on app startup (background) but doesn't discuss:
- Timeout duration
- Effect on app startup time if GitHub is slow/unreachable
- Whether check blocks UI initialization

**Recommendation:** Specify that check must be non-blocking and include timeout (e.g., 5s).

---

#### M6: Modal Accessibility Not Addressed
**Finding:** UpdateModal.svelte section mentions "accessible" but doesn't specify requirements:
- Keyboard navigation (Tab, Enter, Escape)
- Screen reader labels
- Focus management

**Recommendation:** Reference existing ConfirmModal.svelte patterns (found in codebase) for consistency.

---

## Completeness Check Against Requirements

| Requirement | Coverage | Notes |
|-------------|----------|-------|
| Check for updates on startup | ✅ Yes | Mentioned in flow, needs performance clarification (M5) |
| Manual "Check for Updates" button | ✅ Yes | Settings page modification listed |
| Show modal with version + notes | ⚠️ Partial | Version yes, release notes source unclear (C3) |
| "Update Now" downloads & installs | ✅ Yes | Flow covered, error handling missing (C4) |
| "Later" dismisses with indicator | ✅ Yes | Full indicator design added (I1 resolved) |
| GitHub Releases as server | ✅ Yes | Configuration provided |
| Signed updates for security | ✅ Yes | Setup steps included, secret naming inconsistency (M2) |

---

## YAGNI Analysis

✅ **No scope creep detected.** All planned features directly address stated requirements. No unnecessary abstractions or premature optimizations observed.

---

## Files & Paths Verification

| File Path | Status | Notes |
|-----------|--------|-------|
| `.tauri-keys/` | ✅ Valid | New directory, needs .gitignore entry |
| `src/lib/stores/update.ts` | ✅ Valid | New file, follows existing pattern |
| `src/lib/components/UpdateModal.svelte` | ✅ Valid | New file, follows existing pattern |
| `src-tauri/Cargo.toml` | ✅ Exists | Verified |
| `src-tauri/tauri.conf.json` | ✅ Exists | Verified |
| `src-tauri/src/lib.rs` | ✅ Exists | Verified |
| `src-tauri/capabilities/default.json` | ✅ Exists | Verified |
| `package.json` | ✅ Exists | Verified |
| `.github/workflows/release.yml` | ✅ Exists | Verified |
| `src/routes/+layout.svelte` | ✅ Exists | Verified |
| `src/routes/settings/+page.svelte` | ✅ Exists | Verified |
| `src/lib/i18n/sk/index.ts` | ✅ Exists | Verified |
| `src/lib/i18n/en/index.ts` | ✅ Exists | Verified |

---

## Dependencies & Task Ordering

### Correct Order ✅

1. **Setup** (one-time): Generate keys, configure GitHub Secrets
2. **Backend config**: Cargo.toml, tauri.conf.json, capabilities, lib.rs
3. **Frontend dependencies**: package.json
4. **Frontend logic**: update store, UpdateModal component
5. **Integration**: Layout (startup check), Settings (manual check)
6. **i18n**: Translation files
7. **CI/CD**: GitHub Actions workflow
8. **Testing**: Verification via test repository

**Note:** Version synchronization (C1) must happen before step 2.

---

## Summary Statistics

- **Critical Issues:** 5 (must fix before implementation)
- **Important Issues:** 6 (should address for quality)
- **Minor Issues:** 6 (nice to have for completeness)
- **Total Findings:** 17

---

## Round 2 Findings

### Critical Issues (continued)

#### C6: Missing Version Display API Command
**Finding:** Design requires "Current version display" in Settings page but doesn't specify:
- How to get the current app version (Tauri command? package.json import?)
- Whether it's from `tauri.conf.json` or another source
- API method signature

**Current State:** No existing `getVersion` or similar command in `src/lib/api.ts`

**Impact:** Cannot implement version display without defining the data source.

**Required Action:** Specify version retrieval method. Options:
1. Tauri command reading from `tauri.conf.json`
2. Frontend import from `package.json` (but requires build-time version injection)
3. Use `@tauri-apps/api/app` module's `getVersion()` function (recommended)

**Recommendation:** Use `import { getVersion } from '@tauri-apps/api/app'` - standard Tauri approach.

---

### Important Issues (continued)

#### I7: Missing `@tauri-apps/plugin-updater` Package
**Finding:** Design lists modifying `package.json` to add `@tauri-apps/plugin-updater` but doesn't specify version.

**Current `@tauri-apps/api` version:** 2.9.1

**Expected plugin version:** Should be compatible (likely 2.x)

**Recommendation:** Specify exact dependency:
```json
"@tauri-apps/plugin-updater": "^2.0.0"
```

---

#### I8: Update Store Pattern Not Aligned with Codebase
**Finding:** Suggested update store in I4 doesn't match existing store patterns in codebase.

**Current Store Patterns:**
- `theme.ts`: Uses `writable` with custom methods (`init()`, `change()`, `destroy()`)
- `toast.ts`: Uses `writable` with factory function
- Both use async methods and proper cleanup

**Recommendation:** Update store should follow similar pattern:
```typescript
function createUpdateStore() {
  const { subscribe, set, update } = writable<UpdateState>({...});
  return {
    subscribe,
    check: async () => { /* check logic */ },
    dismiss: () => { /* mark as dismissed */ },
    install: async () => { /* download and install */ }
  };
}
export const updateStore = createUpdateStore();
```

---

### Minor Issues (continued)

#### M7: Passive Install Mode Platform Compatibility
**Finding:** Design specifies `"installMode": "passive"` for Windows but doesn't clarify:
- What "passive" means (silent? minimal UI?)
- Whether macOS has equivalent configuration
- User consent requirements on different platforms

**Recommendation:** Add note explaining "passive" mode (MSI quiet installation on Windows, user still sees OS-level install prompts on macOS).

---

#### M8: No Mention of Restart Behavior
**Finding:** Design says "App restarts with new version" but doesn't specify:
- Automatic restart or user-triggered?
- Save user's work first?
- What if user has unsaved changes?

**Recommendation:** Clarify restart behavior. Suggest: Automatic restart after successful install, or require explicit user confirmation if implementing save state preservation later.

---

#### M9: Missing Bundle Configuration Change
**Finding:** Design shows adding `"createUpdaterArtifacts": true` to `bundle` section but current `tauri.conf.json` doesn't have this property.

**Current bundle config:**
```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [...]
}
```

**Recommendation:** Clarify exact placement:
```json
"bundle": {
  "active": true,
  "targets": "all",
  "createUpdaterArtifacts": true,
  "icon": [...]
}
```

---

## Summary Statistics (Final)

- **Critical Issues:** 6 - **ALL RESOLVED ✅**
- **Important Issues:** 8 - **ALL RESOLVED ✅**
- **Minor Issues:** 9 - **7 RESOLVED ✅** (M4, M5 skipped per user request)
- **Total Findings:** 23, **21 resolved**

**All Issues Resolution:**

**Critical (6/6):**
- ✅ C1: Version synchronization strategy with release skill integration
- ✅ C2: Verified Tauri auto-handles platform artifacts (no action needed)
- ✅ C3: CHANGELOG extraction via GitHub Actions, embedded in latest.json
- ✅ C4: Comprehensive error handling table with retry logic
- ✅ C5: Verification strategy with local mock server
- ✅ C6: Version display API specified (getVersion from @tauri-apps/api/app)

**Important (8/8):**
- ✅ I1: \"Later\" indicator design with visual specs and state flow
- ✅ I2: Complete i18n translation keys (SK + EN)
- ✅ I3: Tauri plugin version specified (\"2\")
- ✅ I4: Update store interface defined
- ✅ I5: GitHub Actions job-level env vars with CHANGELOG extraction
- ✅ I6: Permissions specified (updater:default + process:allow-relaunch)
- ✅ I7: JavaScript packages specified (@tauri-apps/plugin-updater ^2.0.0)
- ✅ I8: Update store pattern aligned with codebase

**Minor (7/9):**
- ✅ M1: CLI command verified (npm run tauri signer generate)
- ✅ M2: GitHub Secret naming consistency documented
- ✅ M3: .gitignore verification added
- ⏭️ M4: Skipped (user will test manually)
- ⏭️ M5: Skipped per user request
- ✅ M6: Modal accessibility specified (keyboard, screen reader, focus)
- ✅ M7: Passive install mode clarified (Windows progress window)
- ✅ M8: Restart behavior (automatic after install)
- ✅ M9: Bundle configuration placement shown

---

## Recommendation

**✅ READY FOR IMPLEMENTATION**

All critical and important issues have been resolved. The design now provides:
- Complete version synchronization strategy
- CHANGELOG-based release notes extraction
- Comprehensive error handling
- Full verification/testing approach
- All API methods and permissions specified
- Complete i18n coverage
- Accessibility requirements
- Clear implementation details

The design is implementable without ambiguity.

---

## Next Steps

1. ✅ All critical findings addressed
2. ✅ All important findings resolved
3. ✅ Minor findings addressed (M4, M5 skipped as requested)
4. **Ready:** Proceed to implementation using [02-design.md](_tasks/38-auto-update/02-design.md) as guide
5. Proceed to implementation planning

