---
name: release
description: Bump version, update changelog, commit, tag, push, and let CI publish the container image
---

# Release Workflow

When the user says "release", "/release", or "push, release", execute this workflow.

**What a release is:** a `v*` tag. [release.yml](../../../.github/workflows/release.yml)
then:

1. waits until [test.yml](../../../.github/workflows/test.yml) has tested and published
   `ghcr.io/mcsdodo/kniha-jazd-web:main-<short-sha>` for the tagged commit,
2. promotes that same image to `:vX.Y.Z` and `:latest` (no rebuild),
3. creates a GitHub Release whose text is the `## [X.Y.Z]` section of
   [CHANGELOG.md](../../../CHANGELOG.md).

There is no installer and no auto-updater. The GitHub Release holds notes only, no
assets. See ADR-051 in [DECISIONS.md](../../../DECISIONS.md).

## 1. Check the Integrator Block

The release notes are the contract with the integrator who upgrades the image. Check
them against the code before you pick a version.

```bash
LAST=$(git describe --tags --abbrev=0 --match 'v*')

# a) new migrations since the last release
git diff --name-status "$LAST"..HEAD -- src-tauri/core/migrations | grep up.sql

# b) env vars added or removed since the last release (test files excluded)
git diff -U0 "$LAST"..HEAD -- src-tauri ':!*_tests.rs' \
  | grep -E '^[+-][^+-].*(: &str = "[A-Z][A-Z0-9_]+"|env::var\("[A-Z])'

# c) image contract: ENV, VOLUME, EXPOSE, HEALTHCHECK, CMD, compose example
git diff "$LAST"..HEAD -- Dockerfile.web docker-compose.web.yml
```

Then read `### Pokyny k aktualizácii` under `## [Unreleased]`
(format: [changelog-skill](../changelog-skill/SKILL.md), step 2).

**Stop and fix the changelog first** if one of these is true:

- The block is missing.
- (a) lists a migration, but the row `Migrácie databázy` says `žiadne` or has the wrong
  count.
- (b) lists a variable that the row `Premenné prostredia` does not name. Ignore
  `KNIHA_JAZD_MOCK_*`: they are test hooks, not integrator settings.
- (c) shows a change that the row `Image, volume, port` does not describe.
- A migration contains `DROP` or `DELETE`, and the row `Strata údajov` says `žiadna`.

Fix the block, commit it (`docs: add upgrade notes for vX.Y.Z`), and continue. This is
not a question for the user: the diff gives the facts.

## 2. Determine Version

**Decide it yourself. "Release" means release - do not stop to ask.**

Read the current version from [package.json](../../../package.json). Pick the bump
from the integrator block and the sections of `[Unreleased]`. The first row that
matches wins:

| `[Unreleased]` contains | Bump |
|-------------------------|------|
| an env var `odstránená` or `premenovaná`, a changed meaning or default that breaks an existing config, data loss, or a change to the volume, the data path, the port or the published tags | **major** - 1.4.2 to 2.0.0 |
| `### Pridané`, `### Zastarané`, `### Odstránené`, a `### Zmenené` that alters behavior, any new migration, or a new env var | **minor** - 1.4.2 to 1.5.0 |
| only `### Opravené`, `### Bezpečnosť` or cosmetic `### Zmenené`, with no migration and no env var change | **patch** - 1.4.2 to 1.4.3 |
| no user section, but an integrator block that is not at its no-change values | **minor** - the integrator still needs the notes |

Why a migration is never a patch: a migration is one-way. After it, the previous image
opens the database read-only. A patch must be safe to roll back.

If the user names the bump ("release patch", "minor release"), use that and skip the
table. If the user names a bump that is lower than the table gives, say so once, then
do what the user said.

State the version and the one-line reason in your first message, then keep going:

> Releasing **2.0.0** (major - `GEMINI_API_KEY` is removed and the `receipts` table is
> dropped).

**Only ask when you cannot choose**, which in practice means:
- `[Unreleased]` is empty - ask whether to release at all. "Empty" means: no section
  from `### Pridané` to `### Bezpečnosť`, and every row of the integrator block is at
  its no-change value. The block alone is always there, so it does not count as content.

