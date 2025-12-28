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
# Rust backend tests (61 tests)
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

## Questions?

Open an issue for discussion before starting major work.
