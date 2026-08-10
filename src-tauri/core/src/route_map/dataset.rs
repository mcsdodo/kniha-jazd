//! Bundled candidate-node dataset for generated route maps.
//!
//! 67 nodes (1 home base + 22 towns within 50 km + 44 villages within 20 km)
//! and a 67x67 asymmetric driving-distance matrix in km, generated from
//! OpenStreetMap Overpass + OSRM. Both files are compiled into the binary,
//! so loading cannot fail at runtime for a well-formed build.

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