## 3. Update Version

Edit the version string in these two files:
- [package.json](../../../package.json) - field `"version": "X.Y.Z"`
- [src-tauri/Cargo.toml](../../../src-tauri/Cargo.toml) - field `version = "X.Y.Z"` under `[workspace.package]`

Both workspace members (`core`, `web`) inherit it via `version.workspace = true`,
so there is no other source file to edit.

Then refresh the two lock files. A hand edit does not update them, and a stale
lock file is not visible in a test run:

```bash
npm install --package-lock-only --no-audit --no-fund
cargo check --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web --offline
```

Check that each command changed only the version fields (`git diff --stat`).
Commit all four files together.

## 4. Update CHANGELOG.md

1. Move all content under `## [Unreleased]` to a new version section.
2. Use exactly this heading: `## [X.Y.Z] - YYYY-MM-DD`. release.yml finds the notes by
   `## [X.Y.Z]` and fails if the heading is missing.
3. In the migration row, replace `vX.Y.Z` in the backup file name with the real
   version.
4. Leave a new `## [Unreleased]` at the top with an empty integrator block (all five
   rows at their "no change" value).

Example:
```markdown
## [Unreleased]

### Pokyny k aktualizácii
- **Potrebný zásah:** nie
- **Premenné prostredia:** bez zmeny
- **Migrácie databázy:** žiadne
- **Strata údajov:** žiadna
- **Image, volume, port:** bez zmeny

## [1.1.0] - 2026-10-02

### Pokyny k aktualizácii
- **Potrebný zásah:** nie
- **Premenné prostredia:** pridaná `SYGIC_API_KEY` (voliteľná) ...
...

### Pridané
- ...
```

## 5. Run Tests

First, check if the current branch already has a passing CI run on GitHub:

```bash
gh run list --branch $(git branch --show-current) --limit 5 --json status,conclusion,name,createdAt
```

- If the **most recent** run has `conclusion: "success"` - **skip local tests**, CI already verified them.
- If the most recent run is still in progress (`status: "in_progress"`) - wait or run locally.
- If the most recent run failed, or there are no runs - run tests locally:

```bash
npm run test:backend

# Integration tests need both artifacts the harness starts: the SPA it serves
# and the headless binary it spawns.
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration:tier1
```

If local tests fail, fix issues and retry. Don't proceed until tests pass.

This step is a pre-check only. The real gate is in CI: release.yml publishes nothing
until the full test.yml run on the release commit is green.

## 6. Commit, Tag, and Push

Only once step 5 is green:

```bash
git add -A
git commit -m "chore: release vX.Y.Z"
git tag vX.Y.Z
git push && git push --tags
```

Tag the commit that you push to `main`. release.yml promotes the image that test.yml
published for **that** commit. A tag on a commit that never was the head of a push to
`main` has no `:main-<sha>` image, and the release fails.

## 7. Report Results

test.yml runs the full suite on the release commit (about 5 minutes in September 2026). Then
release.yml promotes the image and creates the GitHub Release. Watch both:

```bash
gh run list --workflow test.yml --branch main --limit 1
gh run list --workflow release.yml --limit 1
gh release view vX.Y.Z --json url --jq .url
```

Report the release URL and the image tags:

- `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z`
- `ghcr.io/mcsdodo/kniha-jazd-web:latest`

**Those two tags and the GitHub Release are all a release owns.** The floating `:main`
and pinned `:main-<short-sha>` tags belong to test.yml, which moves them on every
green build of `main` - never push them from a release (see ADR-031 in
[DECISIONS.md](../../../DECISIONS.md)).

**A release publishes images. It does not deploy.** No running instance pulls
`vX.Y.Z` or `:latest` on its own. A deploy is a separate, deliberate step. Do not
report a release as an update to any live instance.

## Notes

- `Cargo.lock` and `package-lock.json` do not update from a hand edit. Step 3
  refreshes both. Include both in the commit.
- CHANGELOG is in Slovak, with diacritics: `Pokyny k aktualizácii`, `Pridané`,
  `Zmenené`, `Zastarané`, `Odstránené`, `Opravené`, `Bezpečnosť`.
