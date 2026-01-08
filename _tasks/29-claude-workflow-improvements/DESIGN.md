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

**CLAUDE.md** becomes a slim index (~50 lines) that imports these via `@.claude/rules/*.md` syntax.

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
    command: "cd src-tauri && cargo test && npm run build"
    timeout: 300000
```

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

## Implementation Order (Big Bang)

1. Create `.claude/rules/` directory with all rule files
2. Refactor CLAUDE.md to slim index with imports
3. Update `.claude/settings.json` with LSP config
4. Add hooks to skill frontmatter files
5. Update skills to use YAML-style allowed-tools
6. Document named sessions and shortcuts in rules

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

- [ ] LSP provides go-to-definition for Rust code
- [ ] Rules load correctly (verify with `/context`)
- [ ] Skill hooks execute on skill completion
- [ ] CLAUDE.md is under 100 lines
- [ ] All existing functionality preserved
