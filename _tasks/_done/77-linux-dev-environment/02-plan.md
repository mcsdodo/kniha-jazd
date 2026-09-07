**Date:** 2026-09-07
**Subject:** Move day-to-day development from the Windows box to an Ubuntu VM
**Status:** Complete (2026-09-07)

# Linux Dev Environment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task.

**Goal:** A fresh clone on an Ubuntu VM builds, tests, and hooks correctly — with no
`pwsh`, no silently-wrong test runs, and Windows still working.

**Architecture:** No product code changes. Six edits to the dev harness
([package.json](../../package.json), the Claude hooks, three docs, one
`.gitattributes`), then a real acceptance run on the VM. The Rust and Svelte sources are
already platform-clean — see [01-task.md](./01-task.md) Background for the evidence.

**Tech Stack:** npm scripts + `cross-env`, bash hooks, GitHub Actions YAML comments,
markdown.

**Branch:** Tasks 1–6 are safe to land from Windows *before* the move (they all keep
Windows working). Task 7 runs on the VM. Work directly on `main` unless the tree is
mid-feature — see the in-flight Task 75 warning in [01-task.md](./01-task.md).

---

## Task 0 (prerequisite, manual — do this before wiping the Windows checkout)

Not a code change. Confirm nothing is stranded:

1. `git status --short` — the tree currently carries uncommitted
   [Task 75](../75-place-book/) work including **untracked** files
   (`commands_internal/places_cmd.rs`, `places_cmd_tests.rs`, `src/places/`). Untracked
   files are not in any clone. Commit them, or push a WIP branch.
2. `git push` — the VM clones from the remote, not from the Windows disk.
3. Copy the two gitignored secrets out of band so they can be recreated on the VM:
   `.env` (`PAPERLESS_API_TOKEN`) and `local.settings.json` (`gemini_api_key`). Do not
   commit either.

**Verification:** `git status --short` is empty and `git log origin/main..HEAD` shows
nothing unpushed.

---

## Task 1: Make the integration-test npm scripts shell-agnostic

**Files:**
- Modify: [package.json](../../package.json) — the five `test:integration:*` scripts
  and `devDependencies`
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml) — the now-stale
  comment above the env-pinned run step

**Step 1: Add the dependency**

```bash
npm install --save-dev cross-env
```

**Step 2: Rewrite the scripts**

In [package.json](../../package.json), replace the cmd.exe `set X=Y&&` forms:

```json
"test:integration": "wdio run tests/integration/wdio.server.conf.ts",
"test:integration:tier1": "cross-env TIER=1 npm run test:integration",
"test:integration:tier2": "cross-env TIER=2 PARALLEL_TIERS=true npm run test:integration",
"test:integration:tier3": "cross-env TIER=3 PARALLEL_TIERS=true npm run test:integration",
"test:integration:docker": "cross-env WDIO_EXTERNAL_SERVER=1 npm run test:integration",
"test:integration:docker:env": "cross-env WDIO_ENV_PINNED=1 npm run test:integration:docker",
```

`test:integration` and `test:all` are unchanged. The chained case
(`docker:env` → `docker`) still works: `cross-env` exports into the child `npm` process,
so both variables reach wdio.

**Step 3: Fix the stale CI comment**

[test.yml](../../.github/workflows/test.yml) carries this above the
`Run env-pinned integration tests` step:

> The env: block is NOT redundant with the npm script. The scripts use Windows
> `set X=Y&&`, which under sh sets positional parameters and exports nothing…

That justification dies with this task. **Keep the `env:` blocks** — they are explicit
and keep CI correct independent of the scripts — but rewrite the comment to say so,
noting Task 77 as the reason the old hazard is gone.

**Verification:**
- `npm run test:integration:tier1` — the wdio banner lists **only** `tier1` and
  `existing` specs (48 tests), not all four globs. Before this change on Linux it listed
  everything.
- `npm run test:integration:docker` against a running container connects to port
  **3456** and logs `Connecting to external server at http://localhost:3456`, rather
  than spawning a binary on 3457.
