**Date:** 2026-09-04
**Subject:** Publish a floating `:main` Docker image channel from every green main build
**Status:** Planning

## Goal

Make the tip of `main` pullable from ghcr.io without cutting a release, while leaving
`:latest` and `vX.Y.Z` exclusively owned by [`/release`](../../.claude/skills/release-skill/SKILL.md).

## Background

Today nothing is published automatically:

- [test.yml](../../.github/workflows/test.yml) runs on every push to `main` — backend
  tests on three platforms, then a Docker image build feeding three integration tiers
  plus the env-pinned suite. It builds the image, tests it, and **throws it away** (the
  tar is a 1-day artifact).
- [release.yml](../../.github/workflows/release.yml) triggers **only** on a `v*` tag,
  which only ever appears when a human runs
  [`/release`](../../.claude/skills/release-skill/SKILL.md). It pushes
  `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` + `:latest`.

So between releases there is no way to run the current `main` on the homelab short of
building the image by hand. Meanwhile CI already builds and fully tests exactly the
image we would want — it just discards it.

## Requirements

1. **Floating channel.** Every green push build of `main` publishes
   `ghcr.io/mcsdodo/kniha-jazd-web:main`, moving to the newest commit.
2. **Immutable per-commit tag.** The same build also publishes
   `ghcr.io/mcsdodo/kniha-jazd-web:main-<short-sha>` so a bad floating build can be
   rolled back to a specific earlier commit.
3. **Green means green.** Publish only when backend tests (all three platforms), all
   three integration tiers, and the env-pinned suite have passed. A red build
   publishes nothing.
4. **Release channel untouched.** `:latest` and `vX.Y.Z` keep their current meaning and
   stay produced solely by [release.yml](../../.github/workflows/release.yml) on a `v*`
   tag. Nothing about [`/release`](../../.claude/skills/release-skill/SKILL.md) changes.
5. **No publish from pull requests.** Fork PRs get a read-only `GITHUB_TOKEN`; a PR
   must never move a published tag anyway.
6. **Publish the artifact that was tested**, not a rebuild — the pushed image must be
   bit-identical to what the integration suite ran against.
7. Docs describe the channel model so a homelab operator can choose between them:
   [README.md](../../README.md), [README.en.md](../../README.en.md),
   [server-mode.md](../../docs/features/server-mode.md), [CLAUDE.md](../../CLAUDE.md),
   the [release skill](../../.claude/skills/release-skill/SKILL.md),
   [CHANGELOG.md](../../CHANGELOG.md), and an ADR in
   [DECISIONS.md](../../DECISIONS.md) for the rationale.

## Decisions Taken

| Question | Decision | Why |
|----------|----------|-----|
| Floating tag name | `:main` | Names the branch it tracks, so provenance is unambiguous and it extends naturally if another branch is ever published. Considered `:prerelease` and `:edge`. |
| Immutable per-commit tag | Yes, `:main-<short-sha>` | One extra line of YAML buys rollback. Without it, a broken floating build leaves only "wait for the next green main" or "fall back to `:latest`". |
| Rebuild vs. republish tested tar | Republish the tar | The `integration-build-docker` job already uploads the built image as an artifact; loading and re-tagging it publishes the exact bytes the tests passed against, and skips a second multi-minute build. |
| `workflow_dispatch` escape hatch | Not added | [check-file-changes](../../.github/actions/check-file-changes/action.yml) returns `has_code_changes=true` unconditionally for non-PR events (lines 34-41), so every push to `main` already runs the full suite and publishes. YAGNI. |

## Technical Notes

- **The publish job needs no `always()`.** A job-level `if` without a status-check
  function still requires every `needs` job to have succeeded, which is exactly the
  gate requirement 3 asks for. Matrix needs (`backend-tests`,
  `integration-test-docker`) require *all* legs green.
- **Artifact reuse.** `integration-build-docker` writes
  `type=docker,dest=/tmp/kniha-jazd-web.tar` and uploads it as artifact `docker-image`
  (1-day retention — same run, so fine). `docker load` restores it under the tag
  `kniha-jazd-web:test`.
- **Permissions.** The job needs `packages: write` declared at job level, the same way
  the `docker-image` job in [release.yml](../../.github/workflows/release.yml) does.
- **Not in scope:** multi-arch (arm64) images. Neither channel builds them today —
  both [release.yml](../../.github/workflows/release.yml) and the test build produce
  linux/amd64 only. Unchanged by this task.
- **Not in scope:** GHCR retention/pruning of old `main-<sha>` tags. One manifest per
  main commit is small; revisit if the package grows unwieldy.

## Verification

There are no unit tests for workflow YAML. Verification is the real thing:

1. Parse the edited workflow to catch syntax errors before pushing.
2. Push to `main` and watch the run — the push itself exercises the new job.
3. Confirm both tags resolve on ghcr.io and that `:latest` still points at the last
   release, not at the new build.
