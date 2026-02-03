---
paths:
  - "src/**/*.svelte"
  - "src/**/*.ts"
  - "!src/**/*.test.ts"
---

# Svelte Frontend Rules

## Core Principle: Display Only (ADR-008)

Frontend receives pre-calculated values from Rust backend.
**Never** duplicate calculations in TypeScript.

- Calls Tauri commands, renders results
- Tauri IPC is local/fast - no need for client-side calculations
- All business logic lives in Rust backend

## Adding UI Text

1. Add key to `src/lib/i18n/sk/index.ts` (Slovak primary)
2. Add key to `src/lib/i18n/en/index.ts` (English)
3. Use `{LL.key()}` in Svelte components

## i18n Reminder

- All user-facing strings go through i18n
- Don't forget Slovak UI text - all user-facing strings must be translated
- Use `{LL.key()}` syntax in Svelte components

## Adding a New User Flow

1. Write integration test for the UI interaction (see `.claude/rules/integration-tests.md`)
2. Implement frontend UI (calls existing backend commands)
3. If new backend logic needed, add backend unit tests first (see `.claude/rules/rust-backend.md`)

## Key Files Reference

| File | Purpose | When to Modify |
|------|---------|----------------|
| `+page.svelte` files | Page UI | Visual/interaction changes |
| `src/lib/i18n/sk/index.ts` | Slovak translations | New UI text |
| `src/lib/i18n/en/index.ts` | English translations | New UI text |
| `src/lib/components/` | Reusable UI components | Shared UI elements |
| `src/lib/stores/` | Svelte state management | App-wide state |
