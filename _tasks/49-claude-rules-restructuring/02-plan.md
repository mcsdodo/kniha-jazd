# Plan: Claude Rules Restructuring

## Overview

Restructure Claude instructions from a monolithic 447-line `CLAUDE.md` into focused, path-specific rules that load only when relevant.

### Target Structure

```
kniha-jazd/
├── CLAUDE.md                         # Global only (~150-200 lines)
├── .claude/
│   ├── rules/
│   │   ├── rust-backend.md           # src-tauri/**/*.rs
│   │   ├── svelte-frontend.md        # src/**/*.{svelte,ts}
│   │   ├── integration-tests.md      # tests/integration/**/*
│   │   └── migrations.md             # src-tauri/migrations/**/*.sql
│   ├── settings.json                 # (unchanged)
│   └── skills/                       # (unchanged)
├── docs/
│   └── CLAUDE.md                     # Keep (documentation conventions)
├── _tasks/
│   ├── CLAUDE.md                     # Keep (task planning conventions)
│   └── _TECH_DEBT/
│       └── CLAUDE.md                 # Keep (tech debt conventions)
└── tests/
    └── CLAUDE.md                     # Replace with redirect notice
```

---

## Phase 1: Create `.claude/rules/` Structure

### Step 1.1: Create `rust-backend.md`

**File:** `.claude/rules/rust-backend.md`
**Globs:** `src-tauri/**/*.rs`

**Extract these exact sections from root CLAUDE.md:**

| Section Heading | Subsection | Extract? |
|-----------------|------------|----------|
| `### Code Patterns` | "Adding a New Tauri Command" | ✅ Full subsection |
| `### Code Patterns` | "Adding a New Calculation" | ✅ Full subsection |
| `### Code Patterns` | "Adding a New User Flow" | ❌ Keep in root (references both frontend + backend) |
| `### Test Organization` | All | ✅ Full section |
| `### Test Coverage` | "Backend (Rust)..." paragraph | ✅ Only backend portion |
| `### Test Coverage` | "Integration Tests..." paragraph | ❌ Goes to integration-tests.md |
| `### Key Files Quick Reference` | Rust files (lib.rs through schema.rs) | ✅ Rows for .rs files only |

**Add cross-reference:** In "Adding a New Calculation" step 5, update to: "If new UI element, add integration test (see `.claude/rules/integration-tests.md`)"

**Content structure:**
```yaml
---
globs:
  - "src-tauri/**/*.rs"
---
# Rust Backend Rules

## Architecture Reminder
All business logic lives here (ADR-008). Frontend is display-only.

## Adding a New Tauri Command
[from ### Code Patterns]

## Adding a New Calculation
[from ### Code Patterns, with cross-reference update]

## Test Organization
[from ### Test Organization]

## Backend Test Coverage
[from ### Test Coverage - Backend paragraph only]

## Key Files Reference
[Rust rows from ### Key Files Quick Reference table]
```

### Step 1.2: Create `svelte-frontend.md`

**File:** `.claude/rules/svelte-frontend.md`
**Globs:** `src/**/*.svelte`, `src/**/*.ts`, `!src/**/*.test.ts`

**Extract these exact sections from root CLAUDE.md:**

| Section Heading | Subsection | Extract? |
|-----------------|------------|----------|
| `## Architecture: Backend-Only Calculations` | Bullet points about frontend | ✅ "Frontend is display-only" bullets |
| `### Code Patterns` | "Adding UI Text" | ✅ Full subsection |
| `### Common Pitfalls` | "Don't forget Slovak UI text" | ✅ This bullet only |
| `### Key Files Quick Reference` | Frontend files (+page.svelte, i18n) | ✅ Rows for frontend files only |

**Content structure:**
```yaml
---
globs:
  - "src/**/*.svelte"
  - "src/**/*.ts"
  - "!src/**/*.test.ts"
---
# Svelte Frontend Rules

## Core Principle: Display Only (ADR-008)
Frontend receives pre-calculated values from Rust backend.
**Never** duplicate calculations in TypeScript.

## Adding UI Text
[from ### Code Patterns > "Adding UI Text"]

## i18n Reminder
- All user-facing strings go through i18n
- Use `{LL.key()}` in Svelte components

## Key Files Reference
[Frontend rows from ### Key Files Quick Reference table]
```

