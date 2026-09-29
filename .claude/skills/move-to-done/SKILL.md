---
name: move-to-done
description: Use when a task folder in _tasks/ is complete, shipped, abandoned or superseded and must be archived to _tasks/_done/ - "move task NN to done", "archive task NN", or a task marked Complete still listed under Active Tasks in _tasks/index.md.
---

# Move a task to `_done/`

## Overview

An archive is one commit. It moves the folder and repoints every path to the folder. It also moves the task's row in [_tasks/index.md](../../../_tasks/index.md) to **Completed Tasks**. The script [archive.py](./archive.py) does the mechanical part and the checks. You write only the index text.

## Steps

1. Run the script with all the task numbers:
   ```bash
   python3 .claude/skills/move-to-done/archive.py 85 86
   ```
   - It moves each folder with `git mv`. It adds one `../` to every relative link in the moved files. It repoints `_tasks/NN-name` in all other tracked files, code comments included.
   - It prints the date and the release state of each task, and the files to stage.
   - **Exit 2:** a `**Status:**` is not `Complete`. Nothing moved.
     - If the user's request says the task is abandoned, dropped or superseded, that is the confirmation. Run it again with `--force`.
     - Otherwise ask the user: "Task NN has Status '<status>', not Complete. Archive it as not built?" If the user confirms, run it again with `--force`.
   - **Exit 1 with `BROKEN (new, fix it)`:** fix those links before you continue.
2. Edit `_tasks/index.md`:
   - Delete the task's row from **Active Tasks**.
   - Add a row at the top of **Completed Tasks**, in this shape:
     `| NN | [Title](./_done/NN-name/) -- <old Notes text>; <state from step 1> | <date from step 1> |`
   - Start the Notes text with a lowercase letter, as the older rows do. Keep the rest of the text as it was.
   - Replace the text of the `**Last updated:**` line with today's date and one sentence that names the archived tasks. Do not keep the old text.
3. If `01-task.md` has a `**Source:**` line that names an item in [_tasks/_TECH_DEBT/](../../../_tasks/_TECH_DEBT/) which still exists, update that item as [_tasks/CLAUDE.md](../../../_tasks/CLAUDE.md) says ("Update tech debt on completion").
4. Run `python3 .claude/skills/move-to-done/archive.py --check`. Expect `no new broken links`.
5. Commit. Stage only the files from step 1, `_tasks/index.md` and a tech-debt file from step 3. Use this message:
   ```
   docs: archive task 85          (one task)
   docs: archive tasks 85 and 86  (more than one)
   ```
   Add the attribution trailer if the session requires one. Do not push. Do not add a CHANGELOG entry: an archive is an internal docs change.
6. Tell the user: the moved folders, the files with repointed paths, the commit SHA, and that you did not push. Give the count of old broken links from step 4, and do not fix them.

## Common mistakes

| Mistake | Correct action |
|---|---|
| Move the folder with plain `mv` | Use the script. It uses `git mv`, so the history follows the files. |
| Fix old broken links in the same commit | Report them only. They belong to other tasks. |
| Archive a task that is not `Complete` without asking | Exit 2 means ask. Tasks archived unbuilt are valid, but only if the user says so. |
| Archive the other complete tasks too | Archive only the tasks that the user named. Name the others in the report. |
