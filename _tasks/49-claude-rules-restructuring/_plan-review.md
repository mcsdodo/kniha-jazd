# Plan Review: Task 49 - Claude Rules Restructuring

**Plan:** `_tasks/49-claude-rules-restructuring/02-plan.md`
**Task:** `_tasks/49-claude-rules-restructuring/01-task.md`
**Reviewed:** 2026-02-01
**Iterations:** 2 (no new findings in iteration 3)

---

## Summary

| Category | Count |
|----------|-------|
| Critical | 2 |
| Important | 4 |
| Minor | 3 |

**Recommendation:** ~~Needs Revisions~~ → **Ready for Implementation**

The plan is well-structured and covers the core restructuring goals. ~~However, there are critical gaps around the `!command` feature verification and missing content extraction specifics that could block implementation.~~

**Update (2026-02-01):** All Critical and Important findings have been addressed. Plan revised with:
- Feature verification added (all three features confirmed in Claude Code docs)
- Syntax corrected (`!`command`` with backticks)
- Line numbers replaced with section heading references
- Exact extraction boundaries specified
- Cross-reference update step added
- Concrete glob verification method added

---

## Findings

### Critical (Blocks Implementation)

#### C1. `!command` syntax unverified - feature may not exist

**Location:** Phase 5, Steps 5.1-5.3

**Issue:** The plan assumes `!command` syntax, `context: fork`, and `model: haiku` are valid Claude Code skill features. However:
- No documentation link provided verifying these features exist
- The Reddit thread referenced ("7 Claude Code Power Tips Nobody's Talking About") is not authoritative documentation
- Current `.claude/skills/verify-skill/SKILL.md` uses standard markdown + bash code blocks - no evidence `!command` syntax is supported
- If these features don't exist, Phase 5 is entirely blocked

**Fix:** Either:
1. Add verification step to test `!command` syntax with a minimal example before implementing
2. Provide Claude Code official documentation link confirming feature support
3. Mark Phase 5 as "experimental" with fallback plan if features unavailable

---

#### C2. Line number references in plan are inaccurate

**Location:** Phase 1, Steps 1.1-1.4 (line references like "lines 216-220", "lines 138-155")

**Issue:** The plan references specific line numbers for content extraction (e.g., "Database Migration Best Practices section (lines 138-155)"). However, actual CLAUDE.md line numbers don't match:
- "Adding a New Tauri Command" is lines 216-220 in plan but 216-221 in actual file - minor drift
- "Database Migration Best Practices" is at lines 138-155, which is correct
- But if any prior edit occurs, ALL subsequent line numbers become invalid

**Fix:** Replace line number references with section heading references:
- Use `## Database Migration Best Practices` instead of "lines 138-155"
- More robust to future edits and easier to locate during implementation

---

### Important (Should Fix)

#### I1. Missing exact content boundaries for extraction

**Location:** Phase 1, Step 1.1

**Issue:** "Extract from root CLAUDE.md: Test organization (`*_tests.rs` pattern, lines 175-186)" is vague. The implementer needs to know:
- Does "Test organization" include the "Test Coverage" section (lines 188-212)?
- The plan says extract "Backend test coverage details (lines 190-201)" but the actual coverage section runs to line 212

**Fix:** For each rule file, list the EXACT section headings to extract:
```
rust-backend.md extracts:
- ### Test Organization (all)
- ### Test Coverage > Backend (Rust) only
- ### Code Patterns > "Adding a New Tauri Command" only
- ### Code Patterns > "Adding a New Calculation" only
```

---

#### I2. No handling for cross-referenced content

**Location:** Phase 2

**Issue:** Some content in root CLAUDE.md references other sections that may be extracted. For example:
- "Adding a New Calculation" (step 5) says "If new UI element, add integration test" - but integration test details are in `integration-tests.md`
- After extraction, these cross-references may need updating to point to the rules files

**Fix:** Add Step 2.4: "Review extracted content for internal cross-references and update to point to new locations (e.g., 'See .claude/rules/integration-tests.md for test patterns')"

---

#### I3. `tests/CLAUDE.md` redirect creates orphaned reference

**Location:** Phase 3, Step 3.1

**Issue:** The redirect notice says "Testing patterns have moved to `.claude/rules/integration-tests.md`". But:
- Developers may still look at `tests/CLAUDE.md` directly
- Some tooling or scripts might reference `tests/CLAUDE.md`
- The redirect doesn't preserve the full value of the original file

**Fix:** Either:
1. Delete `tests/CLAUDE.md` entirely (rules auto-load means no loss)
2. Or make the redirect more discoverable by adding a symlink/alias (if supported)

Current approach is acceptable but consider noting in plan that this is intentional deprecation.

---

#### I4. Glob patterns may not match expected files

**Location:** Phase 4, Step 4.2

**Issue:** Verification step says "Test that `.rs` files would match `rust-backend.md` globs" but doesn't explain HOW to test this. Claude Code glob loading is automatic - there's no command to verify which rules load for a given file.

