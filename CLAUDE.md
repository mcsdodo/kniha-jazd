# CLAUDE.md

Vehicle logbook (Kniha jázd) desktop app for Slovak legal compliance - tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained.

## Tech Stack

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Database:** SQLite
- **UI Language:** Slovak (i18n-ready)
- **Code Language:** English

## Skill Overrides

When external skills (e.g., `superpowers:brainstorming`, `superpowers:writing-plans`) specify file paths or conventions that conflict with this project's structure, **ALWAYS use this project's conventions**:

| Skill Default | Project Convention |
|---------------|-------------------|
| `docs/plans/` | `_tasks/{NN}-feature/` (via `/task-plan`) |
| Inline decisions | `DECISIONS.md` (via `/decision`) |
| Generic changelog | `CHANGELOG.md` (via `/changelog`) |

**Rule:** Project-specific paths in this file override generic skill defaults.

**Finding next task folder number:** Always use `Glob pattern: _tasks/[0-9][0-9]-*/*` to find files inside numbered folders (NOT `_tasks/*`). Extract the highest folder number and increment by 1.

## Architecture: Backend-Only Calculations

All business logic and calculations live in Rust backend only (ADR-008):
- **`get_trip_grid_data`** - Returns trips + pre-calculated rates, warnings, fuel remaining
- **Frontend is display-only** - Calls Tauri commands, renders results
- **No calculation duplication** - Tauri IPC is local/fast, no need for client-side calculations

```
┌─────────────────────────────────────────────────┐
│              SvelteKit Frontend                 │
│         (Display only - no calculations)        │
├─────────────────────────────────────────────────┤
│              Tauri IPC Bridge                   │
├─────────────────────────────────────────────────┤
│              Rust Backend                       │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐
│  │calculations  │  │ suggestions  │  │  receipts  │
│  └──────────────┘  └──────────────┘  └────────────┘
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐
│  │     db       │  │   export     │  │   gemini   │
│  └──────────────┘  └──────────────┘  └────────────┘
│  ┌──────────────┐  ┌──────────────┐               │
│  │ db_location  │  │  app_state   │               │
│  └──────────────┘  └──────────────┘               │
├─────────────────────────────────────────────────┤
│              SQLite Database                    │
└─────────────────────────────────────────────────┘
```

## Core Principle: Test-Driven Development

**MANDATORY WORKFLOW FOR ALL CODE CHANGES:**

```
1. WRITE failing test first (understand what you're building)
2. WRITE minimal code to pass the test
3. REFACTOR (clean up while tests pass)
4. REPEAT
```

**IMPORTANT:** Never write implementation code without a failing test first.

### What to Test

Focus on **business logic** - the calculations that matter for legal compliance:
- Consumption calculations (l/100km, spotreba, zostatok)
- Margin calculations (must stay ≤20% over TP rate)
- Compensation trip suggestions

**Do NOT write filler tests.** No tests for:
- Trivial CRUD operations
- UI rendering (unless behavior-critical)
- Getters/setters

### Common Pitfalls

- **Don't duplicate calculations in frontend** - ADR-008 prohibits this
- **Don't use `git add -A`** - only stage files from current session (except `/release`)
- **Don't write tests for CRUD** - focus on business logic only
- **Don't forget Slovak UI text** - all user-facing strings go through i18n
- **Don't hardcode year** - app supports year picker, use year parameter

### Database Migration Best Practices

**IMPORTANT:** All database migrations MUST be backward-compatible:

- **Always** add columns with DEFAULT values
- **Never** remove columns (mark as deprecated if needed)
- **Never** rename columns
- **Never** change column types to incompatible types

**Why?** The app supports read-only mode for older versions accessing newer databases. Older app versions must be able to READ data from databases migrated by newer versions.

