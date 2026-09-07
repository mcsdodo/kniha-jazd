---
name: query-sqlite-db
description: Use when inspecting the logbook SQLite database directly for debugging - when data state must be confirmed against the file the server actually opened, or when a sqlite3 query returns "unable to open database"
---

# Query SQLite Database

## Overview

Direct SQLite queries for debugging data state. The app is a container with one
`/data` volume ([ADR-030](../../../DECISIONS.md)), so the database is a plain file on
the host. Query that file.

## Database Paths

`kniha-jazd-web` resolves the path at
[src-tauri/web/src/main.rs:24-30](../../../src-tauri/web/src/main.rs):
`DATABASE_PATH`, else `$KNIHA_JAZD_DATA_DIR/kniha-jazd.db`, else `/data/kniha-jazd.db`.

| Context | Path |
|---------|------|
| **Docker** (compose or `docker run`) | `./data/kniha-jazd.db` **on the host** - the bind mount of `/data`, see [docker-compose.web.yml](../../../docker-compose.web.yml) |
| **Local dev** | `$KNIHA_JAZD_DATA_DIR/kniha-jazd.db`, falling back to `/data` when unset |
| **Explicit override** | `$DATABASE_PATH` |
| **Integration tests** (spawned mode) | a fresh `mkdtemp` directory, deleted after the run - see `onPrepare` in [wdio.server.conf.ts](../../../tests/integration/wdio.server.conf.ts) |

To ask the running server instead of deducing, call the `get_db_location` RPC command.
It reports the path the process actually opened.

## Quick Reference

```bash
# Query (Docker default)
sqlite3 ./data/kniha-jazd.db "SELECT * FROM vehicles;"

# Schema
sqlite3 ./data/kniha-jazd.db ".schema receipts"

# Tables
sqlite3 ./data/kniha-jazd.db ".tables"
```

## `docker exec` does not work

The runtime stage of [Dockerfile.web](../../../Dockerfile.web) installs only
`ca-certificates` and `curl`. There is no `sqlite3` binary in the image. Query the
bind-mounted file on the host.

## Key Tables

| Table | Key Columns |
|-------|-------------|
| `vehicles` | `id`, `name`, `tp_consumption` |
| `trips` | `id`, `vehicle_id`, `start_datetime`, `end_datetime`, `fuel_liters` |
| `receipts` | `id`, `trip_id`, `receipt_datetime`, `liters`, `total_price_eur` |

## Common Mistakes

| Error | Fix |
|-------|-----|
| `sqlite3: command not found` | `sudo apt install sqlite3` |
| `sqlite3` inside the container fails | The image has no `sqlite3`. Query the host bind mount. |
| Guessing which file the server opened | Call the `get_db_location` RPC command. |
| Column `date` not found | Use `receipt_datetime` or `start_datetime` |
| "unable to open database" | Check the path for typos. Confirm the file exists. |