### Step 1.3: Create `integration-tests.md`

**File:** `.claude/rules/integration-tests.md`
**Globs:** `tests/integration/**/*.ts`, `tests/integration/**/*.js`

**Migrate from `tests/CLAUDE.md`:**
- Full content of existing file (211 lines)
- Add YAML frontmatter with globs

**Content structure:**
```yaml
---
globs:
  - "tests/integration/**/*.ts"
  - "tests/integration/**/*.js"
---
# Integration Test Rules

## WebDriverIO + Tauri Integration Tests
[existing content from tests/CLAUDE.md]
```

### Step 1.4: Create `migrations.md`

**File:** `.claude/rules/migrations.md`
**Globs:** `src-tauri/migrations/**/*.sql`

**Extract these exact sections from root CLAUDE.md:**

| Section Heading | Extract? |
|-----------------|----------|
| `### Database Migration Best Practices` | ✅ Full section (strategy, bullets, SQL example, note) |

**Content structure:**
```yaml
---
globs:
  - "src-tauri/migrations/**/*.sql"
---
# Database Migration Rules

## Strategy: Forward-Only (ADR-012)
We do NOT support older app versions reading newer databases.

## Required Patterns
- **Always** add columns with DEFAULT values
- Migrations run automatically on app start
- Backups are created before migrations
- No legacy field sync

## SQL Examples
[SQL code block from original]

## Note
Users must upgrade the app to use migrated databases. Auto-update ensures this happens quickly.
```

---

## Phase 2: Slim Down Root `CLAUDE.md`

### Step 2.1: Remove Extracted Content

Remove from root `CLAUDE.md`:
- [ ] "Adding a New Tauri Command" section → now in `rust-backend.md`
- [ ] "Adding a New Calculation" section → now in `rust-backend.md`
- [ ] Test organization details → now in `rust-backend.md`
- [ ] Backend test coverage list → now in `rust-backend.md`
- [ ] "Adding UI Text" section → now in `svelte-frontend.md`
- [ ] "Database Migration Best Practices" section → now in `migrations.md`
- [ ] Key files table (detailed) → split into rules files

### Step 2.2: Simplify Remaining Content

Keep in root `CLAUDE.md` (truly global):
- [ ] Project overview & tech stack
- [ ] Skill overrides section
- [ ] Architecture diagram (ADR-008 principle)
- [ ] Planning guidelines
- [ ] Core TDD workflow
- [ ] Testing strategy overview (high-level, not detailed coverage)
- [ ] Common pitfalls (generic ones)
- [ ] Running tests commands
- [ ] Project structure (simplified tree)
- [ ] Key business rules
- [ ] Database location info
- [ ] Common commands
- [ ] CI/CD overview
- [ ] Git guidelines
- [ ] Documentation section (skills reference)
- [ ] Task completion checklist

### Step 2.3: Add Rules Reference

Add brief section to root `CLAUDE.md`:
```markdown
## Path-Specific Rules

Detailed patterns for specific file types are in `.claude/rules/`:
- `rust-backend.md` - Rust code patterns, test organization
- `svelte-frontend.md` - Frontend patterns, i18n usage
- `integration-tests.md` - WebdriverIO test patterns
- `migrations.md` - Database migration patterns

These load automatically when working on matching files.
```

### Step 2.4: Update Cross-References

After extraction, review remaining root CLAUDE.md content for references to extracted sections:

| If root content says... | Update to... |
|------------------------|--------------|
| "see Test Coverage section" | "see `.claude/rules/rust-backend.md`" |
| "see Code Patterns" (for Rust) | "see `.claude/rules/rust-backend.md`" |
| "see Code Patterns" (for i18n) | "see `.claude/rules/svelte-frontend.md`" |
| "see Database Migration" | "see `.claude/rules/migrations.md`" |

Also update cross-references WITHIN extracted rules files (done in Step 1.1).

---

## Phase 3: Handle `tests/CLAUDE.md` Migration

### Step 3.1: Replace with Redirect Notice

Replace `tests/CLAUDE.md` content with:
```markdown
# Testing Guidelines

> **Note:** Testing patterns have moved to `.claude/rules/integration-tests.md`
> for better context loading with glob patterns.
>
> The rules load automatically when editing files in `tests/integration/`.
```

This preserves discoverability while avoiding duplication.

---

