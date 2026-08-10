---
name: release
description: Bump version, update changelog, commit, tag, push, and build release installer
---

# Release Workflow

When the user says "release", "/release", or "push, release", execute this workflow.

## 1. Determine Version

**Decide it yourself. "Release" means release — do not stop to ask.**

Read the current version from [package.json](../../../package.json) and pick the
bump from the `[Unreleased]` section of [CHANGELOG.md](../../../CHANGELOG.md):

| `[Unreleased]` contains | Bump |
|-------------------------|------|
| any `### Pridané` (new features) | **minor** — 0.41.0 → 0.42.0 |
| `### Zmenené` that alters existing behaviour or needs operator action (new env var, changed defaults, a field that stops working) | **minor** |
| only `### Opravené` (and cosmetic `Zmenené`) | **patch** — 0.41.0 → 0.41.1 |
| an explicit decision to declare the app stable | **major** — ask first |

If the user names the bump ("release patch", "minor release"), use that and skip
the table.

State the version and the one-line reason in your first message, then keep going:

> Releasing **0.42.0** (minor — the PIN requirement changes existing behaviour and
> needs a new env var).

**Only ask when you genuinely cannot choose**, which in practice means:
- `[Unreleased]` is empty or missing → ask whether to release at all.
- The changes look breaking enough to warrant **1.0.0** → ask, because that is a
  statement about the project, not a mechanical rule.

Anything else — pick, say why, proceed.

## 2. Update Version in All Files

Update version string in these three files:
- `package.json` - field `"version": "X.Y.Z"`
- `src-tauri/Cargo.toml` - field `version = "X.Y.Z"`
- `src-tauri/desktop/tauri.conf.json` - field `"version": "X.Y.Z"`

## 3. Update CHANGELOG.md

1. Move all content under `## [Unreleased]` to a new version section
2. Add today's date in format `## [X.Y.Z] - YYYY-MM-DD`
3. Leave empty `## [Unreleased]` section at top

Example:
```markdown
## [Unreleased]

## [0.2.0] - 2025-12-29

### Pridane
- New feature...
```

## 4. Run Tests

First, check if the current branch already has a passing CI run on GitHub:

```bash
gh run list --branch $(git branch --show-current) --limit 5 --json status,conclusion,name,createdAt
```

- If the **most recent** run has `conclusion: "success"` → **skip local tests**, CI already verified them.
- If the most recent run is still in progress (`status: "in_progress"`) → wait or run locally.
- If the most recent run failed, or there are no runs → run tests locally:

```bash
npm run test:backend
npm run test:integration:tier1
```

If local tests fail, fix issues and retry. Don't proceed until tests pass.

## 5. Build Release

Build BEFORE committing to verify everything works:

```bash
npm run tauri build
```

If build fails, fix issues and retry. Don't proceed until build succeeds.

**Expected non-failure:** the build ends with

```
A public key has been found, but no private key. Make sure to set `TAURI_SIGNING_PRIVATE_KEY`
```

after both bundles are written. That is the updater-signature step, and the key
lives only in GitHub secrets ([release.yml](../../../.github/workflows/release.yml)
supplies it on tag push). The local build has already served its purpose — compile
plus bundle verified — so **continue**. It also means the locally built installers
are unsigned and are not valid auto-update artifacts; distribute the GitHub release
assets instead.

## 6. Commit, Tag, and Push

Only after successful build:

```bash
git add -A
git commit -m "chore: release vX.Y.Z"
git tag vX.Y.Z
git push && git push --tags
```

## 7. Report Results

Show the path to the built installer:
- NSIS installer: `src-tauri/target/release/bundle/nsis/Kniha Jázd_X.Y.Z_x64-setup.exe`

## Notes

- Cargo.lock will auto-update - include it in the commit
- CHANGELOG is in Slovak (Pridane, Zmenene, Opravene)
