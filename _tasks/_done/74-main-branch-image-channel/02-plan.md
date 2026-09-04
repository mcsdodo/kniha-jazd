**Date:** 2026-09-04
**Subject:** Publish a floating `:main` Docker image channel from every green main build
**Status:** Planning

# Main-Branch Image Channel Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Every green push build of `main` publishes `ghcr.io/mcsdodo/kniha-jazd-web:main`
and `:main-<short-sha>`, while `:latest` and `vX.Y.Z` stay owned by
[`/release`](../../.claude/skills/release-skill/SKILL.md).

**Architecture:** One new job appended to [test.yml](../../.github/workflows/test.yml).
It depends on every test job in the workflow, downloads the image artifact that
[integration-build-docker](../../.github/workflows/test.yml) already produced, loads it,
re-tags it and pushes to ghcr.io. No rebuild — the published bytes are the tested bytes.
Everything else in the task is documentation.

**Tech Stack:** GitHub Actions, `docker/login-action@v3`, `actions/download-artifact@v4`,
plain `docker tag` / `docker push`, ghcr.io.

**Branch:** work happens directly on `main` (per user instruction) — no worktree, no
feature branch.

---

## Task 1: Add the publish job to test.yml

**Files:**
- Modify: [.github/workflows/test.yml](../../.github/workflows/test.yml) — append a job
  after `integration-test-docker-env` (currently ends at line 289)

**Step 1: Append the job**

Add at the end of the file:

```yaml
  # Publish the floating `main` channel: the tip of main, pullable without cutting a
  # release. It republishes the exact image artifact the integration tiers just ran
  # against - no rebuild - so the bytes on ghcr.io are the bytes that passed.
  #
  # `needs` covers every test job, and a job-level `if` without a status-check function
  # still requires all of them to have succeeded, so a red or skipped test job means no
  # publish. `:latest` and `vX.Y.Z` are NOT touched here - those stay owned by
  # release.yml on a `v*` tag (ADR-031).
  publish-main-image:
    name: Publish main Docker Image
    needs:
      - backend-tests
      - integration-build-docker
      - integration-test-docker
      - integration-test-docker-env
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - name: Log in to ghcr.io
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Download Docker image
        uses: actions/download-artifact@v4
        with:
          name: docker-image
          path: /tmp

      - name: Load Docker image
        run: docker load -i /tmp/kniha-jazd-web.tar

      - name: Tag and push
        run: |
          IMAGE=ghcr.io/mcsdodo/kniha-jazd-web
          SHORT_SHA=$(echo "${{ github.sha }}" | cut -c1-7)

          docker tag kniha-jazd-web:test "$IMAGE:main"
          docker tag kniha-jazd-web:test "$IMAGE:main-$SHORT_SHA"

          docker push "$IMAGE:main"
          docker push "$IMAGE:main-$SHORT_SHA"

          echo "Published $IMAGE:main and $IMAGE:main-$SHORT_SHA" >> $GITHUB_STEP_SUMMARY
```

**Step 2: Verify the YAML parses**

Run:
```bash
python -c "import yaml; d=yaml.safe_load(open('.github/workflows/test.yml')); print(sorted(d['jobs']))"
```
Expected: a list containing `publish-main-image` alongside the five existing jobs, no
traceback.

**Step 3: Verify the job wiring is what we intend**

Run:
```bash
python -c "import yaml; j=yaml.safe_load(open('.github/workflows/test.yml'))['jobs']['publish-main-image']; print(j['needs']); print(j['if']); print(j['permissions'])"
```
Expected:
```
['backend-tests', 'integration-build-docker', 'integration-test-docker', 'integration-test-docker-env']
github.event_name == 'push' && github.ref == 'refs/heads/main'
{'contents': 'read', 'packages': 'write'}
```

**Step 4: Confirm nothing else in the workflow moved**

Run: `git diff --stat .github/workflows/test.yml`
Expected: additions only, zero deletions.

**Verification:** YAML parses, the job names exactly the four test jobs as `needs`,
and the diff touches nothing that already existed.

---

## Task 2: Record the rationale as ADR-031

**Files:**
- Modify: [DECISIONS.md](../../DECISIONS.md) — new dated section at the top, directly
  under the `# Decisions Log` header and above `## 2026-09-04: Web-First Migration`

**Step 1: Write the entry**

Insert a `## 2026-09-04: Image Publishing Channels` section containing
`### ADR-031: Two Image Channels — `:main` Moves, `:latest` Is Cut`, following the shape
of the neighbouring entries (Context / Decision / Rationale / Consequences). It must
capture:

- Two channels and who owns each: CI owns `:main` / `:main-<sha>`, `/release` owns
  `vX.Y.Z` / `:latest`.
- Why the tested tar is republished rather than rebuilt (identical bytes, no second
  build).
- Why `:main-<sha>` exists (rollback for a floating tag).
- Why no `workflow_dispatch` (pushes to main always run the full suite already).

**Step 2: Verify placement**

Run: `grep -n "^## \|^### ADR-031" DECISIONS.md | head -6`
Expected: `ADR-031` appears above the `2026-09-04: Web-First Migration` section.

**Verification:** ADR-031 is the newest entry and no existing entry was edited.

---

## Task 3: Document the channels for operators

**Files:**
- Modify: [README.md](../../README.md) — the `## Inštalácia` section (Slovak, around
  lines 35-58)