- On Windows (before the move): the same two commands still behave correctly.
- Invariant I1 (see [CLAUDE.md](../../CLAUDE.md)) still holds — the tier scripts remain
  thin aliases delegating to `test:integration`, which is what the Docker CI jobs run.

---

## Task 2: Port the pre-commit hook from PowerShell to bash

**Files:**
- Create: `.claude/hooks/pre-commit.sh`
- Delete: [.claude/hooks/pre-commit.ps1](../../.claude/hooks/pre-commit.ps1)
- Modify: [.claude/settings.json](../../.claude/settings.json)

**Step 1: Write the bash hook**

Behaviour to preserve exactly: read stdin; empty input allows; only gate commands
starting with `git commit`; skip with a warning if `src-tauri` or `cargo` is missing;
exit 2 to block on test failure. Diagnostics go to **stderr** — that is what Claude Code
surfaces on a blocking exit.

```bash
#!/bin/bash
# Pre-commit hook: block a commit if the backend tests fail.
# Exit 0 = allow, exit 2 = block (stderr is shown to Claude).

input=$(cat)
[ -z "$input" ] && exit 0

# Only gate `git commit`, anchored at the start of the command value so that
# e.g. `git log --grep "git commit"` does not trigger a full test run.
if ! echo "$input" | grep -q '"command"[[:space:]]*:[[:space:]]*"git commit'; then
    exit 0
fi

project_dir="${CLAUDE_PROJECT_DIR:-$PWD}"
manifest="$project_dir/src-tauri/Cargo.toml"

if [ ! -f "$manifest" ]; then
    echo "Warning: $manifest not found, skipping backend tests" >&2
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Warning: cargo not found in PATH, skipping backend tests" >&2
    exit 0
fi

echo "=== Pre-commit: running backend tests ===" >&2

if ! cargo test --manifest-path "$manifest" --workspace >&2; then
    echo "COMMIT BLOCKED: backend tests failed." >&2
    exit 2
fi

echo "Backend tests passed. Proceeding with commit." >&2
exit 0
```

Two deliberate differences from the PowerShell original:

- It uses `cargo test --manifest-path … --workspace` instead of `Push-Location src-tauri;
  cargo test`. Equivalent (the manifest is a virtual workspace, so bare `cargo test`
  there already tests all members) but it matches the "never `cd &&`" rule in
  [CLAUDE.md](../../CLAUDE.md).
- The command match is anchored at the start of the value, which is stricter than the
  looser pattern in [post-commit-reminder.sh](../../.claude/hooks/post-commit-reminder.sh).

**Step 2: Point settings.json at it**

In [.claude/settings.json](../../.claude/settings.json), change the PreToolUse hook
command from `pwsh -NoProfile -File .claude/hooks/pre-commit.ps1` to
`bash .claude/hooks/pre-commit.sh`. Leave the `Bash` matcher and the 120000 timeout
alone.

**Step 3: Delete the PowerShell hook**

`git rm .claude/hooks/pre-commit.ps1`

**Verification:**

```bash
chmod +x .claude/hooks/pre-commit.sh

# Non-commit command: allowed, no tests run
echo '{"tool_input":{"command":"git status"}}' | bash .claude/hooks/pre-commit.sh; echo "exit=$?"   # exit=0, no output

# Commit command: runs the backend suite
echo '{"tool_input":{"command":"git commit -m test"}}' | bash .claude/hooks/pre-commit.sh; echo "exit=$?"

# Empty input: allowed
printf '' | bash .claude/hooks/pre-commit.sh; echo "exit=$?"   # exit=0
```

Then make a real commit in a Claude Code session and confirm the hook fires and the
suite runs. On Windows, confirm the same via git-bash.

---

## Task 3: Rewrite the query-sqlite-db skill for the current data-dir model

**Files:**
- Modify: [.claude/skills/query-sqlite-db/SKILL.md](../../.claude/skills/query-sqlite-db/SKILL.md)

The skill still documents `%AppData%/com.notavailable.kniha-jazd.dev/kniha-jazd.db` —
the Tauri desktop location, dead since Task 73 and wrong on every platform.

**Step 1: Replace the path table** with what
[src-tauri/web/src/main.rs:24-30](../../src-tauri/web/src/main.rs) actually resolves and
what [docker-compose.web.yml](../../docker-compose.web.yml) mounts:

