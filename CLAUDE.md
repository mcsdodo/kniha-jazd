# CLAUDE.md

Vehicle logbook (Kniha jázd) desktop app for Slovak legal compliance - tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained.

## Tech Stack

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Database:** SQLite
- **UI Language:** Slovak (i18n-ready)
- **Code Language:** English

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
│  ┌──────────────┐  ┌──────────────┐            │
│  │calculations  │  │ suggestions  │            │
│  └──────────────┘  └──────────────┘            │
│  ┌──────────────┐  ┌──────────────┐            │
│  │     db       │  │   export     │            │
│  └──────────────┘  └──────────────┘            │
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

### Running Tests

```bash
# Rust backend tests (93 tests)
cd src-tauri && cargo test

# E2E integration tests (requires debug build)
npm run test:integration:build

# All tests (backend + integration)
npm run test:all
```

### Test Coverage

**Backend (Rust) - Single Source of Truth (93 tests):**
- `calculations.rs` - 28 tests: consumption rate, spotreba, zostatok, margin, Excel verification
- `receipts.rs` - 17 tests: extraction, confidence, matching
- `db.rs` - 17 tests: CRUD lifecycle, year carryover
- `commands.rs` - 10 tests: receipt matching, trip operations
- `suggestions.rs` - 8 tests: route matching, compensation suggestions
- `export.rs` - 7 tests: export totals, HTML escaping
- `gemini.rs` - 3 tests: JSON deserialization
- `settings.rs` - 3 tests: local settings loading

**Integration Tests (WebdriverIO + tauri-driver):**
- `tests/integration/` - Full app E2E tests via WebDriver protocol
- Runs against debug build of Tauri app
- CI: Windows only (tauri-driver limitation)

All calculations happen in Rust backend. Frontend is display-only (see ADR-008).

### Code Patterns

**Adding a New Tauri Command:**
1. Add function to `commands.rs` with `#[tauri::command]`
2. Register in `main.rs` `invoke_handler`
3. Call from Svelte component via `invoke("command_name", { args })`

**Adding a New Calculation:**
1. Write test in `calculations.rs`
2. Implement in `calculations.rs`
3. Expose via `get_trip_grid_data` or new command
4. Frontend receives pre-calculated value (no client-side calculation)

**Adding UI Text:**
1. Add key to `src/lib/i18n/sk/index.ts` (Slovak primary)
2. Add key to `src/lib/i18n/en/index.ts` (English)
3. Use `{LL.key()}` in Svelte components

### Common Pitfalls

- **Don't duplicate calculations in frontend** - ADR-008 prohibits this
- **Don't use `git add -A`** - only stage files from current session (except `/release`)
- **Don't skip changelog** - every feature/fix needs `/changelog` update
- **Don't write tests for CRUD** - focus on business logic only
- **Don't forget Slovak UI text** - all user-facing strings go through i18n
- **Don't hardcode year** - app supports year picker, use year parameter

## Project Structure

```
kniha-jazd/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── db.rs        # SQLite operations
│   │   ├── models.rs    # Vehicle, Trip structs
│   │   ├── calculations.rs  # Core logic (MOST IMPORTANT)
│   │   ├── suggestions.rs   # Compensation trip logic
│   │   └── export.rs    # PDF generation
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
├── .github/workflows/   # CI/CD pipelines
└── _tasks/              # Planning docs
```

### Key Files Quick Reference

| File | Purpose | When to Modify |
|------|---------|----------------|
| `calculations.rs` | All consumption/margin math | Adding/changing calculations |
| `suggestions.rs` | Compensation trip logic | Route matching, suggestions |
| `db.rs` | SQLite CRUD operations | Schema changes, queries |
| `commands.rs` | Tauri command handlers | New frontend→backend calls |
| `export.rs` | HTML/PDF generation | Report format changes |
| `models.rs` | Data structures | Adding fields to Trip/Vehicle |
| `+page.svelte` files | Page UI | Visual/interaction changes |
| `i18n/sk/index.ts` | Slovak translations | New UI text |

## Key Business Rules

1. **Consumption rate:** `l/100km = liters_filled / km_since_last_fillup × 100`
2. **Legal limit:** Consumption must be ≤120% of vehicle's TP rate
3. **Zostatok:** Fuel remaining = previous - (km × rate/100) + refueled
4. **Compensation:** When over margin, suggest trips to bring it down to 16-19%

## Database Location

- **Windows:** `%APPDATA%\com.notavailable.kniha-jazd\kniha-jazd.db`
  - Example: `C:\Users\<username>\AppData\Roaming\com.notavailable.kniha-jazd\kniha-jazd.db`
- **Backups:** `%APPDATA%\com.notavailable.kniha-jazd\backups\`

## Common Commands

```bash
# Development
npm run tauri dev        # Start app in dev mode

# Build
npm run tauri build      # Production build

# Testing
npm run test:backend     # Rust unit tests (93 tests)
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

Use skills in `.claude/skills/` for documentation workflows:

| Skill | Purpose |
|-------|---------|
| `/task-plan` | Create `_tasks/{NN}-feature/` planning folder (runs brainstorming first) |
| `/decision` | Add ADR/BIZ entry to `DECISIONS.md` |
| `/changelog` | Update `CHANGELOG.md` [Unreleased] section |
| `/release` | Bump version, update changelog, tag, build |

**MANDATORY FINAL STEP:** After completing any feature, fix, or change:
1. Commit all code changes
2. Run `/changelog` to update the [Unreleased] section
3. Commit the changelog update

**WARNING:** Do NOT mark a task as complete without updating the changelog. This applies to:
- Task plans (include changelog as final task)
- Subagent-driven development (final step before finishing)
- Any implementation work

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
- [ ] `/changelog` run to update [Unreleased]?
- [ ] Changelog committed?

For significant decisions during task:
- [ ] `/decision` run to record ADR/BIZ entry?