- Modify: [README.en.md](../../README.en.md) — the `## Installation` section (around
  lines 35-58)
- Modify: [docs/features/server-mode.md](../../docs/features/server-mode.md) — the
  `## Docker Deployment` section (around lines 32-48)
- Modify: [CLAUDE.md](../../CLAUDE.md) — the `## CI/CD` section

**Step 1: Add a channel table to both READMEs**

After the `docker run` block in each, add a short table naming the three tag forms
(`:latest` = last release, `:main` = tip of main, `:main-<sha>` = pinned main build) and
one line saying `:main` is for trying changes before a release. Slovak in
[README.md](../../README.md), English in [README.en.md](../../README.en.md). Keep the
existing "update = pull a newer tag" sentence.

**Step 2: Add the same table to the feature doc**

In [server-mode.md](../../docs/features/server-mode.md), add an `**Image channels:**`
block under the quick start, cross-referencing ADR-031.

**Step 3: Correct the CI/CD section in CLAUDE.md**

State that a green push to `main` publishes `:main` + `:main-<short-sha>`, and that
`release.yml` on a `v*` tag still owns `vX.Y.Z` + `:latest`.

**Step 4: Verify every mention agrees**

Run: `grep -rn "kniha-jazd-web:main" README.md README.en.md docs/ CLAUDE.md`
Expected: the new channel docs in all four files, no stale claim that nothing is
published automatically.

**Verification:** All four docs describe the same channel model.

---

## Task 4: Keep the release skill honest

**Files:**
- Modify: [.claude/skills/release-skill/SKILL.md](../../.claude/skills/release-skill/SKILL.md)
  — the `## 7. Report Results` section (around lines 119-134)

**Step 1: Note the boundary**

Add a line under the published-tags list stating that `/release` owns `vX.Y.Z` and
`:latest` only; `:main` is moved by [test.yml](../../.github/workflows/test.yml) on every
green main build and must not be pushed from a release.

**Step 2: Verify**

Run: `grep -n "main" .claude/skills/release-skill/SKILL.md | head`
Expected: the new boundary note is present.

**Verification:** A future `/release` run cannot mistake `:main` for a release tag.

---

## Task 5: Changelog entry

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) — the `## [Unreleased]` → `### Pridané`
  block

**Step 1: Add the Slovak entry**

One bullet explaining that the tip of `main` is now pullable as
`ghcr.io/mcsdodo/kniha-jazd-web:main` (updated automatically after every green build),
that a specific build can be pinned with `:main-<sha>`, and that `:latest` keeps meaning
"last released version".

**Step 2: Verify**

Run: `grep -n "kniha-jazd-web:main" CHANGELOG.md`
Expected: one hit inside the `[Unreleased]` section.

**Verification:** Entry is Slovak, user-facing, in `[Unreleased]`.

---

## Task 6: Commit and push

**Step 1: Review what is staged**

Run: `git status --short`
Expected: only the files this plan names, plus the planning folder and
[_tasks/index.md](../index.md).

**Step 2: Stage explicitly (never `git add -A`)**

```bash
git add .github/workflows/test.yml DECISIONS.md CHANGELOG.md CLAUDE.md \
        README.md README.en.md docs/features/server-mode.md \
        .claude/skills/release-skill/SKILL.md
```

**Step 3: Commit and push**

```bash
git commit -m "ci: publish floating :main image channel from green main builds"
git push
```

**Verification:** `git status` clean, `git log origin/main -1` shows the commit.

---

## Task 7: Verify the pipeline actually published

This is the real verification — the push in Task 6 triggers the run that exercises the
new job.

**Step 1: Watch the run**

Run: `gh run list --workflow test.yml --limit 1` then
`gh run watch <id>` (the full suite is ~10 minutes).
Expected: all jobs green, including `Publish main Docker Image`.

**Step 2: Confirm the job published**

Run: `gh run view <id> --log --job <publish job id> | tail -20`
Expected: two successful `docker push` lines.

**Step 3: Confirm the tags resolve on the registry**

Run:
```bash
docker manifest inspect ghcr.io/mcsdodo/kniha-jazd-web:main
docker manifest inspect ghcr.io/mcsdodo/kniha-jazd-web:main-$(git rev-parse --short=7 HEAD)
```
Expected: both return a manifest.

**Step 4: Confirm the release channel did not move**

Run: `docker manifest inspect ghcr.io/mcsdodo/kniha-jazd-web:latest`
Expected: still the digest of the last `v*` release, different from `:main`.

**Verification:** Both new tags exist, `:latest` is untouched.

---

## Task 8: Archive the task

**Files:**
- Move: `_tasks/74-main-branch-image-channel/` → `_tasks/_done/74-main-branch-image-channel/`
- Modify: [_tasks/index.md](../index.md)

**Step 1: Move the folder**

```bash
git mv _tasks/74-main-branch-image-channel _tasks/_done/74-main-branch-image-channel
```

**Step 2: Update the index**

Move the row out of **Active Tasks** into **Completed Tasks**, repoint the link at
`_done/`, and refresh the `**Last updated:**` line.

**Step 3: Commit**

```bash
git add _tasks/ && git commit -m "docs: complete task 74 main-branch image channel" && git push
```

**Verification:** `_tasks/index.md` lists task 74 as complete and the folder lives under
`_done/`.
