# Claude Workflow Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Claude Code v2.0.64-v2.1.0 workflow improvements including rules directory, LSP config, and skill hooks.

**Architecture:** Split monolithic CLAUDE.md into focused rule modules, add LSP for Rust code intelligence, and enhance skills with automatic workflow enforcement hooks.

**Tech Stack:** Claude Code configuration (JSON, Markdown), rust-analyzer LSP

---

## Task 1: Create Rules Directory Structure

**Files:**
- Create: `.claude/rules/` directory

**Step 1: Create the rules directory**

```bash
mkdir -p .claude/rules
```

**Step 2: Verify directory exists**

```bash
ls -la .claude/rules
```

Expected: Empty directory created

**Step 3: Commit**

```bash
git add .claude/rules/.gitkeep 2>/dev/null || echo "Directory tracked via files"
```

No commit yet - we'll commit with the first rule file.

---

## Task 2: Create rust-backend.md Rule

**Files:**
- Create: `.claude/rules/rust-backend.md`

**Step 1: Create the rule file**

Create `.claude/rules/rust-backend.md` with Rust-specific patterns extracted from CLAUDE.md:

```markdown
# Rust Backend Rules

## Architecture

All business logic and calculations live in Rust backend only (ADR-008):
- **`get_trip_grid_data`** - Returns trips + pre-calculated rates, warnings, fuel remaining
- **Frontend is display-only** - Calls Tauri commands, renders results
- **No calculation duplication** - Tauri IPC is local/fast, no need for client-side calculations

## Code Patterns

**Adding a New Tauri Command:**
1. Add function to `commands.rs` with `#[tauri::command]`
2. Register in `main.rs` `invoke_handler`
3. Call from Svelte component via `invoke("command_name", { args })`

**Adding a New Calculation:**
1. Write test in `calculations_tests.rs`
2. Implement in `calculations.rs`
3. Expose via `get_trip_grid_data` or new command
4. Frontend receives pre-calculated value (no client-side calculation)

## Test Organization

Tests are split into separate `*_tests.rs` files using the `#[path]` attribute pattern:

```rust
// In calculations.rs
#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
```

This keeps source files clean while maintaining private access (tests are still submodules).

**When adding tests:** Write tests in `*_tests.rs` companion file, not in the source file.

## Key Files

| File | Purpose | When to Modify |
|------|---------|----------------|
| `calculations.rs` | All consumption/margin math | Adding/changing calculations |
| `calculations_tests.rs` | Tests for calculations | Adding calculation tests |
| `suggestions.rs` | Compensation trip logic | Route matching, suggestions |
| `suggestions_tests.rs` | Tests for suggestions | Adding suggestion tests |
| `receipts.rs` | Receipt folder scanning | Receipt processing logic |
| `receipts_tests.rs` | Tests for receipts | Adding receipt tests |
| `db.rs` | SQLite CRUD operations | Schema changes, queries |
| `commands.rs` | Tauri command handlers | New frontend→backend calls |
| `export.rs` | HTML/PDF generation | Report format changes |
| `models.rs` | Data structures | Adding fields to Trip/Vehicle |

## Test Coverage

**Backend (Rust) - Single Source of Truth (108 tests):**
- `calculations_tests.rs` - 28 tests: consumption rate, spotreba, zostatok, margin, Excel verification
- `commands.rs` - 25 tests: receipt matching, period rates, warnings, fuel remaining, year carryover
- `receipts_tests.rs` - 17 tests: folder detection, extraction, scanning
- `db.rs` - 17 tests: CRUD lifecycle, year filtering
- `suggestions_tests.rs` - 8 tests: route matching, compensation suggestions
- `export.rs` - 7 tests: export totals, HTML escaping
- `gemini.rs` - 3 tests: JSON deserialization
- `settings.rs` - 3 tests: local settings loading
```

**Step 2: Verify file created**

```bash
cat .claude/rules/rust-backend.md | head -20
```

Expected: File content displayed

**Step 3: Commit**

```bash
git add .claude/rules/rust-backend.md
git commit -m "docs(rules): add rust-backend rules module"
```

---

## Task 3: Create svelte-frontend.md Rule

**Files:**
- Create: `.claude/rules/svelte-frontend.md`

**Step 1: Create the rule file**

Create `.claude/rules/svelte-frontend.md`:

```markdown
# SvelteKit Frontend Rules

## Core Principle

