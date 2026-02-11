# Task 49: Claude Configuration Improvements

**Status:** ✅ Complete
**Created:** 2026-02-01
**Type:** Developer Experience

## Problem

1. **Bloated context:** Root `CLAUDE.md` is 447 lines, mixing global principles with path-specific patterns. When editing Rust, Claude loads frontend i18n instructions.

2. **Manual data gathering:** Skills like `/verify` require Claude to run commands separately instead of having data pre-injected.

3. **Inefficient model usage:** Exploration/analysis tasks use Opus when Haiku would suffice.

## Goals

### Part A: Rules Restructuring (Hybrid Approach)

1. **Extract path-specific content** from root `CLAUDE.md` into `.claude/rules/` files with YAML frontmatter glob patterns
2. **Keep documentation conventions** co-located (`docs/CLAUDE.md`, `_tasks/CLAUDE.md`, `_tasks/_TECH_DEBT/CLAUDE.md`)
3. **Migrate `tests/CLAUDE.md`** to `.claude/rules/integration-tests.md` (it's technical, benefits from globs)
4. **Slim root `CLAUDE.md`** to ~150-200 lines of truly global content

### Part B: Skill Enhancements (Quick Wins)

5. **Add `!command` to `/verify`** - Pre-inject test results and git status into prompt
6. **Add `context: fork` to `/plan-review`** - Run Phase 1 in isolated context
7. **Route exploration to Haiku** - Use cheaper model for read-only analysis

## Success Criteria

### Part A: Rules
- [ ] `.claude/rules/` folder created with 4 rules files
- [ ] Root `CLAUDE.md` reduced to ~150-200 lines (global content only)
- [ ] `tests/CLAUDE.md` migrated to rules (with redirect notice)
- [ ] All path-specific patterns use YAML frontmatter globs
- [ ] No duplicate instructions between root and rules
- [ ] Existing subdirectory CLAUDE.md files preserved (docs/, _tasks/)

### Part B: Skills
- [ ] `/verify` skill uses `!command` for test results and git status
- [ ] `/plan-review` skill uses `context: fork` for Phase 1
- [ ] `/plan-review` Phase 1 agent uses `model: haiku`

## Non-Goals

- Changing the actual instructions/patterns (only restructuring)
- Adding new rules or conventions beyond restructuring
- Modifying `.claude/settings.json` hooks

## References

- Reddit thread: "7 Claude Code Power Tips Nobody's Talking About"
- Claude Code docs: code.claude.com/docs (rules feature)
