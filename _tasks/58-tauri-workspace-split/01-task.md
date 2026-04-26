**Date:** 2026-04-26
**Subject:** Split `src-tauri/` into a Cargo workspace (core / desktop / web) so the headless web binary stops linking Tauri
**Status:** Planning
**Source:** [`_TECH_DEBT/06-tauri-feature-gating.md`](../_TECH_DEBT/06-tauri-feature-gating.md)

## Problem

Today [`src-tauri/`](../../src-tauri/) is one Cargo crate (`kniha-jazd`) producing two binaries: the Tauri desktop app via [`src-tauri/src/lib.rs`](../../src-tauri/src/lib.rs) and the headless HTTP server via [`src-tauri/src/bin/web.rs`](../../src-tauri/src/bin/web.rs). Cargo links per-crate, not per-binary, so even though `web.rs` never references any `tauri::*` symbol, the linker pulls every Tauri/`gdk`/`webkit`/`soup`/`appindicator`/`rsvg` symbol into the web binary. That forces [`Dockerfile.web`](../../Dockerfile.web) to install ~150 MB of GTK/WebKit runtime libraries the binary will never use.

[`_TECH_DEBT/06-tauri-feature-gating.md`](../_TECH_DEBT/06-tauri-feature-gating.md) proposed two solutions: (a) feature-gate Tauri behind a `desktop` Cargo feature with `#[cfg(feature = "desktop")]` on ~74 wrapper functions, or (b) restructure into a Cargo workspace with separate crates. This task picks **option (b)** because the architectural boundary already exists (Task 55's `wrapper → _internal` pattern is screaming for a crate boundary) and a workspace makes the boundary self-enforcing — no future contributor can accidentally couple core code to Tauri because the dep doesn't exist in `core/Cargo.toml`.

## Goals

1. Split [`src-tauri/`](../../src-tauri/) into three workspace member crates:
   - `kniha-jazd-core` — pure library (DB, calculations, models, server, `_internal` command bodies). No Tauri deps.
   - `kniha-jazd-desktop` — Tauri shell + thin `#[tauri::command]` wrappers that delegate to `core::commands_internal::*`.
   - `kniha-jazd-web` — headless HTTP server binary. Depends only on `core`.
2. Drop GTK/WebKit runtime packages from [`Dockerfile.web`](../../Dockerfile.web) stage 3. Drop them from the builder stage too.
3. Preserve the existing 195-test backend suite and the WebdriverIO integration tiers.
4. Preserve the desktop user experience (`npm run tauri dev`, `npm run tauri build`) — only Cargo manifest paths change.

## Non-Goals

- No business-logic changes. No new commands, no schema changes, no UI changes.
- No changes to the Tauri app's public IPC surface — every existing `#[tauri::command]` wrapper keeps the same name and signature.
- No CI matrix expansion beyond the new `cargo test --workspace` and `cargo build -p kniha-jazd-web` paths.

## Success Criteria

- `cargo build --workspace` succeeds.
- `cargo build -p kniha-jazd-web --release` produces a binary with **zero** GTK/WebKit/Soup/AppIndicator/RSVG symbol references (verified with `ldd`).
- `cargo test --workspace` passes all 195 backend tests with no test relocation logic changes.
- `npm run tauri build` produces a working desktop installer.
- `npm run test:integration:tier1` passes against a debug Tauri build.
- Docker image built from the updated [`Dockerfile.web`](../../Dockerfile.web) is ≤120 MB (target: ~80 MB; today: ~300 MB).
- The Docker container's `/health` endpoint responds and the SPA loads.

## Dependencies

- Task 55 ([Server Mode](../_done/55-server-mode/)) — already complete. Provides the `wrapper → _internal` separation that makes this split mostly mechanical.
- Task 33 ([Web Deployment](../_done/33-web-deployment/)) — already complete. Provides the [`Dockerfile.web`](../../Dockerfile.web) and [`src-tauri/src/bin/web.rs`](../../src-tauri/src/bin/web.rs) entry point this task restructures.

## Risks

- **Tauri config path resolution.** Moving [`src-tauri/tauri.conf.json`](../../src-tauri/tauri.conf.json) into `desktop/` may require adjustments to `frontendDist`, `beforeDevCommand`, and resource paths. Mitigation: build smoke test before further changes.
- **Hidden Tauri couplings.** A `serde` derive or proc-macro could indirectly pull `tauri::*`. Mitigation: post-split `cargo tree -p kniha-jazd-core | grep tauri` should return nothing.
- **CI workflow drift.** [`.github/workflows/test.yml`](../../.github/workflows/test.yml) calls `cargo test`; needs to become `cargo test --workspace`. Same for any release build step.
- **Diesel migrations directory.** [`src-tauri/migrations/`](../../src-tauri/migrations/) embeds via `diesel_migrations` macro relative to the crate root — must move with `db.rs` into `core/` and the embed path verified.

## Related

- [`_TECH_DEBT/06-tauri-feature-gating.md`](../_TECH_DEBT/06-tauri-feature-gating.md) — origin tech debt item; this task implements alternative (b) from that doc.
- [`_done/55-server-mode/`](../_done/55-server-mode/) — established the `wrapper → _internal` pattern that this split promotes to a crate boundary.
- [`_done/33-web-deployment/`](../_done/33-web-deployment/) — defined the headless web binary and Dockerfile this task slims down.
- [`02-plan.md`](./02-plan.md) — concrete file-by-file execution plan.
