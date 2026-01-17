# Claude Workflow Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Claude Code v2.0.64-v2.1.0 workflow improvements including rules directory, LSP config, and skill hooks.

**Architecture:** Split monolithic CLAUDE.md into focused rule modules, add LSP for Rust code intelligence, and enhance skills with automatic workflow enforcement hooks.

**Tech Stack:** Claude Code configuration (JSON, Markdown), rust-analyzer LSP

---

## Phase 1: Create Rule Files (No Commits)

All rule files are created first without committing. This allows verification before atomic commit.

### Task 1: Create Rules Directory

**Files:**
- Create: `.claude/rules/` directory

**Step 1: Create directory and placeholder**

```powershell
New-Item -ItemType Directory -Path ".claude/rules" -Force
New-Item -ItemType File -Path ".claude/rules/.gitkeep" -Force
```

**Verification:**
```powershell
Test-Path ".claude/rules"
```

Expected: `True`

---

### Task 2: Create rust-backend.md Rule

**Files:**
- Create: `.claude/rules/rust-backend.md`

**Step 1: Create the rule file**

Create `.claude/rules/rust-backend.md`:

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
| `commands.rs` | Tauri command handlers | New frontend->backend calls |
| `export.rs` | HTML/PDF generation | Report format changes |
| `models.rs` | Data structures | Adding fields to Trip/Vehicle |
```

**Step 2: Verify file exists (no commit yet)**

```powershell
Test-Path ".claude/rules/rust-backend.md"
```

---

### Task 3: Create svelte-frontend.md Rule

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

**WebdriverIO + tauri-driver:**
- `tests/integration/` - Full app E2E tests via WebDriver protocol
- **Tiered execution**: Tier 1 for PRs, all tiers for main
- Runs against debug build of Tauri app
- DB seeding via Tauri IPC (no direct DB access)
- CI: Windows only (tauri-driver limitation)
```

---

### Task 4: Create testing.md Rule

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
- Margin calculations (must stay <=20% over TP rate)
- Compensation trip suggestions

**Do NOT write filler tests.** No tests for:
- Trivial CRUD operations
- UI rendering (unless behavior-critical)
- Getters/setters

## Running Tests

```bash
# Rust backend tests
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

---

### Task 5: Create git-workflow.md Rule

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

## MANDATORY FINAL STEP

**After completing ANY feature, fix, or change:**
1. Commit all code changes
2. Run `/changelog` to update the [Unreleased] section
3. Commit the changelog update

**WARNING:** Do NOT mark a task as complete without updating the changelog. This applies to:
- Task plans (include changelog as final task)
- Subagent-driven development (final step before finishing)
- Any implementation work

## Use /decision When

- Choosing between multiple valid approaches (document why this one)
- Defining new business logic rules (calculations, limits, validation)
- Making architectural choices (patterns, structure, tech stack)
- After debugging reveals non-obvious requirements
- NOT for: refactoring, bug fixes, or changes that follow existing decisions

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

## CI/CD

GitHub Actions workflow (`.github/workflows/test.yml`):
- **Backend tests**: Run on Windows, macOS, Linux
- **Integration tests**: Run on Windows only (tauri-driver limitation)
- Triggered on push/PR to `main` branch
```

---

### Task 6: Create business-logic.md Rule

**Files:**
- Create: `.claude/rules/business-logic.md`

**Step 1: Create the rule file**

Create `.claude/rules/business-logic.md`:

```markdown
# Business Logic Rules

## Key Business Rules

1. **Consumption rate:** `l/100km = liters_filled / km_since_last_fillup * 100`
2. **Legal limit:** Consumption must be <=120% of vehicle's TP rate
3. **Zostatok:** Fuel remaining = previous - (km * rate/100) + refueled
4. **Compensation:** When over margin, suggest trips to bring it down to 16-19%

## Domain Terms

| Slovak | English | Description |
|--------|---------|-------------|
| Kniha jazd | Trip logbook | The main application |
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

---

## Phase 2: Refactor CLAUDE.md

### Task 7: Backup and Refactor CLAUDE.md

**Files:**
- Backup: `CLAUDE.md` -> `CLAUDE.md.backup`
- Modify: `CLAUDE.md`

**Step 1: Create backup**

```powershell
Copy-Item "CLAUDE.md" "CLAUDE.md.backup"
```

**Step 2: Replace CLAUDE.md with slim index**

Replace entire CLAUDE.md content with:

```markdown
# CLAUDE.md

