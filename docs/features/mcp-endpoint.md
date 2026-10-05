# Feature: Read-Only MCP Endpoint

> An AI assistant reads vehicles, trips and journeys away from home at `/mcp`. No tool can change the logbook.

The endpoint serves the Model Context Protocol (MCP) over HTTP. Its first use is a check
of business trips. The assistant lists the journeys of a month, then looks for a travel
order for each journey in a document archive. The app does not decide which journey needs
a travel order. See [BIZ-026](../../DECISIONS.md#biz-026-journeys-away-from-home).

## User Flow

1. **Mark the home place.** Open the Miesta tab. Click the house icon on the row of the
   home place. Only one place is home. See [place-book.md](./place-book.md).
2. **Connect an MCP client** to `https://<host>/mcp`. Use the host and port of the app.
   If a reverse proxy filters paths, let `/mcp` through.
3. **Call the tools.** The client lists three tools: `list_vehicles`, `list_trips` and
   `list_journeys`.
4. **Read the answer.** Each answer is a JSON object. The assistant applies its own rule
   to `nights` and `total_km`.

## Technical Implementation

### Backend (Rust)

- [mcp/mod.rs](../../src-tauri/core/src/mcp/mod.rs): the tools, the input and output
  types, and `mcp_service`. The module holds only a `LogbookReader`.
- [journeys_cmd.rs](../../src-tauri/core/src/commands_internal/journeys_cmd.rs):
  `LogbookReader`, the read-only facade. It keeps its `Database` private and has three
  reads: `vehicles`, `trips_in_range` and `journeys`.
- [journeys/mod.rs](../../src-tauri/core/src/journeys/mod.rs): `group_journeys` and
  `overlaps`. Both are pure functions. They use no database and no clock.
- [server/mod.rs](../../src-tauri/core/src/server/mod.rs): `.nest_service("/mcp", mcp)`,
  in both router branches, before the SPA fallback.

The tools call the core reads directly. They do not go through `/api/rpc`.

### The tools

Dates are `YYYY-MM-DD`. Both ends are inclusive. A date such as `2026-1-1` is an error.
`vehicle_id` is optional. **If `vehicle_id` is missing, the tool covers all vehicles.**
`is_active` is not a filter: it marks only the vehicle that is selected in the app UI.
The date range has no upper limit: a range of many years returns the whole logbook. The
logbook of one user is small, the tools only read, and a limit would force the model to
page by year (decision of 2026-10-05).

**`list_vehicles()`**

```json
{"vehicles": [{"id": "...", "name": "...", "license_plate": "...", "is_active": true}]}
```

**`list_trips(date_from, date_to, vehicle_id?)`** returns the trip rows (legs) whose start
is in the range. The sort order is `start`, then `odometer`.

```json
{"trips": [{"id": "...", "vehicle_id": "...", "start": "2026-03-02T07:30",
            "end": "2026-03-02T09:10", "origin": "...", "destination": "...",
            "distance_km": 120.0, "purpose": "..."}]}
```

`start` and `end` are `YYYY-MM-DDTHH:MM`. `end` is `null` if the trip has no end time.
`origin` and `destination` are place names.

**`list_journeys(date_from, date_to, vehicle_id?)`** returns each journey that overlaps
the range.

```json
{"home_place": "...",
 "journeys": [{"vehicle_id": "...", "start": "2026-03-02T07:30",
               "end": "2026-03-04T18:00", "nights": 2, "total_km": 740.5,
               "places": ["..."], "purposes": ["..."], "complete": true,
               "leg_ids": ["..."]}]}
```

| Field | Value |
|-------|-------|
| `start` | Start of the first leg |
| `end` | Start of the return leg, or `null` if the journey is incomplete |
| `nights` | Calendar days between the start date and the end date, or `null` |
| `total_km` | Sum of `distance_km` of all legs |
| `places` | The distinct destinations in order, without home |
| `purposes` | The distinct purposes in order. A blank purpose is left out. |
| `complete` | `true` if a return leg exists |
| `leg_ids` | The trip IDs in order |

### The journey rules

A trip row is one **leg**. A **journey** is a chain of legs away from home. The grouping
runs for each vehicle. It sorts the legs by `start_datetime`, then by `odometer`.

1. A leg endpoint is home if its place ID is the ID of the home place.
2. A journey starts at a leg from home to a place that is not home.
3. A journey ends at the next leg from a place that is not home to home.
4. A single leg from home is a **day trip**, and it is not a journey. This is true if its
   saved map has `round_trip = true`, or if the next leg starts at home.
5. A leg from home to home is never a journey.
6. A chain with no return leg is **incomplete**: `complete = false`, `end = null`,
   `nights = null`. This happens in two cases:
   - the chain is open after the last leg of the vehicle;
   - a chain of **two or more legs** meets a new leg from home (a new departure or a
     home to home loop). The new departure starts a new chain.
7. Legs outside a chain are ignored.

A journey **overlaps** the range if one day of it is in the range. A broken chain lasts
until the day of the leg from home that broke it. Only the chain that is open after the
last leg of the vehicle is open towards the future. The reader groups all legs first and filters after, so a
journey that starts before `date_from` is found.

**Example 1: a complete journey.** Home is `Home St 1, Hometown`.

| Start | Origin | Destination | km |
|-------|--------|-------------|----|
| 03-02 07:30 | Home St 1, Hometown | Main Sq 5, City A | 357 |
| 03-02 13:00 | Main Sq 5, City A | Depot Rd 9, City A | 8 |
| 03-04 16:00 | Depot Rd 9, City A | Home St 1, Hometown | 360 |

Result: one journey. `complete` is `true`, `nights` is 2, `total_km` is 725, `places` is
`["Main Sq 5, City A", "Depot Rd 9, City A"]`.

**Example 2: a broken chain.**

| Start | Origin | Destination | km |
|-------|--------|-------------|----|
| 03-02 07:30 | Home St 1, Hometown | Main Sq 5, City A | 357 |
| 03-03 09:00 | Main Sq 5, City A | Hill Rd 2, City B | 90 |
| 03-06 08:00 | Home St 1, Hometown | Mill St 3, Village | 52 |
| 03-06 17:00 | Mill St 3, Village | Home St 1, Hometown | 52 |

Result: two journeys. The first has two legs and no return leg before the new departure,
so it is incomplete (`end` is `null`). The second is complete with `nights` 0. If the
first chain had only one leg, that leg would be a day trip and would give no journey.

### Transport

The service is **stateless**, with `rmcp` 3.5 and streamable HTTP:
`legacy_session_mode(false)`, `NeverSessionManager`, `json_response(true)`, no SSE
keep-alive. The server sends no session ID. A client that cached an ID before a restart
keeps working. The answer is a plain JSON body.

### Read-only rule

The `mcp` module holds a `LogbookReader` and never names `Database`: the server builds
the reader and gives it to `mcp_service`. The test `mcp_read_path_has_no_write_access`
([mcp/tests.rs](../../src-tauri/core/src/mcp/tests.rs)) reads the source of `mcp/` and of
`journeys_cmd.rs`. It accepts only the names in the allowlist `ALLOWED_READS` for each
`crate::` item and each `db.` method. The type `Database` may appear only in the import
and as `db: Arc<Database>`, so every handle is named `db` and the method check sees each
call on it. The test also bans glob imports, `use ... as ...` and a list of write words. A
new function must be added to the allowlist on purpose. A `db` use that the guard cannot
read fails the test.

### Host check and auth

`rmcp` accepts only loopback `Host` headers by default. Behind a reverse proxy the `Host`
header is the public name, so the default rejects every request. The service calls
`disable_allowed_hosts()`. `/api/rpc` has no host check and no auth, and it can write.
A read-only `/mcp` adds no new risk. `/mcp` has no auth either. Do not expose the port to
an untrusted network.

### Errors

A tool error is a **tool result** with `isError: true` and the message as text, not a
JSON-RPC error (MCP spec, "Error Handling"). So the model reads the message and can fix
the call. A JSON-RPC error is only for a protocol fault, for example an unknown tool or
arguments that do not match the input schema.

| Case | Message |
|------|---------|
| A date that is not `YYYY-MM-DD` | `date_from must be YYYY-MM-DD, got '...'` |
| `date_from` after `date_to` | `date_from is after date_to` |
| An unknown vehicle | `Vehicle not found` |
| `list_journeys` and no home place | the message names the Miesta page |
| A database error | `Internal error: ...` |

The first version sent these as JSON-RPC error `-32602`. Some clients do not show a
JSON-RPC error to the model, so the model never saw the hint about the Miesta page.

The home place can be deleted on the Miesta tab when no trip uses it. Then the home
mark is gone with it, and `list_journeys` returns the "no home place" error until the
user marks another place.

"No home place" is an error and not an empty list. An empty list would look like
"no journeys to check".

### Data Flow

```
MCP client -> POST /mcp -> rmcp service -> tool -> spawn_blocking -> LogbookReader -> Database (reads)
                                                                          |
                                           list_journeys: group_journeys (pure) -> filter by range
```

## Key Files

| File | Purpose |
|------|---------|
| [mcp/mod.rs](../../src-tauri/core/src/mcp/mod.rs) | Tools, types, `mcp_service` |
| [mcp/tests.rs](../../src-tauri/core/src/mcp/tests.rs) | Tool tests and the source guard |
| [commands_internal/journeys_cmd.rs](../../src-tauri/core/src/commands_internal/journeys_cmd.rs) | `LogbookReader`, `ReadError` |
| [journeys/mod.rs](../../src-tauri/core/src/journeys/mod.rs) | `group_journeys`, `overlaps` (pure) |
| [journeys/tests.rs](../../src-tauri/core/src/journeys/tests.rs) | The journey rules, all edge cases |
| [server/mod.rs](../../src-tauri/core/src/server/mod.rs) | The `/mcp` route |
| [mcp-endpoint.spec.ts](../../tests/integration/specs/tier2/mcp-endpoint.spec.ts) | `tools/list` over HTTP |
| [home-place.spec.ts](../../tests/integration/specs/tier2/home-place.spec.ts) | The home icon on Miesta, the precondition of `list_journeys` |

## Design Decisions

- **Why stateless?** A cached session ID keeps working after a restart. See
  [ADR-057](../../DECISIONS.md#adr-057-a-read-only-mcp-endpoint-at-mcp-stateless-and-read-only-by-construction).
- **Why not `/api/rpc`?** It accepts write commands and has no auth. The assistant must
  not reach a write.
- **Why a source guard?** A rule in a comment does not stop a later change. The test fails
  when the module gets a path to a write.
- **Why does no vehicle mean all vehicles?** `is_active` marks the vehicle selected in the
  UI. It is not an archive flag.
- **Why are the answers objects?** MCP `structuredContent` must be an object, so the lists
  are wrapped in `vehicles` and `trips`.
- **Why a chain of two legs is incomplete and a single leg is a day trip?** A chain of two
  or more legs is a real trip that lost its return leg. The data must show it. See
  [BIZ-026](../../DECISIONS.md#biz-026-journeys-away-from-home).

## Testing

- **Backend unit tests** own the rules: [journeys/tests.rs](../../src-tauri/core/src/journeys/tests.rs)
  (every journey pattern), [mcp/tests.rs](../../src-tauri/core/src/mcp/tests.rs) (the tool
  answers, the `initialize` handshake, the tool errors, a stale session ID and a public host) and the source guard.
- **Integration test** ([mcp-endpoint.spec.ts](../../tests/integration/specs/tier2/mcp-endpoint.spec.ts))
  owns the HTTP path: `tools/list` returns the three tools.

## Related

- [place-book.md](./place-book.md): the home mark
- [server-mode.md](./server-mode.md): the router, Docker deployment, no auth
- [read-only-mode.md](./read-only-mode.md): `/mcp` works in read-only mode
- [route-maps.md](./route-maps.md): `round_trip` makes a day trip
- [ADR-057](../../DECISIONS.md#adr-057-a-read-only-mcp-endpoint-at-mcp-stateless-and-read-only-by-construction): the endpoint
- [BIZ-026](../../DECISIONS.md#biz-026-journeys-away-from-home): the journey rules
- [ADR-008](../../DECISIONS.md#adr-008-remove-frontend-calculation-duplication): logic in Rust
- [_tasks/_done/89-home-place-mcp/](../../_tasks/_done/89-home-place-mcp/): task and plan
