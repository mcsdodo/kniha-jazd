# Contributing

Thanks for your interest in contributing to Kniha Jázd!

## Development Setup

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.77+
- [Docker](https://docs.docker.com/get-docker/) (to build or run the shipped image)
- Chrome (for the integration tests)

#### On Ubuntu or Debian

Development is Linux-first (see `DECISIONS.md` ADR-036). Install the host packages:

```bash
sudo apt install build-essential sqlite3
```

- `build-essential` gives rustc a `cc` linker. Cargo build scripts need it.
- `sqlite3` is for the `query-sqlite-db` skill. The app does not need it.
- Use **Docker Engine** (native). Do not use Docker Desktop. Docker Engine can put a
  container on the host network with `--network=host`. Several integration specs start a
  mock HTTP server in the test process and give the backend a `127.0.0.1` URL. Those
  specs pass only with a host-network container.
- Install Google Chrome for the integration tests.

You do **not** need `libssl-dev` or `pkg-config`. `reqwest` uses rustls, and
`libsqlite3-sys` is `bundled`. There is no `openssl-sys` in `src-tauri/Cargo.lock`.

### Getting Started

```bash
# Clone the repo
git clone https://github.com/mcsdodo/kniha-jazd.git
cd kniha-jazd

# Install dependencies
npm install
```

Development runs as two processes, in two terminals:

```bash
# 1) backend on port 3456 - leave STATIC_DIR unset so vite serves the SPA
cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web

# 2) frontend on port 5173, proxying /api to localhost:3456
npm run dev
```

### Running Tests

```bash
# Rust backend tests
npm run test:backend

# Integration tests (Chrome against the real server). Build both artifacts first:
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration
```

The integration suite starts Chrome **headed**. It needs a display.

- In a desktop session, it works as is.
- Over SSH, no `DISPLAY` is set and Chrome fails to start. Point `DISPLAY` at the
  desktop, for example `export DISPLAY=:10` for an xrdp session. Run `ls /tmp/.X11-unix/`
  to list the displays the machine has.
- With no display at all, use `xvfb-run -a npm run test:integration`.

### Building

The Docker image is the only shipped artifact:

```bash
docker build -f Dockerfile.web -t kniha-jazd-web:local .
```

## Project Structure

- `src/` - SvelteKit frontend (TypeScript)
- `src-tauri/` - Rust workspace (directory name kept for history)
- `src-tauri/core/` - `kniha-jazd-core`: all business logic, the HTTP server, and the tests
- `src-tauri/core/src/calculations/` - Core business logic
- `src-tauri/core/src/suggestions.rs` - Compensation trip suggestions
- `src-tauri/web/` - `kniha-jazd-web`: the headless server binary

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
| `/release` | Bump version, tag, push - CI publishes the ghcr image |

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
