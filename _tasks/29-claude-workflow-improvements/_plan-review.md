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

**Status:** Ready for User Review
**Iterations:** 3
**Total Findings:** 3 Critical, 7 Important, 9 Minor

### All Findings (Consolidated)

#### Critical

1. [ ] **Wildcard Bash permissions missing** - 01-design.md Section 5 requires `"permissions": { "allow": ["Bash(cargo *)", "Bash(npm *)", "Bash(git *)"] }` but no task addresses this
2. [ ] **YAML-style allowed-tools not addressed** - 01-design.md requires updating skills with `allowed-tools:` YAML syntax, but no task covers this
3. [ ] **Skill hooks syntax unverified** - No test that hooks actually fire; unknown if frontmatter syntax is correct

#### Important

4. [ ] **CLAUDE.md loses critical content** - "MANDATORY FINAL STEP", "/decision when:" guidance, and changelog warnings not moved to rule files
5. [ ] **Test counts duplicated** - Same data in rust-backend.md and testing.md creates sync burden
6. [ ] **No @import syntax verification** - Foundation of refactoring is untested
7. [ ] **PowerShell-specific hook commands** - Cross-platform issue for code-review-skill
8. [ ] **release-skill hook duplicates workflow** - Skill already runs build; hook would run it again
14. [ ] **Task order creates rollback risk** - Individual commits make rollback complex if Task 7 fails
15. [ ] **No backup of CLAUDE.md** - Original content could be lost if refactor fails

#### Minor

9. [ ] **.gitkeep not created** - Empty directory tracking issue
10. [ ] **Size goal inconsistent** - DESIGN says ~50 lines, PLAN says <100 lines
11. [ ] **Unix commands on Windows** - `ls -la`, `wc -l` won't work
12. [ ] **Test count sync burden** - "108 tests" hardcoded in multiple places
13. [ ] **Named sessions placement** - May not be intuitive in git-workflow.md
16. [ ] **rust-analyzer not verified** - No check that it's installed
17. [ ] **Python availability** - JSON validation may fail without Python
18. [ ] **No partial failure guidance** - What to do if stopped mid-implementation
19. [ ] **Command inconsistency** - DESIGN vs PLAN differ on `npm run build` vs `npm run tauri build`

### Recommendation

**Needs revisions before implementation.** The 3 Critical issues represent missing requirements from 01-design.md that would make the implementation incomplete. The Important issues around atomicity (#14, #15) and content loss (#4) could cause problems during implementation.

**Suggested priority:**
1. Add missing tasks for Critical items #1-2
2. Add verification task for skill hooks (#3)
3. Move missing CLAUDE.md content to appropriate rule files (#4)
4. Restructure tasks for atomic commits (#14, #15)
5. Address remaining Important/Minor as desired
