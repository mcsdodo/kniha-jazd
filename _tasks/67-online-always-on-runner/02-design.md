**Date:** 2026-08-07
**Subject:** Run kniha-jazd 24/7 as a homelab web service (single source of truth for DB, receipts, settings)
**Status:** Planning

# Design: Always-On Homelab Deployment

## Summary

Server mode is **already fully built** (tasks [55](../_done/55-server-mode/) + 33): a headless
[kniha-jazd-web](../../src-tauri/web/src/main.rs) binary (axum, no Tauri shell),
[Dockerfile.web](../../Dockerfile.web), an RPC API (`POST /api/rpc`) and a
browser frontend with capability gating. This task is therefore *not* an app feature —
it is a **deployment + data-consolidation** task with one small parity fix in the app.

Target state:

```
Browser / API client (any device on LAN or Tailscale)
        │  https://kniha-jazd.lacny.me
        ▼
Caddy (bare metal, 192.168.0.20)  ← route pushed by caddy-agent on infra LXC
        │  http://192.168.0.112:3456
        ▼
infra LXC (192.168.0.112, Komodo-managed)
  kniha-jazd stack: ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z
        │  /data (LXC-local bind mount)
        ▼
  /data/kniha-jazd.db + /data/receipts/ + /data/backups/ + /data/local.settings.json
```

Outbound integrations keep working unchanged — better, in fact, since they become
LAN-internal: Paperless (`https://documents.lacny.me`), Home Assistant
(`https://homeassistant.lacny.me`), Gemini API (internet).

## Decisions (confirmed with user, 2026-08-07)

| # | Decision | Choice |
|---|----------|--------|
| 1 | Exposure | **LAN + Tailscale only** via `caddy: kniha-jazd.lacny.me` label. No Dockflare/public tunnel. Matches ADR-017 (no auth, trusted network). |
| 2 | Image delivery | **CI-published image**: GH Actions in this repo builds `Dockerfile.web` on release tag → pushes `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` + `latest`. Repo is public → anonymous pulls. Homelab compose pins the version tag; deploy = bump tag + `komodo.ps1 -Stack kniha-jazd`. |
| 3 | Receipts intake | **Paperless-only going forward.** Legacy `doklady` images copied once to `/data/receipts` with a DB path rewrite so history keeps rendering. Gemini folder-scan stays functional against `/data/receipts` but is no longer the primary flow. |
| 4 | Desktop apps | **Browser-only after migration** — hard requirement: *ALL features must work in the webapp* (parity audit below). The gdrive DB folder gets archived to prevent split-brain. |

## Current prod data (verified on this PC)

| Data | Today | After migration |
|------|-------|-----------------|
| SQLite DB | `G:\My Drive\Techlab\Kniha Jazd\db\kniha-jazd.db` (custom_db_path, gdrive) | `/data/kniha-jazd.db` |
| Backups | next to DB in gdrive | `/data/backups/` (app) + nightly vzdump of the LXC |
| Receipt images | `G:\My Drive\Techlab\Kniha Jazd\doklady` (gdrive) | `/data/receipts/` |
| local.settings.json | `%APPDATA%\com.notavailable.kniha-jazd\` per PC (Gemini key, HA + Paperless URLs/tokens) | `/data/local.settings.json` (single copy; `KNIHA_JAZD_DATA_DIR=/data`) |
| Lock file (multi-PC) | gdrive lock dance | obsolete — single server writer |

## Feature parity audit (webapp vs desktop)

Verified against `capabilities_handler` in [server/mod.rs](../../src-tauri/core/src/server/mod.rs) and frontend gating:

| Capability | Server mode today | Verdict |
|------------|-------------------|---------|
| Trips, vehicles, receipts, magic fill, suggestions, exports data | ✅ full RPC coverage (68/72 commands) | OK |
| Export | `export_to_browser` is desktop-only, but browser mode falls back to `export_html` and renders/downloads in-page ([+page.svelte](../../src/routes/+page.svelte) L178-188) | OK |
| Backup create/list/info/delete/retention | ✅ in dispatcher | OK |
| **Restore backup** | ❌ excluded (`restore_backup: false`) | **GAP — work item R1.** Desktop impl is a plain `fs::copy` over the DB ([backup.rs](../../src-tauri/core/src/commands_internal/backup.rs) L438-454); enable the same in the server dispatcher + flip the capability flag. Must force a WAL checkpoint / reopen the pool after copy so the running server picks up the restored file safely. |
| `open_external` | ❌ — used for export preview (has fallback) and Paperless doc links | OK if Paperless links render as plain `<a target="_blank">` in browser mode — **verify during rollout (checklist C3)** |
| `move_database` / `reset_database_location` | ❌ | Irrelevant on server — data dir is fixed by design (this task removes the need) |
| Auto-updater | ❌ | Irrelevant — updates ship as image tags via Komodo |
| File dialogs (folder pickers) | ❌ | Irrelevant — receipts folder is fixed `/data/receipts`; `set_receipts_folder_path` RPC still exists for the one-time setup |
| Theme / auto-update prefs | ⚠ stored in *default* app-data dir, not `KNIHA_JAZD_DATA_DIR` ([settings-architecture.md](../../docs/features/settings-architecture.md) notes this) | **Work item R2** — honor `KNIHA_JAZD_DATA_DIR` so theme survives container recreation. Small fix. |

CORS note: the browser UI is served from the same origin (`https://kniha-jazd.lacny.me`),
so the LAN-only CORS predicate (which only matches `http://` RFC-1918 origins) never
applies — same-origin requests aren't subject to CORS. API scripts send no Origin
header and are likewise unaffected. No change needed.

## Work items

### A. This repo (kniha-jazd)

