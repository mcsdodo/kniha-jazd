# Standardize Rust-TypeScript Naming Conventions

**Date:** 2026-01-09
**Subject:** Fix inconsistent naming between Rust backend and TypeScript frontend
**Status:** Planning

## Problem

The codebase has **inconsistent naming conventions** at the Rust-TypeScript boundary:

| Layer | Current Convention | Issue |
|-------|-------------------|-------|
| **Rust structs** | `snake_case` | Correct for Rust |
| **TypeScript types** | `snake_case` | **Unusual for JS/TS** |
| **PreviewResult** | `camelCase` | **Inconsistent exception** |
| **api.ts params** | `camelCase` | Mismatches types.ts |
| **Components** | Access `trip.distance_km` | Works but non-idiomatic |

## Root Cause

Rust's default serde serialization produces snake_case JSON. TypeScript types were written to match, but this violates JavaScript conventions.

## Impact

- Confusing for developers (which convention to use?)
- IDE autocomplete shows snake_case in JS context
- PreviewResult inconsistency shows lack of design intent
- Technical debt that makes onboarding harder

## Requirements

1. All JSON serialized from Rust to TypeScript should use camelCase
2. All TypeScript interfaces should use camelCase field names
3. All Svelte component field access should use camelCase
4. ExportLabels can stay snake_case (internal Rust usage only)
5. Database Row structs stay snake_case (Diesel mapping)

## Success Criteria

- [ ] Consistent naming convention across the IPC boundary
- [ ] All tests pass (backend + integration)
- [ ] Manual testing confirms app works correctly
- [ ] No snake_case in TypeScript types (except ExportLabels)
