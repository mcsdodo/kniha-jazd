# Review Base Template

This is a shared template for iterative review skills. Individual skills import this and override parameters.

## Shared Components

### Output Format

```
## Iteration N

**Date:** YYYY-MM-DD
**Reviewer:** {REVIEWER_TYPE}

### Critical Issues
[List or "None found"]

### Important Issues
[List or "None found"]

### Minor Issues
[List - may defer]

### Changes Made
- [Bullet list of fixes]

### Assessment
**Quality:** [Poor/Acceptable/Good/Excellent]
**Continue?** [Yes/No - quality gate met]
```

### Review Agent Prompt Template

```
You are a {REVIEWER_TYPE} reviewing {TARGET}.

Focus: {FOCUS}
Reference: {REFERENCE}
Previous findings: {PRIOR_SUMMARY}

Your job:
1. Review everything fresh (don't trust prior reports)
2. Find NEW gaps not caught previously
3. Verify previous Critical/Important issues were fixed
4. Categorize findings:
   - Critical: Must fix (bugs, security, data loss)
   - Important: Should fix (missing features, poor handling)
   - Minor: Nice to have (style, docs)
5. Assess: Quality gate met?

{DOMAIN_SPECIFIC_INSTRUCTIONS}

Return structured findings using the output format.
```

### Anti-Patterns (All Reviews)

- Marking nitpicks as Critical
- Suggesting changes outside focus area
- Over-engineering beyond spec
- Trusting prior reports without verification
- Stopping early without quality gate check
