# Task Planning Conventions

Planning docs for complex features live here. NOT in `.claude/plans/`, NOT in `docs/`,
NOT scattered in feature folders.

---

## Default entry point: `/task-plan`

**For any new feature that needs a task folder, invoke `/task-plan` first.**

It chains the right skills in the right order:

```
superpowers:brainstorming  →  01-task.md
        ↓
superpowers:writing-plans  →  02-plan.md
        ↓
        commit
```

`/task-plan` already enforces the next-folder-number procedure, the metadata header,
the required-skill gates (below), and the commit-before-implementation rule. Use it
unless one of these is true:

- You're **adding to an existing task folder** (e.g., a `03-status.md` follow-up).
- You're **explicitly instructed** to bypass it.
- `/task-plan` is **unavailable** in your environment.

In those cases, follow the manual rules below — every rule that `/task-plan`
automates also applies when you do it by hand.

---

## Required skills (apply whether you used `/task-plan` or not)

Two file types in this folder are gated on a skill invocation. **Do not write them
without the skill** — the skills exist precisely to prevent the failure modes you'll
otherwise reach for. `/task-plan` invokes both for you; manual workflows must invoke
them explicitly.

### Writing plans → invoke `superpowers:writing-plans`

**MANDATORY:** Before writing any `*-plan.md`, invoke the `superpowers:writing-plans`
skill via the `Skill` tool. No exceptions.

The skill enforces:

- Required plan header with `superpowers:executing-plans` handoff line
- Bite-sized steps (2–5 minutes each: write test → run → implement → run → commit)
- Exact file paths and complete code blocks (no "add validation" hand-waves)
- TDD ordering for every task (failing test before implementation)
- Explicit commands with expected pass/fail output

**Failure-mode tell:** If you find yourself drafting tasks like "Implement X" without
numbered sub-steps, stop — that's the pattern the skill prevents. Invoke it.

### Writing designs → invoke `superpowers:brainstorming`

**MANDATORY:** Before writing any `*-design.md`, invoke `superpowers:brainstorming` to
validate user intent and get design approval. Do not write a design doc from a
single-shot interpretation of a user's request.

---

## Critical procedures

### Finding the next folder number

**Always check BOTH locations** — completed tasks move to `_done/`:

```
Glob pattern: _tasks/[0-9][0-9]-*
Glob pattern: _tasks/_done/[0-9][0-9]-*
```

Find the highest `{NN}` across BOTH and increment by 1.

**Do NOT use `ls _tasks/` or `Glob _tasks/*`** — these miss subdirectories.

### Update [index.md](./index.md) on every state change

**MANDATORY** — [index.md](./index.md) is the at-a-glance overview agents and humans
use to find tasks. Keep it accurate:

- **Creating a new task folder** → add a row to **Active Tasks** with status 📋
- **Status changes** (Planning → In Progress → Complete) → update the icon
- **Completing a task** → move the row from **Active Tasks** to **Completed Tasks**
- **Moving a folder to `_done/`** → update the link path to `_done/{NN}-{name}/`

Status icons: 📋 Planning · 🟡 In Progress · ✅ Complete · ❌ Blocked

Stage the index update **in the same commit** as the folder change it reflects, so
history reads coherently:

```bash
git add _tasks/{NN}-{name}/ _tasks/index.md
git commit -m "docs: add task and plan for {feature-name}"
```

### Commit planning docs BEFORE implementation

When using workflow skills (brainstorming, writing-plans, etc.), commit task/design/plan
files BEFORE any implementation code:

1. Complete brainstorming, get user approval on the design.
2. Write the plan (via `superpowers:writing-plans` — see above), get user review.
3. Add the new task to [index.md](./index.md) (see rule above).
4. Ask: "Should I create a feature branch for this work?"
5. Commit planning docs + index update together (see commit example above).
6. Then start implementation.

Rationale: design rationale lands in version control before code does, so reviewers
and future agents can read intent independent of execution.

---

## When to create a task folder

**Create for:**
- Multi-file implementations
- Multi-session work
- Complex features requiring design
- Architectural changes

**Skip for:**
- Simple bug fixes
- Single-file changes
- Quick enhancements

---

## File conventions

### Naming

- **Folders:** `{NN}-{descriptive-name}` (find next NN via the procedure above)
- **Files:** `{NN}-{name}.md` — sequentially numbered

| File | Purpose |
|------|---------|
| `01-task.md` | Task description, user story, requirements |
| `02-plan.md` | Step-by-step implementation plan |
| `02-design.md` | Architecture decisions, diagrams (alternative to plan) |
| `03+` | Additional docs (status, notes, code review, etc.) |

### Metadata header

Every task/design/plan file starts with:

```markdown
**Date:** YYYY-MM-DD
**Subject:** Feature description
**Status:** Planning | In Progress | Complete
```

### Lifecycle

1. **Planning** — create `{NN}-{name}/01-task.md` with requirements
2. **Design** — add `02-plan.md` or `02-design.md` (gated on the required skill above)
3. **Implementation** — reference the plan during coding
4. **Completion** — folder moves to `_done/`; keep for historical reference

---

## Maintenance

### Tech debt cross-references

Tasks often originate from items in [_TECH_DEBT/](./_TECH_DEBT/). When implementing
such tasks:

1. **Link to tech debt** in `01-task.md`:
   ```markdown
   **Source:** _TECH_DEBT/03-issue-name.md
   ```
2. **Update tech debt on completion** — change **Status** to "Fixed", add a
   **Decision Log** entry with PR reference:
   ```markdown
   | YYYY-MM-DD | Implemented fix | PR #NNN merged |
   ```
3. **Cross-reference** — link from the tech debt file to the task folder in its
   **Related** section.

See [_TECH_DEBT/CLAUDE.md](./_TECH_DEBT/CLAUDE.md) for tech debt guidelines.

---

## Boundaries

- **Task files** (plans, status, designs) → stay in `_tasks/`
- **Code documentation** (READMEs, API docs) → stay with code (locality principle)
- **Never mix** task planning with permanent code docs

---

## Reference: folder structure

```
_tasks/
├── index.md                          # Task index (kept current)
├── {NN}-{descriptive-name}/          # Active task folders
│   ├── 01-task.md                    # Requirements
│   ├── 02-plan.md                    # Implementation plan
│   ├── 02-design.md                  # OR design doc (alternative to plan)
│   └── 03-*.md                       # Additional docs
├── _done/                            # Completed task folders, archived
│   └── {NN}-{descriptive-name}/      # Same shape as active tasks
├── _TECH_DEBT/                       # Tech debt tracking
│   ├── {NN}-{issue-name}.md          # Individual tech debt items
│   └── CLAUDE.md                     # Tech debt guidelines
└── CLAUDE.md                         # This file
```
