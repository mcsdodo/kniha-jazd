---
name: mockup
description: Use when you brainstorm, propose, plan or code a change that adds or changes a screen, a dialog, a column or a UI state - draws the change in the page's mockup file in docs/mockups/ first, generates and publishes the claude.ai mockups canvas for review, gets the user's approval, and after the merge turns the proposal into the current state. Also use for "update the mockup", "draw it first", "show me how it would look", or when /verify lists mockup drift.
---

# Mockup first

Each page of the app has a mockup file in [docs/mockups/](../../../docs/mockups/README.md): static
HTML that draws the screens of the page as frames, with English notes under each frame. **The
file is the source.** One private claude.ai Design canvas shows all the frames as artboards: one
canvas page per file, one artboard per frame, plus an intro and an open-points artboard per page.
[scripts/mockups-to-canvas.mjs](../../../scripts/mockups-to-canvas.mjs) generates it from the
files (ADR-067). The canvas is only the published view, where the user reviews and sends comments
to Claude. The registry [docs/mockups/README.md](../../../docs/mockups/README.md) maps each page
to its file and its source paths.

The canvas belongs to one owner: a new artifact is private to the claude.ai account that
publishes it. So its link is not in the public repo, but in the gitignored
`.claude/settings.local.json` of the owner's checkout (ADR-064, ADR-067). This checkout:
!`jq -r '.env.KNIHA_JAZD_MOCKUPS_ARTIFACT // "NOT SET"' "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/.claude/settings.local.json" 2>/dev/null || echo "NOT SET"`

A UI change goes into the mockup file first. The code follows after the user approves it.

## When to use

- A task adds or changes a screen, a dialog, a column, a button or a state the user sees.
- The user asks to see the design before the code.
- `/verify` lists commits under "Mockup Drift" for a page.

Skip it for a change that the user cannot see (backend logic, tests, refactors).

## The one rule

**Change the file, then generate and publish the canvas.** Never edit an artboard on the
canvas, by hand or with a tool. A canvas edit is not in git, and the next publish from the
files erases it.

Each frame has a fixed name, `data-board="<page>-<slug>"` on its `section.frame` (PC) or
`section.screen` (phone). The name is the artboard file `project/<name>.dc.html`. Never change
the name of an existing frame: the artboard would lose its comments and its position. A new
frame gets a new unique name. The generator stops on a missing or a repeated name.

### Publish, canvas link set

1. Read the live index: Artifact `read` with `url` = the link above and `path`
   `project/canvas.json`. The result names the saved local file.
2. Generate into an empty scratch folder `<out>`, merged with the live index. Use `--pages` to
   generate only the changed pages:
   ```bash
   node scripts/mockups-to-canvas.mjs --out <out> --live <saved canvas.json> [--pages mobil,fuelio]
   ```
   The script prints `files` (the artboards it wrote) and `removed` (artboards of these pages
   that are no longer in the source).
3. One Artifact publish: `url` = the link, `root` = `<out>`, `file_path` =
   `<out>/project/canvas.json`, `files` = each printed file mapped to itself, and each `removed`
   path mapped to `null`. A short `label`.
4. If the publish is refused because the canvas changed: read the file that the refusal names.
   If somebody edited an artboard, show the user the change. Copy it into the HTML file if the
   user wants it, then generate and publish again. Use `force` only if the user says so.

### Publish, link "NOT SET" (a new owner or a new checkout)

1. Read the file `.claude/settings.local.json` of the main checkout yourself (not of a
   worktree). "NOT SET" means that the key is not in the file. Never make a second canvas for
   an owner who has one. On a second machine of the same owner, copy the env var line from the
   first machine.
2. Create the canvas: Artifact publish with `type_url` = the Design type (from
   `action: "quickstart"`, `intent: "design"`), `title` "Kniha jázd: UI mockups",
   `auto_open: "after_first_write"`, and nothing else.