## Phase 4: Validation

### Step 4.1: Verify No Duplication

- [ ] Search for duplicate phrases across root CLAUDE.md and rules files
- [ ] Ensure each instruction exists in exactly one place

### Step 4.2: Verify Glob Patterns

**Verification method:** For each rules file, open a matching file and check that Claude loads the rules content.

| Rules File | Test By Opening | Expected in Context |
|------------|-----------------|---------------------|
| `rust-backend.md` | `src-tauri/src/db.rs` | "Adding a New Tauri Command" visible |
| `svelte-frontend.md` | `src/routes/+page.svelte` | "Adding UI Text" visible |
| `integration-tests.md` | `tests/integration/trips.test.ts` | "Atomic Value Setting" visible |
| `migrations.md` | `src-tauri/migrations/*.sql` | "Forward-Only" visible |

**Alternative:** Use `/status` or context inspection to see loaded rules.

### Step 4.3: Validate YAML Frontmatter

- [ ] Each rules file has valid YAML between `---` markers
- [ ] `globs:` is a list (array format with `-` prefix)
- [ ] No YAML syntax errors (online validator or Claude Code error messages)

### Step 4.4: Content Check

- [ ] Root `CLAUDE.md` contains only project-wide content (no file-type-specific patterns)
- [ ] All extracted content appears in exactly one rules file
- [ ] Cross-references updated correctly

---

## Implementation Order

| Step | Description | Files Changed |
|------|-------------|---------------|
| 1.1 | Create `rust-backend.md` | `.claude/rules/rust-backend.md` |
| 1.2 | Create `svelte-frontend.md` | `.claude/rules/svelte-frontend.md` |
| 1.3 | Create `integration-tests.md` | `.claude/rules/integration-tests.md` |
| 1.4 | Create `migrations.md` | `.claude/rules/migrations.md` |
| 2.1 | Remove extracted content from root | `CLAUDE.md` |
| 2.2 | Simplify remaining content | `CLAUDE.md` |
| 2.3 | Add rules reference section | `CLAUDE.md` |
| 3.1 | Replace tests/CLAUDE.md with redirect | `tests/CLAUDE.md` |
| 4.x | Validation checks | (verification only) |

---

## Commit Strategy

**Single commit** after all changes:
```
refactor: restructure Claude instructions with .claude/rules/

- Create .claude/rules/ with path-specific rule files:
  - rust-backend.md (src-tauri/**/*.rs)
  - svelte-frontend.md (src/**/*.{svelte,ts})
  - integration-tests.md (tests/integration/**)
  - migrations.md (src-tauri/migrations/**/*.sql)
- Slim root CLAUDE.md from 447 to ~150-200 lines
- Migrate tests/CLAUDE.md to rules (with redirect notice)
- No instruction changes, only reorganization

Context loads more efficiently - only relevant rules for current files.
```

---

## Rollback Plan

If issues arise:
1. Git revert the single commit
2. All original content preserved in git history
3. No functional changes to codebase

---

## Phase 5: Skill Enhancements (Quick Wins)

> **Feature Verification:** All features below are documented in Claude Code:
> - `!`command`` syntax: https://code.claude.com/docs/en/skills.md ("Inject dynamic context")
> - `context: fork`: https://code.claude.com/docs/en/skills.md ("Run skills in a subagent")
> - `model: haiku`: https://code.claude.com/docs/en/skills.md (frontmatter reference)

### Step 5.1: Add `!`command`` to `/verify` Skill

**File:** `.claude/skills/verify-skill/SKILL.md`

**Current behavior:** Skill tells Claude to run `npm run test:backend` and `git status` as separate steps.

**New behavior:** Use `!`command`` syntax (note: backticks required!) to pre-inject results into the prompt.

**Updated content:**
```yaml
---
name: verify-skill
description: Use before claiming work is complete - runs tests, checks git status, verifies changelog
---

# Verification Before Completion

Run this before saying "task complete" or "done".

## Current Status (Pre-injected)

### Test Results
!`cd src-tauri && cargo test 2>&1 | tail -30`

### Git Status
!`git status`

### Changelog Preview
!`head -25 CHANGELOG.md`

## Checklist

Based on the pre-injected data above:

1. **Tests Pass** - Check "Test Results" section. Do NOT proceed if tests fail.
2. **Code Committed** - Check "Git Status" section. All work-related files should be committed.
3. **Changelog Updated** - Check "Changelog Preview". [Unreleased] section should have entry for this work.

If changelog missing, run /changelog.
```

