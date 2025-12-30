# Contributing

Thanks for your interest in contributing to Kniha Jázd!

## Development Setup

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.77+
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Getting Started

```bash
# Clone the repo
git clone https://github.com/mcsdodo/kniha-jazd.git
cd kniha-jazd

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

### Running Tests

```bash
# Rust backend tests (72 tests)
cd src-tauri && cargo test
```

## Project Structure

- `src/` - SvelteKit frontend (TypeScript)
- `src-tauri/` - Tauri backend (Rust)
- `src-tauri/src/calculations.rs` - Core business logic
- `src-tauri/src/suggestions.rs` - Compensation trip suggestions

## Code Guidelines

- **Code language:** English (variables, comments, commits)
- **UI language:** Slovak
- **Test-driven:** Write failing test first, then implementation
- All calculations happen in Rust backend (see `DECISIONS.md` ADR-008)

## Making Changes

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Write tests for your changes
4. Make your changes
5. Run tests (`cargo test`)
6. Commit with descriptive message
7. Push and create a Pull Request

## Commit Messages

Use conventional commits:
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `refactor:` code refactoring
- `test:` adding tests

## Claude Code Setup

This project includes custom skills and slash commands for [Claude Code](https://claude.com/claude-code).

### Slash Commands

| Command | Purpose |
|---------|---------|
| `/task-plan` | Create planning folder in `_tasks/` with brainstorming |
| `/decision` | Add ADR/BIZ entry to `DECISIONS.md` |
| `/changelog` | Update `CHANGELOG.md` [Unreleased] section |
| `/release` | Bump version, tag, push, build |

### Directory Structure

```
.claude/
├── commands/           # User-invocable slash commands
│   ├── task-plan.md
│   ├── decision.md
│   ├── changelog.md
│   └── release.md
└── skills/             # Auto-invoked by Claude based on context
    ├── task-plan-skill/SKILL.md
    ├── decision-skill/SKILL.md
    ├── changelog-skill/SKILL.md
    └── release-skill/SKILL.md
```

### How It Works

- **Commands** (`/command`) - Manual invocation by typing in Claude Code
- **Skills** - Claude auto-invokes based on task context and description

Skills use `-skill` suffix due to [a bug](https://github.com/anthropics/claude-code/issues/14945) where same-name skill/command conflicts. See `_tasks/_TECH_DEBT/01-skill-command-name-conflict.md`.

### Key Files

- `CLAUDE.md` - Project instructions loaded every conversation
- `DECISIONS.md` - Architecture Decision Records (ADRs)
- `_tasks/` - Feature planning and implementation docs

## Questions?

Open an issue for discussion before starting major work.