```sql
-- Good migration (backward-compatible):
ALTER TABLE trips ADD COLUMN new_field TEXT DEFAULT '';

-- Bad migration (DO NOT DO):
ALTER TABLE trips DROP COLUMN old_field;        -- Older apps will crash!
ALTER TABLE trips RENAME COLUMN old TO new;     -- Older apps won't find it!
```

### Running Tests

```bash
# Rust backend tests (158 tests)
cd src-tauri && cargo test

# E2E integration tests (requires debug build)
npm run test:integration:build

# Integration tests - Tier 1 only (fast, for quick checks)
npm run test:integration:tier1

# All tests (backend + integration)
npm run test:all
```

### Test Organization

Tests are split into separate `*_tests.rs` files using the `#[path]` attribute pattern:

```rust
// In calculations.rs
#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
```

This keeps source files clean while maintaining private access (tests are still submodules).

**When adding tests:** Write tests in `*_tests.rs` companion file, not in the source file.

### Test Coverage

**Backend (Rust) - Single Source of Truth (158 tests):**
- `commands.rs` - 36 tests: receipt matching, period rates, warnings, fuel remaining, year carryover
- `calculations_tests.rs` - 33 tests: consumption rate, spotreba, zostatok, margin, Excel verification
- `receipts_tests.rs` - 17 tests: folder detection, extraction, scanning
- `db_tests.rs` - 15 tests: CRUD lifecycle, year filtering
- `calculations_energy_tests.rs` - 15 tests: BEV battery remaining, energy consumption
- `db_location.rs` - 11 tests: custom paths, lock files, multi-PC support
- `calculations_phev_tests.rs` - 8 tests: PHEV combined fuel + energy
- `settings.rs` - 7 tests: local settings loading/saving
- `app_state.rs` - 6 tests: read-only mode, app state management
- `export.rs` - 6 tests: export totals, HTML escaping
- `gemini.rs` - 4 tests: JSON deserialization

**Integration Tests (WebdriverIO + tauri-driver) - 61 tests:**
- `tests/integration/` - Full app E2E tests via WebDriver protocol
- **Tiered execution**: Tier 1 (39 tests) for PRs, all tiers for main
- Runs against debug build of Tauri app
- DB seeding via Tauri IPC (no direct DB access)
- CI: Windows only (tauri-driver limitation)

All calculations happen in Rust backend. Frontend is display-only (see ADR-008).

### Code Patterns

**Adding a New Tauri Command:**
1. Add function to `commands.rs` with `#[tauri::command]`
2. Register in `lib.rs` `invoke_handler` (not main.rs)
3. If write command, add `check_read_only!(app_state);` guard at start
4. Call from Svelte component via `invoke("command_name", { args })`

**Adding a New Calculation:**
1. Write test in `calculations_tests.rs`
2. Implement in `calculations.rs`
3. Expose via `get_trip_grid_data` or new command
4. Frontend receives pre-calculated value (no client-side calculation)

**Adding UI Text:**
1. Add key to `src/lib/i18n/sk/index.ts` (Slovak primary)
2. Add key to `src/lib/i18n/en/index.ts` (English)
3. Use `{LL.key()}` in Svelte components

## Project Structure

