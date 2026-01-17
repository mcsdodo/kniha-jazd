# Iterative Review Workflow Analysis

**Date:** 2026-01-06
**Subject:** Best practices for AI-powered iterative review workflows
**Status:** Complete (Iteration 3)

---

## Quick Start: Copy-Paste Template

For immediate use, here's an enhanced version of your `_review_prompt.md`:

```markdown
# Iterative Review: {TASK_NAME}

**Target:** {TASK_FOLDER}
**Focus:** {FOCUS_AREA - e.g., "test completeness", "security", "performance"}
**Max iterations:** 4

## Instructions

Iterate up to 4 times (or until quality gate met):

1. **Spawn review agent** - Review {TASK_FOLDER}, find gaps, critically assess
2. **Categorize findings** by severity:
   - **Critical:** Must fix (bugs, security, data loss risks)
   - **Important:** Should fix (missing features, poor error handling)
   - **Minor:** Nice to have (style, optimization)
3. **Apply fixes** for Critical/Important issues
4. **Update `_review.md`** with iteration section (use format below)
5. **Commit changes** after each iteration
6. **Re-read everything** before next iteration

**Quality gate (early exit):** Stop if no Critical or Important issues found.

**Focus constraint:** We don't aim for overengineering but for {FOCUS_AREA}.

## Output Format Per Iteration

```
## Iteration N
**Date:** YYYY-MM-DD
**Focus:** {FOCUS_AREA}

### Critical Issues
[List or "None found"]

### Important Issues
[List or "None found"]

### Minor Issues
[List - may defer]

### Changes Made
- [Bullet list of fixes applied]

### Assessment
Quality: [Poor/Acceptable/Good/Excellent]
Continue? [Yes/No - quality gate met]
```

Repeat after me if you understand before executing.
```

**Why "Repeat after me"?** This confirmation step ensures the agent:
1. Has fully parsed the template (not just skimmed it)
2. Commits to the structured output format
3. Acknowledges the quality gate before starting

---

## Table of Contents

