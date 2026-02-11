# Task: Split commands.rs into Feature Modules

## Problem
`commands.rs` has grown to 3,908 lines with 68 Tauri commands. While internally organized with section comments, the file size makes navigation and maintenance difficult.

## Goal
Split into 9 feature-based modules under `src-tauri/src/commands/` while maintaining all existing functionality and tests.

## Success Criteria
- [ ] All 229 backend tests pass
- [ ] All 61 integration tests pass
- [ ] No functional changes to any commands
- [ ] Clear module boundaries with explicit dependencies
- [ ] lib.rs updated with new import structure

## Reference
- ADR-011 in DECISIONS.md documents this decision
- Analysis performed on 2026-01-29 identified the module structure
