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

**Finding next task folder number:** Check BOTH locations (completed tasks move to `_done/`):
```
Glob pattern: _tasks/[0-9][0-9]-*/*
Glob pattern: _tasks/_done/[0-9][0-9]-*/*
```
Extract the highest folder number across BOTH and increment by 1.

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

## Planning Guidelines

**When creating implementation plans, ALWAYS follow these principles:**

1. **Check ADR-008** - All business logic stays in Rust backend. Frontend is display-only.
2. **Test-first approach** - Write backend unit tests for all use-cases, then implement to make tests pass.
3. **Integration tests for UI flows** - Create integration tests for new user interactions (UI → Backend → Display).
4. **Logical, testable steps** - Break tasks into deliverables that can be verified independently.
5. **Update documentation** - CHANGELOG for user-visible changes, DECISIONS.md for architectural choices.
6. **No overengineering** - Keep it simple and maintainable. Test all use-cases thoroughly, but don't over-abstract.

## Core Principle: Test-Driven Development

**MANDATORY WORKFLOW FOR ALL CODE CHANGES:**

```
1. WRITE failing test first (understand what you're building)
2. WRITE minimal code to pass the test
3. REFACTOR (clean up while tests pass)
4. REPEAT
```

**IMPORTANT:** Never write implementation code without a failing test first.

### Testing Strategy: No Duplication, Full Coverage

**Every use-case needs exactly ONE authoritative test - no gaps, no redundancy.**

```
┌─────────────────────────────────────────────────────────────┐
│                    INTEGRATION TESTS                        │
│   "Does the UI correctly trigger backend and display        │
│    results?" - Test user flows, NOT calculation math        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  BACKEND UNIT TESTS                         │
│   "Given these inputs, is the output correct?"              │
│   - ALL edge cases for calculations (source of truth)       │
│   - ALL business rules exhaustively tested                  │
└─────────────────────────────────────────────────────────────┘
```

**Backend unit tests** - Cover ALL business logic use-cases:
- Consumption calculations (l/100km, spotreba, zostatok)
- Margin calculations (must stay ≤20% over TP rate)
- Compensation trip suggestions
- Every edge case, every boundary condition

**Integration tests** - Cover UI → Backend → Display flows:
- Verify frontend correctly invokes Tauri commands
- Verify results display correctly in UI
- Do NOT re-test calculation logic (already proven in backend tests)

**Example of test ownership:**

| Use-case | Backend Unit Test | Integration Test |
|----------|-------------------|------------------|
| Consumption math | ✅ All edge cases | ❌ Not needed |
| Trip grid shows value | ❌ N/A | ✅ Add trip → verify display |
| 20% margin warning | ✅ Threshold logic | ✅ Warning icon appears |

**Do NOT write filler tests.** No tests for:
- Trivial CRUD operations
- UI rendering (unless behavior-critical)
- Getters/setters
- Duplicating backend tests in integration tests

### Common Pitfalls

- **Don't duplicate calculations in frontend** - ADR-008 prohibits this
- **Don't use `git add -A`** - only stage files from current session (except `/release`)
- **Don't write tests for CRUD** - focus on business logic only
- **Don't forget Slovak UI text** - all user-facing strings go through i18n
- **Don't hardcode year** - app supports year picker, use year parameter

### Database Migration Best Practices

**Migration strategy is forward-only (ADR-012).** We do NOT support older app versions reading newer databases.

- **Always** add columns with DEFAULT values (for migration to succeed)
- **Migrations run automatically** on app start
- **Backups are created** before migrations (existing behavior)
- **No legacy field sync** - don't maintain deprecated columns for backward compat

```sql
-- Standard migration:
ALTER TABLE trips ADD COLUMN new_field TEXT DEFAULT '';

-- Allowed (if needed for cleanup):
ALTER TABLE trips DROP COLUMN deprecated_field;  -- OK after deprecation period
```