**Frontend is display-only** (ADR-008):
- Calls Tauri commands via `invoke()`
- Renders results from backend
- NO calculations in frontend code
- NO business logic duplication

## Adding UI Text (i18n)

1. Add key to `src/lib/i18n/sk/index.ts` (Slovak primary)
2. Add key to `src/lib/i18n/en/index.ts` (English)
3. Use `{LL.key()}` in Svelte components

**UI Language:** Slovak (user-facing)
**Code Language:** English (variables, comments)

## Key Files

| File | Purpose | When to Modify |
|------|---------|----------------|
| `+page.svelte` files | Page UI | Visual/interaction changes |
| `i18n/sk/index.ts` | Slovak translations | New UI text |
| `i18n/en/index.ts` | English translations | New UI text |
| `lib/components/` | Reusable UI components | Shared UI elements |
| `lib/stores/` | Svelte state | App-wide state management |

## Common Pitfalls

- **Don't duplicate calculations in frontend** - ADR-008 prohibits this
- **Don't forget Slovak UI text** - all user-facing strings go through i18n
- **Don't hardcode year** - app supports year picker, use year parameter

## Integration Tests

**WebdriverIO + tauri-driver (61 tests):**
- `tests/integration/` - Full app E2E tests via WebDriver protocol
- **Tiered execution**: Tier 1 (39 tests) for PRs, all tiers for main
- Runs against debug build of Tauri app
- DB seeding via Tauri IPC (no direct DB access)
- CI: Windows only (tauri-driver limitation)
```

**Step 2: Commit**

```bash
git add .claude/rules/svelte-frontend.md
git commit -m "docs(rules): add svelte-frontend rules module"
```

---

## Task 4: Create testing.md Rule

**Files:**
- Create: `.claude/rules/testing.md`

**Step 1: Create the rule file**

Create `.claude/rules/testing.md`:

```markdown
# Testing Rules

## Core Principle: Test-Driven Development

**MANDATORY WORKFLOW FOR ALL CODE CHANGES:**

```
1. WRITE failing test first (understand what you're building)
2. WRITE minimal code to pass the test
3. REFACTOR (clean up while tests pass)
4. REPEAT
```

**IMPORTANT:** Never write implementation code without a failing test first.

## What to Test

Focus on **business logic** - the calculations that matter for legal compliance:
- Consumption calculations (l/100km, spotreba, zostatok)
- Margin calculations (must stay ≤20% over TP rate)
- Compensation trip suggestions

**Do NOT write filler tests.** No tests for:
- Trivial CRUD operations
- UI rendering (unless behavior-critical)
- Getters/setters

## Running Tests

```bash
# Rust backend tests (108 tests)
cd src-tauri && cargo test

# E2E integration tests (requires debug build)
npm run test:integration:build

# Integration tests - Tier 1 only (fast, for quick checks)
npm run test:integration:tier1

# All tests (backend + integration)
npm run test:all
```

## Test Organization

Tests in separate `*_tests.rs` files using `#[path]` attribute:

```rust
#[cfg(test)]
#[path = "calculations_tests.rs"]
mod tests;
```

**When adding tests:** Write in `*_tests.rs` companion file, not source file.

## Common Pitfalls

- **Don't write tests for CRUD** - focus on business logic only
- **Don't skip TDD** - always write failing test first
- **Don't mock internal modules** - test real behavior
```

**Step 2: Commit**

```bash
git add .claude/rules/testing.md
git commit -m "docs(rules): add testing rules module"
```

---

## Task 5: Create git-workflow.md Rule

**Files:**
- Create: `.claude/rules/git-workflow.md`

**Step 1: Create the rule file**

Create `.claude/rules/git-workflow.md`:

```markdown
# Git Workflow Rules

## When to Commit

- **Planned tasks (with todos):** Commit after completing task items as part of workflow
- **Quick fixes/ad-hoc changes:** Ask user before committing - they may want to review first

## Staging Rules

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

## Named Sessions

Name sessions for easy resumption:

```bash
# Name current session
/rename feat-export-pdf

# Resume later
claude --resume feat-export-pdf
```

**Naming convention:** `{type}-{feature}`
- `fix-zostatok-rounding`
- `feat-export-pdf`
- `refactor-db-queries`

## Workflow Shortcuts

| Shortcut | Purpose |
|----------|---------|
| `Ctrl+B` | Background long-running commands (dev server, tests) |
| `Alt+T` | Toggle thinking mode |
| `Ctrl+O` | View conversation transcript |
| `/plan` | Quick entry to plan mode |

