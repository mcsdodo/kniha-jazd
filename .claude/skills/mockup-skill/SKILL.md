---
name: mockup
description: Use before planning or coding a change that adds or changes a screen, a dialog, a column or a UI state - draws the change in the page's mockup file in docs/mockups/ first, publishes it to the claude.ai mockups artifact for review, gets the user's approval, and after the merge turns the proposal into the current state. Also use for "update the mockup", "draw it first", "show me how it would look", or when /verify lists mockup drift.
---

# Mockup first

Each page of the app has a mockup file in [docs/mockups/](../../../docs/mockups/README.md): static
HTML that draws the screens of the page as frames, with English notes under each frame. **The
file is the source.** One private claude.ai artifact holds all the files of the folder, with
`index.html` as its start page. It is only the published view, where the user reviews and sends
comments to Claude. The registry [docs/mockups/README.md](../../../docs/mockups/README.md) maps
each page to its file and its source paths.

The artifact belongs to one owner: a new artifact is private to the claude.ai account that
publishes it. So its UUID is not in the public repo, but in the gitignored
`.claude/settings.local.json` of the owner's checkout (ADR-064). This checkout:
!`jq -r '.env.KNIHA_JAZD_MOCKUPS_ARTIFACT // "NOT SET"' "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/.claude/settings.local.json" 2>/dev/null || echo "NOT SET"`

A UI change goes into the mockup file first. The code follows after the user approves it.

## When to use

- A task adds or changes a screen, a dialog, a column, a button or a state the user sees.
- The user asks to see the design before the code.
- `/verify` lists commits under "Mockup Drift" for a page.

Skip it for a change that the user cannot see (backend logic, tests, refactors).

## The one rule

**Change the file, then publish it.** Never change the artifact with `edit_version`. A direct
artifact edit is not in git, and the next publish from the file erases it.

### Publish, UUID set

1. `get_artifact` with the UUID above, for the live `version`. The tools refuse the short id
   of the link: use the UUID.
2. `list_files` with the UUID. A file keeps the path that this list gives (`mobil.html`).
3. `publish_files` with `slug` = UUID, `base_version` = that version, a short `label`, and only
   the changed files: `files: [{"as": "mobil.html", "path": "<abs path of the file>"}]`. Patch
   mode keeps the other files.

### Publish, UUID "NOT SET" (a new owner or a new checkout)

1. Read the file `.claude/settings.local.json` of the main checkout yourself (not of a worktree). The value above comes from that file,
   so "NOT SET" means that the key is not in the file. Never publish a second artifact for an
   owner who has one. On a second machine of the same owner, copy the env var line from the
   first machine.
2. Read each `docs/mockups/*.html` in full. The publish tool requires it for a file that this
   session did not write.
3. `publish_files` with no `slug`, `title` "Kniha jázd: UI mockups", `icon` "Layout", and all
   the files, each with `as` = its file name (`index.html`, `jazdy.html`, ...).
4. Merge the UUID from the result into the file. Do not overwrite it: it also holds the
   permissions.
   ```bash
   f="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/.claude/settings.local.json"; [ -f "$f" ] || echo '{}' > "$f"
   jq --arg u "<uuid>" '.env = ((.env // {}) + {"KNIHA_JAZD_MOCKUPS_ARTIFACT": $u})' "$f" > "$f.new" && mv "$f.new" "$f"
   ```
5. Give the user the artifact link. The env var loads at the next session start; the line
   above reads the file, so it works at once.

If the session has no `claude-artifacts` tools, change the file anyway and say that the
artifact is not published. The user can publish later, or review the file in a browser.

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
   - A new frame gets the tag in its `<h2>` too.
   - Add the notes for the change as `<li class="prop">`, with the new behaviour in `<b>`.
   - Keep the other frames as they are.
4. Publish it. Give the user the artifact link, name the page file, and list what changed.
   Ask for the review.
5. A comment that the user sends to Claude on the artifact reaches this session. Read it with
   `read_comments` (`thread` for the whole thread). The anchor `file` names the page file.
   Change that file, publish again, then `reply_comment` and `resolve_comment`.
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
   the shared CSS", the page header, the nav, and the theme script at the end.
2. Add a row to the registry with the page, the file and the source paths.
3. Add the new page to the nav of the other files as a plain relative link
   (`<a href="<file>">`). The same link works in the artifact, on GitHub Pages and from a clone.
4. Add a card for the page to [docs/mockups/index.html](../../../docs/mockups/index.html), the
   start page of the artifact and of GitHub Pages.
5. Publish the new file, `index.html` and the other changed files to the existing artifact.

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
| Call a tool with the id from the link | Use the UUID at the top of this skill. |
| Put an artifact link or UUID in a repo file | Keep it in `.claude/settings.local.json` only (ADR-064). |
| Publish a new artifact because the env var is empty | Read `settings.local.json` first. |
| Draw the proposal as if it is built | Mark it with `open`, `Návrh (task NN)` and `li.prop`. |
| Leave the mark after the merge | Remove it, commit, publish. `/move-to-done` checks it. |
| Copy a label from a doc | Grep `sk/index.ts`. Docs can be out of date. |
| Write a backend value (300 m, 5 %) in a note | Name the BIZ/ADR id or the constant. |
