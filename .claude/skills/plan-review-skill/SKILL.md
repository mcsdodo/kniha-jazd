---
name: plan-review-skill
description: Use to review plans, designs, or specifications for completeness and feasibility
---

# Plan Review Skill

Iterative review specialized for plans, designs, and specifications.

**Preset Configuration:**
- Focus: Plan completeness, feasibility, clarity
- Quality Gate: Early exit (no Critical/Important after 1 iteration)
- Reviewer Type: Plan Auditor
- Max Iterations: 4

## When to Use

- Reviewing implementation plans before coding
- Checking design documents for gaps
- Validating task specifications
- Assessing feasibility of proposed approaches

## Required Information

1. **Target** - path to plan/design document (e.g., `_tasks/15-feature/02-plan.md`)
2. **Reference** (optional) - requirements or constraints to compare against

## Workflow

### Step 1: Read Target Plan

Read the entire plan document to understand scope.

### Step 2: Create/Update Progress File

Create `{TARGET_DIR}/_plan-review.md`:

```markdown
# Plan Review

**Target:** {TARGET}
**Started:** YYYY-MM-DD
**Focus:** Completeness, feasibility, clarity
```

### Step 3: Execute Review Loop

For each iteration (max 4):

**Spawn Review Agent:**

```
Task tool (general-purpose):
  description: "Plan review iteration N"
  prompt: |
    You are a Plan Auditor reviewing {TARGET}.

    Your job is to assess this plan for:

    1. **Completeness**
       - Are all requirements addressed?
       - Are there missing tasks?
       - Are edge cases considered?

    2. **Feasibility**
       - Are the tasks achievable as described?
       - Are there hidden complexities?
       - Are dependencies identified?

    3. **Clarity**
       - Can an implementer follow this without questions?
       - Are file paths specific?
       - Are verification steps included?

    4. **YAGNI/DRY**
       - Is there unnecessary scope?
       - Is there duplication between tasks?

    Categorize findings as Critical/Important/Minor.
    Assess: Is this plan ready for implementation?
```

**Apply Fixes:** Update plan based on findings.

**Update Progress:** Append iteration to `_plan-review.md`.

**Commit:**
```bash
git add {TARGET_DIR}/
git commit -m "review(plan): iteration N for {PLAN_NAME}"
```

**Quality Gate:** Exit if no Critical/Important issues.

### Step 4: Final Assessment

```markdown
## Plan Review Complete

**Ready for implementation?** [Yes/No/With revisions]
**Confidence level:** [Low/Medium/High]

### Recommendations
[Specific next steps]
```

## Domain-Specific Checklist

- [ ] All requirements have corresponding tasks
- [ ] Tasks have specific file paths
- [ ] Verification steps are included
- [ ] Dependencies are in correct order
- [ ] No scope creep beyond requirements
- [ ] Complexity is appropriate (not over-engineered)

## Example

```
User: /plan-review _tasks/20-e2e-testing/02-plan.md
Claude: [Reviews plan for completeness, feasibility, clarity]
```
