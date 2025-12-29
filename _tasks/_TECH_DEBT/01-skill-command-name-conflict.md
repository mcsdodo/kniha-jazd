# Tech Debt: Skill/Command Name Conflict Workaround

**Date:** 2025-12-29
**Priority:** Low
**Effort:** Low (<2h)
**Component:** `.claude/skills/`, `.claude/commands/`
**Status:** Open (waiting for upstream fix)

## Problem

Claude Code has a bug where if a **skill** and **slash command** share the same name, the command becomes model-only and cannot be invoked by users directly.

Error: `This slash command can only be invoked by Claude, not directly by users.`

**GitHub Issue:** [anthropics/claude-code#14945](https://github.com/anthropics/claude-code/issues/14945)

## Impact

- Had to rename skills with `-skill` suffix to avoid conflict
- Skills: `task-plan-skill`, `decision-skill`, `changelog-skill`, `release-skill`
- Commands: `/task-plan`, `/decision`, `/changelog`, `/release`
- Awkward naming convention forced by bug

## Root Cause

Bug in Claude Code's command/skill resolution. When both exist with the same name, the slash command is incorrectly marked as model-invocable only.

## Recommended Solution

When the upstream bug is fixed:

1. Rename skill folders back to original names:
   ```bash
   cd .claude/skills
   mv task-plan-skill task-plan
   mv decision-skill decision
   mv changelog-skill changelog
   mv release-skill release
   ```

2. Update `name` field in each `SKILL.md` frontmatter to remove `-skill` suffix

3. Test that `/task-plan` etc. work as user-invocable commands

## Related

- GitHub Issue: https://github.com/anthropics/claude-code/issues/14945
- Skills location: `.claude/skills/*-skill/SKILL.md`
- Commands location: `.claude/commands/*.md`

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-12-29 | Added `-skill` suffix workaround | Bug prevents same-name skill/command coexistence |