```
kniha-jazd/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── lib.rs                # Tauri app setup, invoke_handler
│   │   ├── calculations.rs       # Core logic (MOST IMPORTANT)
│   │   ├── calculations_tests.rs # Tests for calculations
│   │   ├── calculations_energy.rs     # BEV-specific calculations
│   │   ├── calculations_phev.rs       # PHEV combined logic
│   │   ├── suggestions.rs        # Compensation trip logic
│   │   ├── receipts.rs           # Receipt scanning
│   │   ├── commands.rs           # Tauri command handlers
│   │   ├── db.rs                 # SQLite operations
│   │   ├── db_location.rs        # Custom DB path, lock files
│   │   ├── app_state.rs          # App state (read-only mode)
│   │   ├── settings.rs           # Local settings loading
│   │   ├── gemini.rs             # AI receipt OCR
│   │   ├── models.rs             # Vehicle, Trip structs
│   │   ├── schema.rs             # Diesel ORM schema
│   │   └── export.rs             # HTML/PDF generation
│   └── migrations/      # DB schema
├── src/                 # SvelteKit frontend
│   ├── lib/
│   │   ├── components/  # UI components
│   │   ├── stores/      # Svelte state
│   │   └── i18n/        # Translations
│   └── routes/          # Pages
├── tests/
│   ├── integration/     # WebdriverIO + tauri-driver E2E tests
│   └── e2e/             # Playwright smoke tests (frontend only)
├── scripts/             # Development scripts
│   └── test-release.ps1 # Build test releases for update testing
├── _test-releases/      # Local update testing server
├── .github/workflows/   # CI/CD pipelines
└── _tasks/              # Planning docs
```

### Key Files Quick Reference

| File | Purpose | When to Modify |
|------|---------|----------------|
| `lib.rs` | Tauri app setup, command registration | Adding new Tauri commands |
| `calculations.rs` | All consumption/margin math | Adding/changing calculations |
| `calculations_tests.rs` | Tests for calculations | Adding calculation tests |
| `calculations_energy.rs` | BEV battery, energy calculations | Electric vehicle logic |
| `calculations_phev.rs` | PHEV combined fuel + energy | Plug-in hybrid logic |
| `suggestions.rs` | Compensation trip logic | Route matching, suggestions |
| `receipts.rs` | Receipt folder scanning | Receipt processing logic |
| `db.rs` | SQLite CRUD operations | Schema changes, queries |
| `db_location.rs` | Custom DB path, lock files | Database location features |
| `app_state.rs` | Read-only mode, app mode | App state management |
| `settings.rs` | Local settings (theme, paths) | User preferences |
| `gemini.rs` | AI receipt OCR | Receipt recognition |
| `commands.rs` | Tauri command handlers | New frontend→backend calls |
| `export.rs` | HTML/PDF generation | Report format changes |
| `models.rs` | Data structures | Adding fields to Trip/Vehicle |
| `schema.rs` | Diesel ORM schema | After DB migrations |
| `+page.svelte` files | Page UI | Visual/interaction changes |
| `i18n/sk/index.ts` | Slovak translations | New UI text |

## Key Business Rules

1. **Consumption rate:** `l/100km = liters_filled / km_since_last_fillup × 100`
2. **Legal limit:** Consumption must be ≤120% of vehicle's TP rate
3. **Zostatok:** Fuel remaining = previous - (km × rate/100) + refueled
4. **Compensation:** When over margin, suggest trips to bring it down to 16-19%

## Database Location

Paths are based on Tauri `identifier` in config files:

- **Production** (`tauri.conf.json` → `com.notavailable.kniha-jazd`):
  - `%APPDATA%\com.notavailable.kniha-jazd\kniha-jazd.db`
  - Example: `C:\Users\<username>\AppData\Roaming\com.notavailable.kniha-jazd\kniha-jazd.db`
  - Backups: `%APPDATA%\com.notavailable.kniha-jazd\backups\`

- **Development** (`tauri.conf.dev.json` → `com.notavailable.kniha-jazd.dev`):
  - `%APPDATA%\com.notavailable.kniha-jazd.dev\kniha-jazd.db`
  - Example: `C:\Users\<username>\AppData\Roaming\com.notavailable.kniha-jazd.dev\kniha-jazd.db`

### Custom Database Location (Multi-PC Support)

Users can move the database to a custom path (e.g., Google Drive, NAS) via **Settings → Database Location → Change...**

**How it works:**
- Lock file (`kniha-jazd.lock`) prevents simultaneous access from multiple PCs
- Database + backups folder moved together
- App restarts automatically after move
- Path stored in `local.settings.json` (survives reinstalls)

**Read-only mode triggers:**
- Database has migrations from a newer app version (prevents data corruption)
- Lock file held by another PC (prevents concurrent writes)
- All 19 write commands check for read-only mode via `check_read_only!` macro

**Related commands:** `get_db_location`, `move_database`, `reset_database_location`, `get_app_mode`

## Common Commands

```bash
# Development
npm run tauri dev        # Start app in dev mode

