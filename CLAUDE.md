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
# Rust backend tests (61 tests)
cd src-tauri && cargo test
```

### Test Coverage

**Backend (Rust) - Single Source of Truth:**
- `calculations.rs` - 41 tests: consumption rate, spotreba, zostatok, margin, Excel verification
- `suggestions.rs` - 11 tests: route matching, compensation suggestions
- `db.rs` - 9 tests: CRUD operations

All calculations happen in Rust backend. Frontend is display-only (see ADR-008).

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
└── _tasks/              # Planning docs
```

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

# Linting (NOT in agent instructions - use tools)
npm run lint && npm run format
```

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

Keep `README.md` (Slovak) and `README.en.md` in sync with feature changes.
