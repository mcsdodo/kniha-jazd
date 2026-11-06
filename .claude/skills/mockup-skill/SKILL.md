---
name: mockup
description: Use before planning or coding a change that adds or changes a screen, a dialog, a column or a UI state - draws the change in the page's mockup file in docs/mockups/ first, publishes it to the claude.ai artifact for review, gets the user's approval, and after the merge turns the proposal into the current state. Also use for "update the mockup", "draw it first", "show me how it would look", or when /verify lists mockup drift.
---

# Mockup first

Each page of the app has a mockup file in [docs/mockups/](../../../docs/mockups/README.md): static
HTML that draws the screens of the page as frames, with English notes under each frame. **The
file is the source.** The claude.ai artifact of the page is only its published view, where the
user reviews it and sends comments to Claude. The registry
[docs/mockups/README.md](../../../docs/mockups/README.md) maps each page to its file, artifact,
UUID and source paths.

A UI change goes into the mockup file first. The code follows after the user approves it.

## When to use

- A task adds or changes a screen, a dialog, a column, a button or a state the user sees.
- The user asks to see the design before the code.
- `/verify` lists commits under "Mockup Drift" for a page.

Skip it for a change that the user cannot see (backend logic, tests, refactors).

## The one rule

**Change the file, then publish it.** Never change the artifact with `edit_version`. A direct
artifact edit is not in git, and the next publish from the file erases it.

Publish = `get_artifact` (UUID) for the live `version`, then `publish_files` with
`files: [{"as": "index.html", "path": "<abs path of the file>"}]`, `slug` = UUID,
`base_version` = that version, and a short `label`. The tools refuse the short id of the link:
use the UUID.

If the session has no `claude-artifacts` tools, change the file anyway and say that the
artifact is not published. The user can publish later, or review the file in a browser.

## Steps: draw a proposal

1. Find the page in the registry. A new page has no row: see "A new page" below.
2. Read the Svelte files of the change, so that the frame starts from the real current state.
3. Change the mockup file:
   - Wrap each changed element or frame in `<div class="open"><span class="tag">Návrh (task NN)</span> ... </div>`.
   - A new frame gets the tag in its `<h2>` too.
   - Add the notes for the change as `<li class="prop">`, with the new behaviour in `<b>`.
   - Keep the other frames as they are.
4. Publish it. Give the user the artifact link, and list what changed. Ask for the review.
5. A comment that the user sends to Claude on the artifact reaches this session. Read it with
   `read_comments` (`thread` for the whole thread). Change the file, publish again, then
   `reply_comment` and `resolve_comment`.
6. When the user approves, commit the mockup file with the task docs, and add to `01-task.md`:
   ```markdown
   ## Design

   Frames N and M of [docs/mockups/<page>.html](../../docs/mockups/<page>.html), marked "Návrh (task NN)".
   ```

## Steps: after the merge to `main`

1. In the mockup file, remove the `open` wrappers and the `Návrh (task NN)` tags of the task.
   Change the `li.prop` notes to plain `<li>`.
2. Make the frames match the merged code. Check the labels again: the code can differ from the
   proposal.
3. Commit the file and publish it.
4. `bash scripts/mockup-drift.sh` must not list the page.

## A dropped feature

Restore the file from git (`git checkout <commit before the proposal> -- docs/mockups/<page>.html`,
or revert the proposal commit). Then publish it.

## A new page

1. Copy an existing PC mockup file: the shared CSS between "Shared PC mockup CSS" and "End of
   the shared CSS", the page header, the nav, and both scripts at the end.
2. Publish it with `publish_files` and no `slug` (`title` "Kniha jázd PC: <Page>", `icon` "Monitor").
3. Add a row to the registry with the file, the link and the UUID from the publish result.
4. Add the new page to the nav of the other files (`<a href="<artifact url>" data-file="<file>">`),
   and publish them.
5. Add a card for the page to [docs/mockups/index.html](../../../docs/mockups/index.html), the
   start page on GitHub Pages. The index has no artifact.

## Drawing rules

- Draw the code, not the docs. Read the Svelte before you draw a current state.
- Copy each Slovak UI label from `src/lib/i18n/sk/index.ts`. Check it with `grep -F`. A new
  label in a proposal is a proposal for that file too.
- Invented data only, the same world in each mockup: Škoda Octavia BA123XY, year 2026, Domov,
  Kancelária, Klient Poprad, Sklad, Čerpacia stanica D1, "Moja firma s.r.o.", Ján Novák. Never
  use data from the live database: the repo is public.
- Notes in ASD-STE100: short sentences, active voice, one fact per sentence. Say what the user
  sees and does: the trigger, the result, the conditions (env vars, settings, vehicle type) and
  the RPC command.
- Give a value (a limit, a distance, a percentage, a timeout) only if the screen shows it, or if
  its code is in a source path of the page row, so that the drift check sees a change to it.
  Name a value that only the backend knows by its BIZ/ADR id, or by its constant if it has no
  id: "a known place near the GPS point is preselected (BIZ-031)", not "within 300 m". The
  notes own the UI behaviour; the feature docs own the technical part. Do not copy one into
  the other.
- No em-dash, en-dash, curly quotes or ellipsis glyph in the notes. Slovak labels keep their
  own characters.
- An open point (a bug or a gap that you see while you draw) goes into the "Open points"
  section with `file:line`. Do not fix it in the mockup task.
- Check the result: `google-chrome --headless=new --no-sandbox --disable-gpu --hide-scrollbars
  --window-size=1280,6000 --screenshot=<scratch>/x.png file://<abs path>`, then read the PNG.

## Common mistakes

| Mistake | Correct action |
|---|---|
| Change the artifact with `edit_version` | Change the file, then publish it. |
| Call a tool with the id from the link | Use the UUID from the registry. |
| Draw the proposal as if it is built | Mark it with `open`, `Návrh (task NN)` and `li.prop`. |
| Leave the mark after the merge | Remove it, commit, publish. `/move-to-done` checks it. |
| Copy a label from a doc | Grep `sk/index.ts`. Docs can be out of date. |
| Write a backend value (300 m, 5 %) in a note | Name the BIZ/ADR id or the constant. |