| Context | Path |
|---------|------|
| Docker (compose or `docker run`) | `./data/kniha-jazd.db` **on the host** — the bind mount of `/data` |
| Local dev | `$KNIHA_JAZD_DATA_DIR/kniha-jazd.db`, falling back to `/data` when unset |
| Explicit override | `$DATABASE_PATH` |
| Integration tests (spawned mode) | a fresh `mkdtemp` directory, deleted after the run — see [wdio.server.conf.ts](../../tests/integration/wdio.server.conf.ts) `onPrepare` |

**Step 2: Note that `docker exec` will not work.** The runtime stage of
[Dockerfile.web](../../Dockerfile.web) installs only `ca-certificates` and `curl` — there
is no `sqlite3` in the image. Query the bind-mounted file on the host instead.

**Step 3: Rewrite the frontmatter `description`.** It currently advertises the skill for
"when environment variable expansion fails with APPDATA or similar paths", which is the
Windows-desktop failure mode. Re-point it at "inspecting the logbook DB directly for
debugging".

**Step 4: Replace the Common Mistakes rows.** Drop the two `%APPDATA%` / `$env:APPDATA`
rows entirely. Keep the column-name and "unable to open database" rows. Add: `sudo apt
install sqlite3` when the binary is missing, and a pointer to the `get_db_location` RPC
command, which reports the path the running server actually opened.

Leave the Key Tables section as is.

**Verification:** every path in the rewritten skill matches either
[main.rs](../../src-tauri/web/src/main.rs) or
[docker-compose.web.yml](../../docker-compose.web.yml). Run one query against the real
Docker data dir and confirm it returns rows.

---

## Task 4: Fix the Windows-era developer docs

**Files:**
- Modify: [local.settings.json.sample](../../local.settings.json.sample)
- Modify: [CONTRIBUTING.md](../../CONTRIBUTING.md)

**Step 1:** [local.settings.json.sample](../../local.settings.json.sample) shows
`"receipts_folder_path": "C:\\Users\\YourUsername\\Documents\\Receipts"`. Replace with a
POSIX example (`/home/youruser/Documents/Receipts`). Do **not** delete the file — decide
its fate separately; `LocalSettings::load` reads from the data dir, so a repo-root copy
only matters when `KNIHA_JAZD_DATA_DIR` points at the repo root.

**Step 2:** Add an Ubuntu/Debian prerequisites block to the Development Setup section of
[CONTRIBUTING.md](../../CONTRIBUTING.md), under the existing generic list:

```bash
sudo apt install build-essential sqlite3
```

Plus notes: Docker Engine (native, not Docker Desktop); Google Chrome for the
integration tests; and that `libssl-dev` / `pkg-config` are **not** required because
reqwest is rustls-only and `libsqlite3-sys` is `bundled`.

**Step 3:** Add a one-line note that the integration suite launches Chrome **headed**,
so it needs a desktop session or `xvfb-run -a`.

**Verification:** a reader following only [CONTRIBUTING.md](../../CONTRIBUTING.md) on a
bare Ubuntu install gets through `npm run test:backend` and `npm run test:integration`.
Task 7 is that reader.

---

## Task 5: Pin line endings with .gitattributes

**Files:**
- Create: `.gitattributes`

The repo has none, and this machine runs `core.autocrlf=true` — so line endings are
currently a property of each developer's git config, and `git diff` emits
`LF will be replaced by CRLF` warnings.

**Step 1:** Create `.gitattributes` with one line:

```
* text=auto eol=lf
```

`text=auto` auto-detects binaries (the PNGs, `.db` and `.xlsx` files are safe).

**Step 2 — the gate.** Run:

```bash
git add --renormalize .
git status --short
```

- **Empty output** (expected — all 340 affected files are already LF in the index):
  commit `.gitattributes` alone.
- **Non-empty output:** stop. Do not bundle a mass renormalisation into this task.
  Report what changed and let it be a separate, isolated commit.

**Verification:** after committing, `git diff` on a Rust or TypeScript file no longer
prints a CRLF warning, and `git ls-files --eol` reports `w/lf` on Linux.

