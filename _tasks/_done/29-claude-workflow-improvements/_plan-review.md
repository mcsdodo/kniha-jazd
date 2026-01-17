# Plan Review

**Target:** _tasks/29-claude-workflow-improvements/02-plan.md
**Started:** 2026-01-08
**Status:** In Progress
**Focus:** Completeness, feasibility, clarity

## Iteration 1

### New Findings

#### Critical

1. **[Critical] 01-design.md requirement missing: "Wildcard Bash Permissions" not addressed in 02-plan.md**
   - 01-design.md Section 5 specifies adding wildcard bash permissions to settings.json: `"permissions": { "allow": ["Bash(cargo *)", "Bash(npm *)", "Bash(git *)"] }`
   - This is completely absent from the 02-plan.md tasks
   - Impact: Implementation will be incomplete vs design requirements

2. **[Critical] 01-design.md requirement missing: "YAML-style allowed-tools in skills" not addressed**
   - 01-design.md Section 5 and Implementation Step 5 specifies: "Update skills to use YAML-style allowed-tools"
   - No task in 02-plan.md covers converting skills to use the new `allowed-tools:` YAML syntax
   - Impact: This documented feature will not be implemented

3. **[Critical] Skill hooks syntax may be incorrect - no verification step**
   - Plan Tasks 9-11 add `hooks:` to skill frontmatter, but the existing skill files only have `name:` and `description:` in frontmatter
   - The Claude Code documentation should be verified for the correct hooks syntax in skill YAML frontmatter
   - No task to test/verify that hooks actually fire - only manual visual inspection suggested
   - Risk: Hooks may not work and there is no verification mechanism

#### Important

4. **[Important] CLAUDE.md refactored version loses critical content**
   - Original CLAUDE.md (309 lines) includes "MANDATORY FINAL STEP" warnings and "/decision when:" guidance
   - The slim CLAUDE.md in Task 7 (~100 lines) drops these without moving them to rules
   - Specifically missing from rules:
     - "MANDATORY FINAL STEP: After completing any feature..."
     - "Use /decision when..." guidance
     - "WARNING: Do NOT mark a task as complete without updating the changelog"
   - Impact: Important workflow enforcement instructions will be lost

5. **[Important] Rule file duplication: Test counts appear in both rust-backend.md and testing.md**
   - Task 2 (rust-backend.md) includes full "Test Coverage" section with test counts
   - Task 4 (testing.md) references test commands and organization
   - DRY violation - test counts will need updating in two places when tests change

6. **[Important] Missing verification that `@import` syntax actually works**
   - Task 7 uses `@.claude/rules/rust-backend.md` syntax to import rules
   - No verification step to confirm Claude Code loads these imports correctly
   - Should include: "Run `/context` or similar command to verify rules are loaded"

7. **[Important] Hook command in Task 10 (code-review-skill) uses Windows-specific PowerShell**
   - Proposed: `pwsh -NoProfile -Command "Write-Host 'REMINDER: ..."`
   - 01-design.md shows simpler: `echo 'Remember: reviewing, not implementing'`
   - The PowerShell approach may fail on non-Windows CI environments

8. **[Important] release-skill hook duplicates existing workflow logic**
   - Task 11 adds Stop hook: `cargo test && npm run tauri build`
   - But the release-skill SKILL.md already has explicit Steps 4-5 that run tests and build
   - This creates redundancy and potential confusion about when tests actually run

#### Minor

9. **[Minor] Task 1 cannot create .gitkeep on Windows**
   - The .gitkeep file is never created; just the directory
   - Empty directories are not tracked by git - need to create the file or rely on first rule file

10. **[Minor] CLAUDE.md slim version still exceeds stated goal**
    - Plan states CLAUDE.md should be "~50 lines" (from 01-design.md) but verification expects "Under 100 lines"
    - The proposed content in Task 7 is approximately 85-90 lines
    - Inconsistent with design

11. **[Minor] Unix commands in verification steps**
    - Task 12 uses `ls -la`, `wc -l` which are Unix commands
    - Project runs on Windows (per env context: `Platform: win32`)
    - Should use `dir` or PowerShell equivalents for Windows compatibility

12. **[Minor] Test count inconsistencies**
    - Multiple locations reference "108 tests" - creates sync burden when tests change

13. **[Minor] Named sessions documentation placement**
    - Named sessions guidance added to git-workflow.md but 01-design.md presents it as standalone concept
    - Users may not find it intuitively

### Coverage Assessment

**Areas Fully Reviewed:**
- All 13 tasks in 02-plan.md
- Mapping to 01-design.md requirements (6 sections)
- Current file contents (settings.json, SKILL.md files, CLAUDE.md)
- Syntax correctness of proposed changes
- Verification steps presence
- YAGNI/DRY analysis

**Areas Remaining / Partially Covered:**
- Claude Code documentation verification (skill hooks syntax) - would require external docs
- Actual runtime testing of `@import` syntax - cannot verify without execution
- Cross-platform compatibility testing

---

## Iteration 2

### New Findings

#### Important