## CI/CD

GitHub Actions workflow (`.github/workflows/test.yml`):
- **Backend tests**: Run on Windows, macOS, Linux
- **Integration tests**: Run on Windows only (tauri-driver limitation)
- Triggered on push/PR to `main` branch
```

**Step 2: Commit**

```bash
git add .claude/rules/git-workflow.md
git commit -m "docs(rules): add git-workflow rules module"
```

---

## Task 6: Create business-logic.md Rule

**Files:**
- Create: `.claude/rules/business-logic.md`

**Step 1: Create the rule file**

Create `.claude/rules/business-logic.md`:

```markdown
# Business Logic Rules

## Key Business Rules

1. **Consumption rate:** `l/100km = liters_filled / km_since_last_fillup × 100`
2. **Legal limit:** Consumption must be ≤120% of vehicle's TP rate
3. **Zostatok:** Fuel remaining = previous - (km × rate/100) + refueled
4. **Compensation:** When over margin, suggest trips to bring it down to 16-19%

## Domain Terms

| Slovak | English | Description |
|--------|---------|-------------|
| Kniha jázd | Trip logbook | The main application |
| Spotreba | Consumption | Fuel usage rate (l/100km) |
| Zostatok | Remaining | Fuel left in tank |
| TP | Technical passport | Vehicle's official consumption rate |
| Tankovanie | Refueling | Adding fuel to vehicle |

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
npm run test:backend     # Rust unit tests
npm run test:integration # E2E tests (needs debug build)
npm run test:all         # All tests
```
```

**Step 2: Commit**

```bash
git add .claude/rules/business-logic.md
git commit -m "docs(rules): add business-logic rules module"
```

---

## Task 7: Refactor CLAUDE.md to Slim Index

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Replace CLAUDE.md with slim index**

Replace entire CLAUDE.md content with:

```markdown
# CLAUDE.md

Vehicle logbook (Kniha jázd) desktop app for Slovak legal compliance - tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained.

## Tech Stack

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Database:** SQLite
- **UI Language:** Slovak (i18n-ready)
- **Code Language:** English

## Rules

@.claude/rules/rust-backend.md
@.claude/rules/svelte-frontend.md
@.claude/rules/testing.md
@.claude/rules/git-workflow.md
@.claude/rules/business-logic.md

## Skill Overrides

When external skills (e.g., `superpowers:brainstorming`, `superpowers:writing-plans`) specify file paths or conventions that conflict with this project's structure, **ALWAYS use this project's conventions**:

| Skill Default | Project Convention |
|---------------|-------------------|
| `docs/plans/` | `_tasks/{NN}-feature/` (via `/task-plan`) |
| Inline decisions | `DECISIONS.md` (via `/decision`) |
| Generic changelog | `CHANGELOG.md` (via `/changelog`) |

**Rule:** Project-specific paths in this file override generic skill defaults.

## Architecture

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

## Project Structure

```
kniha-jazd/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── calculations.rs       # Core logic (MOST IMPORTANT)
│   │   ├── calculations_tests.rs # Tests for calculations
│   │   ├── suggestions.rs        # Compensation trip logic
│   │   ├── commands.rs           # Tauri command handlers
│   │   ├── db.rs                 # SQLite operations
│   │   ├── models.rs             # Vehicle, Trip structs
│   │   └── export.rs             # PDF generation
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

## Skills

| Skill | When to Use | Purpose |
|-------|-------------|---------|
| `/task-plan` | Starting new feature | Create `_tasks/{NN}-feature/` planning folder |
| `/decision` | Making architectural choices | Add ADR/BIZ entry to `DECISIONS.md` |
| `/changelog` | After completing any work | Update `CHANGELOG.md` [Unreleased] section |
| `/verify` | Before claiming "done" | Run tests, check git status, verify changelog |
| `/release` | Publishing new version | Bump version, update changelog, tag, build |
| `/plan-review` | Before coding | Review plan for completeness, feasibility, clarity |
| `/code-review` | After implementation | Review code quality, run tests, iterate until passing |
| `/test-review` | After feature complete | Check test coverage, add missing tests |

## Task Completion Checklist

Before marking any task complete:
- [ ] Tests pass? (`npm run test:backend` or `npm run test:all`)
- [ ] Code committed with descriptive message?
- [ ] `/changelog` run to update [Unreleased]?
- [ ] Changelog committed?

