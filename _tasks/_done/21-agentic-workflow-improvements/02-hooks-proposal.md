**Date:** 2026-01-05
**Subject:** Claude Code Hooks Proposal for kniha-jazd
**Status:** Proposal

## Overview

This document proposes concrete hook implementations to enforce the project's mandatory workflows:
1. **TDD enforcement** - Tests must pass before committing
2. **Changelog reminder** - Never forget to update changelog after completing work
3. **Session guidance** - Load context at session start

## Claude Code Hook System Summary

### Hook Events Available

| Event | Trigger | Use Case |
|-------|---------|----------|
| `PreToolUse` | Before Claude executes a tool | Block commits without tests, validate operations |
| `PostToolUse` | After tool completion | Run formatters, log operations |
| `Notification` | Claude sends notifications | Remind about workflows, alert on idle |
| `Stop` | Claude finishes responding | Cleanup, final reminders |
| `SessionStart` | Session initialization | Load context, display guidance |
| `UserPromptSubmit` | User submits prompt | Pre-process requests |
| `PreCompact` | Before transcript compaction | Save context |

### Configuration Locations

- `~/.claude/settings.json` - Global user settings
- `.claude/settings.json` - Project settings (committed)
- `.claude/settings.local.json` - Local settings (gitignored)

### Matcher Syntax

- `"*"` - All tools
- `"Bash"` - Specific tool
- `"Write|Edit"` - Multiple tools (regex OR)
- `""` - Empty (for Stop, Notification)

### Exit Codes

- `0` - Success, continue
- `1` - Error, display to user
- `2` - Block the action

### Environment Variables

- `$CLAUDE_PROJECT_DIR` - Project root directory
- `$CLAUDE_TOOL_INPUT` - Tool parameters (JSON via stdin)
- `$CLAUDE_FILE_PATHS` - Affected files
- `$CLAUDE_NOTIFICATION` - Notification message

---

## Proposed Hooks

### Hook 1: Pre-Commit Test Runner

**Purpose:** Run backend tests before any commit to enforce TDD workflow.

**File:** `.claude/settings.json` (committed, team-wide)

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -Command \"$input = [Console]::In.ReadToEnd(); $json = $input | ConvertFrom-Json; if ($json.tool_input.command -match '^git commit') { Write-Host '=== Running backend tests before commit ===' -ForegroundColor Cyan; Push-Location src-tauri; $result = cargo test 2>&1; $exitCode = $LASTEXITCODE; Pop-Location; if ($exitCode -ne 0) { Write-Host 'COMMIT BLOCKED: Tests failed!' -ForegroundColor Red; Write-Host $result; exit 2 } else { Write-Host 'Tests passed. Proceeding with commit.' -ForegroundColor Green } }\"",
            "timeout": 120000
          }
        ]
      }
    ]
  }
}
```

**Explanation:**
- Matches all `Bash` tool uses
- Parses stdin JSON to check if command starts with `git commit`
- Runs `cargo test` in src-tauri directory
- Blocks commit (exit 2) if tests fail
- Allows commit if tests pass

**Cross-platform Note:** Uses PowerShell for Windows compatibility. Could also create a helper script at `.claude/hooks/pre-commit.ps1`.

---

### Hook 2: Post-Commit Changelog Reminder

**Purpose:** Remind to update changelog after committing code changes.

**File:** `.claude/settings.json`

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -Command \"$input = [Console]::In.ReadToEnd(); $json = $input | ConvertFrom-Json; if ($json.tool_input.command -match '^git commit' -and $json.tool_input.command -notmatch 'CHANGELOG') { Write-Host ''; Write-Host '=== REMINDER: Update Changelog ===' -ForegroundColor Yellow; Write-Host 'Run /changelog to update CHANGELOG.md [Unreleased] section' -ForegroundColor Yellow; Write-Host 'This is a MANDATORY step per project guidelines.' -ForegroundColor Yellow; Write-Host '' }\"",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

**Explanation:**
- Triggers after successful `git commit` commands
- Skips reminder if commit message mentions CHANGELOG (likely a changelog commit)
- Displays prominent yellow reminder

---

### Hook 3: Session Start Guidance

**Purpose:** Display project context and workflow reminders at session start.

**File:** `.claude/settings.json`

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -Command \"Write-Host ''; Write-Host '=== kniha-jazd Project Session ===' -ForegroundColor Cyan; Write-Host 'Key Workflows:' -ForegroundColor White; Write-Host '  1. TDD: Write failing test BEFORE implementation' -ForegroundColor Gray; Write-Host '  2. Backend-only calculations (ADR-008)' -ForegroundColor Gray; Write-Host '  3. Run /changelog after completing features' -ForegroundColor Gray; Write-Host '  4. Use /decision for architectural choices' -ForegroundColor Gray; Write-Host ''; Write-Host 'Commands: /task-plan, /changelog, /decision, /release' -ForegroundColor DarkGray; Write-Host ''\"",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

**Note:** SessionStart hook availability may vary. If not supported, this guidance could be added to CLAUDE.md header or shown via Notification hook on first interaction.

---

### Hook 4: Notification on Idle

**Purpose:** Remind about pending tasks when Claude is idle waiting for input.

**File:** `.claude/settings.json`

```json
{
  "hooks": {
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -Command \"Write-Host ''; Write-Host 'Before marking complete, verify:' -ForegroundColor Yellow; Write-Host '  [ ] Tests pass (npm run test:backend)' -ForegroundColor Gray; Write-Host '  [ ] Code committed' -ForegroundColor Gray; Write-Host '  [ ] /changelog updated' -ForegroundColor Gray; Write-Host ''\"",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

---

### Hook 5: Write/Edit Linting (Optional)

**Purpose:** Run TypeScript/Svelte checks after editing frontend files.

**File:** `.claude/settings.json`

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -Command \"$input = [Console]::In.ReadToEnd(); $json = $input | ConvertFrom-Json; $filePath = $json.tool_input.file_path; if ($filePath -match '\\.(ts|svelte)$' -and $filePath -notmatch 'src-tauri') { Write-Host 'Running svelte-check...' -ForegroundColor DarkGray; npm run check 2>&1 | Select-Object -First 20 }\"",
            "timeout": 30000
          }
        ]
      }
    ]
  }
}
```

**Note:** This is optional and may slow down development. Could be made more targeted.

---

## Complete Recommended Configuration

**File:** `.claude/settings.json` (create this file, commit to repo)

```json
{
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
    ],
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -File .claude/hooks/idle-reminder.ps1",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