3. Generate all pages without `--live`, then publish as in step 3 above.
4. Merge the link into the file. Do not overwrite it: it also holds the permissions.
   ```bash
   f="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/.claude/settings.local.json"; [ -f "$f" ] || echo '{}' > "$f"
   jq --arg u "<canvas link>" '.env = ((.env // {}) + {"KNIHA_JAZD_MOCKUPS_ARTIFACT": $u})' "$f" > "$f.new" && mv "$f.new" "$f"
   ```
5. Give the user the canvas link. The env var loads at the next session start; the line
   above reads the file, so it works at once.

If the session has no Artifact tool, change the file anyway and say that the canvas is not
published. The user can publish later, or review the file in a browser.

### Check the generator output

The generator measures each frame in headless Chrome (webdriverio). If you change the
generator or the shared CSS, compare a board with its source frame: open both in Chrome and
compare the position, size and colour of each element under the frame. On 2026-11-17 all 80
artboards matched in the light and the dark theme.

### Local server, only on request

Use it only if the user asks for it, or if the artifact does not open for the user. Run
`python3 -m http.server 8090 --bind 0.0.0.0 --directory docs/mockups` in the background, then
give the Tailscale address of the host (`ip -4 addr show tailscale0`). The LAN address can be
unreachable from the user's PC.

## Steps: draw a proposal

1. Find the page in the registry. A new page has no row: see "A new page" below.
2. Read the Svelte files of the change, so that the frame starts from the real current state.
3. Change the mockup file:
   - Wrap each changed element or frame in `<div class="open"><span class="tag">Návrh (task NN)</span> ... </div>`.
   - A new frame gets the tag in its `<h2>` too, and a new unique `data-board` name.
   - Add the notes for the change as `<li class="prop">`, with the new behaviour in `<b>`.
   - Keep the other frames as they are.
4. Generate and publish the page (`--pages <page>`). Give the user the canvas link, name the
   page file and the changed artboards, and list what changed. Ask for the review.
5. A comment that the user sends to Claude on the canvas reaches this session. Read it with
   the `ArtifactComments` tool. Its anchor names an artboard, `project/<name>.dc.html`: find
   the frame with `grep -n 'data-board="<name>"' docs/mockups/*.html`. The `<page>-intro` and
   `<page>-open-points` artboards are the page header and the "Open points" section. Change
   that file, generate and publish again, then reply to the comment and resolve it.
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
3. Commit the file, then generate and publish the page.
4. `bash scripts/mockup-drift.sh` must not list the page.

## A dropped feature

Restore the file from git (`git checkout <commit before the proposal> -- docs/mockups/<page>.html`,
or revert the proposal commit). Then generate and publish the page: a frame that is gone
comes back in `removed`, and the publish sends it as `null`.

## A new page

1. Copy an existing PC mockup file: the shared CSS between "Shared PC mockup CSS" and "End of
   the shared CSS", the page header, the nav, and the theme script at the end.
2. Add a row to the registry with the page, the file and the source paths.
3. Add the new page to the nav of the other files as a plain relative link
   (`<a href="<file>">`). The link works on GitHub Pages and from a clone. The generator drops
   the nav from the artboards.
4. Add a card for the page to [docs/mockups/index.html](../../../docs/mockups/index.html), the
   start page of GitHub Pages.
5. Give each frame a `data-board` name that starts with the file name (`<page>-...`).
6. Generate and publish the new page. The generator reads the pages from the registry, so the
   new page becomes a new canvas page.

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
| Edit an artboard on the canvas | Change the file, then generate and publish. |
| Rename the `data-board` of an existing frame | Keep the name. A rename loses the comments and the position. |
| Publish `canvas.json` without `--live` to an existing canvas | Read the live index and merge it, or the user's notes are lost. |
| Put the canvas link in a repo file | Keep it in `.claude/settings.local.json` only (ADR-064). |
| Make a new canvas because the env var is empty | Read `settings.local.json` first. |
| Draw the proposal as if it is built | Mark it with `open`, `Návrh (task NN)` and `li.prop`. |
| Leave the mark after the merge | Remove it, commit, publish. `/move-to-done` checks it. |
| Copy a label from a doc | Grep `sk/index.ts`. Docs can be out of date. |
| Write a backend value (300 m, 5 %) in a note | Name the BIZ/ADR id or the constant. |
