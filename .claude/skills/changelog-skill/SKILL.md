---
name: changelog
description: Use after completing user-visible features, fixes, or behavior changes - NOT for internal docs (CLAUDE.md, DECISIONS.md, _tasks/)
model: Sonnet
context: fork
---

# Changelog Update Skill

Updates [CHANGELOG.md](../../../CHANGELOG.md) with changes as they happen, not later.

The changelog has two readers:

- **The user** of the logbook. The sections `Pridané` to `Bezpečnosť` are for this reader.
- **The integrator** who runs the Docker image and upgrades it from one release to the
  next. The block `### Pokyny k aktualizácii` is for this reader.

## When to Use

- After completing a new feature
- After fixing a bug
- After changing existing behavior
- After any change to the contract with the integrator (see the checklist below),
  even when the user sees nothing
- Before committing completed work

## When NOT to Use

- Planning documents (`_tasks/` folder)
- Design docs, task plans, brainstorming notes
- Internal documentation (CLAUDE.md, DECISIONS.md updates)
- Tech debt tracking files
- Refactors, tests and CI changes that change nothing for the user or the integrator

## 1. Integrator Checklist - answer it first

Answer each question from the diff of the change. Do not guess.

| Question | How to check |
|----------|--------------|
| Does it add a migration? (folder [migrations](../../../src-tauri/core/migrations/)) | `git status --short src-tauri/core/migrations` or `git diff --name-status <base> -- src-tauri/core/migrations` |
| Does it add, remove or rename an env var, or change its meaning or default? | `pub const` in `env_vars` in [constants.rs](../../../src-tauri/core/src/constants.rs) and [settings.rs](../../../src-tauri/core/src/settings.rs); `std::env::var` anywhere in [src-tauri](../../../src-tauri/), for example [web/src/main.rs](../../../src-tauri/web/src/main.rs) |
| Does it discard data? | a migration with `DROP` or `DELETE`, or a removed feature that stored data |
| Does it change the image contract? | `ENV`, `VOLUME`, `EXPOSE`, `HEALTHCHECK`, `CMD` in [Dockerfile.web](../../../Dockerfile.web); paths under `/data`; the port; the published tags |
| Does it need a new outbound network target or an API key? | a new external service, for example a routing or geocoding API |

If all answers are "no", skip step 2.

## 2. Update the Integrator Block

`## [Unreleased]` always starts with this block. Keep all five rows. Replace the text of
a row, do not delete the row:

```markdown
## [Unreleased]

### Pokyny k aktualizácii
- **Potrebný zásah:** nie
- **Premenné prostredia:** bez zmeny
- **Migrácie databázy:** žiadne
- **Strata údajov:** žiadna
- **Obraz, zväzok, port:** bez zmeny
```

Rules for the rows:

- **Potrebný zásah** - `áno` if the integrator must do something before or after the
  upgrade (set a variable, export data, change the compose file). Then write the steps
  in order, as an imperative list below the five rows.
- **Premenné prostredia** - name each variable word for word, with `pridaná`,
  `odstránená`, `premenovaná z X na Y` or `zmenený význam`. Say if it is optional and
  what happens without it. Example: `pridaná SYGIC_API_KEY (voliteľná; bez nej mapa
  trasy použije verejný OSRM)`.
- **Migrácie databázy** - the count, and this sentence, because every migration is
  one-way: `Návrat na starší obraz otvorí databázu len na čítanie. Späť vedie záloha
  <DATA_DIR>/backups/kniha-jazd-backup-*-pre-migration-vX.Y.Z.db, ktorú aplikácia
  uloží pred migráciou.`
- **Strata údajov** - what is lost, and the export command to run **before** the
  upgrade.
- **Obraz, zväzok, port** - changed `ENV`, `VOLUME`, `EXPOSE`, paths or tags.

**Remove an env var in two steps.** First add a `### Zastarané` entry that names it, in
one release. Remove it in a later release, and list it as `odstránená` in the block.

In this block, write env names, paths and commands word for word. The integrator copies
them into a compose file or a shell.

## 3. Add the Entry for the User

Add the entry under `## [Unreleased]`, below the integrator block, in this order:

| Section | Use For |
|---------|---------|
| `### Pridané` | New features, new capabilities |
| `### Zmenené` | Modified behavior, updates to existing features |
| `### Zastarané` | Features or env vars that a later release removes |
| `### Odstránené` | Removed features |
| `### Opravené` | Bug fixes, corrections |
| `### Bezpečnosť` | Fixes for a vulnerability or an exposed secret |

Create a section if it does not exist yet. Keep this order.

## 4. Commit With Your Changes

Include the changelog update in the same commit as the code change:

```bash
git add CHANGELOG.md src/...
git commit -m "feat: {description}"
```

## Writing Good Entries

**Do:**
- Write in Slovak, with diacritics
- Start with a short bold title, then one to three sentences
- Keep an entry under about 400 characters
- Focus on what the user sees
- Use consistent terminology

**Don't:**
- Put file names, function names or internal refactors in a user entry
- Write in English (except technical terms)
- Use typographic characters: write `-` or `--` (not an em-dash), straight quotes `"`
  (not the Slovak typographic low and high quotes), and `...` (not an ellipsis glyph)

Env var names, paths and commands are correct in the integrator block. They are also
correct in a user entry when the user must type them.

## Examples

```markdown
### Pokyny k aktualizácii
- **Potrebný zásah:** áno
- **Premenné prostredia:** odstránená `GEMINI_API_KEY` (aplikácia ju ignoruje)
- **Migrácie databázy:** 2 nové. Návrat na starší obraz otvorí databázu len na čítanie. ...
- **Strata údajov:** tabuľka `receipts` sa zruší
- **Obraz, zväzok, port:** bez zmeny

Pred aktualizáciou:
1. Vyexportujte doklady: `sqlite3 -header -csv data/kniha-jazd.db "SELECT * FROM receipts;" > receipts.csv`
2. Odstráňte `GEMINI_API_KEY` z compose súboru.

### Pridané
- **Export do PDF** - prehľad celej knihy jázd v jednom súbore.

### Opravené
- **Výber roka** - zoznam rokov sa po pridaní jazdy hneď aktualizuje.
```

## Notes

- Update the changelog IMMEDIATELY when completing work
- `/release` moves `[Unreleased]` to a version section and refuses to release without
  the integrator block
- The block is per release. The integrator decides to run `:latest` (releases) or
  `:main` (every green commit); for `:main`, `[Unreleased]` is the current state