Vehicle logbook (Kniha jazd) desktop app for Slovak legal compliance - tracks trips, fuel consumption, and ensures the 20% over-consumption margin is maintained.

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
+--------------------------------------------------+
|              SvelteKit Frontend                  |
|         (Display only - no calculations)         |
+--------------------------------------------------+
|              Tauri IPC Bridge                    |
+--------------------------------------------------+
|              Rust Backend                        |
|  +-------------+  +-------------+                |
|  |calculations |  | suggestions |                |
|  +-------------+  +-------------+                |
|  +-------------+  +-------------+                |
|  |     db      |  |   export    |                |
|  +-------------+  +-------------+                |
+--------------------------------------------------+
|              SQLite Database                     |
+--------------------------------------------------+
```

## Project Structure

```
kniha-jazd/
+-- src-tauri/           # Rust backend
|   +-- src/
|   |   +-- calculations.rs       # Core logic (MOST IMPORTANT)
|   |   +-- calculations_tests.rs # Tests for calculations
|   |   +-- suggestions.rs        # Compensation trip logic
|   |   +-- commands.rs           # Tauri command handlers
|   |   +-- db.rs                 # SQLite operations
|   |   +-- models.rs             # Vehicle, Trip structs
|   |   +-- export.rs             # PDF generation
|   +-- migrations/      # DB schema
+-- src/                 # SvelteKit frontend
|   +-- lib/
|   |   +-- components/  # UI components
|   |   +-- stores/      # Svelte state
|   |   +-- i18n/        # Translations
|   +-- routes/          # Pages
+-- tests/
|   +-- integration/     # WebdriverIO + tauri-driver E2E tests
|   +-- e2e/             # Playwright smoke tests (frontend only)
+-- .github/workflows/   # CI/CD pipelines
+-- _tasks/              # Planning docs
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

**Step 3: Verify line count**

```powershell
(Get-Content "CLAUDE.md" | Measure-Object -Line).Lines
```

Expected: Under 100 lines

---

## Phase 3: Verify Before Committing

### Task 8: Verify @import Syntax Works

**Files:**
- None (verification only)

**Step 1: Check rule files exist**

```powershell
Get-ChildItem ".claude/rules/*.md" | Select-Object Name
```

Expected: 5 rule files listed

**Step 2: Start new Claude session to test imports**

Close current Claude session and start a new one. The rules should load automatically.

**Verification:** Ask Claude "What rules are loaded?" or check if rule content appears in context.

**If imports don't work:**
1. Restore backup: `Copy-Item "CLAUDE.md.backup" "CLAUDE.md" -Force`
2. Delete rules: `Remove-Item ".claude/rules" -Recurse -Force`
3. Investigate Claude Code documentation for correct syntax

---

## Phase 4: Update Settings

### Task 9: Add LSP and Permissions to settings.json

**Files:**
- Modify: `.claude/settings.json`

**Step 1: Verify rust-analyzer is installed**

```powershell
rust-analyzer --version
```

Expected: Version number displayed. If not installed, run: `rustup component add rust-analyzer`

**Step 2: Update settings.json**

Add LSP configuration and wildcard permissions while preserving existing hooks:

```json
{
  "permissions": {
    "allow": ["Bash(cargo *)", "Bash(npm *)", "Bash(git *)"]
  },
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

**Step 3: Verify JSON is valid**

```powershell
Get-Content ".claude/settings.json" | ConvertFrom-Json | Out-Null; Write-Host "Valid JSON"
```

Expected: "Valid JSON"

---

## Phase 5: Atomic Commit

### Task 10: Commit All Changes Atomically

**Step 1: Stage all rule files and CLAUDE.md**

```bash
git add .claude/rules/ CLAUDE.md
```

**Step 2: Commit atomically**

```bash
git commit -m "refactor(docs): split CLAUDE.md into modular rules

- Create .claude/rules/ with 5 focused rule modules
- Refactor CLAUDE.md to slim index with @imports
- Preserve critical workflow content in git-workflow.md
- Add MANDATORY FINAL STEP and /decision guidance to rules"
```

**Step 3: Commit settings.json separately**

```bash
git add .claude/settings.json
git commit -m "feat(config): add LSP and wildcard permissions

