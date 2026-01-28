# Task: Feature Documentation Review ✅ COMPLETE

## Objective

Review all feature documentation in `docs/features/` against the convention in `docs/CLAUDE.md` and identify improvements needed.

## Key Convention Points (from docs/CLAUDE.md)

1. **Reference code, don't embed it** - Use `file.rs:L###` pointers instead of full code blocks
2. **Formulas are OK** - Math formulas are stable and useful
3. **Pseudocode is OK** - Algorithm steps describing logic flow (not language-specific)
4. **Keep docs maintainable** - Embedded code creates drift risk

## Completed Work

| Feature | File | Status | Changes |
|---------|------|--------|---------|
| trip-grid-calculation | trip-grid-calculation.md | ✅ Already good | None (gold standard) |
| multi-year-state | multi-year-state.md | ✅ Fixed | 12 code blocks → references |
| settings-architecture | settings-architecture.md | ✅ Fixed | 8 code blocks → references |
| move-database | move-database.md | ✅ Fixed | 4 code blocks → references |
| backup-system | backup-system.md | ✅ Fixed | 4 code blocks → references |
| read-only-mode | read-only-mode.md | ✅ Fixed | 2 code blocks → references |
| receipt-scanning | receipt-scanning.md | ✅ Fixed | 2 code blocks → references |
| export-system | export-system.md | ✅ Fixed | 2 code blocks → references |
| magic-fill | magic-fill.md | ✅ Fixed | 3 code blocks → references |

## Convention Update

Updated `docs/CLAUDE.md` with new section "Code in Documentation: What's Allowed" clarifying:
- ✅ Math formulas, pseudocode, data formats, ASCII diagrams
- ❌ Actual code (Rust/TypeScript), struct definitions

## Review Files

Detailed findings for each doc are in this folder:
- `backup-system.md`
- `read-only-mode.md`
- `trip-grid-calculation.md`
- `export-system.md`
- `move-database.md`
- `receipt-scanning.md`
- `settings-architecture.md`
- `multi-year-state.md`
