**Date:** 2026-09-07
**Subject:** Move day-to-day development from the Windows box to an Ubuntu VM
**Status:** Complete (2026-09-07)

## Goal

Make a fresh `git clone` on an Ubuntu VM a complete development environment — build,
full test suite, Docker image, and the Claude Code hooks — with no PowerShell
dependency and no silently-different behaviour.

## Background

[Task 73](../_done/73-web-first-migration/) dropped the desktop app. The Docker image is
now the only shipped artifact, and [test.yml](../../.github/workflows/test.yml) already
runs the entire integration suite on `ubuntu-latest`. A survey of the tree found the
**product code is already platform-clean**:

- One platform conditional in the whole Rust workspace:
  [src-tauri/web/src/main.rs:100](../../src-tauri/web/src/main.rs), the `#[cfg(unix)]`
  SIGTERM/SIGINT handler. Linux takes the better branch; `#[cfg(not(unix))]` is the
  degraded Ctrl+C-only fallback.
- No `.exe`, `%APPDATA%` or drive-letter paths in `core` or `web`. The only `C:\...` in
  Rust is a string literal at
  [src-tauri/core/src/db_tests.rs:547](../../src-tauri/core/src/db_tests.rs) — a receipt
  path stored as text, never opened.
- No Tauri left in `Cargo.lock`. The single "tauri" hit
  ([dispatcher.rs:462](../../src-tauri/core/src/server/dispatcher.rs)) is a comment
  about the manifest path.
- `libsqlite3-sys` is `bundled` and `reqwest` is `rustls-tls` with
  `default-features = false` (see [core/Cargo.toml](../../src-tauri/core/Cargo.toml)),
  and [Cargo.lock](../../src-tauri/Cargo.lock) contains no `openssl-sys`. The host needs
  a C toolchain and nothing else.
- [README.md](../../README.md), [CONTRIBUTING.md](../../CONTRIBUTING.md) and
  [docs/features/](../../docs/features/) are already written in POSIX shell.

What is Windows-bound is the **dev harness only** — and the worst of it fails
*silently*, which is why this is a task rather than a checklist.

## Requirements

1. **The five `test:integration:*` npm scripts must export their env vars under `sh`.**
   They currently use cmd.exe `set X=Y&&`. npm runs scripts through `sh` on Linux, where
   `set TIER=1` sets positional parameters and exports nothing. `getSpecs()`
   ([wdio.server.conf.ts:64](../../tests/integration/wdio.server.conf.ts)) then falls
   through to all four tier globs, and `test:integration:docker` spawns a local binary
   on port 3457 instead of driving the container on 3456. No error is raised — the wrong
   thing just happens. [test.yml](../../.github/workflows/test.yml) already documents
   this and works around it with `env:` blocks.
2. **No `pwsh` dependency.** [.claude/settings.json](../../.claude/settings.json) runs
   `pwsh -NoProfile -File .claude/hooks/pre-commit.ps1` as a committed PreToolUse hook.
3. **The sqlite skill must name the real DB path.**
   [query-sqlite-db/SKILL.md](../../.claude/skills/query-sqlite-db/SKILL.md) still
   documents `%AppData%/com.notavailable.kniha-jazd.dev/kniha-jazd.db` — the dead Tauri
   location, wrong on *every* platform since Task 73.
4. **[local.settings.json.sample](../../local.settings.json.sample) shows a path a Linux
   developer can use.** It currently offers a `C:\Users\YourUsername\...` example.
5. **Line endings are pinned by the repo**, not by each developer's `core.autocrlf`.
6. **CONTRIBUTING.md names the Ubuntu prerequisites** so the next person does not
   rediscover them.
7. **Everything above keeps working on Windows.** The `windows-latest` backend leg of
   the CI matrix is unchanged, and a Windows contributor must still be able to run the
   suite. This repo is public.
8. **Acceptance is a real run on the VM**, ending with `paperless-integration.spec.ts`
   passing in Docker mode — the spec that structurally *cannot* pass on Docker Desktop
   for Windows.

## Decisions Taken

