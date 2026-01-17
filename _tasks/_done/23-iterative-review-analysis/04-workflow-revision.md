# Workflow Revision: Two-Phase Review

**Date:** 2025-01-06
**Status:** Implemented

## Original Design Issue

The original skills (based on Option C from `03-plan-option-c.md`) conflated two activities:
1. **Review/Analysis** - Finding issues
2. **Implementation** - Fixing issues

The skills had an "Apply Fixes" step that modified source files automatically during iteration.

## Problem

This approach:
- Removed user oversight (changes happened without approval)
- Hid reasoning (user didn't see findings before they were "fixed")
- Couldn't handle judgement calls ("Is this over-engineered?" needs human decision)
- Lost the review artifact (no record of what was analyzed)

## Revised Workflow

All review skills now follow a **two-phase approach**:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Phase 1        │────▶│  User Decision  │────▶│  Phase 2        │
│  Review         │     │  Point          │     │  Apply          │
│                 │     │                 │     │                 │
│ AI finds issues │     │ User approves/  │     │ AI implements   │
│ Documents them  │     │ rejects/modifies│     │ approved fixes  │
│ Commits review  │     │                 │     │ User verifies   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### Phase 1: Review (Findings Only)
- Iterate on the review document (`_plan-review.md`, `_code-review.md`, `_test-review.md`)
- **Commit only the review file** after each iteration
- Quality gate: No new findings for 1 iteration (review is comprehensive)
- **STOP and present to user** for approval

### Phase 2: Apply Approved Changes
- Only proceed after user direction
- Apply approved findings to source files
- Track resolution in review document
- Commit changes

## Key Changes to Skills

| Skill | Original | Revised |
|-------|----------|---------|
| plan-review | Modify plan during iteration | Document findings → User approves → Then modify |
| code-review | Fix code during iteration | Document findings → User approves → Then fix |
| test-review | Add tests during iteration | Document gaps → User approves → Then add tests |

## Rationale

This mirrors professional review processes:
- **Code review:** Reviewer comments → Author decides what to change
- **Editorial review:** Editor marks issues → Writer accepts/rejects
- **Audit:** Auditor reports findings → Management decides remediation

The reviewer's job is to **produce comprehensive analysis**. The author's job is to **decide what to do with it**.

## Files Modified

- `.claude/skills/plan-review-skill/SKILL.md`
- `.claude/skills/code-review-skill/SKILL.md`
- `.claude/skills/test-review-skill/SKILL.md`
