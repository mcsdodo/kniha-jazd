**Date:** 2026-05-03
**Subject:** Trip route map generator (POC) — design
**Status:** Planning

## Architecture

```
┌─────────────────────────────────────────────────┐
│              poc.html (single file)             │
│   ┌─────────────────────────────────────────┐   │
│   │  Form: target km, algorithm switch      │   │
│   └─────────────────────────────────────────┘   │
│                       ↓                         │
│   ┌─────────────────────────────────────────┐   │
│   │  Algorithm (in-browser JS)              │   │
│   │  - Reads matrix.json + villages.json    │   │
│   │  - Picks waypoint sequence (offline)    │   │
│   └─────────────────────────────────────────┘   │
│                       ↓                         │
│   ┌─────────────────────────────────────────┐   │
│   │  OSRM /route (one network call)         │   │
│   │  - Returns polyline geometry for chosen │   │
│   │    sequence                             │   │
│   └─────────────────────────────────────────┘   │
│                       ↓                         │
│   ┌─────────────────────────────────────────┐   │
│   │  Leaflet render: polyline only, no pins │   │
│   └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

All algorithm iteration is offline against [matrix.json](./matrix.json) (zero
network calls). The single OSRM `/route` call happens *after* the algorithm picks
the final waypoint sequence, purely to fetch the polyline geometry for rendering.

## Data files (committed alongside this design)

### [villages.json](./villages.json)

- 67 nodes total: home base at index 0, 22 cities/towns at indices 1..22 (within
  50 km aerial of home, any population), 44 villages at indices 23..66 (within
  20 km aerial of home, population ≥ 500).
- Each node: `{ idx, name, lat, lon, population?, kind }` where `kind` is one
  of `home`, `town`, `village`.
- Population from OSM tags. Used by the algorithm as a tie-breaker / weighting
  hint (prefer larger settlements as primary destinations, smaller villages as
  fine-tuning detours).
- Filter rationale: small hamlets (pop < 500) tend to be on minor roads where
  routing is unreliable and inclusion in a "proof-of-driving" route looks
  unnatural. The 500-pop floor keeps the dataset to villages with real road
  infrastructure.

### [matrix.json](./matrix.json)

- 67×67 driving distance matrix in km (sourced via OSRM `/table` call on
  2026-05-03; see [_fetch-matrix.ps1](./_fetch-matrix.ps1) for the script).
- **Asymmetric**: `dist[A][B] ≠ dist[B][A]` preserved (one-way streets, different
  routing direction). The algorithm respects this by treating sequence ordering
  as significant.
- Distances rounded to 0.1 km — sufficient precision for ±5% tolerance matching.
- File size ~38 KB committed.

## Algorithm — both implementations switchable in UI

Both share the same input/output contract:

```
Input:  targetKm: number
Output: { sequence: number[], totalKm: number, errorPercent: number }
        where sequence[0] = sequence[last] = 0 (home), and intermediates are
        node indices 1..66.
```

### A. Insertion heuristic (greedy)

```
1. Pick primary destination D where 2 * matrix[0][D] ≈ targetKm
   (i.e., direct out-and-back to D approximates the target).
2. sequence = [0, D, 0]
3. While currentKm < targetKm * (1 - tolerance) AND len(sequence) < maxStops + 2:
     gap = targetKm - currentKm
     For each unvisited node V (1..66):
       For each insertion position P in sequence (excluding endpoints):
         Compute newKm if V inserted at P
         delta = newKm - currentKm
         score = abs(delta - gap)   // how close insertion brings us to target
       Track best (V, P)
     If best score < currentScore: insert V at P
     Else: break (no insertion improves)
4. Return sequence + total
```

Cost: O(N² × maxStops) matrix lookups per call. With N=66 and maxStops=5 that's
~22k lookups, sub-millisecond.

### B. Genetic algorithm

```
- Chromosome: [home] + variable-length permutation of node indices 1..66 + [home]
              (length 1..maxStops villages between)
- Fitness: 1 / (1 + abs(totalKm - targetKm))   (higher is better)
- Population: 50 chromosomes
- Generations: 100
- Selection: tournament (size 3)
- Crossover: order crossover (OX) — preserves permutation validity
- Mutation: with p=0.2, randomly insert/remove/swap one node
- Elitism: top 2 chromosomes carried over each generation
- Return: best chromosome found
```

Cost: 50 × 100 = 5000 fitness evaluations × ~5 matrix lookups = ~25k lookups,
still well under 100 ms.

### Why both

Insertion heuristic is deterministic and predictable but can get stuck in local
optima (e.g., always picks the same primary destination for a given target).
GA explores more of the search space and produces variety between runs (good
for "I need to generate evidence for several similar trips that shouldn't all
look identical"), at the cost of being slower and non-deterministic.

The UI toggle lets us A/B them empirically and decide later which (or both) to
keep when promoting beyond POC.

### Why 67 candidate nodes is right-sized

With 22 cities/towns spanning a 50 km radius, the algorithm has strategic
"destination" options that define route shape (e.g., heading toward Poprad vs.
Rožňava is a real geographic decision). The 44 villages within 20 km provide
fine-grained padding options for the algorithm to land precisely on target km
— a 1.4 km mini-loop through Smižany is a different lever than a 38 km swing
out to Levoča and back. Together the pool covers both coarse and fine route
adjustment without being overwhelmed by hamlet-level minor roads.

## Multi-session split

```
SINGLE_SESSION_MAX_KM = 400
if targetKm > SINGLE_SESSION_MAX_KM:
    sessionCount = ceil(targetKm / SINGLE_SESSION_MAX_KM)
    perSession = targetKm / sessionCount
    Generate `sessionCount` independent loop routes, each targeting `perSession`.
    Render with a tab strip / numbered selector ("Session 1 of N").
