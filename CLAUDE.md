# CLAUDE.md

Vehicle logbook (Kniha jázd) for Slovak legal compliance - tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained. It ships as a single Docker image (`ghcr.io/mcsdodo/kniha-jazd-web`) serving a browser UI; there is no desktop build.

## Tech Stack

- **Frontend:** SvelteKit + TypeScript (static SPA, served by the backend)
- **Backend:** Rust - `kniha-jazd-core` (all logic) + `kniha-jazd-web` (Axum HTTP server)
- **Transport:** JSON-RPC over `POST /api/rpc`
- **Database:** SQLite
- **Deployment:** Docker image, one `/data` volume
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
- **Frontend is display-only** - Calls backend commands over RPC, renders results
- **No calculation duplication** - the RPC round-trip is same-host and cheap, no need for client-side calculations

```
┌─────────────────────────────────────────────────┐
│               SvelteKit Frontend                │
│        (Display only - no calculations)         │
├─────────────────────────────────────────────────┤
│    HTTP  -  POST /api/rpc { command, args }     │
├─────────────────────────────────────────────────┤
│  kniha-jazd-web  -  Axum server + static SPA    │
├─────────────────────────────────────────────────┤
│  kniha-jazd-core  -  all business logic         │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐
│  │ calculations │  │ suggestions  │  │  receipts  │
│  └──────────────┘  └──────────────┘  └────────────┘
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐
│  │      db      │  │    export    │  │   gemini   │
│  └──────────────┘  └──────────────┘  └────────────┘
│  ┌──────────────┐  ┌──────────────┐               │
│  │    server    │  │  app_state   │               │
│  └──────────────┘  └──────────────┘               │
├─────────────────────────────────────────────────┤
│      SQLite Database  -  one /data volume       │
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
- Verify frontend correctly invokes backend RPC commands
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
# Rust backend tests (use --manifest-path, never cd &&)
cargo test --manifest-path src-tauri/Cargo.toml --workspace

# Run a single backend test by name filter
cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core "test_name_filter"

# Integration tests need two artifacts the harness uses: the SPA the server serves
# (build/) and the headless binary WebdriverIO spawns. Rebuild after changes.
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web

# Integration tests - Tier 1 only (fast, for quick checks)
npm run test:integration:tier1

# Integration tests against an already-running container on port 3456
npm run test:integration:docker

# All tests (backend + integration)
npm run test:all
```

**Test scripts and CI (invariant I1).** Every npm test script reaches GitHub Actions.
`test.yml` invokes `test:backend`, `test:integration:docker` and
`test:integration:docker:env` directly. `test:integration:tier1/2/3` are thin
aliases: they set `TIER` (and `PARALLEL_TIERS`) and delegate to
`test:integration`, which is exactly what the Docker jobs run - CI sets the same
`TIER` env vars itself. `test:all` is `test:backend && test:integration`. So the
tier scripts satisfy I1 through the script they delegate to, not by appearing in
the workflow by name.

#### Iteration strategy: focused runs, not full sweeps

**While debugging a failing spec, run only that spec — not the whole suite.** A full
integration sweep takes ~10 minutes; a single spec runs in under a minute. Use
focused runs to iterate on a fix, and reserve a full sweep for the final verification
once you believe everything passes.

```bash
# Single spec - WebdriverIO spawns kniha-jazd-web itself on port 3457
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/legal-compliance.spec.ts

# Multiple specs, same spawned server
npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/legal-compliance.spec.ts \
  --spec tests/integration/specs/tier2/time-column.spec.ts

# Single spec, Docker mode (container must already be up on port 3456)
WDIO_EXTERNAL_SERVER=1 npx wdio run tests/integration/wdio.server.conf.ts \
  --spec tests/integration/specs/tier2/legal-compliance.spec.ts
```

Run the full suite (`npm run test:integration*` without `--spec`) only:
- After all focused runs pass and you're confident the fix is complete.
- Before merging or claiming the work done.
- When you suspect cross-spec interaction issues that focused runs would miss.

## Project Structure

