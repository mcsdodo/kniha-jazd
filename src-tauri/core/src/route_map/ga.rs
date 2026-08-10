//! Genetic route selection.
//!
//! Picks a loop of settlements from the bundled [`Dataset`] that starts and
//! ends at the home base (node 0) and whose total driving distance lands as
//! close as possible to a target. The distance matrix is asymmetric, so the
//! order of the intermediate stops is significant.
//!
//! Randomness is business logic (ADR-008): per-run variety is the point, so
//! two maps generated for similar distances look visibly different. The
//! [`RouteRng`] trait mirrors the `Jitter` split in
//! `calculations::time_inference` — production uses [`ThreadRouteRng`], tests
//! use [`SeededRouteRng`] for reproducible routes.

use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::route_map::Dataset;

/// How far a generated route may fall from its target distance before it is
/// worth telling the user about, as a fraction.
///
/// The single home for this number. It is applied to the *road* distance the
/// finished route actually covers, which is what the user compares against the
/// trip's recorded kilometres — not to the matrix distance the algorithm
/// optimises internally. Keeping one constant stops the display and the tests
/// drifting onto two different measurements of "close enough".
pub const TOLERANCE: f64 = 0.05;

/// Home base node index; every route starts and ends here.
const HOME: usize = 0;
/// Chromosomes per generation.
const POP: usize = 50;
/// Number of generations evolved before the fittest route is returned.
const GENS: usize = 100;
/// Probability that a child is mutated.
const MUT: f64 = 0.25;
/// Fittest chromosomes carried into the next generation unchanged.
const ELITE: usize = 2;
/// Population members sampled per tournament selection.
const TOUR: usize = 3;
/// Maximum intermediate stops between the two home visits.
const MAX_STOPS: usize = 5;

/// Source of randomness for route generation. Production uses
/// [`ThreadRouteRng`]; tests use [`SeededRouteRng`] to get reproducible runs.
pub trait RouteRng {
    /// Uniform in `[0, n)`. Callers must pass `n > 0`.
    fn below(&mut self, n: usize) -> usize;
    /// Uniform in `[0.0, 1.0)`.
    fn unit(&mut self) -> f64;
}

/// Production [`RouteRng`] backed by `rand::thread_rng`.
pub struct ThreadRouteRng;

impl RouteRng for ThreadRouteRng {
    fn below(&mut self, n: usize) -> usize {
        rand::thread_rng().gen_range(0..n)
    }
    fn unit(&mut self) -> f64 {
        rand::thread_rng().gen_range(0.0..1.0)
    }
}

/// Deterministic [`RouteRng`] for tests and reproducible runs.
pub struct SeededRouteRng(StdRng);

impl SeededRouteRng {
    pub fn new(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }
}

impl RouteRng for SeededRouteRng {
    fn below(&mut self, n: usize) -> usize {
        self.0.gen_range(0..n)
    }
    fn unit(&mut self) -> f64 {
        self.0.gen_range(0.0..1.0)
    }
}

/// A generated round trip.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// Node indices, first and last are 0 (home).
    pub sequence: Vec<usize>,
    /// Matrix distance of the sequence, in km.
    pub total_km: f64,
}

/// Intermediate stops of a route: unique node indices in `1..ds.len()`,
/// between 1 and [`MAX_STOPS`] of them. The two home visits are implicit.
type Stops = Vec<usize>;

/// Driving distance of the loop `HOME -> stops -> HOME`.
fn total_km(stops: &[usize], ds: &Dataset) -> f64 {
    let mut total = 0.0;
    let mut prev = HOME;
    for &stop in stops {
        total += ds.distance(prev, stop);
        prev = stop;
    }
    total + ds.distance(prev, HOME)
}

/// Closer to the target is fitter; peaks at 1.0 for an exact hit.
fn fitness(stops: &[usize], ds: &Dataset, target_km: f64) -> f64 {
    1.0 / (1.0 + (total_km(stops, ds) - target_km).abs())
}

/// A random node other than home.
fn random_stop(ds: &Dataset, rng: &mut dyn RouteRng) -> usize {
    1 + rng.below(ds.len() - 1)
}

/// A random chromosome: 1..=[`MAX_STOPS`] distinct stops.
fn random_stops(ds: &Dataset, rng: &mut dyn RouteRng) -> Stops {
    let k = 1 + rng.below(MAX_STOPS);
    let mut stops = Stops::with_capacity(k);
    while stops.len() < k {
        let stop = random_stop(ds, rng);
        if !stops.contains(&stop) {
            stops.push(stop);
        }
    }
    stops
}

