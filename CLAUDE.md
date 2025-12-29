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

## Documentation

- Task planning: `_tasks/` folder (see `_tasks/CLAUDE.md`)
- **Decisions log:** [`DECISIONS.md`](DECISIONS.md) - all ADRs and business logic decisions
- Keep docs close to code (locality principle)
- Update docs after code changes

## Recording Decisions

**IMPORTANT:** When making architectural or business logic decisions during conversations, brainstorming, or debugging - add them to `DECISIONS.md`.

Format: Date, context, decision, reasoning. Keep it simple and sequential.

## Documentation Requirements

**MANDATORY: Keep user-facing docs in sync with code changes.**

When implementing or modifying features, update:

1. **README.md** (Slovak) - Feature list, usage instructions
2. **README.en.md** (English) - Mirror Slovak changes
3. **CHANGELOG.md** - Document what changed

### What Requires Documentation Update

- New features → Add to feature list + usage section
- Changed behavior → Update affected sections
- New UI elements → Update screenshots if significant change
- Removed features → Remove from docs

### CHANGELOG Format

```markdown
## [Unreleased]
### Pridané
- New feature description

### Zmenené
- Changed behavior description

### Opravené
- Bug fix description
```

Keep entries concise. Write CHANGELOG in Slovak (matches primary audience).

### When to Update CHANGELOG

**Update `## [Unreleased]` immediately when implementing changes:**

- New features → `### Pridané`
- Changed behavior → `### Zmenené`
- Bug fixes → `### Opravené`

Do this as part of the commit, not later. When releasing, move [Unreleased] items to a new version section.
