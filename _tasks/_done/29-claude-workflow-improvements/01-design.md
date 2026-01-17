# Claude Code Workflow Improvements - Design Document

**Created:** 2026-01-08
**Approach:** Big Bang (all features at once)

## Overview

Comprehensive improvements to Claude Code workflows based on v2.0.64 - v2.1.0 release notes. Enhances developer experience, workflow enforcement, and maintainability.

## 1. LSP Configuration

**File:** `.claude/settings.json`

Add rust-analyzer for Rust code intelligence:

```json
{
  "lsp": {
    "rust-analyzer": {
      "command": "rust-analyzer",
      "filetypes": ["rs"]
    }
  },
  "hooks": {
    // existing hooks unchanged
  }
}
```

**Benefits:**
- Go-to-definition in Rust code
- Find references across `src-tauri/`
- Hover documentation

## 2. Rules Directory Structure

**New directory:** `.claude/rules/`

Split CLAUDE.md into focused modules:

```
.claude/rules/
├── rust-backend.md      # Rust patterns, Tauri commands, test organization
├── svelte-frontend.md   # SvelteKit patterns, i18n, display-only principle
├── testing.md           # TDD workflow, what to test, running tests
├── git-workflow.md      # Commit guidelines, staging rules, PR creation
└── business-logic.md    # Consumption calculations, legal limits, margins
```

**CLAUDE.md** becomes a slim index (under 100 lines) that imports these via `@.claude/rules/*.md` syntax.

**Critical content to preserve in rules:**
- "MANDATORY FINAL STEP" workflow enforcement → `git-workflow.md`
- "/decision when:" guidance → `git-workflow.md`
- Changelog warnings → `git-workflow.md`

## 3. Skill Hooks

Add hooks to skill frontmatter for automatic workflow enforcement:

### verify-skill
```yaml
hooks:
  - event: Stop
    command: "cd src-tauri && cargo test --quiet"
```

### code-review-skill
```yaml
hooks:
  - event: PreToolUse
    matcher: Edit
    command: "echo 'Remember: reviewing, not implementing'"
```

### release-skill
```yaml
hooks:
  - event: Stop
    command: "cd src-tauri && cargo test"
    timeout: 300000
```

**Note:** Release-skill already runs `npm run tauri build` in its workflow steps - hook only runs tests to avoid duplication.

**Cross-platform consideration:** Use simple shell commands (`echo`, `cd`) that work on both Windows and Unix. Avoid PowerShell-specific syntax in hooks.

## 4. Named Sessions

**Usage patterns** (no configuration needed):

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

## 5. Additional Enhancements

### Wildcard Bash Permissions
```json
{
  "permissions": {
    "allow": ["Bash(cargo *)", "Bash(npm *)", "Bash(git *)"]
  }
}
```

### YAML-style allowed-tools in skills
```yaml
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
```

### Workflow Shortcuts Documentation
| Shortcut | Purpose |
|----------|---------|
| `Ctrl+B` | Background long-running commands |
| `Alt+T` | Toggle thinking mode |
| `Ctrl+O` | View conversation transcript |
| `/plan` | Quick entry to plan mode |

## Implementation Order (Atomic)

**Phase 1: Create all files (no commits)**
1. Create `.claude/rules/` directory with all rule files
2. Create slim CLAUDE.md with imports (keep backup of original)
3. Update `.claude/settings.json` with LSP config + wildcard permissions

**Phase 2: Verify before committing**
4. Verify @import syntax loads rules correctly
5. Verify rust-analyzer is installed and LSP works

**Phase 3: Commit atomically**
6. Commit all rule files + CLAUDE.md together (single atomic commit)
7. Commit settings.json changes

**Phase 4: Skill updates**
8. Add hooks to skill frontmatter files
9. Test that hooks fire correctly
10. Commit skill changes

## Files to Create/Modify

| File | Action |
|------|--------|
| `.claude/rules/rust-backend.md` | Create |
| `.claude/rules/svelte-frontend.md` | Create |
| `.claude/rules/testing.md` | Create |
| `.claude/rules/git-workflow.md` | Create |
| `.claude/rules/business-logic.md` | Create |
| `CLAUDE.md` | Refactor to slim index |
| `.claude/settings.json` | Add LSP config |
| `.claude/skills/verify-skill/SKILL.md` | Add hooks |
| `.claude/skills/code-review-skill/SKILL.md` | Add hooks |
| `.claude/skills/release-skill/SKILL.md` | Add hooks |

## Success Criteria

- [ ] rust-analyzer installed (`rust-analyzer --version` succeeds)
- [ ] LSP provides go-to-definition for Rust code
- [ ] Rules load correctly (new Claude session shows rule content)
- [ ] Skill hooks execute on skill completion (test verify-skill runs cargo test)
- [ ] CLAUDE.md is under 100 lines
- [ ] Critical workflow content preserved in rule files
- [ ] All backend tests pass
- [ ] Clean git status after implementation

## Rollback Procedure

If @import syntax doesn't work or rules don't load:
1. `git checkout HEAD~1 -- CLAUDE.md` to restore original
2. Delete `.claude/rules/` directory
3. Revert settings.json changes if needed
