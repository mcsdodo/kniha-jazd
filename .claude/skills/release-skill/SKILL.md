---
name: release
description: Bump version, update changelog, commit, tag, push, and let CI publish the container image
---

# Release Workflow

When the user says "release", "/release", or "push, release", execute this workflow.

**What a release is:** a `v*` tag that makes CI build and push
`ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` (plus `latest`). There is **no GitHub
Release, no installer, and no auto-updater** — the container image is the entire
artifact. Do not create a release, upload assets, or look for signing keys.

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

## 2. Update Version in Both Files

Update the version string in these two files:
- `package.json` — field `"version": "X.Y.Z"`
- `src-tauri/Cargo.toml` — field `version = "X.Y.Z"` under `[workspace.package]`

Both workspace members (`core`, `web`) inherit it via `version.workspace = true`,
so there is nothing else to edit.

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

# Integration tests need both artifacts the harness starts: the SPA it serves
# and the headless binary it spawns.
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration:tier1
```

If local tests fail, fix issues and retry. Don't proceed until tests pass.

## 5. Verify the Image Builds

[test.yml](../../../.github/workflows/test.yml) has a `Docker Image Build` job, so a
green CI run in step 4 already proves the image builds — skip this step.

Otherwise, build locally before committing so a broken Dockerfile is caught here and
not on the tag:

```bash
docker build -f Dockerfile.web -t kniha-jazd-web:local .
```

Don't proceed until it succeeds. The local image is a smoke check only — it is never
pushed; CI rebuilds and publishes from the tag.

## 6. Commit, Tag, and Push

Only once steps 4 and 5 are green:

```bash
git add -A
git commit -m "chore: release vX.Y.Z"
git tag vX.Y.Z
git push && git push --tags
```

## 7. Report Results

The tag triggers [release.yml](../../../.github/workflows/release.yml), whose
`docker-image` job pushes:

- `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z`
- `ghcr.io/mcsdodo/kniha-jazd-web:latest`

**Those two tags are all a release owns.** The floating `:main` and pinned
`:main-<short-sha>` tags belong to [test.yml](../../../.github/workflows/test.yml),
which moves them on every green build of `main` — never push them from a release, and
never report them as release artifacts (see ADR-031 in
[DECISIONS.md](../../../DECISIONS.md)).

Report the run and the image tag, e.g. with:

```bash
gh run list --workflow release.yml --limit 1
```

Do **not** report an installer path or a GitHub Release URL — neither is produced.
The homelab instance updates by pulling the new tag.

## Notes

- Cargo.lock will auto-update - include it in the commit
- CHANGELOG is in Slovak (Pridane, Zmenene, Opravene)