14. **[Important] Task order creates rollback risk**
    - Tasks 2-6 create and commit rule files individually, then Task 7 refactors CLAUDE.md
    - If Task 7 fails (e.g., import syntax doesn't work), rolling back requires reverting multiple commits
    - Better approach: create all rule files uncommitted, verify imports work, then commit atomically

15. **[Important] No backup of original CLAUDE.md before replacement**
    - Task 7 says "Replace entire CLAUDE.md content" without preserving original
    - If refactored version has issues, recovery requires `git checkout` through multiple commits

#### Minor

16. **[Minor] rust-analyzer installation not verified**
    - Task 8 adds LSP config but doesn't verify rust-analyzer is installed
    - Should add: `rust-analyzer --version` verification step

17. **[Minor] `python -m json.tool` may not be available on Windows**
    - Task 8 uses Python for JSON validation
    - PowerShell's `ConvertFrom-Json` would be more reliable for Windows

18. **[Minor] No guidance for partial implementation failure**
    - If implementation stops after Task 6 but before Task 7, project has both full CLAUDE.md AND separate rule files
    - Plan should specify recovery: "If stopped here, either complete Task 7 or delete `.claude/rules/`"

### Refined Analysis

- Finding #10 confirmed: Proposed CLAUDE.md is ~102 lines (exceeds "under 100" criterion)
- Finding #8 confirmed: release-skill already runs build, hook would duplicate

### Coverage Assessment

**Areas Newly Explored:**
- Implementation sequence and atomicity (rollback scenarios)
- Prerequisite verification (rust-analyzer, python)
- Partial failure recovery guidance

**Remaining:**
- Claude Code skill hooks syntax (requires external documentation)
- Runtime verification of `@import` syntax

---

## Iteration 3

### New Findings

#### Minor

19. **[Minor] 01-design.md/02-plan.md command inconsistency for release-skill hook**
    - 01-design.md specifies: `npm run build`
    - 02-plan.md specifies: `npm run tauri build`
    - These are different commands - could cause confusion during implementation

### Coverage Assessment

Only 1 minor finding - review is now comprehensive.

---

## Review Summary

**Status:** Complete
**Iterations:** 3
**Total Findings:** 3 Critical, 7 Important, 9 Minor

### All Findings (Consolidated)

#### Critical

1. [x] **Wildcard Bash permissions missing** - Added to Task 9 in 02-plan.md
2. [x] **YAML-style allowed-tools not addressed** - Removed from 01-design.md (not a real Claude Code feature)
3. [x] **Skill hooks syntax unverified** - Added Task 14 verification step in 02-plan.md

#### Important

4. [x] **CLAUDE.md loses critical content** - Added to git-workflow.md rule (Task 5)
5. [x] **Test counts duplicated** - Removed from rust-backend.md, kept only test organization
6. [x] **No @import syntax verification** - Added Task 8 verification phase
7. [x] **PowerShell-specific hook commands** - Changed to cross-platform `echo` in Task 12
8. [x] **release-skill hook duplicates workflow** - Hook now only runs tests, not build (Task 13)
14. [x] **Task order creates rollback risk** - Restructured to phased approach with atomic commit
15. [x] **No backup of CLAUDE.md** - Added backup step in Task 7

#### Minor

9. [x] **.gitkeep not created** - Fixed in Task 1 with PowerShell commands
10. [x] **Size goal inconsistent** - Updated 01-design.md to say "under 100 lines"
11. [x] **Unix commands on Windows** - All commands now use PowerShell
12. [x] **Test count sync burden** - Removed hardcoded counts from rules
13. [ ] **Named sessions placement** - Kept in git-workflow.md (acceptable)
16. [x] **rust-analyzer not verified** - Added verification in Task 9
17. [x] **Python availability** - Changed to PowerShell ConvertFrom-Json
18. [x] **No partial failure guidance** - Added "Partial Failure Recovery" section
19. [x] **Command inconsistency** - Fixed 01-design.md to match (tests only, no build)

---

## Resolution

**Addressed:** 18 findings
**Skipped:** 1 finding (named sessions placement - acceptable as-is)
**Status:** Complete

### Applied Changes

**01-design.md:**
- Fixed size goal: "under 100 lines" (was ~50)
- Added critical content preservation note
- Fixed release-skill hook (tests only, no build duplication)
- Added cross-platform command note
- Restructured implementation order to phased/atomic approach
- Added rollback procedure
- Updated success criteria with verification steps

**02-plan.md:**
- Restructured into 7 phases with atomic commits
- Added CLAUDE.md backup step (Task 7)
- Added @import verification task (Task 8)
- Added wildcard Bash permissions to settings.json (Task 9)
- Added rust-analyzer installation check (Task 9)
- Changed all commands to PowerShell (Windows-compatible)
- Changed JSON validation to ConvertFrom-Json
- Added "MANDATORY FINAL STEP" and "/decision when:" to git-workflow.md rule
- Removed test counts from rules (avoid sync burden)
- Changed code-review hook to cross-platform echo
- Changed release-skill hook to tests only
- Added skill hooks verification step (Task 14)
- Added "Partial Failure Recovery" section