# Build
npm run tauri build      # Production build

# Testing
npm run test:backend     # Rust unit tests (158 tests)
npm run test:integration # E2E tests (needs debug build)
npm run test:all         # All tests

# Linting (NOT in agent instructions - use tools)
npm run lint && npm run format
```

## Testing Auto-Update Locally

To test the auto-update flow without publishing to GitHub Releases:

```powershell
# 1. Build a test release (temporarily bumps version, then reverts)
.\scripts\test-release.ps1              # 0.17.2 → 0.18.0 (minor bump)
.\scripts\test-release.ps1 -BumpType patch  # 0.17.2 → 0.17.3

# 2. Start mock update server
node _test-releases/serve.js

# 3. Run app with test endpoint (new terminal)
set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json && npm run tauri dev
```

The app runs at current version but detects the test release as an available update.

**Note:** For auto-update to work, set `TAURI_SIGNING_PRIVATE_KEY` before building. Without it, you can test the installer manually but auto-update will reject unsigned builds.

See `_test-releases/README.md` for detailed test scenarios.

## CI/CD

GitHub Actions workflow (`.github/workflows/test.yml`):
- **Backend tests**: Run on Windows, macOS, Linux
- **Integration tests**: Run on Windows only (tauri-driver limitation)
- Triggered on push/PR to `main` branch

## Git Guidelines

**When to commit:**
  - **Planned tasks (with todos):** Commit after completing task items as part of workflow
  - **Quick fixes/ad-hoc changes:** Ask user before committing - they may want to review first

**Only commit files you changed in THIS session.** Before committing:
1. Run `git status` to see all modified files
2. Stage only files related to your current task
3. Do NOT include unrelated staged files from previous sessions

```bash
# Good: stage specific files
git add src-tauri/src/db.rs src-tauri/src/commands.rs

# Bad: stage everything blindly
git add -A  # Only use for releases or when you've reviewed ALL changes
```

**Exception:** `/release` intentionally uses `git add -A` because releases should include all pending changes.

## Git Worktrees

Worktree directory: `.worktrees/` (project-local, gitignored)

## Documentation

Use skills in `.claude/skills/` for workflows:

| Skill | When to Use | Purpose |
|-------|-------------|---------|
| `/task-plan` | Starting new feature | Create `_tasks/{NN}-feature/` planning folder |
| `/decision` | Making architectural choices | Add ADR/BIZ entry to `DECISIONS.md` |
| `/changelog` | After user-visible changes | Update `CHANGELOG.md` [Unreleased] section |
| `/verify` | Before claiming "done" | Run tests, check git status, verify changelog |
| `/release` | Publishing new version | Bump version, update changelog, tag, build |
| `/plan-review` | Before coding | Review plan for completeness, feasibility, clarity |
| `/code-review` | After implementation | Review code quality, run tests, iterate until passing |
| `/test-review` | After feature complete | Check test coverage, add missing tests |

**Use `/decision` when:**
- Choosing between multiple valid approaches (document why this one)
- Defining new business logic rules (calculations, limits, validation)
- Making architectural choices (patterns, structure, tech stack)
- After debugging reveals non-obvious requirements
- NOT for: refactoring, bug fixes, or changes that follow existing decisions

Keep `README.md` (Slovak) and `README.en.md` in sync with feature changes.

### Task Completion Checklist

Before marking any task complete:
- [ ] Tests pass? (`npm run test:backend` or `npm run test:all`)
- [ ] Code committed with descriptive message?

For significant decisions during task:
- [ ] `/decision` run to record ADR/BIZ entry?