```

Rationale: a single day's driving from one home base rarely exceeds 400 km
practically. Multiple sessions = multiple separate proof-of-driving entries.

## Rendering

- **Leaflet** via CDN: [unpkg.com/leaflet@1.9.4](https://unpkg.com/leaflet@1.9.4).
- **Tiles**: OSM standard at `https://tile.openstreetmap.org/{z}/{x}/{y}.png` with
  attribution per [OSM tile usage policy](https://operations.osmfoundation.org/policies/tiles/).
- **Polyline**: single SVG line, weight 5, color `#0066cc` (Google Maps-ish),
  rendered from the GeoJSON LineString returned by OSRM.
- **No markers**: explicitly omit `L.marker()` calls. No popup, no tooltip.
- **Auto-fit**: `map.fitBounds(polyline.getBounds(), { padding: [20, 20] })`
  on each generation.
- **Info overlay** (corner): "Generated 142 km · target 140 km · +1.4% · 3 stops".

## OSRM call

For chosen sequence `[0, D1, D2, ..., 0]` (with coords looked up from
[villages.json](./villages.json)), construct:

```
https://router.project-osrm.org/route/v1/driving/
  {lon0,lat0;lon1,lat1;...;lon0,lat0}
  ?geometries=geojson&overview=full&steps=false
```

Response → `routes[0].geometry` (GeoJSON LineString) → Leaflet polyline.

If the OSRM call fails (rate-limit, network), surface the error in the UI with
a "Retry" button. The chosen sequence is preserved so retry doesn't re-run the
algorithm.

See [router.project-osrm.org](https://router.project-osrm.org/) for the public
demo's [API docs](https://project-osrm.org/docs/v5.5.1/api/).

## Error handling

| Condition                             | Behavior                                  |
|---------------------------------------|-------------------------------------------|
| `targetKm < 5`                        | Refuse: "Too short for a meaningful loop" |
| `targetKm` < 2 × dist(home, nearest)  | Render direct out-and-back to nearest, warn that target was unreachable |
| Algorithm can't reach tolerance       | Render best attempt, show actual error %  |
| OSRM rate-limited (HTTP 429)          | Show error + Retry button, sequence kept  |
| OSRM network error                    | Same as above                             |
| Multi-session per-session is too small| Reduce session count, recompute split     |

## Testing strategy (POC scope)

Per project [ADR-008](../../DECISIONS.md), business logic belongs in Rust — but
this POC explicitly lives outside the app. Testing is informal:

- **Manual eyeball**: generate a dozen routes at varying targets (50, 100, 150,
  200, 300 km), screenshot each, judge whether they "look like" plausible drives.
- **Tolerance check**: log actual-vs-target km in the info overlay; verify ≥80%
  of attempts land within ±5% target across 10+ runs.
- **Algorithm comparison**: same target, both algorithms — does GA produce
  meaningfully different routes? Is one consistently better at hitting target?

If the POC graduates to in-app feature, all of this gets re-implemented in Rust
under the project's TDD discipline. POC validation results inform that scope.

## File layout

```
_tasks/61-route-map-poc/
├── 01-task.md               (committed — see ./01-task.md)
├── 02-design.md             (this file)
├── villages.json            (committed — 67-node dataset)
├── matrix.json              (committed — 67×67 driving distance matrix)
├── _fetch-matrix.ps1        (committed — regeneration script)
├── 03-plan.md               (created later by writing-plans skill)
└── poc.html                 (implementation — created during plan execution)
```

Direct links:
[01-task.md](./01-task.md) ·
[villages.json](./villages.json) ·
[matrix.json](./matrix.json) ·
[_fetch-matrix.ps1](./_fetch-matrix.ps1)

## Open questions deferred to plan / implementation phase

- Exact GA hyperparameters (population size, mutation rate) — pick defaults
  now, tune empirically once we can render results.
- UI affordance for the algorithm toggle — radio buttons, dropdown, or just two
  side-by-side "Generate (heuristic)" / "Generate (GA)" buttons.
- Whether to surface the chosen sequence textually (e.g., "Home → Levoča →
  Krompachy → Home") in the info overlay or hide it for cleaner screenshots.
