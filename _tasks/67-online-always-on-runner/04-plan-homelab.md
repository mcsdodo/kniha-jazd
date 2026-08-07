**Date:** 2026-08-07
**Subject:** Deploy kniha-jazd web app as always-on Komodo stack + migrate data off gdrive
**Status:** Planning

# Kniha Jázd Homelab Deployment Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
>
> **This plan is written for the home.notavailable repository.** Copy it there (e.g. into a `_tasks/{NN}-kniha-jazd-stack/` folder per that repo's conventions) and execute it from that repo. It assumes zero knowledge of the kniha-jazd codebase — everything needed is in this file.

**Goal:** Run the [kniha-jazd](https://github.com/mcsdodo/kniha-jazd) vehicle-logbook web app 24/7 on the infra LXC, reachable at `https://kniha-jazd.lacny.me` (LAN + Tailscale, no public exposure), with its SQLite DB, settings, and backups living in one place on the server.

**Architecture:** Image-based Komodo stack (no build) pulling the CI-published `ghcr.io/mcsdodo/kniha-jazd-web` image (public package, anonymous pull). Single container, port 3456 published on the host (verified free on 192.168.0.112), caddy-agent label routes `kniha-jazd.lacny.me`. Data in an LXC-local bind mount `/opt/kniha-jazd/data` — deliberately **not** `/mnt/shared_configs` (SQLite over NFS risks lock corruption, and no new writers on `.201` exports). App is a headless Rust (axum) binary serving both the SPA and a JSON-RPC API; it has **no authentication by design** (LAN trust model) — do NOT add dockflare labels.

**Tech Stack:** Komodo (stack + secrets), Docker Compose, caddy-agent labels, ghcr.io.

**Prerequisite:** The kniha-jazd release publishing the image must be done first (app-side plan Task 3/5 — release ≥ **v0.39.0**), and the ghcr package made public. Verify before starting:

```bash
docker manifest inspect ghcr.io/mcsdodo/kniha-jazd-web:v0.39.0 > /dev/null && echo OK
```

---

### Task 1: Add the Gemini API key to Komodo secrets

The app uses Google Gemini for receipt OCR ("magic fill"). The key lives on the Windows PC in `%APPDATA%\com.notavailable.kniha-jazd\local.settings.json` (`gemini_api_key` field).

**Files:**
- Modify: `compose.stacks/_komodo/core.config.toml`

**Step 1:** Add (value copied from the PC's local.settings.json — never commit the key anywhere else):

```toml
KNIHA_JAZD_GEMINI_API_KEY = "<gemini_api_key from PC>"
```

**Step 2:** Run the secrets check: `compose.stacks/_komodo/check-secrets.ps1` (if the repo workflow requires it). Do not commit `core.config.toml` if it is gitignored — follow the repo's existing secret-handling convention.

---

### Task 2: Create the stack

**Files:**
- Create: `compose.stacks/infra/kniha-jazd/docker-compose.yml`
- Modify: `compose.stacks/_komodo/komodo.toml`

Follow the `/adding-new-service` skill checklist alongside these concrete contents.

**Step 1: Write the compose file**

```yaml
services:
  kniha-jazd:
    image: ghcr.io/mcsdodo/kniha-jazd-web:v0.39.0   # pin; bump per app release
    container_name: kniha-jazd
    ports:
      - "3456:3456"          # REQUIRED — caddy-agent emits HOST_IP:3456 as upstream
    volumes:
      - ${KNIHA_JAZD_DATA_DIR}:/data
    environment:
      - PORT=3456
      - KNIHA_JAZD_DATA_DIR=/data
      - DATABASE_PATH=/data/kniha-jazd.db
      - GEMINI_API_KEY=${GEMINI_API_KEY}
    labels:
      autoheal: true
      caddy: ${KNIHA_JAZD_DOMAIN}
      caddy.reverse_proxy: "{{upstreams 3456}}"
    restart: always          # autoheal-managed: unless-stopped would make a failed heal permanent (task 116)
```

Notes: the image ships its own `HEALTHCHECK` (curl `/health`), so autoheal works out of the box; container logs go to stdout for the observability stack.

**Step 2: Register the stack in komodo.toml**

Add in the infra stacks section:

```toml
[[stack]]
name = "kniha-jazd"
description = "Kniha jázd — vehicle logbook web app (kniha-jazd.lacny.me)"
tags = ["infra"]
deploy = true
after = ["caddy-infra"]
[stack.config]
server_id = "infra"
git_provider = "github.com"
git_account = "mcsdodo"
repo = "mcsdodo/home.notavailable"
branch = "main"
run_directory = "compose.stacks/infra/kniha-jazd"
file_paths = ["docker-compose.yml"]
config_files = []
auto_pull = false
environment = """
KNIHA_JAZD_DOMAIN=kniha-jazd.lacny.me
KNIHA_JAZD_DATA_DIR=/opt/kniha-jazd/data
GEMINI_API_KEY=[[KNIHA_JAZD_GEMINI_API_KEY]]
"""
```

**Step 3: Update `compose.stacks/service-map.md`** — add the stack under the infra LXC (192.168.0.112) per that file's format.

**Step 4: Commit** (per repo convention — planning docs and stack files, no secrets):

```bash
git add compose.stacks/infra/kniha-jazd/ compose.stacks/_komodo/komodo.toml compose.stacks/service-map.md
git commit -m "feat: add kniha-jazd stack (always-on vehicle logbook, task 67 in kniha-jazd repo)"
```

---

### Task 3: Deploy and verify (empty data — no migration yet)

**Step 1: Deploy via the `/deploying-stacks` skill** (never `ssh && docker compose up`):

```powershell
cd compose.stacks/_komodo
.\komodo.ps1 -Stack kniha-jazd
```

**Step 2: Verify container + route**

```bash
ssh root@192.168.0.112 "docker ps --filter name=kniha-jazd --format '{{.Names}} {{.Status}}'"
# expect: kniha-jazd Up ... (healthy)
curl -s https://kniha-jazd.lacny.me/health
# expect: ok
curl -s -X POST https://kniha-jazd.lacny.me/api/rpc -H 'Content-Type: application/json' \
  -d '{"command":"get_vehicles","args":{}}'
# expect: []  (fresh empty DB)
```

Open `https://kniha-jazd.lacny.me` in a browser — the app UI should load (Slovak, empty state). If 502: `/debugging-caddy-routes` skill.

---

### Task 4: Migrate the data (DB + settings — deliberately NOT receipt images)

Context (decided in the kniha-jazd repo, task 67 design): the DB moves from gdrive; receipt *metadata* is inside the DB; the legacy receipt image files stay behind in gdrive as an archive (in-app previews for those 68 pre-2026-05 receipts will 404 — accepted). New receipts flow via Paperless (`documents.lacny.me`), which the app calls directly.

**Step 1: Close the desktop app on ALL PCs** (it holds the gdrive DB). Verify gdrive has finished syncing.

**Step 2: Copy the DB** (from the Windows PC):

```powershell
scp "G:\My Drive\Techlab\Kniha Jazd\db\kniha-jazd.db" root@192.168.0.112:/opt/kniha-jazd/data/kniha-jazd.db
```

**Step 3: Seed `/opt/kniha-jazd/data/local.settings.json`** on the LXC. Copy the token/key values from the PC's `%APPDATA%\com.notavailable.kniha-jazd\local.settings.json` — do NOT copy the file verbatim (it contains machine-specific `custom_db_path`, `receipts_folder_path`, and server toggles that must not travel):

```json
{
  "gemini_api_key": "<from PC>",
  "theme": "system",
  "auto_check_updates": false,
  "backup_retention": { "enabled": true, "keepCount": 3 },
  "date_prefill_mode": "previous",
  "hidden_columns": ["tripNumber"],
  "ha_url": "https://homeassistant.lacny.me",
  "ha_api_token": "<from PC>",
  "paperless_url": "https://documents.lacny.me",
  "paperless_api_token": "<from PC>",
  "paperless_enabled": true,
  "paperless_field_name_datetime": "receipt_datetime",
  "paperless_field_name_liters": "litres",
  "paperless_field_name_total": "total_amount"
}
```

Key omissions are intentional: no `custom_db_path` (DB is at its default `/data` location now), no `receipts_folder_path` (Paperless-only; folder-scan stays dormant).

**Step 4: Restart the stack** to pick up the DB + settings — redeploy via Komodo (`.\komodo.ps1 -Stack kniha-jazd`), not a manual docker restart.

**Step 5: Verify data arrived**

```bash
curl -s -X POST https://kniha-jazd.lacny.me/api/rpc -H 'Content-Type: application/json' \
  -d '{"command":"get_vehicles","args":{}}'
# expect: JSON array with the real vehicle(s), not []
```

---

### Task 5: Full parity checklist (browser)

Walk every feature at `https://kniha-jazd.lacny.me` — this was a hard requirement ("ALL features in webapp"):

- [ ] C1 Trips grid renders with calculated rates, warnings, fuel remaining; add + edit + delete a test trip
- [ ] C2 Receipts list shows legacy receipt metadata (image previews 404 — expected and accepted)
- [ ] C3 Paperless document links open `documents.lacny.me` docs; invoice picker lists Paperless docs
- [ ] C4 Magic fill (Gemini OCR) works — needs the API key from Task 1/4
- [ ] C5 Home Assistant integration test button succeeds (`homeassistant.lacny.me` reachable from the LXC)
- [ ] C6 Export renders/downloads HTML (print path)
- [ ] C7 Backup: create a backup, then **restore** it (requires app release ≥ v0.39.0), page reloads with restored data
- [ ] C8 Settings save (company info + theme) and survive a stack redeploy (persisted in `/data`)
- [ ] C9 Year switching works across all years present in the data

Any failure here blocks cutover (Task 6) — fix or escalate first.

---

### Task 6: Cutover — retire the gdrive copy

**Step 1:** On the Windows PC, rename `G:\My Drive\Techlab\Kniha Jazd\db` → `db_MIGRATED-2026-MM-DD`. Desktop apps now fail to open it — intentional split-brain prevention. (Rollback before this step = stop the stack, keep using gdrive; nothing at the source was mutated.)

**Step 2:** Repoint API consumers to `https://kniha-jazd.lacny.me/api/rpc` — notably the `kniha-jazd-trip-logging` Claude skill on the PC (it targets the app's local HTTP API).

**Step 3:** Backups sanity: app-level backups land in `/opt/kniha-jazd/data/backups/` (create one via UI and check); the LXC itself is covered by the existing nightly vzdump. Optional follow-up (out of scope): rclone push of the backups dir to gdrive for off-site.

**Step 4:** Mark the task complete per repo conventions (index/status updates), and report completion back to the kniha-jazd repo task folder (`_tasks/67-online-always-on-runner/` — add a status note there).
