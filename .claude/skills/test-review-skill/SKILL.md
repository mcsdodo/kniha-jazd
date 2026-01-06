---
name: test-review-skill
description: Use to review test coverage for completeness and quality
---

# Test Review Skill

Iterative review specialized for test coverage analysis.

**Preset Configuration:**
- Focus: Test completeness, edge cases, quality
- Quality Gate: 2 consecutive iterations with no new issues (convergence)
- Reviewer Type: Test Coverage Analyst
- Max Iterations: 4

## When to Use

- After implementing a feature to verify test coverage
- Before release to check critical paths are tested
- When adding tests to existing code
- Periodic test health checks

## Required Information

1. **Target** - code module to check coverage for (e.g., `src-tauri/src/calculations.rs`)
2. **Reference** (optional) - business rules or plan defining what should be tested

## Workflow

### Step 1: Identify Test Files

Find tests for target:
```bash
# Rust
grep -r "test_" src-tauri/src/ --include="*.rs" | grep -i {module_name}

# Or check test modules
cat src-tauri/src/{module}.rs | grep -A5 "#\[cfg(test)\]"
```

### Step 2: Create Progress File

Create `{TARGET_DIR}/_test-review.md`:

```markdown
# Test Coverage Review

**Target:** {TARGET}
**Reference:** {REFERENCE}
**Started:** YYYY-MM-DD
**Focus:** Completeness, edge cases, test quality
```

### Step 3: Execute Review Loop

For each iteration (max 4):

**Spawn Review Agent:**

```
Task tool (general-purpose):
  description: "Test review iteration N"
  prompt: |
    You are a Test Coverage Analyst reviewing tests for {TARGET}.

    Reference (if provided): {REFERENCE}

    Your job:

    1. **Coverage Analysis**
       - What functions/methods exist in {TARGET}?
       - Which have corresponding tests?
       - Which are missing tests?

    2. **Edge Case Analysis**
       - Are boundary conditions tested?
       - Are error paths tested?
       - Are empty/null inputs tested?

    3. **Test Quality**
       - Do tests test actual logic (not mocks)?
       - Are assertions meaningful?
       - Are tests independent?

    4. **Business Logic**
       - Are calculation formulas verified?
       - Are business rules from {REFERENCE} tested?

    Focus on GAPS not on what's already well-tested.
    We don't want filler tests - only meaningful coverage.

    Categorize findings as Critical/Important/Minor.
```

**Apply Fixes:** Add missing Critical/Important tests.

**Run Tests:**
```bash
npm run test:backend
```

**Update Progress:** Append iteration.

**Commit:**
```bash
git add -A
git commit -m "test: add coverage for {TARGET} (iteration N)"
```

**Quality Gate (Convergence):**
- Track issues found per iteration
- Exit when 2 consecutive iterations find no new issues
- OR max iterations reached

### Step 4: Final Assessment

```markdown
## Test Review Complete

**Coverage assessment:** [Sparse/Adequate/Comprehensive]
**Exit reason:** [Convergence/Max iterations]

### Tests Added
[Summary of new tests]

### Remaining Gaps (if any)
[Minor items deferred]
```

## Domain-Specific Checklist

- [ ] Core business logic has tests
- [ ] Edge cases covered (empty, null, boundary)
- [ ] Error paths tested
- [ ] No tests that only test mocks
- [ ] Tests are independent (can run in any order)
- [ ] Tests have meaningful assertions

## Example

```
User: /test-review src-tauri/src/calculations.rs against DECISIONS.md
Claude: [Analyzes test coverage, identifies gaps, adds tests until convergence]
```