---

## Helper Scripts

### File: `.claude/hooks/pre-commit.ps1`

```powershell
# Pre-commit hook: Run tests before allowing git commit
# Blocks commit if tests fail

$input = [Console]::In.ReadToEnd()
$json = $input | ConvertFrom-Json

# Only run for git commit commands
if ($json.tool_input.command -notmatch '^git commit') {
    exit 0
}

Write-Host ""
Write-Host "=== Pre-commit: Running backend tests ===" -ForegroundColor Cyan
Write-Host ""

Push-Location src-tauri
try {
    $output = cargo test 2>&1
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0) {
        Write-Host "COMMIT BLOCKED: Backend tests failed!" -ForegroundColor Red
        Write-Host ""
        Write-Host $output
        Write-Host ""
        Write-Host "Fix failing tests before committing." -ForegroundColor Yellow
        exit 2  # Block the commit
    }

    Write-Host "All tests passed. Proceeding with commit." -ForegroundColor Green
    Write-Host ""
    exit 0
}
finally {
    Pop-Location
}
```

### File: `.claude/hooks/post-commit-reminder.ps1`

```powershell
# Post-commit hook: Remind about changelog updates

$input = [Console]::In.ReadToEnd()
$json = $input | ConvertFrom-Json

# Only for git commit commands
if ($json.tool_input.command -notmatch '^git commit') {
    exit 0
}

# Skip if commit message mentions changelog
if ($json.tool_input.command -match 'changelog|CHANGELOG') {
    exit 0
}

Write-Host ""
Write-Host "======================================" -ForegroundColor Yellow
Write-Host " REMINDER: Update Changelog" -ForegroundColor Yellow
Write-Host "======================================" -ForegroundColor Yellow
Write-Host ""
Write-Host "Run /changelog to update CHANGELOG.md [Unreleased] section" -ForegroundColor White
Write-Host "This is a MANDATORY step per CLAUDE.md guidelines." -ForegroundColor Gray
Write-Host ""

exit 0
```

### File: `.claude/hooks/idle-reminder.ps1`

```powershell
# Idle notification hook: Show completion checklist

Write-Host ""
Write-Host "=== Task Completion Checklist ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Before marking work complete:" -ForegroundColor White
Write-Host "  [ ] Tests pass (npm run test:backend)" -ForegroundColor Gray
Write-Host "  [ ] Code committed with descriptive message" -ForegroundColor Gray
Write-Host "  [ ] /changelog run to update [Unreleased]" -ForegroundColor Gray
Write-Host "  [ ] Changelog committed" -ForegroundColor Gray
Write-Host ""

exit 0
```

---

## Implementation Plan

### Phase 1: Basic Hooks (Recommended First)

1. Create `.claude/hooks/` directory
2. Add `pre-commit.ps1` script
3. Add `post-commit-reminder.ps1` script
4. Create `.claude/settings.json` with PreToolUse and PostToolUse hooks
5. Test with a sample commit

### Phase 2: Enhanced Notifications

1. Add `idle-reminder.ps1`
2. Add Notification hook configuration
3. Test idle behavior

### Phase 3: Optional Enhancements

1. Add SessionStart hook (if supported)
2. Add Write/Edit linting hooks
3. Fine-tune timeouts and messaging

---

## Considerations

### Windows Compatibility

All scripts use PowerShell for Windows compatibility. The project runs on Windows per the environment info.

### Cross-Platform Alternative

For cross-platform support, create shell scripts with `.sh` extension and use conditional logic:

```json
{
  "type": "command",
  "command": "if [ -f .claude/hooks/pre-commit.sh ]; then sh .claude/hooks/pre-commit.sh; else pwsh -NoProfile -File .claude/hooks/pre-commit.ps1; fi"
}
```

### Performance

- Pre-commit tests add ~30-60 seconds to commit time
- This is acceptable for TDD enforcement
- Can be disabled locally via `.claude/settings.local.json` if needed

### Limitations

- No native PreCommit/PostCommit hooks yet (feature request open)
- Using Bash matcher with pattern matching is the workaround
- Exit code 2 blocks operation but may not work in all scenarios

---

## Integration with Existing Skills

### /changelog Skill Integration

The post-commit reminder explicitly mentions `/changelog` command, creating a natural workflow connection.

### /decision Skill Integration

Could add a hook that detects architectural discussions and prompts for `/decision` documentation.

### /release Skill Integration

The pre-commit hook ensures tests pass, which is also a requirement before release.

---

## References

- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks)
- [Claude Code Hook Guide](https://code.claude.com/docs/en/hooks-guide)
- [Git Workflow Automation Feature Request](https://github.com/anthropics/claude-code/issues/4834)
- [GitButler Claude Code Hooks Integration](https://docs.gitbutler.com/features/ai-integration/claude-code-hooks)
- [Demystifying Claude Code Hooks](https://www.brethorsting.com/blog/2025/08/demystifying-claude-code-hooks/)
