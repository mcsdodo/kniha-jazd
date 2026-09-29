---
name: verify
description: Use before claiming work is complete - runs tests, checks git status, verifies changelog
---

# Verification Before Completion

Run this before saying "task complete" or "done".

## Current Status (Pre-injected)

### Test Results
!`cargo test --manifest-path src-tauri/Cargo.toml --workspace 2>&1 | tail -30`

### Git Status
!`git status`

### Changelog Preview
!`awk '/^## \[Unreleased\]/{f=1} f && /^## \[[0-9]/{exit} f' CHANGELOG.md | head -40`

### Migrations and Env Vars Not Yet Committed or Pushed
!`git status --short src-tauri/core/migrations; git diff -U0 origin/main -- src-tauri | grep -E '^[+-][^+-].*(: &str = "[A-Z][A-Z0-9_]+"|env::var\("[A-Z])'`

## Checklist

Based on the pre-injected data above:

1. **Tests Pass** - Check "Test Results" section. Do NOT proceed if tests fail.
2. **Code Committed** - Check "Git Status" section. All work-related files should be committed.
3. **Changelog Updated** - Check "Changelog Preview". [Unreleased] section should have entry for this work (skip for internal docs like CLAUDE.md, _tasks/).
4. **Upgrade Notes Match** - Check "Migrations and Env Vars". If it lists a migration or an env var, the `### Pokyny k aktualizácii` block in [Unreleased] must name it. Ignore `KNIHA_JAZD_MOCK_*` test hooks. The block is always present, so its presence alone proves nothing.

If changelog or upgrade notes are missing, run /changelog.

See CLAUDE.md for project constraints.
