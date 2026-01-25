# Task: Feature Documentation Review

## Objective

Review all feature documentation in `docs/features/` against the convention in `docs/CLAUDE.md` and identify improvements needed.

## Key Convention Points (from docs/CLAUDE.md)

1. **Reference code, don't embed it** - Use `file.rs:L###` pointers instead of full code blocks
2. **Formulas are OK** - Math formulas are stable and useful
3. **Keep docs maintainable** - Embedded code creates drift risk

## Findings Structure

Each feature has its own file: `{feature-name}.md` with:
- Convention compliance assessment
- Specific issues found
- Recommended changes

## Features to Review

| Feature | File | Status |
|---------|------|--------|
| backup-system | backup-system.md | Pending |
| read-only-mode | read-only-mode.md | Pending |
| trip-grid-calculation | trip-grid-calculation.md | Pending |
| export-system | export-system.md | Pending |
| move-database | move-database.md | Pending |
| receipt-scanning | receipt-scanning.md | Pending |
| settings-architecture | settings-architecture.md | Pending |
| multi-year-state | multi-year-state.md | Pending |

**Note:** `magic-fill.md` already reviewed in conversation - needs refactoring to remove embedded code.