/// Tournament selection: sample [`TOUR`] members, return the fittest.
fn tournament<'a>(pop: &'a [Stops], fits: &[f64], rng: &mut dyn RouteRng) -> &'a Stops {
    let mut best = 0;
    let mut best_fitness = f64::NEG_INFINITY;
    for _ in 0..TOUR {
        let idx = rng.below(pop.len());
        if fits[idx] > best_fitness {
            best_fitness = fits[idx];
            best = idx;
        }
    }
    &pop[best]
}

/// Order crossover: a random contiguous slice of `a`, then the entries of `b`
/// not already present, capped at [`MAX_STOPS`].
///
/// The empty-parent guards are defensive: every chromosome carries at least one
/// stop, so in practice neither branch is taken.
fn crossover(a: &[usize], b: &[usize], rng: &mut dyn RouteRng) -> Stops {
    if a.is_empty() {
        return b.to_vec();
    }
    if b.is_empty() {
        return a.to_vec();
    }
    let start = rng.below(a.len());
    let end = start + rng.below(a.len() - start);
    let mut child: Stops = a[start..=end].to_vec();
    for &stop in b {
        if child.len() < MAX_STOPS && !child.contains(&stop) {
            child.push(stop);
        }
    }
    child
}

/// With probability [`MUT`], insert, remove or swap a stop.
fn mutate(mut stops: Stops, ds: &Dataset, rng: &mut dyn RouteRng) -> Stops {
    if rng.unit() > MUT {
        return stops;
    }
    let op = rng.unit();
    if op < 0.34 && stops.len() < MAX_STOPS {
        let stop = loop {
            let candidate = random_stop(ds, rng);
            if !stops.contains(&candidate) {
                break candidate;
            }
        };
        let at = rng.below(stops.len() + 1);
        stops.insert(at, stop);
    } else if op < 0.67 && stops.len() > 1 {
        let at = rng.below(stops.len());
        stops.remove(at);
    } else if stops.len() >= 2 {
        let i = rng.below(stops.len());
        let j = rng.below(stops.len());
        stops.swap(i, j);
    }
    stops
}

/// Index of the fittest chromosome; ties go to the earliest.
fn fittest(fits: &[f64]) -> usize {
    let mut best = 0;
    for (i, f) in fits.iter().enumerate() {
        if *f > fits[best] {
            best = i;
        }
    }
    best
}

/// Evolve a round trip whose length is as close to `target_km` as the dataset
/// allows. The returned sequence starts and ends at home and visits each
/// intermediate node at most once.
///
/// `ds` must hold at least two nodes (home plus one candidate stop), which the
/// bundled dataset always does.
pub fn generate_route(target_km: f64, ds: &Dataset, rng: &mut impl RouteRng) -> RouteResult {
    let rng: &mut dyn RouteRng = rng;

    let mut pop: Vec<Stops> = (0..POP).map(|_| random_stops(ds, rng)).collect();
    let mut fits: Vec<f64> = pop.iter().map(|c| fitness(c, ds, target_km)).collect();

    for _ in 0..GENS {
        let mut ranked: Vec<usize> = (0..pop.len()).collect();
        ranked.sort_by(|&a, &b| fits[b].total_cmp(&fits[a]));

        let mut next: Vec<Stops> = ranked[..ELITE].iter().map(|&i| pop[i].clone()).collect();
        while next.len() < POP {
            let a = tournament(&pop, &fits, rng);
            let b = tournament(&pop, &fits, rng);
            let child = crossover(a, b, rng);
            next.push(mutate(child, ds, rng));
        }

        pop = next;
        fits = pop.iter().map(|c| fitness(c, ds, target_km)).collect();
    }

    let best = &pop[fittest(&fits)];
    let mut sequence = Vec::with_capacity(best.len() + 2);
    sequence.push(HOME);
    sequence.extend_from_slice(best);
    sequence.push(HOME);

    RouteResult {
        total_km: total_km(best, ds),
        sequence,
    }
}

/// [`generate_route`] with production randomness.
pub fn generate_route_random(target_km: f64, ds: &Dataset) -> RouteResult {
    generate_route(target_km, ds, &mut ThreadRouteRng)
}
