//! Bundled candidate-node datasets for generated route maps.
//!
//! Home set: 67 nodes (1 home base + 22 towns within 50 km + 44 villages
//! within 20 km) and a 67x67 asymmetric driving-distance matrix in km,
//! generated from OpenStreetMap Overpass + OSRM.
//!
//! Bratislava set (task 91): the 17 city districts, with no matrix. The loop
//! starts at the trip's own place, so the matrix depends on that place and is
//! fetched at generation time (see [`Dataset::anchored`]).
//!
//! All files are compiled into the binary, so loading cannot fail at runtime
//! for a well-formed build.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    pub idx: usize,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub kind: String,
}

#[derive(Deserialize)]
struct VillagesFile {
    #[serde(rename = "generatedAt")]
    generated_at: String,
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct AreaFile {
    #[serde(rename = "generatedAt")]
    generated_at: String,
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct MatrixFile {
    distances: Vec<Vec<f64>>,
}

pub struct Dataset {
    pub nodes: Vec<Node>,
    pub matrix: Vec<Vec<f64>>,
    pub version: String,
}

const VILLAGES_JSON: &str = include_str!("../../assets/villages.json");
const MATRIX_JSON: &str = include_str!("../../assets/matrix.json");
const BRATISLAVA_JSON: &str = include_str!("../../assets/bratislava.json");

impl Dataset {
    pub fn bundled() -> Self {
        let v: VillagesFile =
            serde_json::from_str(VILLAGES_JSON).expect("bundled villages.json must parse");
        let m: MatrixFile =
            serde_json::from_str(MATRIX_JSON).expect("bundled matrix.json must parse");
        Self {
            nodes: v.nodes,
            matrix: m.distances,
            version: v.generated_at,
        }
    }

    /// The Bratislava city districts (task 91), indices 1..=17, and the file
    /// version. No matrix: it depends on the anchor, see [`Dataset::anchored`].
    pub fn bratislava_districts() -> (Vec<Node>, String) {
        let f: AreaFile =
            serde_json::from_str(BRATISLAVA_JSON).expect("bundled bratislava.json must parse");
        (f.nodes, f.generated_at)
    }

    /// The anchor at index 0, then `candidates` in file order. `matrix` must
    /// be square over exactly these nodes, in this order.
    pub fn anchored(
        anchor: Node,
        candidates: Vec<Node>,
        matrix: Vec<Vec<f64>>,
        version: String,
    ) -> Self {
        let mut nodes = Vec::with_capacity(candidates.len() + 1);
        nodes.push(Node { idx: 0, ..anchor });
        nodes.extend(candidates);
        Self {
            nodes,
            matrix,
            version,
        }
    }

    pub fn distance(&self, from: usize, to: usize) -> f64 {
        self.matrix[from][to]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