---

## Task 6: Record the ADR

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md)

Use the [`/decision`](../../.claude/skills/decision-skill/SKILL.md) skill. One short ADR
covering the durable convention, not the mechanics:

- **Decision:** the development environment is Linux-first. Committed Claude Code hooks
  are bash; npm scripts that set environment variables use `cross-env`.
- **Why:** the only shipped artifact is a Debian container and CI runs the whole
  integration suite on Linux, so the dev machine should match. `settings.json` has no
  per-platform hook selection, and bash is available on all three platforms (git-bash on
  Windows) while `pwsh` is not.
- **Consequence:** a new `.ps1` hook is a regression. The `windows-latest` and
  `macos-latest` backend CI legs stay — they are now the only guard on the
  `#[cfg(not(unix))]` branch in [main.rs](../../src-tauri/web/src/main.rs).

**Verification:** entry present with the next free ADR number, index/table of contents
in [DECISIONS.md](../../DECISIONS.md) updated to match.

---

## Task 7: VM acceptance run

**Files:** none — this is verification.

Run in order on the Ubuntu VM, from a clean `git clone`, in a terminal **inside the
desktop session** (Chrome runs headed):

| # | Command | Expected |
|---|---------|----------|
| 1 | `npm ci` | clean install, no native build failures |
| 2 | `npm run i18n && npm run check` | no errors — `i18n` must run first, nothing else regenerates [i18n-types.ts](../../src/lib/i18n/i18n-types.ts) |
| 3 | `npm run build` | the static SPA is produced in `build/` |
| 4 | `cargo test --manifest-path src-tauri/Cargo.toml --workspace` | green |
| 5 | `cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web` | debug binary at `src-tauri/target/debug/kniha-jazd-web` (no `.exe`) |
| 6 | `npm run test:integration:tier1` | **only** tier1 + existing specs listed, green — this is the Task 1 proof |
| 7 | `npm run test:integration` | full suite green in spawned mode (port 3457) |
| 8 | `docker build -f Dockerfile.web -t kniha-jazd-web:local .` | image builds |
| 9 | container up with `--network=host`, then `npm run test:integration:docker` | green **including `paperless-integration.spec.ts`** — the Windows-impossible spec |
| 10 | a real `git commit` in a Claude Code session | bash pre-commit hook fires, runs the backend suite, allows the commit |
| 11 | `sqlite3 ./data/kniha-jazd.db ".tables"` | returns tables — the Task 3 paths are right |

For step 9, mirror CI exactly:

```bash
mkdir -p data
docker run -d --name kniha-jazd-web \
  --network=host \
  -v "$PWD/data:/data" \
  -v "$PWD/tests/integration/data:/testdata:ro" \
  -e KNIHA_JAZD_DATA_DIR=/data \
  -e DATABASE_PATH=/data/kniha-jazd.db \
  -e KNIHA_JAZD_MOCK_GEMINI_DIR=/testdata/mocks \
  -e KNIHA_JAZD_MOCK_GEOCODER_DIR=/testdata/geocoder \
  -e PORT=3456 \
  kniha-jazd-web:local
```

**Corrected on 2026-09-07.** The command first written here omitted
`KNIHA_JAZD_MOCK_GEOCODER_DIR`, which CI does pass. Without it `places.spec.ts` fails.
Run the suite under `xvfb-run`, not on a desktop session - see the Display note in
[01-task.md](01-task.md).

Also recreate the two gitignored local files from Task 0 before running anything that
touches Paperless or Gemini: `.env` and `local.settings.json`.

**Verification:** all eleven rows pass. Step 9 is the one that was never achievable on
the Windows box — if it goes green, the move has paid for itself.

---

## Out of Scope

- Trimming `windows-latest` / `macos-latest` from the backend CI matrix. They stay.
- devcontainer / Nix / any reproducible-environment tooling.
- A CHANGELOG entry. This is all internal dev tooling; per
  [CLAUDE.md](../../CLAUDE.md) the changelog is for user-visible changes.
- Deleting [local.settings.json.sample](../../local.settings.json.sample) — only its
  path example is corrected here.