**Benefits:**
- Claude sees results immediately without tool calls
- Faster verification workflow
- Data already in context for decision-making

### Step 5.2: Add `context: fork` to `/plan-review` Skill

**File:** `.claude/skills/plan-review-skill/SKILL.md`

**Current behavior:** Phase 1 uses `Task tool (general-purpose)` which runs in shared context.

**New behavior:** Add `context: fork` to YAML frontmatter for isolated execution.

**Change to frontmatter:**
```yaml
---
name: plan-review-skill
description: Use when about to implement a plan...
context: fork
---
```

**Benefits:**
- Phase 1 exploration doesn't consume main conversation context
- Cleaner separation between analysis and implementation phases
- Main context stays focused on user interaction

### Step 5.3: Route `/plan-review` Phase 1 to Haiku

**File:** `.claude/skills/plan-review-skill/SKILL.md`

**Current behavior:** Phase 1 agent uses default model (Opus).

**New behavior:** Specify `model: haiku` for the Task tool call.

**Update Phase 1 section:**
```markdown
## Phase 1: Review → Separate Agent

Spawn single agent for entire Phase 1:

Task tool (general-purpose):
  description: "Plan review: {PLAN_NAME}"
  model: haiku  # ← ADD THIS - cheaper model for read-only analysis
  prompt: |
    Review {TARGET} and create {TARGET_DIR}/_plan-review.md.
    ...
```

**Benefits:**
- ~10x cost reduction for Phase 1 analysis
- Haiku is sufficient for read-only exploration and documentation
- Opus reserved for Phase 2 where implementation decisions matter

---

## Updated Implementation Order

| Step | Description | Files Changed |
|------|-------------|---------------|
| 1.1 | Create `rust-backend.md` | `.claude/rules/rust-backend.md` |
| 1.2 | Create `svelte-frontend.md` | `.claude/rules/svelte-frontend.md` |
| 1.3 | Create `integration-tests.md` | `.claude/rules/integration-tests.md` |
| 1.4 | Create `migrations.md` | `.claude/rules/migrations.md` |
| 2.1 | Remove extracted content from root | `CLAUDE.md` |
| 2.2 | Simplify remaining content | `CLAUDE.md` |
| 2.3 | Add rules reference section | `CLAUDE.md` |
| 2.4 | Update cross-references | `CLAUDE.md`, rules files |
| 3.1 | Replace tests/CLAUDE.md with redirect | `tests/CLAUDE.md` |
| 4.1 | Verify no duplication | (verification only) |
| 4.2 | Verify glob patterns load correctly | (verification only) |
| 4.3 | Validate YAML frontmatter | (verification only) |
| 4.4 | Content check (no path-specific in root) | (verification only) |
| **5.1** | **Add `!`command`` to verify skill** | `.claude/skills/verify-skill/SKILL.md` |
| **5.2** | **Add `context: fork` to plan-review** | `.claude/skills/plan-review-skill/SKILL.md` |
| **5.3** | **Route Phase 1 to Haiku** | `.claude/skills/plan-review-skill/SKILL.md` |

---

## Updated Commit Strategy

**Two commits** (rules restructuring separate from skill changes):

### Commit 1: Rules Restructuring
```
refactor: restructure Claude instructions with .claude/rules/

- Create .claude/rules/ with path-specific rule files:
  - rust-backend.md (src-tauri/**/*.rs)
  - svelte-frontend.md (src/**/*.{svelte,ts})
  - integration-tests.md (tests/integration/**)
  - migrations.md (src-tauri/migrations/**/*.sql)
- Slim root CLAUDE.md from 447 to ~150-200 lines
- Migrate tests/CLAUDE.md to rules (with redirect notice)
- No instruction changes, only reorganization

Context loads more efficiently - only relevant rules for current files.
```

### Commit 2: Skill Enhancements
```
feat: enhance skills with !command, context:fork, and model routing

- /verify: Use !command to pre-inject test results and git status
- /plan-review: Add context:fork for isolated Phase 1 execution
- /plan-review: Route Phase 1 to Haiku model for cost efficiency

Faster verification, cleaner context, lower costs.
```
