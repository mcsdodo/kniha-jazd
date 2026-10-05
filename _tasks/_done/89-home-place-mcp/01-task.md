**Date:** 2026-10-05
**Subject:** Home place setting, journey grouping in core, and a read-only MCP endpoint at `/mcp`
**Status:** Complete
**Depends on:** [Task 88: Places as Entities](../../_done/88-places-as-entities/01-task.md). Write `02-plan.md` only after 88 is merged, because 88 moves the code this plan will cite.

## Goal

An AI assistant must read the logbook to find business trips. Then it checks that each
journey has a travel order (`cestovný príkaz`) in a document archive. The assistant must
not be able to change the logbook.

`POST /api/rpc` has no auth and accepts write commands, so the assistant cannot use it.
This task adds a separate surface that can only read: an MCP endpoint at `/mcp` with
three tools. One tool groups trip rows into journeys away from home. For that, the app
needs a home place, which it does not have today.

This task comes from a homelab task (sub-project S1, decisions D3 and D6). The homelab
side redeploys the stack, sets the home place and connects the assistant. This task
does not deploy.

## Current state

After [task 88](../../_done/88-places-as-entities/01-task.md), a place is a record with an `id`, and
each trip has `origin_place_id` and `destination_place_id`. The app still has no home. The
only home base is node 0 of the static route-map dataset
([ga.rs:29](../../../src-tauri/core/src/route_map/ga.rs#L29)), which is not a user setting.

| Item | Today (after task 88) |
|------|-------|
| Places | Table `places` with `id`, `name`, `normalised_name`, coordinates. UI on the Miesta tab. |
| Round-trip map | `trip_routes.round_trip -> Bool` ([schema.rs:113](../../../src-tauri/core/src/schema.rs#L113)) marks a saved map that goes there and back on one trip row. |
| Router | [server/mod.rs:158-190](../../../src-tauri/core/src/server/mod.rs#L158): `/health`, `/api/rpc`, `/api/capabilities`, SPA fallback. Axum 0.8. |
| MCP | None. `rmcp` is not a dependency. |

## Requirements

### 1. Home place

- Add a column `places.is_home` (BOOLEAN NOT NULL DEFAULT 0), with a migration. A partial
  unique index (`WHERE is_home = 1`) allows only one home.
- RPC command `set_home_place(id | null)`. It clears the old mark and sets the new one in
  one transaction, and it guards with `check_read_only!`. `list_places` returns `isHome`.
- **UI:** a home icon button on each row of the Miesta list.
  - A click on a row marks that place as home. A click on the current home removes the mark.
  - A new mark replaces the old one.
  - The home row shows a filled icon. The other rows show an outline icon.
  - All strings go through i18n, Slovak first.

### 2. Journey grouping in `kniha-jazd-core`

A trip row is one **leg**. A **journey** is a chain of legs away from home.

**Home match.** A leg endpoint is home if its place ID is the ID of the place with
`is_home = 1`.

**Order.** Sort the legs of one vehicle by `start_datetime`, then by `odometer`. Several
legs can start on the same day.

**Rules:**

1. A journey **starts** at a leg whose origin is home and whose destination is not home.
2. A journey **ends** at the next leg whose origin is not home and whose destination is home.
3. A **single-leg day trip** is never a journey. A home -> X leg is a day trip if:
   - its saved map has `round_trip = true`, also when it is the newest row; or
   - the next leg starts at home again (a new departure or a home -> home loop).
4. A home -> home leg (a loop) is never a journey.
5. A chain away from home with no return leg is an **incomplete** journey:
   `complete = false`, `end = null`, `nights = null`. This applies in two cases:
   - the chain is still open after the last leg of the vehicle (also a single
     home -> X leg with no round-trip map); or
   - the chain has two or more legs, and the car then leaves home again or drives a
     home -> home loop before any return leg (home -> A, A -> B, home -> C). The new
     departure starts a new chain.
6. Legs outside a journey are ignored. Examples: a return leg with no departure before it,
   and away legs before the first departure in the data.

**Overlap.** A journey overlaps a date range if any part of it is in the range. An
incomplete journey is open at the end. The query returns each journey that overlaps the
range, so a journey across a month boundary shows in both months. The grouping reads all
legs of the vehicle, groups them, and then filters the journeys. So a journey that starts
before `date_from` is still found.

**Fields of a journey:**

| Field | Value |
|-------|-------|
| `vehicle_id` | The vehicle |
| `start` | `start_datetime` of the first leg |
| `end` | `start_datetime` of the return leg, or `null` |
| `nights` | Calendar days between the date of `start` and the date of `end`, or `null` |
| `total_km` | Sum of `distance_km` of all legs |
| `places` | The distinct destinations, in order (not home) |
| `purposes` | The distinct purposes, in order |
| `complete` | `true` if a return leg exists |
| `leg_ids` | The trip IDs, in order |

**No accounting rule.** The app does not decide which journey needs a travel order. The
consumer applies its own rule to `nights` and `total_km`.

**No home place.** If no place has `is_home = 1`, the grouping returns an error. It must not
return an empty list, because "no journeys" would look like "nothing to check".

**Structure.** A pure function takes the legs, the home place ID and the set of trip IDs with
a round-trip map. It returns the journeys and does no DB access. A thin wrapper reads the
DB and calls it. Add a DB read for the trips of a vehicle in a date range (for
`list_trips`).

### 3. MCP endpoint

- Transport: **streamable HTTP at `/mcp`** on the existing Axum server, with the `rmcp`
  crate (3.5).
- **Stateless:** `legacy_session_mode(false)`, no session IDs. A client that cached a
  session ID must keep working after a server restart. Use `json_response(true)`.
- **Host check off.** `rmcp` accepts only loopback `Host` headers by default
  (`StreamableHttpServerConfig::allowed_hosts`). Behind a reverse proxy, the host is the
  public name, so the default rejects every request. `/api/rpc` has no host check and
  accepts writes, so a read-only `/mcp` without the check adds no new risk.
- The tools call the core read functions directly. They do not go through `/api/rpc`.
  The MCP module imports no write function.
- No auth, the same as `/api/rpc`.
- Mount `/mcp` before the SPA fallback, in both router branches.

**Tools, exactly these three:**

```
list_vehicles() -> {vehicles: [{id, name, license_plate, is_active}]}

list_trips(date_from, date_to, vehicle_id=None) -> {trips: [
  {id, vehicle_id, start, end, origin, destination, distance_km, purpose}
]}   # vehicle_id=None -> all vehicles

list_journeys(date_from, date_to, vehicle_id=None) -> {
  home_place: "...",
  journeys: [{vehicle_id, start, end, nights, total_km, places, purposes,
              complete, leg_ids}]
}   # error if the home place is not set
```

- Dates are `YYYY-MM-DD`. Both ends are inclusive.
- `vehicle_id=None` means **all vehicles**. `is_active` is not an archive flag: it marks the
  one vehicle that is selected in the UI (`set_active_vehicle_internal`). A merged
  `list_trips` result is sorted by `start_datetime`, then by `odometer`.
- Each tool returns a JSON object, because MCP `structuredContent` must be an object.
  So the lists are wrapped in `vehicles` and `trips`.
- Each tool has a description that says what it returns and that it is read-only.

## Tests

### Backend unit tests (journey grouping)

The fixture uses invented addresses. Home = `Home St 1, Hometown`.

| Pattern | Expected |
|---|---|
| home -> City A (357 km), 3 short legs in City A, City A -> home 2 days later | 1 journey, `nights` = 2, `complete` = true |
| home -> City A -> City B -> City A -> home over 4 days | 1 journey, `places` in order |
| home -> Village (52 km), one leg, the next leg starts at home | 0 journeys |
| home -> Village, round-trip map, newest row | 0 journeys |
| home -> home loop (204 km) | 0 journeys |
| home -> Workshop -> other town -> home, same day, 92 km | 1 journey, `nights` = 0 |
| home -> City A and back on the same day, 1011 km in total | 1 journey, `nights` = 0, `total_km` = 1011 |
| a journey from Jan 27 to Feb 10 | returned for the January range and for the February range |
| the last leg leaves home (no round-trip map), no return yet | 1 journey, `complete` = false, `end` = null |
| home -> City A -> City B, then home -> Village -> home (no return from City B) | 2 journeys: the first `complete` = false, `end` = null, `nights` = null; the second complete |
| home -> City A -> City B, then a home -> home loop | 1 journey, `complete` = false |
| two legs with the same `start_datetime` | the order comes from `odometer` |
| home place not set | error |

### Integration tests

- `tools/list` on `/mcp` returns exactly the three tools.
- Miesta: a click on the home icon of a place marks it, and a reload keeps the mark.

## Decisions to record (`/decision`)

- **ADR:** MCP at `/mcp` over stateless streamable HTTP with `rmcp`; read-only by
  construction; no auth; host check off, and why.
- **BIZ:** the journey rules, the home match by place ID, the round-trip-map day trip,
  and "no home place is an error".

## Documentation

- New [docs/features/mcp-endpoint.md](../../../docs/features/): the three tools, the
  journey rules, stateless transport, read-only rule, host check.
- [docs/features/place-book.md](../../../docs/features/place-book.md): the home mark.
- Check **every** other feature doc in [docs/features/](../../../docs/features/) and update
  each doc that the home place or `/mcp` affects (for example
  [server-mode.md](../../../docs/features/server-mode.md) for the new path). In the commit message, list
  each doc as "updated" or "checked, no change".

## Done when

- [ ] The home mark is in the database (migration) and in the Miesta UI (home icon).
- [ ] The journey grouping is in core with the unit tests above.
- [ ] `/mcp` serves the three tools, and the integration test passes.
- [ ] [DECISIONS.md](../../../DECISIONS.md) and [CHANGELOG.md](../../../CHANGELOG.md) are updated. The changelog has the
      `### Pokyny k aktualizácii` block (new migration, new `/mcp` path).
- [ ] The feature docs are updated (see Documentation).
- [ ] [README.md](../../../README.md) and [README.en.md](../../../README.en.md) mention the MCP endpoint.
- [ ] The change is on `main`, and the publish job built the `:main` image.

This repo is public. Do not add a homelab address, host, IP or real trip data.