- **R1 — Enable `restore_backup` in server mode** (TDD: dispatcher test first).
  Add to sync dispatcher, set `restore_backup: true` in capabilities, ensure DB
  connections are safely cycled after the file copy (checkpoint/reopen). This is the
  only true parity gap.
- **R2 — Theme/auto-update prefs honor `KNIHA_JAZD_DATA_DIR`** (small fix + test).
- **R3 — CI: publish web image.** New workflow (or job in `release.yml`) on `v*` tags:
  `docker build -f Dockerfile.web` → push `ghcr.io/mcsdodo/kniha-jazd-web:{version}`
  and `:latest` using `GITHUB_TOKEN` (`packages: write`). Make the package public once.
  x86_64 only (all Docker LXCs are amd64).
- **R4 — Migration helper script** (`scripts/migrate-to-server.*`): one-time SQL to
  rewrite `receipts.file_path` prefixes
  (`G:\My Drive\Techlab\Kniha Jazd\doklady\X.jpg` → `/data/receipts/X.jpg`,
  backslash→slash). Runs against a copy; verify counts before/after.
- **R5 — Docs**: feature doc update ([server-mode.md](../../docs/features/server-mode.md) gains a "homelab deployment"
  section or new `docs/features/homelab-deployment.md`), [CHANGELOG](../../CHANGELOG.md) entry, `/decision`
  entry (ADR: homelab instance is the canonical deployment; desktop demoted to
  browser client).

### B. home.notavailable repo (homelab)

Follow `/adding-new-service` skill there. New stack `compose.stacks/infra/kniha-jazd/`:

```yaml
services:
  kniha-jazd:
    image: ghcr.io/mcsdodo/kniha-jazd-web:v0.39.0   # pinned, bumped per release
    ports:
      - "3456:3456"        # REQUIRED — caddy upstream is HOST_IP:3456
    volumes:
      - ${KNIHA_JAZD_DATA_DIR}/:/data
    environment:
      - GEMINI_API_KEY=${GEMINI_API_KEY}   # already in core.config.toml
    labels:
      autoheal: true
      caddy: ${KNIHA_JAZD_DOMAIN}          # kniha-jazd.lacny.me
      caddy.reverse_proxy: "{{upstreams 3456}}"
    restart: always        # autoheal-managed ⇒ never unless-stopped (task 116 rule)
```

- `komodo.toml` `[[stack]]` entry: `server_id = "infra"`, `after = ["caddy-infra"]`,
  env vars from `core.config.toml`.
- Data dir: **LXC-local disk** (e.g. `/opt/kniha-jazd/data`), *not*
  `/mnt/shared_configs` — SQLite on NFS risks locking corruption, and the standing
  rule forbids new writers on `.201` exports. LXC 108/112 is vzdump-backed nightly.
- Image already ships a `HEALTHCHECK` on `/health` — autoheal works out of the box.
- Container logs go to stdout → picked up by the existing alloy/observability stack.
- Optional follow-up (not in scope): rclone push of `/data/backups` → gdrive for an
  off-site copy.

### C. Migration runbook (one evening, reversible)

1. Deploy the stack with an empty `/data`; verify `https://kniha-jazd.lacny.me/health`
   and that the UI loads (fresh DB).
2. Stop using desktop apps; make a final desktop backup.
3. Copy data from this PC:
   `scp "G:\My Drive\Techlab\Kniha Jazd\db\kniha-jazd.db"` → server `/data/`,
   `doklady/*` → `/data/receipts/`.
4. Run the R4 path-rewrite SQL; spot-check receipt images render (checklist below).
5. Seed `/data/local.settings.json`: Gemini key, `receipts_folder_path: "/data/receipts"`,
   HA + Paperless URLs/tokens (copy from this PC's file). Restart container.
6. **Parity checklist** — walk every feature in the browser:
   C1 trips grid + calculations, C2 receipts list + images, C3 Paperless doc links
   open, C4 magic fill (Gemini), C5 HA integration test button, C6 export HTML/print,
   C7 backup create + **restore** (R1), C8 settings save, C9 year switching.
7. Archive `G:\My Drive\Techlab\Kniha Jazd\db` → rename to `db_MIGRATED-2026-MM-DD`;
   desktop apps now fail to open it (intentional). Optionally uninstall.
8. Repoint API consumers (the `kniha-jazd-trip-logging` skill) to
   `https://kniha-jazd.lacny.me/api/rpc`.

Rollback at any point before step 7: stop the stack, keep using gdrive DB — nothing
was mutated at the source (all copies).

## Risks / notes

- **No auth on a LAN-reachable writable app**: accepted (ADR-017 trust model,
  user-confirmed). Public exposure explicitly rejected; revisit with CF Access +
  Dockflare if ever needed.
- **Restore-over-live-DB semantics** (R1): desktop already does `fs::copy` onto an
  open DB; server must checkpoint/reopen to avoid serving a stale WAL. Covered by a
  dispatcher-level test.
- **Port 3456 collision** on infra LXC: verify free before komodo deploy.
- **Concurrent writers gone**: the gdrive lock-file feature stays in the codebase for
  desktop users but is dead weight for this deployment — no action.

## Related

- [server-mode.md](../../docs/features/server-mode.md) — existing server-mode capability (tasks [55](../_done/55-server-mode/), 33)
- ADR-015 (RPC over REST), ADR-016 (`_internal` extraction), ADR-017 (LAN-only, no auth) in [DECISIONS.md](../../DECISIONS.md)
- [home.notavailable](https://github.com/mcsdodo/home.notavailable): `compose.stacks/CLAUDE.md` (Caddy routing, autoheal rules),
  `_komodo/CLAUDE.md` (secrets), task 116 (restart-policy rule)
