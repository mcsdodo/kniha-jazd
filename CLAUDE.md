# CLAUDE.md

Vehicle logbook (Kniha jázd) desktop app for Slovak legal compliance - tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained.

## Tech Stack

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Database:** SQLite
- **UI Language:** Slovak (i18n-ready)
- **Code Language:** English

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
# Rust backend tests
cd src-tauri && cargo test

# Frontend tests
npm test

# E2E tests
npm run test:e2e
```

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
