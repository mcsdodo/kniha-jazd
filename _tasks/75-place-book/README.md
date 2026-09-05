**Date:** 2026-09-05
**Subject:** Place book — curated place list with coordinates
**Status:** Planning

## This folder is intentionally almost empty in the repository

Task 75 turns the free-text `origin` / `destination` strings into a curated place book:
one entry per place, holding coordinates, editable from a **Miesta** section in
[settings](../../src/routes/settings/+page.svelte), and feeding both the trip form's
autocomplete and the route maps built in
[task 72](../72-route-map-origin-destination/).

Before any of it is built, the existing place strings get a one-off cleanup — three
spellings of one petrol station, `SNV` and `BA` abbreviations no geocoder resolves, a
couple of typos. That cleanup is a data fix, not a feature: the app never learns to
merge places, because it only needs doing once.

The working files for both — the analysis scripts, the editable fix list, the writeup
and the generated SQL — **exist only on the maintainer's machine**, excluded by
[.gitignore](./.gitignore). They name real trip endpoints, including home and office
addresses, together with trip ids and dates spanning 2023–2026. This repository is
public, so none of that is committed.

The consequence is worth stating plainly: **this task cannot be reproduced from the
repository alone.** It needs the production database, which is not here and will not be.

## What is committed

Nothing but this note and the [.gitignore](./.gitignore) that enforces it. The design
decisions that outlive the cleanup — how the book resolves a name, what it stores, when
it asks — will land in [DECISIONS.md](../../DECISIONS.md) and in the feature doc under
[docs/features/](../../docs/features/), written so they carry no addresses.
