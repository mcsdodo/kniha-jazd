# Decisions Log

Architecture Decision Records (ADRs) and business logic decisions. **Newest first.**

---

## 2026-09-04: Image Publishing Channels

### ADR-031: Two Channels — `:main` Moves, `:latest` Is Cut

**Builds on** [ADR-030](#adr-030-the-desktop-app-is-deleted-the-container-is-the-only-build).

**Context:** ADR-030 made the ghcr image the only artifact this project ships, and a `v*` tag the only thing that publishes one. That left a gap between releases: [test.yml](./.github/workflows/test.yml) builds the image on every push to `main`, runs three integration tiers and the env-pinned suite against it, and then **throws it away** — the tar is a one-day artifact. Running the tip of `main` on the homelab meant building it by hand, so changes went untested-in-real-use until someone decided to cut a version.

**Decision:** Publish two channels with a hard ownership boundary.

| Tag | Moves | Published by | Means |
|-----|-------|--------------|-------|
| `:main` | yes, per green build | [test.yml](./.github/workflows/test.yml) `publish-main-image` | tip of `main`, all tests green |
| `:main-<short-sha>` | never | same job | that exact commit, pinnable |
| `:latest` | yes, per release | [release.yml](./.github/workflows/release.yml) | last version someone cut |
| `vX.Y.Z` | never | same job | that release |

CI never touches `:latest` or `vX.Y.Z`; [`/release`](./.claude/skills/release-skill/SKILL.md) never touches `:main`. Both halves are stated in the workflow comments and in the release skill so neither drifts into the other's tags.

**Reasoning, point by point:**

1. **The tested tar is republished, not rebuilt.** `integration-build-docker` already uploads the built image as an artifact so the tier jobs can share it; the publish job downloads that same artifact, `docker load`s it and re-tags. The bytes on ghcr.io are therefore the bytes the suite passed against — a rebuild could differ (base-image drift, a dependency resolving differently) and would prove nothing that the first build already proved, at the cost of several minutes.

2. **`needs` is the whole gate, and it needs no `always()`.** The job depends on `backend-tests`, `integration-build-docker`, `integration-test-docker` and `integration-test-docker-env`. A job-level `if` without a status-check function still requires every `needs` job to have succeeded, and matrix needs require *all* legs — so a single red tier or a failing Windows backend run publishes nothing.

3. **`:main-<short-sha>` exists because a floating tag cannot be rolled back.** If a green-but-broken build moves `:main`, the alternatives without a pinned tag are "wait for the next green main" or "drop back to `:latest`", which can be many commits behind. One extra `docker tag` line buys a real rollback target. The cost is one manifest per main commit; pruning is a problem for later, not a reason to skip it now.

4. **`:main` over `:prerelease` or `:edge`.** The tag names the branch it tracks, so its provenance is unambiguous at a glance in a compose file, and the scheme extends if another branch is ever published. `:edge` is the Docker-ecosystem idiom but says nothing about where the bytes came from.

5. **No `workflow_dispatch` escape hatch.** [check-file-changes](./.github/actions/check-file-changes/action.yml) short-circuits to `has_code_changes=true` for every non-PR event, so the docs-only skip applies to pull requests only and *every* push to `main` already runs the full suite and publishes. A manual trigger would duplicate a path that cannot be skipped.

**Consequences:**

- **A green `main` is now a published artifact, so main is a deployment target.** Anything merged is immediately pullable by the homelab. This raises the stakes on merging directly to `main` — the safety net is that the publish gate is the full suite, not a subset.
- **Pull requests publish nothing.** The `if` requires `github.event_name == 'push'` on `refs/heads/main`; fork PRs get a read-only `GITHUB_TOKEN` and must never be able to move a published tag regardless.
- **Both channels remain linux/amd64 only.** Neither this job nor `release.yml` builds arm64, so a Raspberry Pi still needs its own build. Unchanged by this decision, and worth its own task if it ever matters.

**Related:** [Task 74](./_tasks/_done/74-main-branch-image-channel/), [ADR-030](#adr-030-the-desktop-app-is-deleted-the-container-is-the-only-build), [docs/features/server-mode.md](./docs/features/server-mode.md).

---

## 2026-09-04: Web-First Migration

### ADR-030: The Desktop App Is Deleted; the Container Is the Only Build

**Supersedes** [ADR-001](#adr-001-desktop-app-with-tauri--sveltekit). **Completes** [ADR-024](#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client).

**Context:** ADR-024 named the always-on Docker deployment canonical and demoted the desktop app to "a browser client that keeps working". That left two of everything: two export paths, two harnesses, two release pipelines, and a CI run that spent ~15 minutes per push proving a desktop build nobody launched still worked. [Task 58](./_tasks/_done/58-tauri-workspace-split/) had already moved every piece of business logic into `kniha-jazd-core`, so the desktop crate held 2 tests against core's 569. Keeping it was paying full price for a second product.

**Decision:** Delete it. `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` is the only artifact this project builds, ships and tests.

1. **`src-tauri/desktop/` is gone**, along with the updater, the signing keys, the frontend's `@tauri-apps` dependencies, and the custom-database-location feature — one `/data` volume needs no path picker and no multi-PC lock dance. The workspace directory keeps the name `src-tauri/` because renaming it would rewrite every path in the repo's history for no functional gain.
2. **A `v*` tag publishes the ghcr image and nothing else** — no GitHub Release, no installer, no release notes. `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_KEY_PASSWORD` become dead repository secrets.
3. **Docker + Chrome is the only test harness.** The tauri-driver path is gone, and with it the EdgeDriver version-chasing block whose own comment recorded CI jobs hanging "for hours via retries". WebdriverIO now spawns the headless `kniha-jazd-web` binary locally, or talks to a container.
4. **Playwright and vitest are deleted rather than wired up** (D1). Both were test scripts no CI job invoked; vitest matched zero files, and per [ADR-008](#adr-008-remove-frontend-calculation-duplication) the frontend holds no logic to unit-test. A test script that exists but runs nowhere is worse than no script, because it reads as coverage.
5. **Receipt-processing progress is an accepted loss** (D3). The desktop path emitted progress events over `app.emit`; no SSE or polling replacement is built, and the UI that consumed them is deleted rather than stubbed. ADR-024 already made Paperless the sole intake channel.

**Reasoning:** The deletion had to be paid for before it was taken. Web-mode export ignored the user's hidden columns and sort direction, and dropped the synthetic "Prvý záznam" row carrying the year-opening odometer — so deleting desktop first would have silently degraded the printed logbook. Both closed before anything was removed. The same rule drove the test work: every mode-conditional skip was fixed and proven green under the surviving harness *while the Tauri harness still existed*, so coverage this migration would otherwise have dropped showed up as a red job rather than as a deleted file.

That surfaced three tests that were not covering what they claimed. `export.spec.ts` had never asserted anything under the desktop harness — that path opens the system browser, creating no WebDriver window, so every assertion sat inside an `if` that could not be true. The BEV badge test was skipped as flaky but failed deterministically on a case mismatch against a CSS `text-transform`. And the env-pinned suite, covering [Task 68](./_tasks/_done/68-env-managed-settings-ui/) and [Task 69](./_tasks/_done/69-pin-gated-secret-reveal/), had never run in CI at all.

**Two consequences worth stating outright:**

- **GitHub Releases stop entirely.** Already-installed desktop copies are not migrated or redirected; they keep polling an updater endpoint that will never serve another release, which is harmless. They are simply obsolete. The only announcement is the [CHANGELOG](./CHANGELOG.md) entry — no in-app notice, no farewell build.
- **Folder-scanned receipts survive as an unmaintained path, not the intake channel.** The local scanning UI still works and is still tested, but Paperless is the supported route. Deleting the folder path is a separate decision about a feature, not about a deployment target.

**Coverage invariants this migration was held to** — both mechanically checked:

- **I1:** every `test:*` script in [package.json](./package.json) is invoked by a job in [test.yml](./.github/workflows/test.yml), or the script no longer exists. The tier scripts satisfy this by delegating to `test:integration`, which the Docker jobs run.
- **I2:** `grep -rnE "describeNotIn[A-Za-z]+\(|it\.skip\(|this\.skip\(\)" tests/integration/specs/` returns nothing. All nine skip constructs were fixed, removed with their feature, or made moot by there being one mode.

**Related:** [Task 73](./_tasks/_done/73-web-first-migration/), [ADR-017](#adr-017-lan-only-cors-without-authentication) (unchanged — still no authentication), [ADR-018](#adr-018-workspace-members-over-feature-flags), [docs/features/server-mode.md](./docs/features/server-mode.md).

---

## 2026-09-03: Copy Trip Row

### BIZ-024: A Copied Row Is Dated Today, Clamped Into the Year Being Viewed

**Context:** [Task 71](./_tasks/_done/71-copy-trip-row/) adds a copy button that duplicates a trip's route into a new row dated today. "Today" is unambiguous only while the grid shows the current year. The year picker means the user may well be looking at 2025 in September 2026 — and the literal current date then belongs to a year the open grid cannot display.

**Decision:** The target date is resolved against the year the grid is showing ([trip_copy.rs](./src-tauri/core/src/calculations/trip_copy.rs)):

| Viewed year | Target date |
|-------------|-------------|
| the current year | today |
| a past year | 31 December of that year |
| a future year | 1 January of that year |

**Reasoning:** Saving a trip into 2026 while the user is looking at 2025 makes the row vanish the instant it is written, with no error and nothing on screen to explain it — the worst kind of failure, because it looks like the save was lost. Clamping keeps the row where the user is looking. The direction of the clamp follows from which end of the year is nearest to "now": a past year is behind us, so its last day is closest; a future year is ahead, so its first is.

**Consequence — only the time-of-day travels, never the source date.** The copied row takes the source trip's start `HH:MM` and its end `HH:MM` **plus the source's day span**. Carrying the day offset is what keeps a 22:00 → 02:00 trip overnight; dropping it would land the end four hours *before* the start. One accepted edge: an overnight trip copied into a past year starts 31 December and ends 1 January of the next — the start stays inside the viewed year, which is what the grid needs.

**Not copied: fuel, energy, costs, notes, invoice links.** A fill-up is a one-off event, not a property of a route. Copying `fuel_liters` would feed a fabricated fill-up into the consumption rate and the [20 % margin](#biz-003-legal-margin-limit) calculation. The command returns a `CopiedTripDefaults` struct that has no such fields, so the exclusion holds at compile time rather than relying on a frontend that remembers to skip them.

**Not copied either: an implausible distance.** A stored `distance_km` outside `(0, 9999]` is corruption from the old delta-accumulation bug, and `tryAutoFillDistance` already refuses to seed a new row from such a route. The copy path is another way to seed a row, so it applies the same bound and returns `0.0`, which the frontend renders as an empty field for the user to fill.

**A copied distance is a default, not a decision.** The seeded km is treated as auto-filled, not user-typed: changing the route on a copied row replaces it, exactly as picking a route on a blank row fills it. Without that, the row would keep the old route's km while its times re-inferred around the new one — a 400 km journey saved as 47 km, feeding the consumption and margin math. A distance the user actually types wins over both.

**Per [ADR-008](#adr-008-remove-frontend-calculation-duplication) the rule lives in Rust**, split the same way [BIZ-014](#biz-014-opt-in-auto-fill-of-trip-startend-times)'s time inference is: a pure function taking `(source, year, today)` with no clock and no DB, plus a thin wrapper supplying both. `year` arrives off the wire in server mode, so an out-of-range value returns `Err` rather than panicking inside `from_ymd_opt(...).expect(...)`.

**Known limitation — "today" is the host's day, not the viewer's.** The wrapper reads `Local`, which is the user's calendar day on the desktop but the *server's* in server mode: a container on `TZ=UTC` serving a browser on UTC+2 dates a 01:00 copy to the previous day. The grid's own `defaultNewDate` derives its date in UTC, so the two new-row paths can also disagree by a day near midnight. Resolving it properly means passing the client's date in as a parameter; until then the year clamp still guarantees the row lands in the visible grid.

**Related:** [Task 71](./_tasks/_done/71-copy-trip-row/) — [01-task.md](./_tasks/_done/71-copy-trip-row/01-task.md), [02-plan.md](./_tasks/_done/71-copy-trip-row/02-plan.md); [BIZ-014](#biz-014-opt-in-auto-fill-of-trip-startend-times) (whose jitter the copy deliberately suppresses); [BIZ-008](#biz-008-odo-auto-calculation).

---

## 2026-08-10: Route Map Integration

### ADR-028: Only the Polyline Is Persisted; Tiles Live in a Disposable Cache

**Context:** [Task 70](./_tasks/70-route-map-integration/) generates a plausible driving route for a trip, draws it in the app, and rasterises it into the printed export as an attachment page. Three artefacts fall out of that pipeline and each could plausibly be stored: the chosen route (waypoints + the OSRM polyline), the OSM raster tiles the printed image is composited from, and the finished PNG itself. Storing all three is the obvious reading of "save the map" — and it is the expensive one. A rendered attachment is several hundred KB; a year of them would grow the database by roughly 10 MB, land in every automatic backup, and force [Task 32](./_tasks/32-portable-csv-backup/)'s portable CSV backup to base64 a binary column that no other table needs.

**Decision:** The database stores **only what cannot be recomputed**: the waypoints, the encoded polyline OSRM returned, the target and road distances, the dataset version and a timestamp — a few KB per trip, in a `trip_routes` table ([migration](./src-tauri/core/migrations/2026-08-10-100000_add_trip_routes/)).

1. **No image is persisted anywhere.** The attachment PNG is rasterised at export time and base64-embedded into the HTML document; nothing writes it to disk ([render.rs](./src-tauri/core/src/route_map/render.rs)).
2. **OSM tiles are cached**, in a cache subdirectory of the app data dir (`tile_cache_dir` in [route_maps.rs](./src-tauri/core/src/commands_internal/route_maps.rs)) — a directory deliberately separate from the database and the backups folder, so that deleting it is always safe.
3. **The bounding box and the GA seed are not stored either.** The box is derived from the polyline in microseconds; the seed has nothing to replay, because "regenerate" means *a fresh random route*, not the same one again.

**Reasoning:** Cache loss must cost a re-fetch and nothing else, and that property is what keeps the rest of the app untouched by this feature. The now-removed Move Database feature moved the database and its backups folder; the tile cache stayed behind and simply refilled. Backups need no new format. The portable CSV backup stays textual. Had the PNG been a column, every one of those would have needed a decision.

**Two guards keep cache loss cheap rather than lossy** ([tiles.rs](./src-tauri/core/src/route_map/tiles.rs)):

- **Failures are never cached.** A non-2xx response and an empty body both return an error *before* anything is written. Caching an error page or zero bytes poisons that tile for every future export, and the only cure is finding and deleting a directory the user does not know exists.
- **Tiles are written to a temp file and renamed.** A crash or a full disk mid-write leaves either the whole tile or nothing — never a truncated PNG that is then served from cache indefinitely.

**Trade-offs accepted:** A cold cache makes the first export slower, and OSM's tile usage policy caps concurrency at two connections, so that cost is real. It is bounded in practice: every route starts at the same home base, so successive maps overlap heavily at the zoom levels involved. Regenerating a saved map is also impossible by design — the polyline *is* the saved result, and a user who wants a different route generates and saves a new one.

**Related:** [Task 70](./_tasks/70-route-map-integration/) — [01-task.md](./_tasks/70-route-map-integration/01-task.md), [02-design.md](./_tasks/70-route-map-integration/02-design.md); [ADR-029](#adr-029-waypoints-persist-as-coordinates-not-dataset-indices); [docs/features/route-maps.md](./docs/features/route-maps.md).

---

### ADR-029: Waypoints Persist as Coordinates, Not Dataset Indices

**Context:** Routes are assembled by a genetic algorithm choosing settlements from a bundled 67-node Slovak dataset ([Task 61](./_tasks/61-route-map-poc/)). Inside that algorithm a route *is* a list of indices into the dataset — that is the whole representation, and the [POC](./_tasks/61-route-map-poc/02-design.md) persisted nothing else. Storing indices is smaller, self-validating, and reads naturally. It is also a shape that cannot express a point which is not in the dataset.

**Decision:** A waypoint persists as `{ lat, lon, name?, node_idx? }` ([models.rs](./src-tauri/core/src/models.rs)). The **coordinates are the identity**; `node_idx` is optional provenance recording that the generator picked this point out of the dataset, and is absent for any point that did not come from there.

**Reasoning:** V1 has exactly one route producer, but the planned V2 editor adds a second: a user dragging a point onto a road that is not one of the 67 nodes. An index-only column cannot represent that waypoint at all, so V2 would have to migrate every stored route — rewriting rows whose original meaning ("node 14") is only recoverable by consulting the same dataset version that wrote them. Paying two floats per waypoint now avoids a migration that gets harder the more routes exist. Storing coordinates also decouples saved routes from the dataset: regenerating the node set (or changing the home base) cannot silently re-point an existing map at a different village.

**Consequence — `node_idx` must never be defaulted.** `0` is a valid index: it is the home base, the node every route begins and ends at. A `#[serde(default)]`, an `unwrap_or(0)`, or a `NOT NULL DEFAULT 0` column would turn "a human placed this point" into "this point is home". It is `Option<i32>`, and is omitted from the stored JSON entirely when absent — never written as a zero.

**Trade-offs accepted:** The stored waypoint list can drift from the dataset — a node renamed or moved in a future dataset version leaves old routes carrying the old name and coordinates. That is the correct behaviour for a logbook: a saved map is evidence of what was recorded, not a live query. `dataset_version` records which node set produced the route, and is nullable so that a V2 route containing hand-placed points can record that it corresponds to no single dataset version.

**Related:** [Task 70](./_tasks/70-route-map-integration/) — [02-design.md](./_tasks/70-route-map-integration/02-design.md); [ADR-028](#adr-028-only-the-polyline-is-persisted-tiles-live-in-a-disposable-cache); [docs/features/route-maps.md](./docs/features/route-maps.md).

---

## 2026-08-10: PIN-Gated Secret Reveal

### ADR-027: Secrets Leave the Backend Only Through a Throttled, PIN-Gated Command

**Context:** [ADR-025](#adr-025-env-pinned-secrets-are-echoed-back-to-the-settings-page) accepted that "a pinned token is readable by anyone who can reach the app", on the grounds that this is "the same trust boundary [ADR-017](#adr-017-lan-only-cors-without-authentication) already accepts for the data itself". That reasoning had a hole: ADR-017's boundary is a **CORS allowlist**, and CORS is a browser control. It governs what a page from another origin may do; it does nothing about a direct HTTP client. Verified against a running instance — `curl` with no `Origin` header reached every RPC command, and `get_receipt_settings` returned the Gemini key in full. Three commands were handing out credentials with no challenge: `get_receipt_settings`, `get_local_settings_for_ha`, and (from ADR-025) `token_env_value` on the two settings responses.

**Decision:**

1. **No ordinary read returns a secret.** Settings responses report only `hasToken` / `has_gemini_api_key`. `get_local_settings_for_ha` is deleted outright — it returned the full HA token and nothing referenced it.
2. **One dedicated command, `reveal_secret(field, pin)`**, with `field` a closed enum so it cannot be aimed at arbitrary settings. It returns the *effective* value (env override included), because the point of revealing is to see what is live.
3. **Authorization is decided by code path, not by a claim.** The Tauri wrapper passes `RevealAuth::LocalTrusted`; the HTTP dispatcher always passes `RevealAuth::Pin`, and a missing `pin` argument becomes `Pin("")` rather than a local caller. The dispatcher cannot construct `LocalTrusted`, so the desktop exemption is structural rather than a permission flag to bypass.
4. **The PIN comes from `KNIHA_JAZD_REVEAL_PIN`**, is compared in constant time, and is required on **every** reveal — no session, no caching. With the variable unset, reveal is disabled on the server; the server still starts, because a forgotten variable should cost an eye icon, not the homelab.
5. **Throttled**: 5 consecutive failures lock reveal out for 60s, escalating to 5/15/60 minutes. The counter is global, not per-IP — a per-IP counter is defeated by rotating source addresses on a LAN.

**Reasoning:** Credentials are qualitatively different from the app's own data: they grant access to Google, Home Assistant, and Paperless, far beyond anything in this database. ADR-017's no-login model remains right for trips and invoices, and this changes nothing there. Throttling is what makes a short (4-character) PIN meaningful — 10,000 combinations fall in seconds unthrottled, so without it the gate would be decoration.

**Supersedes:** the trade-off paragraph of [ADR-025](#adr-025-env-pinned-secrets-are-echoed-back-to-the-settings-page). Revealing the live value is still the behavior — the operator wants to confirm *which* credential is active — but it is no longer free to anyone on the network.

**Trade-offs accepted:** losing the PIN loses in-app reveal (the values remain readable where they were set); and an attacker on the tailnet can lock the operator out by burning attempts, which on a closed network is the right failure direction — denial beats disclosure.

**Related:** [Task 69](./_tasks/69-pin-gated-secret-reveal/) — [01-task.md](./_tasks/69-pin-gated-secret-reveal/01-task.md), [02-design.md](./_tasks/69-pin-gated-secret-reveal/02-design.md); [docs/features/settings-architecture.md](./docs/features/settings-architecture.md).

---

## 2026-08-10: Command Side Effects Belong in Core

### ADR-026: A Command's Side Effects Live in Core, Not in the Tauri Wrapper

**Context:** `get_trip_grid_data` pushed the suggested fillup to a Home Assistant `input_text` helper. That push lived in the Tauri wrapper ([desktop/src/commands/statistics.rs](./src-tauri/desktop/src/commands/statistics.rs)), while the server's RPC dispatcher called `build_trip_grid_data` and returned. Both paths computed identical data; only one had the side effect. When [ADR-024](#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client) made the server canonical, the push silently stopped — no error, no log, nothing to notice. The helper's doc comment even recorded the (then-true) reasoning: "consumed only by other desktop wrappers."

**Decision:** A Tauri command's **side effects** live in core alongside its calculation, and the server dispatcher performs them too. Concretely:

1. **Both frontends share one rule.** `ha_fillup_push_payload(vehicle, grid) -> Option<(entity_id, value)>` decides *whether and what* to push; `push_ha_input_text` performs it. Both live in [core's integrations module](./src-tauri/core/src/commands_internal/integrations.rs) and are called from both frontends.
2. **A command belongs to exactly one dispatcher.** `get_trip_grid_data` moved from `dispatch_sync` to [dispatcher_async.rs](./src-tauri/core/src/server/dispatcher_async.rs) (the push needs a runtime), and the sync arm was **deleted** rather than left as a second implementation — two arms for one command is how they drift.
3. **Side effects are regression-tested at the server boundary**, not only as units: a test dispatches the real command against a stub HA and asserts the request arrives.

**Reasoning:** "Desktop-only" was a property of the deployment, not of the code, and deployments change. Anything a wrapper does beyond delegating is invisible to the other frontend, and the failure is silent by construction — a fire-and-forget side effect that never fires looks exactly like one that isn't configured.

**Scope check:** an audit of the desktop wrappers found no other divergence. The remaining multi-line wrappers are either pure delegation with long argument lists (`create_trip`, `create_vehicle`) or legitimately desktop-only capabilities — native dialogs (`move_database`, `export_to_browser`), the updater, and server control (`start_server`).

**Related:** [ADR-024](#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client), [Task 52](./_tasks/52-ha-suggested-fillup-push/) (which introduced the push), [docs/features/home-assistant.md](./docs/features/home-assistant.md).

---

## 2026-08-08: Env-Managed Settings in the UI

### ADR-025: Env-Pinned Secrets Are Echoed Back to the Settings Page

**Context:** [ADR-024](#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client)'s Docker deployment configures Gemini/HA/Paperless through environment variables, which win over [local.settings.json](./local.settings.json.sample) at every read (`LocalSettings::load_effective` in [settings.rs](./src-tauri/core/src/settings.rs)) and cause the setter commands to refuse writes. The Settings page knew none of this: it rendered ordinary editable inputs holding env-provided values, and the guard only surfaced as a red error toast *after* the user typed. Task 68 makes the pinning visible. Doing that for a masked token forces a choice — reveal the variable *name*, or the variable's *value*.

**Decision:**

1. **Pinned fields render disabled**, each marked with a badge naming its variable (`HA_API_TOKEN`, `PAPERLESS_URL`, …) and an "env-managed" hint. Marking is per-field, not per-section: a deployment may pin `HA_URL` and leave the token file-managed.
2. **The eye icon reveals the actual value** for an env-pinned secret. `get_ha_settings` / `get_paperless_settings` gained `tokenEnvValue`, populated **only** when the variable pins the field; file-stored tokens keep today's write-only `hasToken: bool` and are never sent.
3. **The backend guards stay the enforcement boundary.** Disabling inputs is UX only — a browser client can POST `/api/rpc` directly, so the guards in [integrations.rs](./src-tauri/core/src/commands_internal/integrations.rs) keep refusing pinned writes.
4. **The page sends `null` for pinned fields** instead of their current value, so pinning one field doesn't block editing its neighbour.

**Reasoning:** On a homelab box the operator wants to confirm *which* token is actually live — the name of a variable they typed into their own compose file tells them nothing they don't know. The exposure is narrow by construction: only values the operator already controls through the deployment, never a secret that was typed into the app. For Gemini and Home Assistant this is not even new — `get_receipt_settings` and `get_local_settings_for_ha` already returned those values in full.

**Trade-offs accepted:** Server mode serves this page over LAN/tailnet, so a pinned token is readable by anyone who can reach the app. That is the same trust boundary [ADR-017](#adr-017-lan-only-cors-without-authentication) already accepts for the data itself. Rejected: revealing only the variable name (safer, but answers a question the operator isn't asking) and hiding env-managed sections entirely (loses the connection status, which is the main reason to open the page).

**Related:** [Task 68](./_tasks/68-env-managed-settings-ui/) — [02-design.md](./_tasks/68-env-managed-settings-ui/02-design.md), [docs/features/settings-architecture.md](./docs/features/settings-architecture.md), [ADR-024](#adr-024-homelab-server-is-the-canonical-deployment-desktop-becomes-a-browser-client).

---

## 2026-08-07: Always-On Homelab Deployment

### ADR-024: Homelab Server is the Canonical Deployment; Desktop Becomes a Browser Client

**Context:** The app so far ran as a desktop install whose SQLite database was synced between PCs via a gdrive-synced folder (with lock files guarding multi-PC access). Server mode ([docs/features/server-mode.md](./docs/features/server-mode.md)) and the standalone web binary ([ADR-018](#adr-018-workspace-members-over-feature-flags)) made a browser-only deployment viable, and restore-backup parity closed the last functional gap for browser users. Task 67 asks: where should the single source of truth live?

**Decision:** The canonical instance is the always-on Docker deployment on the user's homelab, reachable at `https://kniha-jazd.lacny.me` over LAN + Tailscale only (no public exposure). Concretely:

1. **Single server-side data home:** database, settings, and backups all consolidate into the container's `/data` directory (one volume to back up, one place where state lives).
2. **gdrive-synced database retired:** with one always-on server there is nothing to sync; the multi-PC lock-file dance becomes unnecessary for daily use.
3. **No auth, tailnet-extended trust:** extends [ADR-017](#adr-017-lan-only-cors-without-authentication)'s LAN trust model to the Tailscale tailnet — both are closed, owner-controlled networks, so the "trusted network, no login flow" reasoning carries over unchanged.
4. **Paperless-only receipt intake:** going forward receipts come exclusively via Paperless-ngx. Legacy folder-scanned receipt images are intentionally left behind on the old desktop machine; their metadata rows stay in the database.
5. **Versioned ghcr image is the deployment artifact:** each `v*` release publishes `ghcr.io/mcsdodo/kniha-jazd-web:vX.Y.Z` (+ `latest`) via the `docker-image` job in [release.yml](./.github/workflows/release.yml); the homelab pins/pulls these tags instead of building from source.
6. **Desktop demoted to a browser client:** existing desktop installs keep working but point at the server URL rather than owning a local database copy.

**Reasoning:** One always-on canonical instance eliminates the whole class of sync/lock/conflict problems the gdrive setup existed to mitigate, while server mode already proved the browser UI is feature-complete for daily use. Publishing a versioned image makes homelab upgrades a tag bump with a clean rollback path. Keeping exposure to LAN + tailnet preserves the no-auth simplicity that [ADR-017](#adr-017-lan-only-cors-without-authentication) committed to.

**Trade-offs accepted:**
- Availability now depends on the homelab being up (mitigated by Tailscale reachability and the `/data` backup story).
- Legacy folder-scanned receipt **images** are no longer reachable from the app (metadata retained); re-scanning into Paperless is the recovery path if ever needed.

**Related:** [Task 67](./_tasks/67-online-always-on-runner/) — [02-design.md](./_tasks/67-online-always-on-runner/02-design.md), [docs/features/server-mode.md](./docs/features/server-mode.md), [ADR-017](#adr-017-lan-only-cors-without-authentication), [ADR-018](#adr-018-workspace-members-over-feature-flags).

---

## 2026-07-15: Multi-Invoice Support (1 Fuel + N Other per Trip)

### BIZ-023: Invoice Cardinality, Sum-on-Assign, and Cent-Exact Money Math

**Context:** A trip could hold at most ONE invoice total (`receipts.trip_id UNIQUE`, `paperless_trip_links.trip_id PRIMARY KEY`), but real rides carry a fuel-up plus parking/wash/toll documents. Task 66 removes the constraint; the business rules below govern how multiple invoices interact with the trip's authoritative fields (see [docs/features/multi-invoice.md](./docs/features/multi-invoice.md)).

**Decision:**

1. **Cardinality:** max **1 Fuel** invoice per trip across BOTH sources combined (local receipt + Paperless — a trip has exactly one fill-up by design); **unlimited Other** invoices. Enforced by partial unique indexes within each store and by the backend assign pre-check across stores (error surfaced as „Jazda už má doklad o tankovaní"; the trip picker greys out fuel-covered trips via `can_attach = false`).
2. **Trip stays authoritative; sum-on-assign for Other:** assigning an Other invoice adds its amount to `trip.other_costs_eur` and appends the note; unassigning subtracts. Invoices are attached proof — manual entry without documents keeps working, and export reads trip fields only (untouched).
3. **Manual overwrite allowed, divergence surfaced:** the user can hand-edit `other_costs_eur` at any time; when it diverges from the sum of attached Other invoices, the grid shows a sum-mismatch warning (with both numbers) — never a block. Trips where any attached Other invoice has an unknown amount (legacy backfill) are excluded from the check entirely (no false warnings).
4. **Cent-exact money math (HARD requirement):** every EUR add/subtract goes through integer cents (`to_cents`/`from_cents`/`money_add`/`money_sub` in [calculations/mod.rs](./src-tauri/core/src/calculations/mod.rs)) — never raw `f64` arithmetic. Repeated assign/unassign cycles are bit-exact. All amount comparisons (double-count guard, sum-mismatch, picker compatibility) are done in cents, **replacing the ±0.01 epsilon** — picker verdict and assign behavior can never disagree on borderline values. `money_sub` clamps at 0; a zero result is stored as `None`, not `Some(0.0)`.
5. **`applied_amount_cents` snapshot rule:** each assignment stores the exact cents it added to the trip. Unassign subtracts **the snapshot**, never the live invoice price (the user may edit the price after assigning; the sum-mismatch indicator is what surfaces such edits). `NULL` snapshot = link-only assignment (double-count guard, unknown-amount invoice, legacy pre-migration link) — subtracts nothing. Fuel unassign never touches other costs; unassigning an orphaned receipt (trip deleted) just clears the link.
6. **Double-count guard (cent-exact):** if the trip has zero Other invoices and `other_costs_eur` already equals the invoice amount to the cent, assignment is link-only — the "manually pre-entered, now attaching proof" case.

**Reasoning:** Sum-on-assign keeps the trip the single source of truth for reporting while making the common flow (assign documents, totals follow) automatic. The snapshot decouples "what this invoice contributed" from "what the invoice says now", so unassign is always an exact reversal. Integer cents eliminate float drift that would otherwise accumulate over assign/unassign cycles and make cent-exact comparisons meaningless.

**Supersedes:** the schema half of [ADR-019](#adr-019-paperless-trip-link-table-is-symmetric-trip_id-primary-key) — [paperless_trip_links](./src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql) was rebuilt with `paperless_document_id` as PRIMARY KEY plus `assignment_type`/`amount_eur`/`title`/`applied_amount_cents` snapshots (the sum check must work offline; the grid never calls the Paperless server).

**Related:** [Task 66](./_tasks/66-multi-invoice/), [docs/features/multi-invoice.md](./docs/features/multi-invoice.md), [migration 2026-07-15-100000_multi_invoice](./src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql), [ADR-012](#) (forward-only migrations — the rebuild is one atomic migration directory).

---

## 2026-05-21: Datetime Is The Only Source of Trip Order

### ADR-022: Drop `sort_order`; `start_datetime` Drives Both Display and Calculation

**Context:** Trips had a separate `sort_order` integer column (defined in the [baseline migration](./src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)) that could drift from `start_datetime`. The "+" button used `sort_order` for UI insertion, which propagated the drift and produced confusing "date-warning" red rows for users with chronologically valid data. Two orderings (display vs. calculation) coexisted, and they were free to disagree.

**Decision:** Drop the `sort_order` column entirely (migration [2026-05-21-100000_drop_sort_order](./src-tauri/core/migrations/2026-05-21-100000_drop_sort_order/)). `start_datetime` is the only source of trip order. New trips are auto-positioned by their datetime. The only way to change a trip's position is to change its datetime.

**Consequences:**
- Manual reordering (up/down arrows, drag-and-drop hypotheticals, the `reorder_trip` Tauri command and its `shift_trips_from_position` DB helper) is removed.
- Same-datetime trips are tiebroken by `created_at` ASC, then by `id` for full determinism.
- The `calculate_date_warnings` Rust helper, the `date_warnings` field on [TripGridData](./src-tauri/core/src/models.rs), and the `.date-warning` CSS class are all removed — date-order drift is structurally impossible, so the warning concept no longer applies.
- Migration drops the column with no data repair needed — order is computed at query time, so existing inconsistent `sort_order` values simply cease to matter.

**Reference:** [Task 65](./_tasks/_done/65-datetime-is-order/).

---

## 2026-05-04: Unified Invoice Picker

### ADR-020: Inline `InvoiceData` at the IPC Boundary (vs. `load_invoice(InvoiceRef)`)

**Context:** Task 64 unifies the trip-picker for both local OCR'd receipts and Paperless-ngx documents behind a single `Invoice` trait + `check_invoice_trip_compatibility(&dyn Invoice, &Trip)` compat check (see [docs/features/unified-invoice-picker.md](./docs/features/unified-invoice-picker.md)). The original design ([02-design.md](./_tasks/_done/64-unified-invoice-picker/02-design.md)) proposed a centralized `load_invoice(db, &InvoiceRef) -> Box<dyn Invoice>` boundary function: pass an `InvoiceRef`, get back a fully-loaded invoice regardless of source. For local receipts that's trivial (`db.get_receipt_by_id`). For Paperless documents it isn't — the [paperless_trip_links](./src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql) table only stores `(trip_id, doc_id)`, with no doc data cached locally. Document state lives in Paperless-ngx and is fetched live.

**Decision:** Carry **inline `InvoiceData`** through the IPC boundary alongside `InvoiceRef`. The frontend already has the full Paperless row from `get_paperless_invoices`; it passes the relevant fields (datetime, liters, total_price_eur, title, assignment_type) inline. Receipts ignore the inline data — backend loads from DB by ID. Paperless docs use the inline data directly via `PaperlessInvoiceView<'a>` (a thin trait adapter at the boundary).

**Alternatives considered:**

- **Add a `paperless_documents_cache` table.** Rejected — significant scope creep just to enable a uniform load fn signature. The cache would need invalidation rules, sync logic, and would duplicate Paperless's source-of-truth role.
- **Make `load_invoice` async and fetch single doc from Paperless API per modal-open.** Rejected — adds a network round-trip in the hot UI path (proximity-sorted trip list rendered after every Assign click), and would require restructuring the sync compat check + dispatcher into async.

**Trade-offs accepted:**

- Two boundary functions instead of one (Tauri command body deserializes `InvoiceRef + InvoiceData`; sync `_internal` matches on `InvoiceRef` to either load from DB or wrap inline data). Outside this two-line dispatch, the entire codebase consumes `&dyn Invoice` — the trait abstraction goal is preserved.
- Frontend must remember to send `invoiceData = null` for receipts (caught at compile time via TS types: `Receipt`-backed adapter's `getData(): null` vs `PaperlessInvoiceRow`-backed adapter's `getData(): InvoiceData`).

**Consequences:**

- Source-dispatch confined to two named locations: [commands_internal/invoices.rs](./src-tauri/core/src/commands_internal/invoices.rs) (Rust `match InvoiceRef`) and [src/lib/invoice.ts](./src/lib/invoice.ts) (TS `adaptInvoice` factory). Outside these spots, source-checking is a smell.
- Adding a third invoice source = one Rust trait impl, one TS adapter class, one match arm in each boundary fn.
- 8 receipt-side compat tests (regression net for the compat-check refactor) all preserved their behavior — proves the trait abstraction is faithful.

**Related:** [Task 64](./_tasks/_done/64-unified-invoice-picker/), [docs/features/unified-invoice-picker.md](./docs/features/unified-invoice-picker.md), [ADR-008](#). Builds on [ADR-019](#) (Paperless schema).

---

### ADR-021: `mismatch_override` is Receipt-Only (Paperless Path Accepts-and-Ignores)

**Context:** Local receipts can be assigned to a trip even when their data conflicts with the trip's existing `fuel_liters` / `fuel_cost_eur` / `other_costs_eur`. The user explicitly confirms via the modal's "Assign and confirm" button, which sets `mismatch_override = true` on the [receipts](./src-tauri/core/migrations/2026-02-03-100000_receipt_assignment_type/up.sql) row. This persists across sessions: the assigned receipt card shows a "✓ Potvrdené" badge, signalling the user has reviewed and accepted the discrepancy. The [paperless_trip_links](./src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql) table has no equivalent column.

**Decision:** The unified `assign_invoice_to_trip_internal` accepts `mismatch_override: bool` for both sources, but for the Paperless arm the flag is documented as accepted-and-ignored (`let _ = mismatch_override;` with a doc comment). Schema extension to add an override column to `paperless_trip_links` is deferred until a real user need surfaces.

**Trade-offs accepted:** Paperless docs assigned with a mismatch don't surface a "Potvrdené" badge after the fact. The user can still proceed with the assignment via the same modal flow; only the persisted "I confirmed this" state is missing.

**Related:** [Task 64](./_tasks/_done/64-unified-invoice-picker/), [02-design.md "Out of Scope"](./_tasks/_done/64-unified-invoice-picker/02-design.md), [03-plan.md "Loss of mismatch_override for Paperless"](./_tasks/_done/64-unified-invoice-picker/03-plan.md).

---

## 2026-05-03: Paperless-ngx Integration Foundations

### ADR-019: Paperless Trip-Link Table is Symmetric (`trip_id PRIMARY KEY`)

> **Superseded (2026-07-15) by [BIZ-023](#biz-023-invoice-cardinality-sum-on-assign-and-cent-exact-money-math):** multi-invoice support ([Task 66](./_tasks/66-multi-invoice/)) rebuilt the table — `paperless_document_id` is now the PRIMARY KEY (a trip can carry many links), and rows carry `assignment_type`, `amount_eur`/`title`, and `applied_amount_cents` snapshots. The UPSERT is keyed on `paperless_document_id` only. See [migration 2026-07-15-100000_multi_invoice](./src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql) and [docs/features/multi-invoice.md](./docs/features/multi-invoice.md).

**Context:** [paperless_trip_links](./src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql) mirrors the receipt↔trip 1:1 relationship. The existing [receipts](./src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql) table uses `id PRIMARY KEY, trip_id UNIQUE` because receipts carry their own metadata (OCR fields, file path, currency, etc.). Paperless documents live remotely; the link row holds nothing but the IDs.

**Decision:** Use `trip_id TEXT PRIMARY KEY` and `paperless_document_id INTEGER UNIQUE`. A separate surrogate `id` would add no information.

**Consequences:** UPSERT requires deleting both potential prior links (by `trip_id` *and* by `paperless_document_id`) before inserting — encapsulated in [db::upsert_paperless_link](./src-tauri/core/src/db.rs). Tests in [db_tests.rs](./src-tauri/core/src/db_tests.rs) cover the create-then-replace and the unique-doc-invariant cases.

**Related:** [Task 60](./_tasks/60-paperless-integration/), [paperless_trip_links migration](./src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql).

---

### BIZ-015: Paperless DRF Auth Header is `Token`, Not `Bearer`

**Context:** The Home Assistant integration uses `Authorization: Bearer <token>` because HA's REST API expects OAuth2-style bearer tokens. Paperless-ngx uses Django REST Framework token authentication, which expects `Authorization: Token <token>`. A future maintainer copy-pasting the HA wrapper would silently break Paperless auth (responses become 401).

**Decision:** Hardcode `Token` in [test_paperless_connection_internal](./src-tauri/core/src/commands_internal/integrations.rs) and [PaperlessClient::auth](./src-tauri/core/src/paperless.rs); cover with an explicit regression test ([test_paperless_connection_uses_token_auth_header_not_bearer](./src-tauri/core/src/commands_internal/integrations_tests.rs)) and a complementary negative test ([test_paperless_connection_rejects_bearer_header](./src-tauri/core/src/commands_internal/integrations_tests.rs)).

**Consequences:** Every new Paperless HTTP call must use the `Token` prefix. Future Paperless-related issues should grep for `Authorization` first.

**Related:** [Task 60](./_tasks/60-paperless-integration/), [DRF token authentication docs](https://www.django-rest-framework.org/api-guide/authentication/#tokenauthentication).

---

### BIZ-016: Paperless v1 is Single-Vehicle Scoped (`vehicle_id` Intentionally Unused)

**Context:** [get_paperless_invoices_internal(app_dir, db, vehicle_id, year)](./src-tauri/core/src/commands_internal/paperless_cmd.rs) takes a `vehicle_id` parameter but does not filter Paperless results by it. Paperless documents have no native vehicle dimension; the user's tagging scheme uses only `fuel` / `car` for the kniha-jazd integration. Today the user has a single primary vehicle, so multi-vehicle visibility is invisible.

**Decision:** Keep `vehicle_id` on the signature for forward compatibility but intentionally ignore it in v1. Document the deferral via `let _ = vehicle_id;` and a doc-comment in the function so it doesn't read as a bug.

**Alternatives considered:**
- *Drop the parameter from v1.* Rejected — Tasks 13/14 (frontend) already use a vehicle-scoped pattern; changing the IPC contract later is more churn than carrying a no-op param.
- *Implement vehicle scoping via a `vehicle:{name}` Paperless tag now.* Rejected — adds a tagging contract the user hasn't asked for, and the current single-vehicle user has no reason to bear that complexity yet.

**Trade-offs accepted:**
- Multi-vehicle users would see the same invoice list on every vehicle's [doklady](./src/routes/doklady/+page.svelte) page. Acceptable: the current user is single-vehicle; multi-vehicle support is a future iteration gated on explicit user demand.

**Related:** [Task 60](./_tasks/60-paperless-integration/), [paperless_cmd.rs:get_paperless_invoices_internal](./src-tauri/core/src/commands_internal/paperless_cmd.rs).

---

## 2026-04-27: Default-OFF for Route-Based Time Inference

### BIZ-014: Opt-In Auto-Fill of Trip Start/End Times

**Context:** Version 0.33.0 introduced silent auto-fill of new-row start/end datetimes from the most recent matching route (with ±15 min / ±15% jitter; see [calculations/time_inference.rs](./src-tauri/core/src/calculations/time_inference.rs)). The feature is technically correct but UX-hostile: the user types `startDatetime` and `endDatetime`, then picks origin and destination, and their typed values are silently overwritten. There was no indication that this was intentional and no escape hatch — even users who knew about the feature could not opt out short of code changes.

**Decision:** Make `infer_trip_times: Option<bool>` an opt-in setting on [LocalSettings](./src-tauri/core/src/settings.rs) that defaults to OFF (`None` and `Some(false)` both mean disabled). When enabled, surface every inference with a 6-second toast that includes a `Vrátiť` ("Undo") button restoring the pre-inference values for that single row and clearing the row's `inferredKey` so the user can deliberately re-trigger inference if they change their mind.

**Alternatives considered:**
- *Default ON with a discovery toast.* Preserves prior behavior for existing users while adding an in-app way to learn about the feature. Rejected because the very first inference still surprises the user — the no-surprise principle wins over discoverability for an action that overwrites typed input.
- *Default ON without any toast.* The 0.33.0 status quo. Rejected as user-hostile.
- *Remove the feature entirely.* Rejected — users who legitimately repeat the same routes find auto-fill valuable; an opt-in toggle keeps the value while removing the surprise.

**Trade-offs accepted:**
- Existing users who relied on the auto-fill lose it silently after upgrade. Mitigation: prominent [CHANGELOG](./CHANGELOG.md) entry and the in-app discovery path via the toast (visible the first time they enable the toggle).

**Implementation note:** The gate lives at the public command boundary (`get_inferred_trip_time_for_route_internal` in [commands_internal/trips.rs](./src-tauri/core/src/commands_internal/trips.rs)), not inside the pure helpers `compute_inferred_times` / `inferred_trip_time_for_route`. ADR-008 (frontend calculation duplication) and ADR-014 (jitter stays in Rust) are preserved: the calculation core stays a pure function (testable with deterministic jitter); the user setting is read at the orchestration layer.

**Related:** [Task 59](./_tasks/59-time-inference-toggle/), original feature in [v0.33.0 changelog entry](./CHANGELOG.md).

---

## 2026-04-26: Cargo Workspace Split for Tauri/Web Boundary

### ADR-018: Workspace Members Over Feature Flags

**Context:** The headless [`web` binary](./src-tauri/web/src/main.rs) lived in the same crate as the Tauri desktop app, so Cargo linked the entire transitive Tauri/GTK/WebKit dependency graph into the binary even though it never called any Tauri API. The Docker runtime image therefore had to ship ~150 MB of GUI runtime libraries that were never used. Two solutions were on the table: (a) feature-gate `tauri` behind a `desktop` Cargo feature (`#[cfg(feature = "desktop")]` on every wrapper), or (b) split [`src-tauri/`](./src-tauri/) into a workspace with separate crates ([`core/`](./src-tauri/core/), [`desktop/`](./src-tauri/desktop/), [`web/`](./src-tauri/web/)).

**Decision:** Workspace split (option b). [`kniha-jazd-core`](./src-tauri/core/Cargo.toml) is a pure library with no Tauri deps; [`kniha-jazd-desktop`](./src-tauri/desktop/Cargo.toml) holds the Tauri shell + thin `#[tauri::command]` wrappers; [`kniha-jazd-web`](./src-tauri/web/Cargo.toml) depends only on core. Boundary enforced by Cargo's per-crate dep graph, not by `#[cfg]` discipline.

**Reasoning:**
- The `wrapper → _internal` pattern from [Task 55 Server Mode](./_tasks/_done/55-server-mode/) was already screaming for a crate boundary — every `_internal` function was framework-free, every wrapper was Tauri-only.
- Workspace split is **self-enforcing**: a future contributor cannot accidentally couple core code to Tauri because the dep does not exist in [`core/Cargo.toml`](./src-tauri/core/Cargo.toml). With feature flags, that discipline lives in `#[cfg(feature = "desktop")]` annotations on ~74 wrapper functions — easy to forget, easy to break.
- Calendar cost was roughly equal (~3 days for either option).
- Side benefit: two binaries that need separate version metadata, separate publishing cadence, and separate CI build steps line up naturally with two crate manifests.

**Trade-offs accepted:**
- Three Cargo manifests instead of one — slightly more boilerplate when adding new deps (decide which crate gets it).
- Desktop wrappers became thin delegators — extra layer of indirection for any `#[tauri::command]`.
- Migration was mechanical but touched ~30 files in 27 commits.

**Result:** Web binary's dep graph (`cargo tree -p kniha-jazd-web`) contains zero Tauri packages. [Dockerfile.web](./Dockerfile.web) drops GTK/WebKit runtime libs (~150 MB savings, image goes from ~300 MB to ~80 MB target). All 280 backend tests preserved across the move.

**Related:** [Task 58](./_tasks/58-tauri-workspace-split/) (implementation), [Tech Debt #06](./_tasks/_TECH_DEBT/06-tauri-feature-gating.md) (origin).

---

## 2026-04-23: Server Mode Architecture

### ADR-017: LAN-Only CORS Without Authentication

**Context:** The embedded HTTP server exposes the full app API on the local network. Should it require authentication (password, token, etc.)?

**Decision:** No authentication. CORS allowlist restricts origins to RFC 1918 private IP ranges (`10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`) and `localhost`. Any request from a non-LAN origin is blocked by the browser's CORS preflight.

**Reasoning:**
- Target environment is a home or small office LAN — all devices on the network are trusted
- Adding authentication would require password management UI, token storage, and login flow — significant complexity for minimal benefit
- CORS enforcement happens in the browser, which is the only client (no curl/API use case)
- If the user's LAN is compromised, authentication wouldn't help much anyway (attacker could sniff traffic on unencrypted HTTP)
- Same trust model as other LAN devices (printers, NAS, smart home)

**Trade-offs:**
- Anyone on the same LAN can access the app without a password
- No protection against malicious devices on the network (accepted risk for simplicity)

---

### ADR-016: _internal Extraction Pattern for Command Reuse

**Context:** Tauri commands take `tauri::State<Database>` wrappers injected by the framework. The Axum RPC dispatcher has `Arc<Database>` directly. How should both call paths share the same business logic?

**Decision:** Extract pure `_internal` functions from each Tauri command. These take `&Database` and/or `&AppState` as plain references. The Tauri `#[command]` wrapper extracts from `State<>`, the RPC dispatcher passes `&state.db` directly. Both call the same `_internal` function.

**Pattern:**
```
Tauri command (thin wrapper) ──→ _internal(db, args) ←── RPC dispatcher
```

**Reasoning:**
- Zero behavior change — existing tests verify the `_internal` functions work correctly
- No new traits or abstractions needed — just function extraction
- Tauri wrappers become trivially thin (extract state, call internal, return)
- Clean separation: framework concerns (State extraction) vs business logic (pure functions)
- 68 out of 72 commands extracted; 4 remain Tauri-only (file dialogs, DB replacement)

**Rejected alternatives:**
- *Trait-based abstraction* — over-engineered for what is a simple call delegation
- *Separate REST routes* — would require maintaining a parallel API surface (see ADR-015)

---

### ADR-015: RPC Over REST for Server Mode API

**Context:** The embedded HTTP server needs to expose the same 68 commands that Tauri IPC provides. Should we create individual REST endpoints (`GET /api/vehicles`, `POST /api/trips`, etc.) or use a single RPC endpoint?

**Decision:** Single `POST /api/rpc` endpoint accepting `{ "command": "get_vehicles", "args": { ... } }` JSON. The dispatcher maps command names to `_internal` functions.

**Reasoning:**
- Mirrors the Tauri IPC model exactly — `invoke("command", args)` maps 1:1 to `POST /api/rpc` with `{ command, args }`
- No need to design, document, or version 68 separate REST routes
- Frontend adapter is trivial: swap `invoke()` for `fetch('/api/rpc')` based on runtime detection
- Adding new commands requires zero HTTP routing changes — just register in the dispatcher
- Not a public API — only consumed by the same frontend code, so REST conventions (proper HTTP methods, status codes per resource) add no value

**Trade-offs:**
- Not RESTful — all operations are POST, no resource-based URLs
- No HTTP caching (all POST) — acceptable for a LAN app with local-speed responses
- Error responses are always 400 with a string message — no structured error codes

---

## 2026-04-15: Time Inference for New Trip Rows

### ADR-014: Jitter Stays in Rust; Testability via `Jitter` Trait

**Context:** Task 56 introduces auto-fill of start/end datetimes on new trip rows from the most recent matching `(vehicle_id, origin, destination)` trip. To prevent machine-identical timestamps across days, the inferred start is jittered by ±15 minutes and duration by ±15 %. The question was where the jitter should live: Rust backend (consistent with ADR-008) or Svelte frontend (where non-determinism is "easier to test" by injecting a mock random fn).

**Decision:** All inference logic — DB lookup, base-time extraction, **and** the random jitter — lives in the Rust backend. The Tauri command `get_inferred_trip_time_for_route` returns the *final* ISO start/end strings; the frontend writes them directly without any computation.

**Testability pattern:** A `Jitter` trait abstracts the source of randomness:

```rust
pub trait Jitter {
    fn minutes(&mut self) -> i64;        // [-15, 15]
    fn duration_factor(&mut self) -> f64; // [0.85, 1.15]
}
pub struct ThreadRngJitter;     // production: rand::thread_rng
struct StubJitter { /* test */ } // tests: deterministic returns
```

Unit tests (4 in `time_inference.rs`, 4 in `commands_tests.rs`) supply a `StubJitter` so assertions are exact. Production code constructs `ThreadRngJitter` inside the thin `#[tauri::command]` wrapper and calls the same pure helper.

**Reasoning:**
- ADR-008 protects against having calculation logic in two places. Jitter that produces values written into trip records *is* business logic — same category as consumption rates, not the same category as `toFixed()` formatting.
- The trait split keeps tests pure (no `rand::thread_rng()` calls in test code) without requiring randomness to cross the Tauri boundary.
- Future requirement changes (e.g., "use ±10 min instead of ±15") become a one-line change in one place.

**Rejected alternatives:**
- *Frontend jitter (initially proposed)* — would have meant a value-producing computation in Svelte, breaking ADR-008. Rejected during design review.
- *Eager seeding inside `compute_inferred_times`* — would have hard-coded `rand::thread_rng()` and made tests non-deterministic.

---

## 2026-02-12: HA Sensor Display Conversion

### ADR-013: HA Sensor Percentage-to-Liters Conversion Lives in Frontend

**Context:** The new HA real fuel level feature fetches a percentage (0-100%) from a Home Assistant sensor and needs to convert it to liters (`value × tankSize / 100`) for display on the zostatok line. ADR-008 requires all business logic calculations in the Rust backend only.

**Decision:** This conversion stays in the Svelte frontend as display formatting.

**Reasoning:**
- ADR-008 protects against **duplicating calculation logic** (consumption rates, margins, zostatok from trip data). This conversion transforms an external HA sensor reading for display only.
- The backend never uses this value for any calculation — it calculates zostatok independently from trip/fillup data.
- Same category as `toFixed()` or `toLocaleString()` — formatting an external value for display.
- No duplication risk: the HA fuel level and the computed zostatok are independent data sources shown side by side.

---

## 2026-01-29: No Backward Compatibility for Older App Versions

### ADR-012: Forward-Only Database Migration Strategy

**Context:** When adding new database columns or changing schemas, we previously considered maintaining backward compatibility so older app versions could still read databases modified by newer versions.

**Decision:** We are **NOT** enforcing backward compatibility for older app versions reading newer databases.

**What this means:**
- Older app versions may fail to read databases migrated by newer versions
- We don't need to keep legacy columns populated (e.g., `end_time` alongside `end_datetime`)
- Migration strategy is forward-only: users must upgrade the app to use migrated databases
- Code should not include "backward compat" workarounds for legacy fields

**What we DO maintain:**
- Data integrity during migrations (no data loss)
- Clean upgrade path (migrations run automatically on app start)
- Backup creation before migrations (existing behavior)

**Reasoning:**
- Simplifies code by removing legacy field sync logic
- Single-user desktop app - no need for multi-version DB access
- Auto-update ensures users get latest version quickly
- Reduces maintenance burden of dual-column strategies

**Impact on CLAUDE.md:** The database migration guidelines about "older app versions must be able to READ data" should be removed or updated to reflect this decision.

---

## 2026-01-29: Commands Module Split

### ADR-011: Split commands.rs into Feature Modules

**Context:** `commands.rs` has grown to 3,908 lines with 68 Tauri commands. While internally organized with section comments, the file size makes navigation and maintenance difficult.

**Decision:** Split into 9 feature-based modules under `src-tauri/src/commands/`:

| Module | Lines | Commands | Purpose |
|--------|-------|----------|---------|
| `common.rs` | ~180 | 0 | Shared helpers, macros (`check_read_only!`), types |
| `vehicles.rs` | ~130 | 5 | Vehicle CRUD |
| `trips.rs` | ~220 | 8 | Trip CRUD, routes, year-start helpers |
| `statistics.rs` | ~1,170 | 3 | Grid data, calculations, magic fill |
| `backup.rs` | ~400 | 11 | Backup/restore operations |
| `export.rs` | ~280 | 2 | HTML export |
| `receipts.rs` | ~710 | 8 | Receipt scanning, assignment |
| `settings.rs` | ~310 | 15 | Theme, columns, DB location |
| `integrations.rs` | ~180 | 8 | Home Assistant, Gemini API |

**Key decisions:**
- `statistics.rs` exports 3 public helpers for use by `export.rs`: `calculate_period_rates()`, `calculate_fuel_remaining()`, `calculate_fuel_consumed()`
- Year-start helpers (`get_year_start_*`) live in `trips.rs` but are `pub(crate)` for statistics/export
- Tests remain in `commands_tests.rs` initially (can split later)
- `lib.rs` invoke_handler imports from submodules

**Phased approach:**
1. Extract low-risk: `common`, `vehicles`, `backup`
2. Extract complex: `statistics`, `export`, `trips`
3. Extract integrations: `receipts`, `settings`, `integrations`

**Reasoning:**
- Reduces cognitive load when editing a specific feature
- Clearer module boundaries and dependencies
- Enables parallel development on different features
- No functional changes - pure refactoring

---

## 2026-01-12: Additional Costs Recognition

### BIZ-013: Other Cost Invoice Recognition and Assignment

**Context:** Users want to scan and assign non-fuel receipts (car wash, parking, service, etc.) to trips, similar to existing fuel receipt workflow.

**Options considered:**
1. New `ReceiptType` enum with categories (Fuel, CarWash, Parking, Toll, Service, Other)
2. Separate `CostInvoice` table parallel to `Receipt`
3. Binary classification using existing `liters` field (null = other cost)

**Decision:** Use multi-stage matching for classification.

- **Fuel receipt**: `liters != null` AND trip exists where `date + liters + price` match
- **Other cost receipt**: `liters == null` OR no matching trip found

**Why multi-stage:** A receipt for windshield washer fluid (2L / 5€) has liters but isn't fuel. Since no trip has "2L fuel for 5€", it won't match and becomes "other cost" automatically.

**Additional decisions:**
- **Single cost per trip:** One "other cost" invoice per trip. Assignment blocked if `other_costs_eur` already populated.
- **No type categories:** User writes description manually in `other_costs_note` field.
- **Same folder:** All receipts (fuel + other) in same folder, AI auto-classifies.
- **Minimal schema change:** Only 2 new columns: `vendor_name`, `cost_description`.

**Reasoning:**
- Simplest implementation (~6h vs ~13h for enum approach)
- No new enums or types to maintain
- Existing `liters` field already indicates receipt type
- Backward compatible - existing fuel receipts unchanged
- User already has freedom to write any description in note field

**Trade-offs:**
- Cannot filter by specific cost type (parking vs car wash) - only fuel vs other
- User accepted this limitation in favor of simplicity

---

## 2026-01-05: Fuel Carryover

### BIZ-012: Year-End Fuel Carryover Between Years

**Context:** ADR-009 originally specified "zostatok starts fresh (full tank assumption)" for each new year. However, this didn't reflect reality - fuel doesn't magically reset on January 1st.

**Previous behavior:** Each year started with full tank assumption, ignoring actual fuel state from December 31st.

**Decision:** Fuel (zostatok) now carries over from the previous year's ending state.

**Implementation:**
- `get_year_start_zostatok()` calculates carryover from previous year's last trip
- If no previous year data exists, falls back to full tank assumption
- This also prepares for EV support where battery SoC carries over between years

**Reasoning:**
- Matches real-world behavior (fuel doesn't reset on Jan 1)
- Provides accurate consumption tracking across year boundaries
- Enables proper EV battery state tracking (future feature)

**Note:** This supersedes the "zostatok starts fresh" part of ADR-009. The ODO carryover behavior from ADR-009 remains unchanged.

---

## 2025-12-30: Receipt Organization

### ADR-010: Receipt Year Filtering

**Context:** Users may organize receipts in different folder structures - either flat (all files in one folder) or year-based (files in YYYY subfolders like `2024/`, `2025/`). The app needs to handle both cases and filter receipts by year while maintaining clear behavior.

**Decision:**
- **Flat mode:** Files directly in receipts folder → shown in all years (no year filtering)
- **Year-based mode:** Files in YYYY subfolders (e.g., `2024/`) → filtered by selected year
- **Invalid structure:** Mixed content (files + folders) or non-year folders → warning shown, files not loaded
- **Year determination priority:**
  1. Primary: Use `receipt_date.year()` from OCR recognition
  2. Fallback: Use `source_year` from folder name (for unprocessed receipts)
- **Mismatch warning:** When folder year differs from OCR-detected receipt date year, show indicator to user

**Reasoning:**
- Users have different organizational preferences; supporting both flat and year-based is flexible
- OCR date is more accurate than folder placement (user may misfile receipts)
- Folder year serves as fallback for new/unprocessed receipts before OCR runs
- Warning on mismatch helps users identify misfiled receipts without blocking workflow

---

## 2025-12-25: Year Picker

### ADR-009: Year-Scoped Vehicle Logbook

**Context:** Each year is a standalone "kniha jázd" for legal purposes.

**Decision:**
- Year picker in header next to vehicle dropdown
- Stats and trips scoped to selected year
- App starts on current calendar year
- Export only shows years with actual data
- ODO carries over from previous year, zostatok starts fresh (full tank assumption)

**Reasoning:** Slovak legal requirements treat each year as independent logbook. Fresh zostatok per year simplifies accounting.

---

## 2025-12-25: Architecture Refactor

### ADR-008: Remove Frontend Calculation Duplication

**Context:** Frontend (`src/lib/calculations.ts`) duplicated Rust backend calculations (`src-tauri/src/calculations.rs`) "for instant UI responsiveness."

**Problem:**
- ~500 lines of duplicate code
- 21 frontend tests duplicating 41 backend tests
- Risk of logic divergence between frontend and backend
- Double maintenance burden

**Options considered:**
1. Keep duplication - test both implementations
2. Move all to Rust - frontend calls Tauri commands
3. Move all to frontend - backend becomes thin data layer

**Decision:** Move all calculations to Rust backend only.

**Reasoning:**
- Tauri IPC is local and fast (microseconds, not network)
- No other clients will ever exist - single desktop app
- Rust backend already has 41 well-tested calculation functions
- Single source of truth eliminates divergence risk
- Frontend becomes simpler display-only logic

**Implementation:** Add `get_trip_grid_data` Tauri command returning pre-calculated values.

---

## 2025-12-23: UI/UX Decisions

### ADR-007: Database Backup/Restore

**Context:** User needs ability to backup and restore database for data safety.

**Decision:**
- Backups stored in `{app_data_dir}/backups/`
- Manual trigger only (no auto-backup)
- Filename: `kniha-jazd-backup-YYYY-MM-DD-HHmmss.db`
- Restore: Full DB replacement with confirmation showing date, counts, warning
- Keep all backups (no auto-deletion)

**Reasoning:** Simple, transparent backup system. User controls when to backup/restore.

---

### ADR-006: Navigation Header

**Context:** Settings button was buried at bottom of page, requiring scroll.

**Decision:** Top header bar with "Kniha jázd | Nastavenia" navigation links.

**Reasoning:** Always visible, no scrolling needed, clear app structure.

---

### ADR-005: Totals Section Redesign

**Context:** Original single-row totals were cramped and unclear.

**Decision:**
- Two-row layout for totals
- Rename "Km" to "Celkovo najazdené" for clarity
- Show fuel totals and cost summary on separate row

**Reasoning:** Better readability, clearer labels for legal documentation.

---

## 2025-12-23: Calculation Logic Fixes

### BIZ-011: Legal Limit Based on Average Consumption

**Context:** Should the 20% over-limit warning use the last fill-up rate or overall average?

**Decision:** Use **average consumption** (total_fuel / total_km × 100) for legal compliance check.

**Reasoning:** Legal compliance is about the overall picture, not a single fill-up. If average is 6.00 and limit is 6.12 (5.1 × 1.2), we're compliant even if one fill-up was higher.

---

### BIZ-010: Retroactive Consumption Rate Application

**Context:** When a fill-up occurs, which trips should use that rate?

**Decision:** Apply the rate **retroactively** to ALL trips since the previous fill-up.

**Example:** If trips A, B, C happen, then fill-up on C gives rate 6.0 → A, B, and C all show 6.0 l/100km.

**Reasoning:** Matches Excel behavior. The rate represents the consumption for that entire period.

---

### BIZ-009: Same-Day Trip Ordering

**Context:** Multiple trips on the same date need deterministic ordering for correct calculations.

**Decision:** Sort by date, then by **odometer** as tiebreaker.

**Reasoning:** Odometer is sequential and represents actual trip order. Using created_at would fail for imported data.

---

### BIZ-008: ODO Auto-Calculation

**Context:** Manual ODO entry is error-prone and redundant since ODO = previous ODO + km driven.

**Decision:** Auto-calculate ODO when km is entered: `ODO = previousODO + km`. User can still manually override.

**Reasoning:** Reduces data entry errors, matches Excel workflow where this was a formula.

---

## 2024-12-23: Business Logic Decisions

### BIZ-007: Fill-up Detection

**Context:** How to distinguish regular trips from fill-ups?

**Decision:** Auto-detect. If liters field is filled → it's a fill-up. No separate entry types.

**Reasoning:** Simpler UX, matches Excel behavior.

---

### BIZ-006: UI Display Order vs Export Order

**Context:** How to show trips in UI vs PDF export?

**Decision:**
- UI: Newest trips on top (reverse chronological) - easier access
- Export: Oldest first (chronological) - matches Excel/legal format

---

### BIZ-005: Route Distance Memory

**Context:** User often drives same routes.

**Decision:** Store origin→destination pairs with their distances. When user selects a known route, auto-fill the km field.

**Reasoning:** Reduces data entry, fewer errors.

---

### BIZ-004: Compensation Trip Suggestions

**Context:** How to help user plan trips to stay within legal margin?

**Decision:**
1. Calculate km needed to bring margin under limit
2. First, try to find existing route from current location matching needed km (±10%)
3. Fallback: Suggest buffer trip with configurable purpose (e.g., "služobná cesta")
4. Target margin: 16-19% (provides safety buffer below 20% limit)

**Reasoning:** Maintaining a buffer below the 20% limit helps ensure compliance even with measurement variations.

---

### BIZ-003: Legal Margin Limit

**Context:** What's the allowed over-consumption?

**Decision:** Max 20% over the vehicle's TP (technical passport) consumption rate.

**Example:** TP = 5.1 l/100km → Max allowed = 6.12 l/100km

---

### BIZ-002: Pouzita Spotreba (Used Consumption Rate)

**Context:** What rate is used to calculate fuel consumption between fill-ups?

**Decision:**
- Initial value: TP rate from vehicle (e.g., 5.1 l/100km)
- After first fill-up: Use the calculated l/100km from that fill-up
- Rate carries forward until next fill-up recalculates it

**Validation:** Matches Excel pattern - each fill-up sets the rate for subsequent trips.

---

### BIZ-001: Consumption Rate Calculation

**Context:** How is l/100km calculated?

**Decision:** On each fill-up: `l/100km = liters_filled / km_since_last_fillup × 100`

**Validation:** Verified against Excel data - formula matches exactly.

---

## 2024-12-23: Architecture Decisions

### ADR-004: Code in English, UI in Slovak

**Context:** User is Slovak, app is for Slovak legal requirements.

**Decision:**
- All code, variables, comments: English
- UI text: Slovak with i18n support for future translation

**Reasoning:**
- English code is industry standard, easier to maintain
- Slovak UI serves the primary user
- i18n-ready for potential future users

---

### ADR-003: Test-Driven Development

**Context:** Need reliable calculations for legal compliance (20% margin rule).

**Decision:** TDD with focus on business logic tests only

**Reasoning:**
- Calculation errors = legal compliance issues
- Tests must be meaningful, not filler
- Focus: consumption calculations, margin checks, compensation suggestions
- Skip: trivial CRUD, UI rendering, getters/setters

---

### ADR-002: SQLite for Local Storage

**Context:** Need to store trips, vehicles, and calculated data.

**Decision:** SQLite (single local file)

**Reasoning:**
- Simple, portable, robust
- Single file easy to backup/move
- Can still export to Excel/CSV for accountants
- No server needed for personal logbook

---

### ADR-001: Desktop App with Tauri + SvelteKit

**Context:** Need to build a vehicle logbook app to replace Excel spreadsheet.

**Options considered:**
1. Electron + React/Vue - Cross-platform, larger bundle (~150MB+)
2. Tauri + SvelteKit - Cross-platform, Rust backend, small bundle (~10-20MB)
3. Python + PyQt - Good for data apps, simpler
4. C# WPF - Windows-only, excellent Excel interop
5. .NET MAUI + Blazor - Cross-platform, C# everywhere

**Decision:** Tauri + SvelteKit

**Reasoning:**
- User said "don't limit ourselves" - open to learning Rust
- Best end-user experience (small, fast, native)
- Svelte is the simplest modern frontend framework
- No need for Excel interop - reimplementing functionality, not integrating