**Note:** Users must upgrade the app to use migrated databases. Auto-update ensures this happens quickly.

### Running Tests

```bash
# Rust backend tests (195 tests)
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

**Backend (Rust) - Authoritative source for all business logic (195 tests):**
- `commands_tests.rs` - 61 tests: receipt matching, period rates, warnings, fuel remaining, year carryover, BEV energy, receipt assignment, backup cleanup, magic fill
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

**Integration Tests (WebdriverIO + tauri-driver) - UI flow verification (61 tests):**
- `tests/integration/` - Full app E2E tests via WebDriver protocol
- **Purpose**: Verify UI correctly invokes backend and displays results
- **NOT for**: Re-testing calculation logic (that's backend's job)
- **Tiered execution**: Tier 1 (39 tests) for PRs, all tiers for main
- Runs against debug build of Tauri app
- DB seeding via Tauri IPC (no direct DB access)
- CI: Windows only (tauri-driver limitation)

**Remember:** Backend tests = "Is the calculation correct?" | Integration tests = "Does the UI work?"

### Code Patterns

**Adding a New Tauri Command:**
1. Add function to `commands.rs` with `#[tauri::command]`
2. Register in `lib.rs` `invoke_handler` (not main.rs)
3. If write command, add `check_read_only!(app_state);` guard at start
4. Call from Svelte component via `invoke("command_name", { args })`

**Adding a New Calculation:**
1. Write failing test in `calculations_tests.rs` (cover all edge cases)
2. Implement in `calculations.rs` to make test pass
3. Expose via `get_trip_grid_data` or new command
4. Frontend receives pre-calculated value (no client-side calculation)
5. If new UI element, add integration test for display verification

**Adding a New User Flow:**
1. Write integration test for the UI interaction
2. Implement frontend UI (calls existing backend commands)
3. If new backend logic needed, add backend unit tests first (see above)

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
│   │   ├── commands_tests.rs     # Tests for commands
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
├── _tasks/              # Planning docs
└── docs/
    └── features/        # Feature documentation (technical walkthroughs)
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
| `commands_tests.rs` | Tests for commands | Adding command tests |
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
npm run test:backend     # Rust unit tests (195 tests)
npm run test:integration # E2E tests (needs debug build)
npm run test:all         # All tests

# Linting (NOT in agent instructions - use tools)
npm run lint && npm run format
```

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

### Feature Documentation

After completing a planned feature, create a **Feature Doc** in `docs/features/`:

```bash
docs/
├── CLAUDE.md              # Convention guide for docs folder
└── features/
    ├── move-database.md   # Example: database relocation feature
    └── {feature-name}.md  # Your new feature doc
```

**What to document:** User flow + technical implementation + design rationale. See `docs/CLAUDE.md` for template and conventions.

**When to create:** After completing `_tasks/` plans, or when documenting complex existing features.

### Skills

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
| `/test-update` | Testing auto-update | Test Tauri auto-update with mock release server |

**Use `/decision` when:**
- Choosing between multiple valid approaches (document why this one)
- Defining new business logic rules (calculations, limits, validation)
- Making architectural choices (patterns, structure, tech stack)
- After debugging reveals non-obvious requirements
- NOT for: refactoring, bug fixes, or changes that follow existing decisions

Keep `README.md` (Slovak) and `README.en.md` in sync with feature changes.

### Task Completion Checklist

Before marking any task complete:
- [ ] All use-cases have tests? (backend for logic, integration for UI flows)
- [ ] No test duplication? (don't re-test backend logic in integration tests)
- [ ] Tests pass? (`npm run test:backend` or `npm run test:all`)
- [ ] Code committed with descriptive message?
- [ ] Documentation updated? (CHANGELOG for user-visible, DECISIONS.md for "why")
- [ ] Feature doc created? (`docs/features/{feature}.md` for complex features)

For significant decisions during task:
- [ ] `/decision` run to record ADR/BIZ entry?