1. [Quick Start Template](#quick-start-copy-paste-template)
2. [Current State Analysis](#1-current-state-your-_review_promptmd)
3. [Research Findings](#2-research-findings)
4. [Synthesized Best Practices](#3-synthesized-best-practices)
5. [Recommended Prompt Template](#4-recommended-prompt-template)
6. [Comparison](#5-comparison-current-vs-recommended)
7. [Implementation Options](#6-implementation-options)
8. [Sources](#7-sources)
9. [Next Steps](#8-next-steps)
10. Appendices A-D

---

## Executive Summary

This analysis examines best practices for implementing iterative review workflows in AI coding assistants, specifically for use cases like the `_review_prompt.md` pattern. The research synthesizes findings from industry leaders (Anthropic, Google, Addy Osmani), academic research (Self-Refine), and practical implementations (superpowers skills, awesome-reviewers).

---

## 1. Current State: Your `_review_prompt.md`

```markdown
iterate following 4 times: spawn a review agent, look at WHATEVER_TASK_FOLDER -
review, find gaps, suggest improvements, critically assess. We don't aim for
overengineering but for WHATEVER_IS_THE_FOCUS_PLACEHOLDER. After each iteration
update _review.md file with iteration, changes made - re-read everything and
iterate over.
```

**Strengths:**
- Fixed iteration count (4) - prevents infinite loops
- Progress tracking via `_review.md`
- Focus constraint (WHATEVER_IS_THE_FOCUS_PLACEHOLDER)
- Cumulative context (re-read everything)

**Gaps Identified:**
- No stopping criteria if issues resolved early
- No severity categorization of findings
- No structured output format
- Missing explicit reviewer role definition
- No quality gate validation

---

## 2. Research Findings

### 2.1 Core Patterns

#### Self-Refine Pattern (Academic)
**Source:** [Self-Refine: Iterative Refinement with Self-Feedback](https://learnprompting.org/docs/advanced/self_criticism/self_refine)

Three-step cycle:
1. **Generate** initial output
2. **Feedback** - model critiques its own output
3. **Refine** - apply feedback, repeat

**Key insight:** GPT-4 shows +8.7 units improvement for code optimization, +13.9 units for code readability when using Self-Refine.

**Stopping condition:** Model states "no further improvements possible"

#### AgentCoder Research (Iteration Optimization)
**Source:** [AgentCoder: Multiagent-Code Generation with Iterative Testing](https://arxiv.org/html/2312.13010v2)

Empirical data on optimal iteration counts:

| Iterations | HumanEval Pass@1 | Improvement |
|------------|------------------|-------------|
| 1 | 74.4% | baseline |
| 3 | 77.8% | +3.4% |
| 5 | 79.9% | +5.5% |

**Key finding:** Pass rates increase from 74.4% to 79.9% with 5 iterations - but most gains come in first 3-4 iterations. Diminishing returns after iteration 5.

#### Reflection Pattern (ByteByteGo)
**Source:** [Top AI Agentic Workflow Patterns](https://blog.bytebytego.com/p/top-ai-agentic-workflow-patterns)

- Agent generates output, enters "critique mode"
- Examines problems, inconsistencies, opportunities
- Produces improved version addressing identified issues
- Cycle repeats until quality threshold met

**When to use:** Tasks prioritizing quality over speed with subjective elements.

#### Multi-Pass Code Review (Greptile)
**Source:** [AI Code Reviews: The Ultimate Guide](https://www.greptile.com/what-is-ai-code-review)

- Beyond-the-diff checks with iterative passes
- First pass: integration risks, cross-layer issues
- Second pass: rescan expanded diff, summarize changes
- Impact-ranked summary for human reviewers

### 2.2 Industry Best Practices

#### Anthropic's Approach
**Source:** [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices)

- "Claude's outputs tend to improve significantly with iteration. While the first version might be good, after 2-3 iterations it will typically look much better."
- Separate agent reviews: "have one Claude write code while another reviews or tests it"
- Use `/clear` between tasks to prevent context bloat

#### Addy Osmani's Small Loops
**Source:** [My LLM Coding Workflow Going into 2026](https://addyosmani.com/blog/ai-coding-workflow/)

- Avoid "huge leaps" - iterate in small loops
- Each iteration: Define task > Generate > Test > Review > Commit
- Treat AI output "as if it came from a junior developer"
- Cross-model review: "have Claude write code, ask Gemini to review"

#### Quality Gates as Coaching Signals
**Source:** [10 Best Practices for Building Reliable AI Agents](https://www.uipath.com/blog/ai/agent-builder-best-practices)

- Validation tools (SonarQube, ESLint, tests) provide automatic feedback
- "Tight loop where validation tools become coaching signals"
- Gate production release: move agents to production only after evaluations pass

### 2.3 Existing Skill Patterns

#### Superpowers Code Reviewer
**Location:** `.claude/plugins/cache/superpowers-marketplace/.../code-reviewer.md`

**Structure:**
```markdown
## Review Checklist
- Code Quality (separation of concerns, error handling, DRY, edge cases)
- Architecture (design decisions, scalability, security)
- Testing (tests test logic not mocks, edge cases covered)
- Requirements (plan requirements met, no scope creep)
- Production Readiness (migration strategy, backward compatibility)

## Output Format
### Strengths
### Issues
#### Critical (Must Fix)
#### Important (Should Fix)
#### Minor (Nice to Have)
### Recommendations
### Assessment
**Ready to merge?** [Yes/No/With fixes]
```

**Key insight:** Severity categorization prevents treating everything as critical.

#### Spec Compliance Reviewer
**Location:** `.../spec-reviewer-prompt.md`

**Critical principle:**
> "The implementer finished suspiciously quickly. Their report may be incomplete, inaccurate, or optimistic. You MUST verify everything independently."

**Verification areas:**
- Missing requirements
- Extra/unneeded work
- Misunderstandings

---

## 3. Synthesized Best Practices

### 3.1 Iteration Structure

| Aspect | Recommendation | Rationale |
|--------|----------------|-----------|
| **Iteration count** | 3-5 (configurable) | Research shows 3-5 iterations optimal; diminishing returns after |
| **Stopping criteria** | Explicit quality gate OR "no further improvements" statement | Prevents wasted iterations |
| **Progress tracking** | Cumulative document with iteration headers | Maintains context, shows evolution |
| **Focus constraint** | Single focus per review cycle | Prevents scope creep in review |

### 3.2 Reviewer Role Definition

```markdown
You are a [Senior Code Reviewer / Plan Auditor / Architecture Critic].

Your role is to:
1. Review {TARGET} against {REFERENCE/STANDARD}
2. Find gaps, inconsistencies, risks
3. Categorize findings by severity
4. Suggest concrete improvements
5. Assess overall quality

You are NOT aiming for overengineering. Focus on: {SPECIFIC_FOCUS}
```

### 3.3 Output Format (Standardized)

```markdown
## Iteration N

**Date:** YYYY-MM-DD
**Reviewer:** [Agent type]
**Focus:** [What we're looking for]

### Findings

#### Critical (Must Fix)
[Bugs, security issues, data loss risks, broken functionality]

#### Important (Should Fix)
[Architecture problems, missing features, poor error handling]

#### Minor (Nice to Have)
[Style, optimization, documentation improvements]

### Edge Cases Identified
[Specific scenarios not covered]

### Suggestions
| Priority | Suggestion | Rationale |
|----------|------------|-----------|

### Assessment
**Quality Level:** [Poor / Acceptable / Good / Excellent]
**Continue iteration?** [Yes - issues remain / No - quality gate met]

### Changes Made This Iteration
- [List of actual changes applied]
```

### 3.4 Quality Gates

**Option A: Fixed iteration with early exit**
```
Iterate N times OR until reviewer states "no critical/important issues remain"
```

**Option B: Convergence-based**
```
Continue until:
- No new critical/important issues found in 2 consecutive iterations
- OR maximum iterations reached
```

**Option C: Validation-based**
```
Continue until:
- Tests pass
- Linter clean
- Reviewer assessment = "Ready to merge"
```

**Which quality gate to use?**

| Use Case | Recommended Gate | Why |
|----------|------------------|-----|
| Plan/design review | **Option A** (early exit) | No automated validation possible |
| Code review | **Option C** (validation) | Tests provide objective feedback |
| Test coverage review | **Option B** (convergence) | Subjective, needs multiple passes |
| Security review | **Option A** with min 2 iterations | Critical findings need fresh eyes |

### 3.5 Multi-Pass Strategy

For comprehensive reviews, use specialized passes:

| Pass | Focus | Reviewer Type |
|------|-------|---------------|
| 1 | Spec compliance | "Did we build what was requested?" |
| 2 | Code quality | "Is it well-built?" |
| 3 | Architecture | "Is it well-designed?" |
| 4 | Security/Edge cases | "What could go wrong?" |

---

## 4. Recommended Prompt Template

### Generic Iterative Review Skill

```markdown
---
name: iterative-review
description: Multi-iteration review with quality gates
---

# Iterative Review Workflow

## Parameters
- **TARGET:** {TASK_FOLDER or CODE_PATH}
- **REFERENCE:** {PLAN, SPEC, or STANDARD to compare against}
- **FOCUS:** {SPECIFIC_FOCUS - what we're optimizing for}
- **MAX_ITERATIONS:** {3-5, default 4}
- **STOPPING_CRITERIA:** {quality gate definition}

## Process

### Pre-Review
1. Read {TARGET} completely
2. Read {REFERENCE} if provided
3. Understand the {FOCUS} constraint

### Each Iteration

**Step 1: Spawn Review Agent**
```
Task tool (general-purpose or superpowers:code-reviewer):
  description: "Review iteration N for {TARGET}"
  prompt: |
    You are reviewing {TARGET} for iteration N of N.

    Focus: {FOCUS}
    Reference: {REFERENCE}

    Previous iterations found: [summary of prior findings]

    Your job:
    1. Review everything fresh (don't trust prior reports blindly)
    2. Find NEW gaps not caught in previous iterations
    3. Verify previous issues were actually fixed
    4. Categorize by severity (Critical/Important/Minor)
    5. Assess: Continue reviewing or quality gate met?
```

**Step 2: Apply Changes**
- Fix Critical issues immediately
- Address Important issues
- Note Minor for later

**Step 3: Update Progress File**
- Append iteration section to `_review.md`
- Include: findings, changes made, assessment

**Step 4: Evaluate Stopping Criteria**
- If quality gate met: Exit loop
- If max iterations reached: Summarize remaining issues
- Otherwise: Continue to next iteration

### Post-Review
1. Final summary of all iterations
2. Remaining issues (if any)
3. Commit changes with descriptive message

## Quality Gates

Choose one:
- [ ] **Fixed iterations:** Complete all N iterations
- [ ] **Early exit:** Stop when "no critical/important issues" for 1 iteration
- [ ] **Convergence:** Stop when 2 consecutive iterations find no new issues
- [ ] **Validation:** Stop when tests pass + linter clean + assessment positive

## Anti-Patterns to Avoid

- Marking nitpicks as Critical
- Suggesting changes outside {FOCUS}
- Over-engineering beyond spec
- Trusting implementer reports without verification
- Stopping early without explicit quality gate check
```

---

## 5. Comparison: Current vs Recommended

| Aspect | Current (`_review_prompt.md`) | Recommended |
|--------|-------------------------------|-------------|
| Iteration count | Fixed (4) | Configurable (3-5) with early exit |
| Stopping criteria | None (always 4) | Quality gate or convergence |
| Output format | Unstructured | Severity-categorized |
| Role definition | Implicit | Explicit reviewer persona |
| Focus constraint | Placeholder | Required parameter |
| Progress tracking | `_review.md` update | Structured iteration sections |
| Verification | Not specified | "Don't trust reports" principle |
| Commits | After each iteration | After each iteration (good!) |

---

## 6. Implementation Options

### Option A: Enhanced `_review_prompt.md`
Simplest - improve existing pattern with structured output and stopping criteria.

**Pros:** Minimal setup, use immediately, no skill infrastructure needed
**Cons:** Copy-paste each time, no standardization across projects

### Option B: Dedicated Skill
Create `.claude/skills/iterative-review/SKILL.md` with full template.

**Pros:** Reusable, `/iterative-review` invocation, version controlled
**Cons:** More setup, single template may not fit all use cases

### Option C: Parameterized Skill
Create skill that accepts parameters for different review types:
- `plan-review` - reviewing plans/designs
- `code-review` - reviewing implementations
- `test-review` - reviewing test coverage

**Pros:** Flexible, specialized per domain, one skill many uses
**Cons:** More complex, parameter parsing required

### Decision Matrix

| Scenario | Recommended | Rationale |
|----------|-------------|-----------|
| Quick one-off review | **Option A** | Just use Quick Start template |
| Regular project reviews | **Option B** | Consistency across sessions |
| Multi-project standard | **Option C** | Parameterize for flexibility |
| Learning/experimenting | **Option A** | Iterate on template first |

### Recommended Path

1. **Start with Option A** (Quick Start template) - validate the workflow
2. **Graduate to Option B** once pattern is proven - create skill
3. **Evolve to Option C** if multiple specialized variants needed

---

## 7. Sources

### Primary Sources
- [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices) - Anthropic Engineering
- [Self-Refine: Iterative Refinement with Self-Feedback](https://learnprompting.org/docs/advanced/self_criticism/self_refine) - Learn Prompting
- [Top AI Agentic Workflow Patterns](https://blog.bytebytego.com/p/top-ai-agentic-workflow-patterns) - ByteByteGo
- [My LLM Coding Workflow Going into 2026](https://addyosmani.com/blog/ai-coding-workflow/) - Addy Osmani

### Secondary Sources
- [Effective Prompt Engineering for AI Code Reviews](https://graphite.com/guides/effective-prompt-engineering-ai-code-reviews) - Graphite
- [AI Prompts for Code Review](https://5ly.co/blog/ai-prompts-for-code-review/) - 5ly Blog
- [Awesome Reviewers](https://github.com/baz-scm/awesome-reviewers) - BAZ (3000+ curated prompts)
- [CLAUDE.md Best Practices](https://arize.com/blog/claude-md-best-practices-learned-from-optimizing-claude-code-with-prompt-learning/) - Arize AI
- [Skill Authoring Best Practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices) - Claude Docs

### Existing Patterns Analyzed
- `superpowers:code-reviewer` skill
- `superpowers:requesting-code-review` skill
- `spec-reviewer-prompt.md`
- `code-quality-reviewer-prompt.md`

---

## 8. Next Steps

1. **Decide implementation approach** (Option A/B/C)
2. **Implement chosen approach**
3. **Test with real review scenario**
4. **Iterate on skill based on results**

---

## Appendix A: Worked Example

Here's how the enhanced template was used for task-22 (test completeness review):

**Original prompt:**
```markdown
# Iterative Review: Test Completeness

**Target:** _tasks/22-test-completeness/
**Focus:** test completeness - ensuring all business logic has test coverage
**Max iterations:** 4
```

**Result after 2 iterations:**

| Iteration | Critical | Important | Minor | Changes Made |
|-----------|----------|-----------|-------|--------------|
| 1 | 1 (year carryover untested) | 2 (integration tests, edge cases) | 3 | Analysis documented |
| 2 | 0 | 0 | 1 | Added 3 year carryover tests |

**Early exit at iteration 2:** Quality gate met - no Critical or Important issues remaining.

**Key takeaway:** The structured severity categorization allowed early termination, saving 2 iterations while ensuring quality.

---

## Appendix B: Existing Skills That Could Be Leveraged

| Skill | Purpose | Use Case |
|-------|---------|----------|
| `superpowers:requesting-code-review` | Code review after implementation | Post-code review |
| `superpowers:code-reviewer` (agent) | Detailed code quality review | Code quality pass |
| `superpowers:receiving-code-review` | Process incoming review feedback | Review response |
| `verify-skill` | Pre-completion verification | Quality gate check |
| `superpowers:brainstorming` | Design exploration | Plan review preparation |

---

## Appendix C: Cross-Model Review Strategy

**Source:** [Addy Osmani's LLM Workflow](https://addyosmani.com/blog/ai-coding-workflow/)

For critical reviews, consider cross-model validation:

```
1. Claude writes/reviews code
2. Gemini reviews Claude's output: "Can you review this for errors or improvements?"
3. Claude incorporates Gemini feedback
```

**Why it works:** Different models have different blind spots. Cross-model review catches subtle issues that single-model review misses.

**When to use:** Production-critical code, security-sensitive changes, complex algorithms.

---

## Appendix D: Anti-Patterns from Research

| Anti-Pattern | Why It's Bad | Better Approach |
|--------------|--------------|-----------------|
| Infinite iteration | Wasted tokens, no convergence | Fixed max + early exit |
| No severity levels | Everything feels equally urgent | Critical/Important/Minor |
| Trusting reports | Implementers are optimistic | Independent verification |
| Vague feedback | "Improve error handling" | Specific file:line + fix |
| Scope creep | Review becomes refactoring | Focus constraint |
| No progress tracking | Lost context between iterations | Cumulative `_review.md` |
