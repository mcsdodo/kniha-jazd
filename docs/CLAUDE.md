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

## Code in Documentation: What's Allowed

The project convention is "reference code, don't embed it" — but pseudocode and conceptual examples are acceptable.

| Type | Allowed | Example |
|------|---------|---------|
| **Math formulas** | Yes | `consumption = liters / km × 100` |
| **Pseudocode** | Yes | Algorithm steps describing logic flow |
| **Data formats** | Yes | JSON config examples, lock file structure |
| **ASCII diagrams** | Yes | Data flow, state machines |
| **Actual code** | No | Rust functions, TypeScript components |
| **Struct definitions** | No | Full field listings from source |

**Why pseudocode is OK but code isn't:**
- Pseudocode describes *intent*, not implementation
- Actual code will drift when refactored
- Pseudocode remains stable as long as the algorithm is the same

**Good example (pseudocode):**
```
for each trip in chronological order:
    add distance to period total
    if full tank fillup:
        calculate consumption rate
        reset period total
```

**Bad example (actual code):**
```rust
fn calculate_period_rates(trips: &[Trip]) -> Vec<PeriodRate> {
    // This will become stale when impl changes
}
```
