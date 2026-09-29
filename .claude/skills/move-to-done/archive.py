#!/usr/bin/env python3
"""Move task folders from _tasks/ to _tasks/_done/ and repoint every path to them.

Usage (from anywhere inside the repo):
    python3 .claude/skills/move-to-done/archive.py 85 86      # archive tasks 85 and 86
    python3 .claude/skills/move-to-done/archive.py --force 41 # archive although Status is not Complete
    python3 .claude/skills/move-to-done/archive.py --check    # link check only, no changes

It does not edit _tasks/index.md (the row text needs judgment) and never commits.
Exit codes: 0 ok, 1 error or new broken link, 2 a Status is not Complete (ask the user).
"""
import datetime
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
TASKS = ROOT / "_tasks"
DONE = TASKS / "_done"
LINK = re.compile(r"\]\(([^)\s#]+)")
UP_LINK = re.compile(r"\]\(((?:\.\./)+)")


def git(*args: str) -> str:
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True).strip()


def find_task(nn: str) -> Path:
    active = sorted(TASKS.glob(f"{nn}-*/"))
    if len(active) == 1:
        return active[0]
    if not active and list(DONE.glob(f"{nn}-*/")):
        sys.exit(f"task {nn} is already in _tasks/_done/")
    sys.exit(f"task {nn}: expected one folder _tasks/{nn}-*, found {len(active)}")


def status_of(folder: Path) -> str:
    task = folder / "01-task.md"
    if not task.exists():
        return "(no 01-task.md)"
    m = re.search(r"^\*\*Status:\*\*\s*(.+)$", task.read_text(encoding="utf-8"), re.M)
    return m.group(1).strip() if m else "(no Status line)"


def is_complete(status: str) -> bool:
    return status.startswith(("Complete", "✅ Complete"))


def release_state(folder: Path) -> tuple[str, str]:
    """Date and release state of the last commit that touched the folder.

    A task that is not Complete was not built: its date is today and it has no release.
    """
    status = status_of(folder)
    if not is_complete(status):
        return datetime.date.today().isoformat(), f"archived unbuilt (Status: {status})"
    rel = folder.relative_to(ROOT).as_posix()
    sha, date = git("log", "-1", "--format=%H %ad", "--date=short", "--", rel).split()
    # --merged HEAD: only tags this checkout can reach, not a later release on another line.
    tags = git("tag", "--contains", sha, "--merged", "HEAD", "--sort=creatordate", "--list", "v*").splitlines()
    if tags:
        return date, f"released in {tags[0]}"
    try:
        subprocess.check_call(["git", "-C", str(ROOT), "merge-base", "--is-ancestor", sha, "origin/main"],
                              stderr=subprocess.DEVNULL)
        return date, "on `main`, not released"
    except subprocess.CalledProcessError:
        return date, "committed locally, not pushed"


def rewrite(path: Path, fn) -> bool:
    old = path.read_text(encoding="utf-8")
    new = fn(old)
    if new != old:
        path.write_text(new, encoding="utf-8")
        return True
    return False


def archive(folder: Path) -> list[Path]:
    name = folder.name
    rel_old = f"_tasks/{name}"
    rel_new = f"_tasks/_done/{name}"
    git("mv", rel_old, rel_new)
    moved = ROOT / rel_new
    changed = [moved]

    # The moved files sit one level deeper: every relative-up link needs one more "../",
    # and a link back into the folder by its repo path must name _done/.
    def fix_moved(text: str) -> str:
        text = UP_LINK.sub(lambda m: f"]({m.group(1)}../", text)
        return re.sub(r"(\]\((?:\.\./)+)" + re.escape(rel_old), r"\1" + rel_new, text)

    for md in moved.rglob("*.md"):
        rewrite(md, fix_moved)

    # Every other tracked file: the repo path, and sibling links from active task folders.
    hits = subprocess.run(["git", "-C", str(ROOT), "grep", "-lI", "-e", rel_old, "-e", f"](../{name}"],
                          capture_output=True, text=True).stdout.split()
    for hit in hits:
        path = ROOT / hit
        if path.is_relative_to(moved) or hit == "_tasks/index.md":
            continue

        def fix_ref(text: str) -> str:
            text = re.sub(r"(?<!_done/)" + re.escape(rel_old), rel_new, text)
            if path.parent.parent == TASKS:  # an active sibling task folder
                text = text.replace(f"](../{name}", f"](../_done/{name}")
            return text

        if rewrite(path, fix_ref):
            changed.append(path)
    return changed


def broken_links(files: list[Path]) -> set[tuple[str, str, str]]:
    """(source file, link as written, resolved absolute path) for each link that resolves to nothing."""
    out = set()
    for f in files:
        for md in ([f] if f.is_file() else f.rglob("*.md")):
            if md.suffix != ".md" or not md.exists():
                continue
            for target in LINK.findall(md.read_text(encoding="utf-8")):
                if re.match(r"[a-z]+:", target):
                    continue
                resolved = os.path.normpath(md.parent / target)
                if not os.path.exists(resolved):
                    out.add((md.relative_to(ROOT).as_posix(), target, resolved))
    return out


def all_markdown() -> list[Path]:
    return [ROOT / p for p in git("ls-files", "*.md").splitlines()]


def report_links(files: list[Path], known_broken: set[str]) -> int:
    """A broken link is new if its resolved path was not already broken before the run."""
    broken = sorted(broken_links(files))
    new = [b for b in broken if b[2] not in known_broken]
    old = [b for b in broken if b[2] in known_broken]
    for src, target, _ in new:
        print(f"BROKEN (new, fix it): {src} -> {target}")
    if old:
        print(f"{len(old)} broken link(s) that were there before, in files this run touched (report, do not fix):")
        for src, target, _ in old:
            print(f"  {src} -> {target}")
    if not new:
        print("no new broken links")
    return 1 if new else 0


def main() -> int:
    args = sys.argv[1:]
    force = "--force" in args
    nums = [a for a in args if not a.startswith("--")]

    if "--check" in args:
        touched = git("diff", "--name-only", "HEAD").splitlines() + git("diff", "--cached", "--name-only").splitlines()
        files = sorted({ROOT / t for t in touched if (ROOT / t).suffix == ".md" and (ROOT / t).exists()})
        # A broken link is new if its target existed at HEAD: the uncommitted move broke it.
        at_head = {os.path.normpath(ROOT / p) for p in git("ls-tree", "-r", "-t", "--name-only", "HEAD").splitlines()}
        files.append(TASKS / "index.md")
        known = {b[2] for b in broken_links(files) if b[2] not in at_head}
        return report_links(files, known)

    if not nums:
        print(__doc__)
        return 1

    folders = [find_task(n) for n in nums]
    not_complete = [(f.name, status_of(f)) for f in folders if not is_complete(status_of(f))]
    if not_complete and not force:
        for name, st in not_complete:
            print(f"STOP: {name} has Status '{st}', not Complete. Ask the user; rerun with --force if they confirm.")
        return 2

    known_broken = {b[2] for b in broken_links(all_markdown())}
    changed = []
    for folder in folders:
        date, state = release_state(folder)
        changed += archive(folder)
        print(f"archived {folder.name}: date {date}, {state}")

    print("\nfiles to stage (plus _tasks/index.md after you edit it):")
    for p in dict.fromkeys(changed):
        print(f"  {p.relative_to(ROOT).as_posix()}")
    print()
    return report_links(changed, known_broken)


if __name__ == "__main__":
    sys.exit(main())