- Add rust-analyzer LSP for Rust code intelligence
- Add wildcard Bash permissions for cargo/npm/git"
```

**Step 4: Clean up backup**

```powershell
Remove-Item "CLAUDE.md.backup" -ErrorAction SilentlyContinue
```

---

## Phase 6: Skill Hooks

### Task 11: Add Hook to verify-skill

**Files:**
- Modify: `.claude/skills/verify-skill/SKILL.md`

**Step 1: Update frontmatter**

Add hooks to the YAML frontmatter:

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

---

### Task 12: Add Hook to code-review-skill

**Files:**
- Modify: `.claude/skills/code-review-skill/SKILL.md`

**Step 1: Update frontmatter**

Add hooks to the YAML frontmatter (using cross-platform echo):

```yaml
---
name: code-review-skill
description: Use to review code implementations for quality, correctness, and best practices
hooks:
  - event: PreToolUse
    matcher: Edit
    command: "echo REMINDER: Reviewing only - document findings, do not apply fixes yet"
---
```

Keep the rest of the file unchanged.

---

### Task 13: Add Hook to release-skill

**Files:**
- Modify: `.claude/skills/release-skill/SKILL.md`

**Step 1: Update frontmatter**

Add hooks (tests only - build is already in skill workflow):

```yaml
---
name: release-skill
description: Bump version, update changelog, commit, tag, push, and build release installer
hooks:
  - event: Stop
    command: "cd src-tauri && cargo test"
    timeout: 300000
---
```

Keep the rest of the file unchanged.

---

### Task 14: Verify Skill Hooks Work

**Step 1: Test verify-skill hook**

Run `/verify` and confirm that cargo tests run automatically at the end.

**Step 2: Commit skill changes**

```bash
git add .claude/skills/
git commit -m "feat(skills): add workflow enforcement hooks

- verify-skill: auto-run tests on Stop
- code-review-skill: reminder before Edit
- release-skill: auto-run tests on Stop"
```

---

## Phase 7: Final Verification

### Task 15: Run Full Verification

**Step 1: Run backend tests**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass

**Step 2: Check git status**

```bash
git status
```

Expected: Clean working tree (or only untracked backup files)

**Step 3: Verify rule count**

```powershell
(Get-ChildItem ".claude/rules/*.md").Count
```

Expected: 5

**Step 4: Verify CLAUDE.md line count**

```powershell
(Get-Content "CLAUDE.md" | Measure-Object -Line).Lines
```

Expected: Under 100 lines

---

### Task 16: Update Changelog

**Step 1: Run changelog skill**

Run `/changelog` to add entry for this work.

Entry should mention:
- Rules directory for modular CLAUDE.md
- LSP configuration for Rust code intelligence
- Skill hooks for workflow enforcement
- Wildcard Bash permissions

**Step 2: Commit changelog**

```bash
git add CHANGELOG.md
git commit -m "docs: update changelog for workflow improvements"
```

---

## Success Criteria

- [ ] `.claude/rules/` directory with 5 rule files
- [ ] CLAUDE.md under 100 lines with `@` imports
- [ ] Critical workflow content preserved in git-workflow.md
- [ ] LSP configuration in settings.json
- [ ] Wildcard Bash permissions in settings.json
- [ ] Skill hooks in verify-skill, code-review-skill, release-skill
- [ ] All backend tests pass
- [ ] Changelog updated

---

## Partial Failure Recovery

**If stopped after Phase 1-2 but before Phase 5:**
- Rules and CLAUDE.md exist but are uncommitted
- Either: Complete Phase 5 to commit, OR restore backup and delete `.claude/rules/`

**If @import syntax doesn't work:**
1. Restore: `Copy-Item "CLAUDE.md.backup" "CLAUDE.md" -Force`
2. Delete: `Remove-Item ".claude/rules" -Recurse -Force`
3. Investigate correct syntax in Claude Code documentation

**If skill hooks don't fire:**
1. Check Claude Code version supports skill hooks
2. Verify YAML frontmatter syntax
3. Revert skill changes if needed: `git checkout HEAD -- .claude/skills/`