For significant decisions during task:
- [ ] `/decision` run to record ADR/BIZ entry?
```

**Step 2: Verify line count**

```bash
wc -l CLAUDE.md
```

Expected: Under 100 lines

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "refactor(docs): slim CLAUDE.md with rule imports"
```

---

## Task 8: Add LSP Configuration to settings.json

**Files:**
- Modify: `.claude/settings.json`

**Step 1: Update settings.json**

Add LSP configuration while preserving existing hooks:

```json
{
  "lsp": {
    "rust-analyzer": {
      "command": "rust-analyzer",
      "filetypes": ["rs"]
    }
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -File .claude/hooks/pre-commit.ps1",
            "timeout": 120000
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -File .claude/hooks/post-commit-reminder.ps1",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

**Step 2: Verify JSON is valid**

```bash
cat .claude/settings.json | python -m json.tool > /dev/null && echo "Valid JSON"
```

Expected: "Valid JSON"

**Step 3: Commit**

```bash
git add .claude/settings.json
git commit -m "feat(config): add rust-analyzer LSP configuration"
```

---

## Task 9: Add Hooks to verify-skill

**Files:**
- Modify: `.claude/skills/verify-skill/SKILL.md`

**Step 1: Update verify-skill frontmatter**

Add hooks to the frontmatter:

```yaml
---
name: verify-skill
description: Use before claiming work is complete - runs tests, checks git status, verifies changelog
hooks:
  - event: Stop
    command: "cd src-tauri && cargo test --quiet"
---
```

Keep the rest of the file unchanged.

**Step 2: Commit**

```bash
git add .claude/skills/verify-skill/SKILL.md
git commit -m "feat(skills): add Stop hook to verify-skill for automatic test run"
```

---

## Task 10: Add Hooks to code-review-skill

**Files:**
- Modify: `.claude/skills/code-review-skill/SKILL.md`

**Step 1: Update code-review-skill frontmatter**

Add hooks to the frontmatter:

```yaml
---
name: code-review-skill
description: Use to review code implementations for quality, correctness, and best practices
hooks:
  - event: PreToolUse
    matcher: Edit
    command: "pwsh -NoProfile -Command \"Write-Host 'REMINDER: Reviewing only - document findings, do not apply fixes yet' -ForegroundColor Yellow\""
---
```

Keep the rest of the file unchanged.

**Step 2: Commit**

```bash
git add .claude/skills/code-review-skill/SKILL.md
git commit -m "feat(skills): add PreToolUse hook to code-review-skill for edit reminder"
```

---

## Task 11: Add Hooks to release-skill

**Files:**
- Modify: `.claude/skills/release-skill/SKILL.md`

**Step 1: Update release-skill frontmatter**

Add hooks to the frontmatter:

```yaml
---
name: release-skill
description: Bump version, update changelog, commit, tag, push, and build release installer
hooks:
  - event: Stop
    command: "cd src-tauri && cargo test && npm run tauri build"
    timeout: 300000
---
```

Keep the rest of the file unchanged.

**Step 2: Commit**

```bash
git add .claude/skills/release-skill/SKILL.md
git commit -m "feat(skills): add Stop hook to release-skill for final verification"
```

---

## Task 12: Verify Implementation

**Files:**
- None (verification only)

**Step 1: Verify rules are accessible**

```bash
ls -la .claude/rules/
```

Expected: 5 rule files listed

**Step 2: Verify CLAUDE.md line count**

```bash
wc -l CLAUDE.md
```

Expected: Under 100 lines

**Step 3: Check settings.json validity**

```bash
cat .claude/settings.json | python -m json.tool > /dev/null && echo "Valid JSON"
```

Expected: "Valid JSON"

**Step 4: Run backend tests to ensure nothing broke**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass

**Step 5: Check git status**

```bash
git status
```

Expected: Clean working tree

---

## Task 13: Update Changelog

**Step 1: Run changelog skill**

Run `/changelog` to add entry for this work.

Entry should mention:
- Rules directory for modular CLAUDE.md
- LSP configuration for Rust code intelligence
- Skill hooks for workflow enforcement

**Step 2: Commit changelog**

```bash
git add CHANGELOG.md
git commit -m "docs: update changelog for workflow improvements"
```

---

## Success Criteria

- [x] `.claude/rules/` directory with 5 rule files
- [x] CLAUDE.md under 100 lines with `@` imports
- [x] LSP configuration in settings.json
- [x] Skill hooks in verify-skill, code-review-skill, release-skill
- [x] All backend tests pass
- [x] Changelog updated
