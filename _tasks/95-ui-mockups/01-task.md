**Date:** 2026-11-06
**Subject:** Mockup-first workflow for UI changes, and PC mockups of all web flows
**Status:** In Progress

## Goal

Each change to a screen goes through a mockup first. The user reviews the mockup on
claude.ai, then the code follows. The mockups show the current state of every screen.

The mobile mockup "Kniha jázd na mobile" already works this way (task
[93](../93-mobile-ui/01-task.md), task [94](../94-mobile-route-alternatives/01-task.md), and the
ODO change of the mobile trip card in commit `5247a3b`). This task adds:

1. Six PC mockup artifacts, one per page: Jazdy, Doklady, Fuelio, Mapa, Miesta, Nastavenia.
2. The workflow and the harness changes that make the mockup a step of each UI feature.

## Decisions

| Question | Answer | Who |
|---|---|---|
| How many PC artifacts | One per page (6). | User |
| Where the HTML lives | In the repo, [docs/mockups/](../../docs/mockups/README.md), as the source. Each file is published to a private claude.ai artifact for review. (First decided: claude.ai only. Changed after the notes and the feature docs showed drift with nothing to catch it.) | User |
| Fidelity | Same as the mobile mockup: hand-drawn HTML/CSS in the app colours, invented data, Slovak UI labels, English STE notes. | User |
| Where a proposal goes | In place in the page mockup file, with an orange "Návrh (task NN)" mark. The mark goes away after the merge. A dropped feature restores the file from git. | User |
| Where the skill and the registry live | In this repo: [.claude/skills/mockup-skill/](../../.claude/skills/mockup-skill/SKILL.md) and [docs/mockups/README.md](../../docs/mockups/README.md). | User |
| A CI check | None. Skills, `/verify` and the commit reminder carry the rule. | User |
| Notes and feature docs | The mockup notes own the UI behaviour, the feature docs own the technical part. A note gives a value only if the screen shows it or its code is in a source path of the page; a backend value is named by its BIZ/ADR id or its constant. No mockup links in the feature docs: the registry maps the pages. | User and Claude |

## Facts found in the research

- `edit_version` refuses the short id of the link (`TyYTKii3cU4ryvN98giUGW`). It needs the
  UUID of the artifact. So the registry stores the UUID.
- A comment that the user sends to Claude on an artifact reaches the session that published
  or watches the artifact. The session reads it with `read_comments` and answers with
  `reply_comment` and `resolve_comment`.
- The mobile artifact CSS already has the proposal styles: `.open` (a dashed orange box with a
  tag) and `li.prop` (an orange note marker).

## Work

Part A, the workflow:
- [x] New skill `mockup` ([SKILL.md](../../.claude/skills/mockup-skill/SKILL.md)).
- [x] Mockup files and registry in [docs/mockups/](../../docs/mockups/README.md): page, file, artifact link, UUID, source paths.
- [x] Drift script [scripts/mockup-drift.sh](../../scripts/mockup-drift.sh): the commits that changed a source path after the last change to the mockup file.
- [x] The step in `/task-plan`, `/plan-review`, `/verify`, `/move-to-done`, [_tasks/CLAUDE.md](../CLAUDE.md).
- [x] The mockup reminder in [post-commit-reminder.sh](../../.claude/hooks/post-commit-reminder.sh).
- [x] [svelte-frontend.md](../../.claude/rules/svelte-frontend.md), [CLAUDE.md](../../CLAUDE.md), [docs/CLAUDE.md](../../docs/CLAUDE.md).
- [x] ADR in [DECISIONS.md](../../DECISIONS.md).

Part B, the PC mockups:
- [x] Jazdy (`/` and the layout shell).
- [x] Doklady (`/doklady`).
- [x] Fuelio (`/fuelio`).
- [x] Mapa (`/mapa`).
- [x] Miesta (`/miesta`).
- [x] Nastavenia (`/settings`).
- [x] Cross-links between all 7 artifacts.

The files, the artifacts and their UUIDs are in the registry
[docs/mockups/README.md](../../docs/mockups/README.md).

## Open points found while drawing

Each mockup has an "Open points" section with `file:line`. They are not fixed in this task.
The most important ones:

- No read-only handling on "/", `/doklady`, `/fuelio`, `/mapa` and `/settings`. The dry runs
  skip `check_read_only!`, so a cascade or writeback dialog opens and the write then fails.
  Some settings writes (theme, 3 preferences, backup cleanup) have no guard at all.
- A new vehicle is saved with `is_active: true`, and the other vehicles stay active
  (`src-tauri/core/src/commands_internal/vehicles.rs:78`).
- "Pridať vozidlo" drops the Home Assistant sensor fields.
- Clearing a Home Assistant or Paperless URL or token in the field does not clear it.
- "Automatické čistenie" counts only pre-update backups, and the build no longer makes them.
- "Účel kompenzačnej jazdy" has no effect.
- Doklady: the backend ignores the vehicle; English backend errors show in the Slovak UI.
- Mapa: a failed delete shows the route generation error text.
- Unused i18n keys (`dbLocation*`, most `update.*`, the old compensation suggestion keys and others).

No CHANGELOG entry: this is internal tooling and docs.
