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

## Path-Specific Rules

Detailed patterns for specific file types are in `.claude/rules/`:
- `rust-backend.md` - Rust code patterns, test organization, key files
- `svelte-frontend.md` - Frontend patterns, i18n usage
- `integration-tests.md` - WebdriverIO test patterns
- `migrations.md` - Database migration patterns

These load automatically when working on matching files.

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

## Project Structure

```
kniha-jazd/
├── src-tauri/           # Rust backend
│   ├── src/             # Source files (see .claude/rules/rust-backend.md)
│   └── migrations/      # DB schema (see .claude/rules/migrations.md)
├── src/                 # SvelteKit frontend (see .claude/rules/svelte-frontend.md)
│   ├── lib/
│   │   ├── components/  # UI components
│   │   ├── stores/      # Svelte state
│   │   └── i18n/        # Translations
│   └── routes/          # Pages
├── tests/
│   ├── integration/     # WebdriverIO tests (see .claude/rules/integration-tests.md)
│   └── e2e/             # Playwright smoke tests (frontend only)
├── scripts/             # Development scripts
├── .github/workflows/   # CI/CD pipelines
├── _tasks/              # Planning docs
└── docs/
    └── features/        # Feature documentation
```

## Key Business Rules

1. **Consumption rate:** `l/100km = liters_filled / km_since_last_fillup × 100`
2. **Legal limit:** Consumption must be ≤120% of vehicle's TP rate
3. **Zostatok:** Fuel remaining = previous - (km × rate/100) + refueled
4. **Compensation:** When over margin, suggest trips to bring it down to 16-19%

## Database Location

Paths are based on Tauri `identifier` in config files:

- **Production** (`tauri.conf.json` → `com.notavailable.kniha-jazd`):
  - `%APPDATA%\com.notavailable.kniha-jazd\kniha-jazd.db`
  - Backups: `%APPDATA%\com.notavailable.kniha-jazd\backups\`

- **Development** (`tauri.conf.dev.json` → `com.notavailable.kniha-jazd.dev`):
  - `%APPDATA%\com.notavailable.kniha-jazd.dev\kniha-jazd.db`
  - Backups: `%APPDATA%\com.notavailable.kniha-jazd.dev\backups\`

### Custom Database Location (Multi-PC Support)

Users can move the database via **Settings → Database Location → Change...**

- Lock file (`kniha-jazd.lock`) prevents simultaneous access from multiple PCs
- Database + backups folder moved together
- Path stored in `local.settings.json` (survives reinstalls)
- All write commands check for read-only mode via `check_read_only!` macro

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

```
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