```
kniha-jazd/
├── src-tauri/           # Rust workspace (name kept for history)
│   ├── core/            # kniha-jazd-core: all logic + tests
│   │   ├── src/         # Source files (see .claude/rules/rust-backend.md)
│   │   └── migrations/  # DB schema (see .claude/rules/migrations.md)
│   └── web/             # kniha-jazd-web: headless HTTP server binary
├── src/                 # SvelteKit frontend (see .claude/rules/svelte-frontend.md)
│   ├── lib/
│   │   ├── components/  # UI components
│   │   ├── stores/      # Svelte state
│   │   └── i18n/        # Translations
│   └── routes/          # Pages
├── tests/
│   └── integration/     # WebdriverIO tests (see .claude/rules/integration-tests.md)
├── scripts/             # Development scripts
├── .github/workflows/   # CI/CD pipelines
├── Dockerfile.web       # The only build artifact
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

One data directory per deployment. There is no location picker and no lock file -
the container owns its volume, so nothing can open the database from a second
machine.

| Env var | Default | Purpose |
|---------|---------|---------|
| `KNIHA_JAZD_DATA_DIR` | `/data` | Directory holding the DB, receipts and backups |
| `DATABASE_PATH` | `<DATA_DIR>/kniha-jazd.db` | Override just the DB file path |
| `STATIC_DIR` | `/var/www/html` | Built SvelteKit assets; leave unset in local dev so vite serves the UI |
| `PORT` | `3456` | HTTP listen port |

- **Docker:** `docker-compose.web.yml` mounts the host's `./data` at `/data`; backups
  land in `<DATA_DIR>/backups/`.
- **Local dev:** set `KNIHA_JAZD_DATA_DIR` to a scratch folder before starting the
  server, otherwise it falls back to `/data`.

Read-only mode still exists - it is entered when the database carries migrations
this build does not know (i.e. it was written by a newer image). Write commands
guard with the `check_read_only!` macro.

**Related commands:** `get_db_location`, `get_app_mode`

## Common Commands

```bash
# Development - two processes, two terminals.
# 1) backend (leave STATIC_DIR unset so it serves no SPA):
cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
# 2) frontend on :5173, proxying /api to localhost:3456 (see vite.config.ts):
npm run dev

# Build (the only shipped artifact)
docker build -f Dockerfile.web -t kniha-jazd-web:local .

# Testing
npm run test:backend     # Rust unit tests (whole workspace)
npm run test:integration # WebdriverIO, spawns kniha-jazd-web (needs npm run build first)
npm run test:all         # All tests

# i18n — REQUIRED after editing src/lib/i18n/{sk,en}/index.ts.
# Nothing else regenerates i18n-types.ts (the generator otherwise only runs in
# vite dev watch mode), so `npm run check` reports phantom errors until this runs.
npm run i18n
```

## CI/CD

GitHub Actions workflow (`.github/workflows/test.yml`):
- **Backend tests**: Run on Windows, macOS, Linux
- **Integration tests**: Linux only - the Docker image is built once, then Chrome
  drives it through three parallel tier jobs plus the env-pinned suite
- Triggered on push/PR to `main` branch
- **Publish**: a green push to `main` republishes the tested image artifact (no rebuild)
  as `ghcr.io/mcsdodo/kniha-jazd-web:main` + `:main-<short-sha>`. PRs publish nothing.

`.github/workflows/release.yml` runs on a `v*` tag and publishes
`ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` + `:latest`. It creates **no GitHub Release**
and no installers.

**Two channels, one boundary (ADR-031):** CI owns `:main` / `:main-<sha>`; `/release`
owns `vX.Y.Z` / `:latest`. Neither writes the other's tags.

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
git add src-tauri/core/src/db.rs src-tauri/core/src/commands_internal/trips.rs

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
    ├── server-mode.md     # Example: HTTP server + Docker deployment
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
| `/release` | Publishing new version | Bump version, update changelog, tag, push - CI publishes the ghcr image |
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
- [ ] All use-cases have tests? (backend for logic, integration for UI flows)
- [ ] No test duplication? (don't re-test backend logic in integration tests)
- [ ] Tests pass? (`npm run test:backend` or `npm run test:all`)
- [ ] Code committed with descriptive message?
- [ ] Documentation updated? (CHANGELOG for user-visible, DECISIONS.md for "why")
- [ ] Feature doc created? (`docs/features/{feature}.md` for complex features)

For significant decisions during task:
- [ ] `/decision` run to record ADR/BIZ entry?
