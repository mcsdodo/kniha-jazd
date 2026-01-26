# Task 38: Magic String Constants Refactoring

## Summary

Replace hardcoded magic string constants throughout the codebase with properly typed enums (Rust) and const objects (TypeScript) to improve type safety, maintainability, and IDE support.

## Problem Statement

The codebase contains numerous magic string literals used for:
- Domain values (vehicle types, receipt statuses, confidence levels)
- UI state (filters, sort directions, toast types)
- Configuration (themes, locales, backup types)
- Event names and keys

These scattered strings are:
1. Error-prone (typos cause runtime bugs)
2. Hard to refactor (find-and-replace is risky)
3. Missing IDE autocomplete support
4. Not self-documenting

## Scope

**In Scope:**
- Rust: Convert string constants to enums where appropriate
- TypeScript: Convert string literals to const objects with `as const`
- Update all usages to reference the new constants
- Ensure type safety with proper TypeScript types

**Out of Scope:**
- Tauri command names (already wrapped in api.ts)
- SQL query strings (necessary for raw queries)
- Error messages (dynamic content)
- Test data strings (only test infrastructure)

## Success Criteria

- [ ] All identified magic strings converted to constants/enums
- [ ] No string literal comparisons for domain values
- [ ] Full TypeScript type inference for all const objects
- [ ] All tests pass after refactoring
- [ ] No runtime behavior changes

## Analysis Summary

**4 iterations of analysis completed:**
- Iteration 1: Core domain values (vehicle types, statuses, etc.)
- Iteration 2: Infrastructure strings (file extensions, formats, etc.)
- Iteration 3: Edge cases (events, keyboard keys, etc.)
- Iteration 4: Final sweep (API fields, storage keys, etc.)

See `03-plan.md` for detailed findings and implementation plan.
