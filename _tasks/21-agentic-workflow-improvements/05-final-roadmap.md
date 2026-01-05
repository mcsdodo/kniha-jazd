**Date:** 2026-01-05
**Subject:** Final Roadmap - Agentic Workflow Improvements
**Status:** Revised after Iteration 2 Review

---

# Executive Summary

**REVISED:** After critical review (see `_review.md`), the original proposal was significantly simplified. The 80/20 rule applies - focus on automation that enforces quality, not comprehensive documentation.

## Final Recommendations (Post-Review)

| Rank | Recommendation | Impact | Effort | Why |
|------|----------------|--------|--------|-----|
| 1 | **Pre-commit test hook** | Critical | 1 hour | Only item that actually enforces TDD |
| 2 | **Post-commit changelog reminder** | High | 30 min | Addresses real problem (forgotten changelogs) |
| 3 | **verify-skill** | Medium | 30 min | Prevents premature "done" claims |
| 4 | **Move Common Pitfalls in CLAUDE.md** | Low | 10 min | Quick visibility improvement |

**CUT from original proposal:**
- ~~Restructure CLAUDE.md~~ (unnecessary - current structure is fine)
- ~~tdd-skill~~ (redundant - pre-commit hook enforces tests)
- ~~review-skill~~ (redundant - superpowers:requesting-code-review exists)
- ~~debug-skill~~ (documentation, not automation)
- ~~pre-implementation-skill~~ (bureaucracy - task-plan already covers this)
- ~~SessionStart hook~~ (doesn't exist in Claude Code)
- ~~Idle notification hook~~ (noise, will be ignored)
- ~~Write/Edit linting hook~~ (too slow, errors visible in dev anyway)

## Key Metrics

| Metric | Current State | Target |
|--------|---------------|--------|
| Commits blocked if tests fail | 0% | 100% |
| Changelog reminder after commits | No | Yes |
| Implementation time | N/A | <4 hours |
| New files created | N/A | 4 files |

## Success Criteria

1. **Commits blocked if tests fail** - Pre-commit hook works
2. **Changelog reminder shown after commits** - Post-commit hook works
3. **verify-skill usable** - Skill invocable and runs checks

---

# REVISED Implementation Roadmap (Post-Review)

> **Note:** This roadmap was significantly simplified after Iteration 1 review.
> See `_review.md` for full rationale. Original 4-phase plan reduced to 2 phases.

## Phase 1: Essential Automation (2-2.5 hours)

**Goal:** Implement the two hooks that actually enforce quality.

### Task 1.1: Create Pre-Commit Test Hook (1 hour)

**Files to create:**
- `.claude/hooks/pre-commit.ps1` - Test runner script
- `.claude/settings.json` - Hook configuration

**Implementation:**
```json
// .claude/settings.json
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "pwsh -NoProfile -File .claude/hooks/pre-commit.ps1",
        "timeout": 120000
      }]
    }]
  }
}
```

**pre-commit.ps1 (with error handling from Iteration 2):**

```powershell
# Pre-commit hook: Block commits if backend tests fail
# Exit 0 = allow, Exit 2 = block

$inputText = [Console]::In.ReadToEnd()
if (-not $inputText) { exit 0 }

try {
    $json = $inputText | ConvertFrom-Json
} catch {
    exit 0  # Don't block on parse errors
}

if ($json.tool_input.command -notmatch '^git commit') {
    exit 0
}

$projectDir = if ($env:CLAUDE_PROJECT_DIR) { $env:CLAUDE_PROJECT_DIR } else { (Get-Location).Path }
$srcTauri = Join-Path $projectDir "src-tauri"

if (-not (Test-Path $srcTauri)) {
    Write-Host "Warning: src-tauri not found at $srcTauri" -ForegroundColor Yellow
    exit 0
}

Write-Host "`n=== Pre-commit: Running backend tests ===" -ForegroundColor Cyan

Push-Location $srcTauri
try {
    cargo test 2>&1 | Tee-Object -Variable testOutput
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`nCOMMIT BLOCKED: Tests failed!" -ForegroundColor Red
        exit 2
    }
    Write-Host "Tests passed. Proceeding with commit." -ForegroundColor Green
} finally {
    Pop-Location
}
exit 0
```

**Why:** Only item that actually enforces TDD.

---

### Task 1.2: Create Post-Commit Changelog Reminder (30 min)

**Add to settings.json:**
```json
"PostToolUse": [{
  "matcher": "Bash",
  "hooks": [{
    "type": "command",
    "command": "pwsh -NoProfile -File .claude/hooks/post-commit-reminder.ps1",
    "timeout": 5000
  }]
}]
```

**post-commit-reminder.ps1 (with error handling from Iteration 2):**

```powershell
# Post-commit reminder: Prompt for changelog update

$inputText = [Console]::In.ReadToEnd()
if (-not $inputText) { exit 0 }

try {
    $json = $inputText | ConvertFrom-Json
} catch {
    exit 0
}

if ($json.tool_input.command -notmatch '^git commit') {
    exit 0
}

if ($json.tool_input.command -match 'changelog|CHANGELOG') {
    exit 0  # Skip if this is a changelog commit
}

Write-Host ""
Write-Host "=== REMINDER: Update Changelog ===" -ForegroundColor Yellow
Write-Host "Run /changelog to update CHANGELOG.md [Unreleased] section" -ForegroundColor White
Write-Host ""

exit 0
```

**Why:** Addresses most common workflow gap.

---

### Task 1.3: Create verify-skill (30 min)

**File to create:**
- `.claude/skills/verify-skill/SKILL.md`

**Content (simplified from original):**

```markdown
---
name: verify-skill
description: Use before claiming work is complete - runs tests, checks git status, verifies changelog
---

# Verification Before Completion

Run this before saying "task complete" or "done".

## Checklist

### 1. Tests Pass
```bash
npm run test:backend
```
Do NOT proceed if tests fail.

### 2. Code Committed
```bash
git status
```
All work-related files should be committed.

### 3. Changelog Updated
Check CHANGELOG.md [Unreleased] section has entry for this work.
If not, run /changelog.

## Quick Verification
```powershell
# Run tests
cd src-tauri && cargo test

# Check status
git status

# Check changelog
head -20 CHANGELOG.md
```

See CLAUDE.md for project constraints.
```

---

### Task 1.4: Add Hook Skip Mechanism (15 min)

**Options for skipping hooks (document in CLAUDE.md):**

1. **Local override** - Add empty hooks in `.claude/settings.local.json`:
   ```json
   {
     "hooks": {
       "PreToolUse": [],
       "PostToolUse": []
     }
   }
   ```

2. **Temporary rename** - Rename `.claude/settings.json` to `.claude/settings.json.bak`

3. **Git native** - Use `git commit --no-verify` (hook won't match the command pattern)

### Task 1.5: Test Hooks (15 min)

**Verification steps:**

1. Add temporary failing test to `src-tauri/src/calculations.rs`:
   ```rust
   #[test]
   fn test_hook_verification_DELETE_ME() {
       assert!(false, "This test should fail - delete after testing");
   }
   ```

2. Attempt `git commit` - should see "COMMIT BLOCKED: Tests failed!"

3. Remove the failing test

4. Commit again - should succeed and show changelog reminder

---

## Phase 2: Minimal Documentation Updates (30 min)

### Task 2.1: Move Common Pitfalls in CLAUDE.md (10 min)

Move lines 114-121 (Common Pitfalls section) to after line 50 (after TDD section).

**Current location:** Line 114
**New location:** Line ~50 (after "What to Test" section)

This is the only CLAUDE.md change needed.

---

### Task 2.2: Add Skill Hints to Skills Table (10 min)

Update the skills table in CLAUDE.md:

```markdown
| Skill | Purpose | Trigger |
|-------|---------|---------|
| `/task-plan` | Create planning folder | "plan feature", "new task" |
| `/changelog` | Update CHANGELOG.md | After completing work (MANDATORY) |
| `/decision` | Record ADR/BIZ | "should I use X or Y?" |
| `/release` | Version bump + build | "release", "publish" |
| `/verify` | Check before "done" | Before claiming complete |
```

---

### Task 2.3: Add Cross-Reference to Existing Skills (10 min)

Add ONE line to each existing skill file:

```markdown
## Project Context
See CLAUDE.md for project constraints (ADR-008, TDD, changelog requirements).
```

Files: task-plan-skill, decision-skill, changelog-skill, release-skill

---

## Implementation Checklist (Simplified)

### Phase 1: Essential Automation (~1.5 hours)
- [ ] Create `.claude/hooks/` directory
- [ ] Create `.claude/hooks/pre-commit.ps1` (use script from Task 1.1)
- [ ] Create `.claude/hooks/post-commit-reminder.ps1` (use script from Task 1.2)
- [ ] Create `.claude/settings.json` with PreToolUse and PostToolUse hooks
- [ ] Create `.claude/skills/verify-skill/SKILL.md` (use template from Task 1.3)
- [ ] Test hooks (Task 1.5): failing test blocks, passing test shows reminder
- [ ] Document skip mechanism in hooks README or CLAUDE.md

### Phase 2: Documentation Updates (~30 min)
- [ ] Move Common Pitfalls section in CLAUDE.md (line 114 to ~line 63)
- [ ] Add trigger hints to skills table in CLAUDE.md
- [ ] Add one-line cross-reference to 4 existing skills

---

## What NOT To Do

The following were in the original proposal but are CUT:

| Item | Reason |
|------|--------|
| CLAUDE.md restructure | Unnecessary - current structure is fine |
| tdd-skill | Redundant - pre-commit hook enforces tests |
| review-skill | Redundant - superpowers:requesting-code-review exists |
| debug-skill | Documentation, not automation |
| pre-implementation-skill | Bureaucracy - task-plan covers this |
| SessionStart hook | Doesn't exist in Claude Code |
| Idle notification hook | Noise, will be ignored |
| Write/Edit linting hook | Too slow |
| Verbose skill cross-references | Duplicates CLAUDE.md |
| ARCHITECTURE.md updates | Not needed |

---

## Summary

**Original proposal:** 4 phases, ~1 day, 10+ new files
**Revised plan (after Iteration 2):** 2 phases, ~2 hours, 4 new files

| Metric | Original | Iteration 1 | Iteration 2 |
|--------|----------|-------------|-------------|
| Hooks | 5 | 2 | 2 (with improved scripts) |
| New skills | 5 | 1 | 1 |
| CLAUDE.md changes | Full restructure | One section move | One section move |
| Time estimate | 1 day | 3 hours | 2 hours |
| Files created | 10+ | 4 | 4 |

**Key Iteration 2 improvements:**
- Added error handling to PowerShell scripts (path validation, stdin parsing)
- Added hook verification test procedure
- Documented skip mechanism options
- Confirmed all Iteration 1 cuts were correct

The 80/20 rule wins. Ready for implementation.

---

## References

- `_review.md` - Iteration 1 & 2 reviews with full rationale
- `02-hooks-proposal.md` - Original hook specs (historical)
- `03-skills-proposal.md` - Original skill specs (historical, verify-skill only used)
