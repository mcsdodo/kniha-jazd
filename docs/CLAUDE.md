# docs/CLAUDE.md

This folder contains **Feature Documentation** — technical walkthroughs that document how features work end-to-end.

## Convention: Feature Documentation

**What it is:** A hybrid between use-case documentation and technical design docs. Each feature doc covers:
- **User-facing flow** — What the user sees and does
- **Technical implementation** — Backend steps, data flow, file locations
- **Design rationale** — Why it works this way (links to ADRs if applicable)

**When to create:**
- After completing a planned feature (from `_tasks/`)
- When documenting existing complex features for onboarding
- When a feature spans multiple files/modules and needs a unified explanation

**Naming convention:** `{feature-name}.md` (kebab-case, descriptive)

Examples:
- `move-database.md` — Database relocation + multi-PC support
- `receipt-scanning.md` — AI-powered receipt OCR flow
- `consumption-calculation.md` — Core business logic walkthrough

## Template

```markdown
# Feature: {Feature Name}

> One-line description of what this feature does for users.

## User Flow

1. Step user takes
2. What they see
3. Result

## Technical Implementation

### Frontend
- Component location: `src/routes/...`
- Key functions: `handleX()`, `submitY()`

### Backend (Rust)
- Command: `command_name` in `commands.rs:L###`
- Core logic: `module.rs` functions

### Data Flow
```
User Action → Frontend → Tauri IPC → Backend → Database
                                   ↓
                              Response → Frontend → UI Update
```

## Key Files

| File | Purpose |
|------|---------|
| `path/to/file.rs` | Description |

## Design Decisions

- **Why X?** — Explanation (see ADR-### if applicable)
- **Why not Y?** — Trade-off reasoning

## Related

- ADR-###: Related architectural decision
- `_tasks/##-feature/`: Original planning docs
```

## Maintenance

Feature docs should be updated when:
- Implementation changes significantly
- New edge cases are discovered
- Related ADRs are added

Keep docs accurate — outdated documentation is worse than no documentation.