| Question | Decision | Why |
|----------|----------|-----|
| How to fix the npm env-var scripts | `cross-env` devDependency | Satisfies requirement 7. Plain `TIER=1 npm run …` is POSIX-clean but breaks cmd.exe, and this is a public repo with a Windows CI leg. One small dev-only dependency. |
| Keep the `env:` blocks in test.yml | Yes | They are explicit and CI-correct either way. Only the *comment* justifying them goes stale and must be rewritten — it currently claims the block is load-bearing because of Windows `set`. |
| Install pwsh on Ubuntu vs. port the hook | Port to bash | Claude Code's `settings.json` has no per-platform hook selection, and the sibling PostToolUse hook already invokes `bash`. git-bash covers Windows, so bash is the single form that works everywhere. Installing PowerShell 7 on the VM would keep two implementations alive for no gain. |
| JSON parsing in the bash hook | grep the raw payload | [post-commit-reminder.sh](../../.claude/hooks/post-commit-reminder.sh) already does exactly this. Adding a `jq` dependency to match one field would be new setup on every machine. |
| `.gitattributes` scope | `* text=auto eol=lf`, gated on producing zero diff | 340 tracked files are already LF-in-index / CRLF-in-worktree, so normalising costs nothing and silences the `LF will be replaced by CRLF` warnings. If `git add --renormalize .` *does* produce a diff, that is a separate call — see plan Task 5. |
| Chrome headless | Not changed | [wdio.server.conf.ts:194-197](../../tests/integration/wdio.server.conf.ts) sets only `--no-sandbox --disable-gpu`. Adding `--headless` would change the thing local runs and CI have in common, for no benefit here. **Corrected 2026-09-07:** the original reason given here - "the VM has a desktop session, so headed Chrome works" - is wrong. Headed Chrome on the unattended xrdp desktop fails two tier2 specs every time. The decision stands, but the run command is `xvfb-run`, not the desktop. See the Display note below. |
| ADR in DECISIONS.md | Yes, one short entry | "Dev environment is Linux-first; committed hooks are bash" is a convention that constrains future contributions. Without it, the next `.ps1` hook is a reasonable-looking addition. |

## Technical Notes

- **Carry the in-flight work across first.** At writing time the tree holds uncommitted
  [Task 75](../75-place-book/) work: modified
  [db.rs](../../src-tauri/core/src/db.rs),
  [db_tests.rs](../../src-tauri/core/src/db_tests.rs),
  [commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs) and the
  [2026-09-07-100000_add_places](../../src-tauri/core/migrations/2026-09-07-100000_add_places/)
  migration, plus **untracked** `commands_internal/places_cmd.rs`,
  `commands_internal/places_cmd_tests.rs` and `core/src/places/`. Untracked files do not
  travel with a clone — commit them or push a WIP branch before the machine switch, or
  that work is stranded on the Windows box.
- **Gitignored local config must be recreated by hand on the VM:** `.env` (Paperless
  token) and `local.settings.json` (Gemini key, `receipts_folder_path`) — neither is in
  the repo, by design; the committed template is
  [local.settings.json.sample](../../local.settings.json.sample).
  `receipts_folder_path` is an `Option<String>` stored as plain
  text and never resolved against the filesystem at load, so a Linux path — or no value
  at all — is fine.
- **Display.** Chrome runs headed. **Use `xvfb-run`, not the VM's desktop session.**
  This reverses the original note, which told the reader to use the desktop and treated
  `xvfb-run` as a fallback. On the xrdp desktop `:10` (3440x1440)
  `time-inference-toggle` and `paperless-integration` fail every time. The cause inside
  the browser was not isolated; only the dependence on the display is established. The same specs pass against the same
  container under `xvfb-run -a -s "-screen 0 1280x1024x24"`. Details in
  [.claude/rules/integration-tests.md](../../.claude/rules/integration-tests.md).
- **`--network=host` is the actual payoff.** Per
  [.claude/rules/integration-tests.md](../../.claude/rules/integration-tests.md),
  several specs start a mock HTTP server *in the test process* bound to `127.0.0.1` and
  hand the backend that URL over RPC. Docker Desktop for Windows cannot put a container
  in the host's network namespace, so `paperless-integration.spec.ts` fails locally on
  Windows every time and the developer falls back to spawned mode or trusts CI. On Linux
  it just works, and local Docker-mode runs finally match CI exactly.
- **Ubuntu host prerequisites:** `build-essential` (rustc needs a `cc` linker, and
  [Cargo.lock](../../src-tauri/Cargo.lock) pulls the `cc` crate for build scripts),
  Node 20+, Rust 1.77+, Docker
  Engine (native — no Docker Desktop), Google Chrome, and `sqlite3` for the query skill.
  **Not** needed: `libssl-dev` / `pkg-config` (reqwest is rustls-only, no `openssl-sys`
  in the lockfile) or a system SQLite library (`libsqlite3-sys` is `bundled`).
- **Not in scope:** trimming the `windows-latest` / `macos-latest` legs from the backend
  CI matrix. They stay — they are cheap, and once no human runs Windows they become the
  only guard on the `#[cfg(not(unix))]` branch.
- **Not in scope:** devcontainer, Nix, or any reproducible-environment tooling. Plain
  apt + rustup + nvm on the VM.
- **No CHANGELOG entry.** Everything here is internal dev tooling; per
  [CLAUDE.md](../../CLAUDE.md) the changelog covers user-visible changes only.

## Verification

Run on the VM from a clean clone — the full checklist is [02-plan.md](02-plan.md)
Task 7. The two that prove the point:

1. `npm run test:integration:tier1` runs **only** tier1 + existing (48 tests), not all
   four globs. **Verified 2026-09-07:** 11 spec files, 48 tests, no tier2/tier3 spec.
2. `npm run test:integration:docker` passes `paperless-integration.spec.ts` against a
   `--network=host` container. **Verified 2026-09-07:** 32 of 32 spec files passed with
   no retries, under `xvfb-run`.