**Fix:** Add concrete verification method:
```
For each rules file, edit a matching file (e.g., src-tauri/src/db.rs)
and verify the rules content appears in Claude's context.
Alternative: check Claude Code documentation for glob debugging.
```

---

### Minor (Nice to Have)

#### M1. Plan target of ~150-200 lines is arbitrary

**Location:** Phase 2, Step 2.2, Step 4.3

**Issue:** "Root CLAUDE.md is ~150-200 lines (reduced from 447)" is a target without clear justification. What matters is:
- No path-specific content remains
- No duplication exists
- Core principles are preserved

**Fix:** Reframe as outcome-based: "Root CLAUDE.md contains only project-wide content (no file-type-specific patterns)" rather than line count target.

---

#### M2. Single commit vs atomic commits

**Location:** Commit Strategy section

**Issue:** Plan proposes single commit for Phase 1-4, which means:
- If validation fails at Step 4.x, rollback loses all work
- Can't bisect issues to specific rule files

**Fix:** Consider atomic commits per phase:
- Commit 1: Create all `.claude/rules/` files
- Commit 2: Slim root CLAUDE.md
- Commit 3: Handle tests/CLAUDE.md migration

This is minor because the rollback plan addresses recovery, but atomic commits are cleaner.

---

#### M3. No verification that frontmatter YAML is valid

**Location:** Phase 1, all steps

**Issue:** Each rule file uses YAML frontmatter with globs. There's no step verifying the YAML is syntactically valid or that Claude Code accepts the format.

**Fix:** Add to Phase 4: "Validate YAML frontmatter syntax (can use online YAML validator or Claude Code error messages)"

---

## Completeness Check vs 01-task.md

| Requirement from 01-task.md | Covered in Plan? | Notes |
|----------------------------|------------------|-------|
| Extract path-specific content to .claude/rules/ | Yes | Phase 1 |
| Keep docs/CLAUDE.md, _tasks/CLAUDE.md, _tasks/_TECH_DEBT/CLAUDE.md | Yes | Explicitly preserved |
| Migrate tests/CLAUDE.md to rules | Yes | Phase 3 |
| Slim root CLAUDE.md to ~150-200 lines | Yes | Phase 2 |
| 4 rules files created | Yes | rust-backend, svelte-frontend, integration-tests, migrations |
| No duplicate instructions | Yes | Phase 4 validation |
| /verify uses !command | Yes (but unverified feature) | Phase 5.1 |
| /plan-review uses context:fork | Yes (but unverified feature) | Phase 5.2 |
| /plan-review Phase 1 uses model:haiku | Yes (but unverified feature) | Phase 5.3 |

All requirements addressed, but Phase 5 features need verification.

---

## Dependencies Check

| Step | Depends On | Correctly Ordered? |
|------|------------|-------------------|
| 1.1-1.4 (Create rules) | None | Yes |
| 2.1 (Remove from root) | 1.x (rules exist) | Yes |
| 2.2 (Simplify root) | 2.1 (removed content) | Yes |
| 2.3 (Add rules reference) | 1.x (rules exist) | Yes |
| 3.1 (Redirect tests/CLAUDE.md) | 1.3 (integration-tests.md exists) | Yes |
| 4.x (Validation) | All creation steps | Yes |
| 5.x (Skill enhancements) | Independent | Yes |

Order is correct.

---

## Recommendation

**Status:** ~~Needs Revisions~~ → **Ready for Implementation**

~~Before implementation:~~
1. ~~**Must fix C1:** Verify `!command`, `context: fork`, `model: haiku` features exist in Claude Code, or mark Phase 5 as optional/experimental~~
2. ~~**Must fix C2:** Replace line number references with section heading references~~
3. ~~**Should fix I1:** Specify exact section boundaries for extraction to avoid ambiguity~~
4. ~~**Should fix I2:** Add cross-reference update step~~

~~Other findings are improvements but won't block implementation.~~

---

## Resolution (2026-02-01)

| Finding | Status | Resolution |
|---------|--------|------------|
| C1 | ✅ Addressed | Features verified in Claude Code docs. Syntax corrected to `!`command`` |
| C2 | ✅ Addressed | Line numbers replaced with section heading tables |
| I1 | ✅ Addressed | Exact extraction boundaries added as tables per rule file |
| I2 | ✅ Addressed | Step 2.4 added for cross-reference updates |
| I3 | ⏭️ Skipped | Current redirect approach is acceptable |
| I4 | ✅ Addressed | Concrete verification method table added to Step 4.2 |
| M1 | ✅ Addressed | Reframed as outcome-based in Step 4.4 |
| M2 | ⏭️ Skipped | Single commit acceptable with rollback plan |
| M3 | ✅ Addressed | YAML validation added as Step 4.3 |

**Plan is now ready for implementation.**
