**Date:** 2026-08-07
**Subject:** Task 67 — app-repo work: restore-backup parity + web image CI + docs
**Status:** Complete (v0.39.0 released 2026-08-07; image `ghcr.io/mcsdodo/kniha-jazd-web:v0.39.0` published and anonymously pullable — verified from infra LXC)

# Homelab Deployment (App Side) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the last webapp parity gap (`restore_backup` in server mode) and publish a versioned `kniha-jazd-web` Docker image from CI, so the homelab plan ([04-plan-homelab.md](./04-plan-homelab.md)) can deploy it.

**Architecture:** Server mode already exists ([02-design.md](./02-design.md)). This plan only: (1) registers the existing `restore_backup_internal` in the server RPC dispatcher and flips the capability flag — the frontend restore UI then appears in browser mode with zero frontend changes (it's gated on `$capabilities.features.restoreBackup` and already reloads the page after restore); (2) adds a `docker-image` job to the release workflow pushing `ghcr.io/mcsdodo/kniha-jazd-web`; (3) updates docs.

**Tech Stack:** Rust (axum dispatcher, diesel/SQLite), GitHub Actions, ghcr.io.

**Prerequisite reading:** [.claude/rules/rust-backend.md](../../.claude/rules/rust-backend.md), [docs/features/server-mode.md](../../docs/features/server-mode.md).

---

### Task 1: Enable `restore_backup` in the server RPC dispatcher

The internal function exists ([backup.rs](../../src-tauri/core/src/commands_internal/backup.rs) `restore_backup_internal`, ~L438) and is re-exported via `commands_internal::backup::*`. It is a plain `fs::copy` of the backup over the DB file — identical semantics to desktop (which also runs it against an open connection; the DB uses SQLite's default rollback journal, no WAL, so readers pick up the replaced file on next access via the change counter). Only the dispatcher registration is missing.

**Files:**
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) (Backup section starts ~L629; inline `mod tests` at ~L807 — this file keeps tests inline, follow that local pattern, NOT the `*_tests.rs` companion pattern)

**Step 1: Write the failing test**

Add to `mod tests` in `dispatcher.rs` (after `write_command_fails_in_read_only_mode`). Note: `test_state()` uses an in-memory DB, which can't serve a file-restore test — build a file-backed state instead:

```rust
    #[test]
    fn restore_backup_roundtrip() {
        // File-backed DB matching get_db_paths_for_dir layout:
        // <app_dir>/kniha-jazd.db, backups in <app_dir>/backups.
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::new(dir.path().join("kniha-jazd.db")).unwrap();
        let state = ServerState {
            db: std::sync::Arc::new(db),
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: dir.path().to_path_buf(),
            static_dir: std::env::temp_dir(),
        };

        let vehicle_args = |name: &str, plate: &str| {
            json!({
                "name": name,
                "licensePlate": plate,
                "initialOdometer": 0.0,
                "vehicleType": "Ice",
                "tankSizeLiters": 50.0,
                "tpConsumption": 6.5
            })
        };

        // One vehicle → snapshot → second vehicle → restore → one vehicle again.
        dispatch_sync("create_vehicle", vehicle_args("Original", "BA-111AA"), &state).unwrap();
        let backup = dispatch_sync("create_backup", json!({}), &state).unwrap();
        let filename = backup["filename"].as_str().unwrap().to_string();

        dispatch_sync("create_vehicle", vehicle_args("Second", "BA-222BB"), &state).unwrap();
        let vehicles = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(vehicles.as_array().unwrap().len(), 2);

        dispatch_sync("restore_backup", json!({ "filename": filename }), &state).unwrap();

        let vehicles = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(vehicles.as_array().unwrap().len(), 1);
        assert_eq!(vehicles[0]["name"], "Original");
    }

    #[test]
    fn restore_backup_fails_in_read_only_mode() {
        let state = test_state();
        state.app_state.enable_read_only("Test read-only");
        let result = dispatch_sync("restore_backup", json!({ "filename": "x.db" }), &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("režime len na čítanie"));
    }
```

If `BackupInfo` serializes `filename` under a different key, check the struct's serde attributes in [backup.rs](../../src-tauri/core/src/commands_internal/backup.rs) (~L427 `Ok(BackupInfo { filename, ... })`) and adjust the accessor — do not change the struct.

**Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core restore_backup`
Expected: both tests FAIL with error containing `Unknown command: restore_backup`.

**Step 3: Register the command in the dispatcher**

In the `// Backup (10)` section of `dispatch_sync` (after the `"get_backup_path"` arm, before the section ends), add:

```rust
        "restore_backup" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                filename: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::restore_backup_internal(
                &state.app_dir,
                &state.app_state,
                a.filename,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
```

Also bump the section comment `// Backup (10)` → `// Backup (11)`.

**Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core restore_backup`
Expected: PASS (both new tests plus the existing `restore_backup_internal` tests).

**Step 5: Commit**

```bash
git add src-tauri/core/src/server/dispatcher.rs
git commit -m "feat(server): dispatch restore_backup in server mode"
```

---

### Task 2: Flip the `restore_backup` capability flag

The frontend restore UI ([+page.svelte](../../src/routes/settings/+page.svelte) ~L1692) is gated on this flag; flipping it is the entire frontend story. The browser flow already reloads the page 1.5 s after restore (~L850), which re-reads the restored DB — correct for server mode.

**Files:**
- Modify: [src-tauri/core/src/server/mod.rs](../../src-tauri/core/src/server/mod.rs) (`capabilities_handler` ~L79-94; `capabilities_endpoint` test ~L307)

**Step 1: Extend the existing capabilities test to fail**

In the `capabilities_endpoint` test, after the existing asserts, add:

```rust
        assert_eq!(body["features"]["restore_backup"], true);
```

**Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core capabilities_endpoint`
Expected: FAIL — `restore_backup` is currently `false`.

**Step 3: Flip the flag**

In `capabilities_handler`, change `"restore_backup": false,` → `"restore_backup": true,`.

**Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core capabilities_endpoint`
Expected: PASS.

**Step 5: Full backend suite (guard against regressions)**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kniha-jazd-core`
Expected: all tests PASS.

**Step 6: Commit**

```bash
git add src-tauri/core/src/server/mod.rs
git commit -m "feat(server): expose restore_backup capability in browser mode"
```

---

### Task 3: CI — publish `kniha-jazd-web` image to ghcr.io on release

**Files:**
- Modify: [.github/workflows/release.yml](../../.github/workflows/release.yml) (add a job; existing jobs: `check-tests`, `backend-tests`, `integration-build`, `integration-tests`, `build` at L263)

**Step 1: Add the `docker-image` job**

Append after the `build` job, mirroring `build`'s gating exactly (release proceeds when tests passed on this commit or in this run):

```yaml
  docker-image:
    name: Publish Web Docker Image
    needs: [check-tests, backend-tests]
    if: |
      always() &&
      (needs.check-tests.outputs.tests_passed == 'true' ||
       needs.backend-tests.result == 'success')
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to ghcr.io
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: Dockerfile.web
          push: true
          tags: |
            ghcr.io/mcsdodo/kniha-jazd-web:${{ github.ref_name }}
            ghcr.io/mcsdodo/kniha-jazd-web:latest
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

**Step 2: Sanity-check the Dockerfile still builds locally (optional but recommended, ~10 min)**

Run: `docker build -f Dockerfile.web -t kniha-jazd-web:ci-check .`
Expected: image builds; `docker run --rm -d -p 3456:3456 kniha-jazd-web:ci-check` then `curl http://localhost:3456/health` → `ok`. Stop and remove the container afterwards.

**Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: publish kniha-jazd-web image to ghcr.io on release"
```

**Note:** The job runs on the next `v*` tag. After the first release, make the ghcr package **public** once (GitHub → mcsdodo → Packages → kniha-jazd-web → Package settings → Change visibility) so the homelab can pull anonymously.

---

### Task 4: Documentation

**Files:**
- Modify: [docs/features/server-mode.md](../../docs/features/server-mode.md)
- Modify: [CHANGELOG.md](../../CHANGELOG.md) — via the `/changelog` skill
- Modify: [DECISIONS.md](../../DECISIONS.md) — via the `/decision` skill

**Step 1: Update server-mode.md**

- Capabilities JSON example (~L157): `"restore_backup": true`.
- "Why 4 commands excluded" design decision (~L210): now **3** excluded (`export_to_browser`, `move_database`, `reset_database_location`); explain restore is served since it's the same `fs::copy` the desktop performs, and the browser reloads after restore. Update the "68 of 72" figure (~L96) to 69 of 72.
- Add a short **Homelab Deployment** subsection under "Docker Deployment": image `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` published by the release workflow; canonical always-on instance runs on the homelab (link to [_tasks/67-online-always-on-runner/02-design.md](./02-design.md)).

**Step 2: Update CHANGELOG (user-visible changes)**

Invoke `/changelog`. Entries under `[Unreleased]`:
- Added: restore backup now available in browser/server mode.
- Added: official Docker image `ghcr.io/mcsdodo/kniha-jazd-web` published on each release.

**Step 3: Record the deployment decision**

Invoke `/decision` for an ADR: *"Homelab server is the canonical deployment; desktop becomes a browser client"* — covering: single `/data` home for DB/settings/backups, gdrive sync retired, LAN+Tailscale exposure without auth (extends ADR-017 from LAN to tailnet), Paperless-only receipt intake (legacy folder-scanned receipt images intentionally left behind — metadata retained).

**Step 4: Commit**

```bash
git add docs/features/server-mode.md CHANGELOG.md DECISIONS.md
git commit -m "docs: server-mode restore parity + homelab deployment decision"
```

---

### Task 5: Verify and release

**Step 1: Run `/verify`** — full backend suite + git status + changelog check.

**Step 2: Server-mode smoke (focused, not the full 10-min sweep)**

```bash
WDIO_SERVER_MODE=1 npx wdio run tests/integration/wdio.server.conf.ts --spec tests/integration/specs/tier1/smoke.spec.ts
```
(Use whatever tier1 spec exists; this confirms the server-mode harness still boots. Skip if the harness requires the debug build and time is short — backend tests are the gate.)

**Step 3: Run `/release`** — bumps version (suggest **0.39.0** — new feature), tags, pushes. The tag triggers the release workflow, which now also publishes `ghcr.io/mcsdodo/kniha-jazd-web:v0.39.0`. Verify the image appears under GitHub Packages, then make it public (one-time, see Task 3 note).

**Step 4: Update task status** — set [01-task.md](./01-task.md) / this plan's Status to reflect progress; update [_tasks/index.md](../index.md) (📋 → 🟡 → ✅ per state).

---

**Handoff:** When the image is published and public, execute [04-plan-homelab.md](./04-plan-homelab.md) (copied into the home.notavailable repo).
