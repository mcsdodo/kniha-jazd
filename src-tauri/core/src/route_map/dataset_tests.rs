use super::*;
use crate::route_map::dataset::Node;

#[test]
fn bundled_dataset_loads_67_nodes() {
    let ds = Dataset::bundled();
    assert_eq!(ds.nodes.len(), 67);
    assert_eq!(ds.nodes[0].kind, "home");
    assert_eq!(ds.matrix.len(), 67);
    assert!(
        ds.matrix.iter().all(|row| row.len() == 67),
        "matrix must be square"
    );
}

#[test]
fn dataset_distance_is_asymmetric_and_positive() {
    let ds = Dataset::bundled();
    assert!(ds.distance(0, 1) > 0.0);
    assert_eq!(ds.distance(5, 5), 0.0);
    // The matrix is directional (one-way streets, different routing each way),
    // and the GA relies on that — sequence order has to be significant.
    assert_ne!(
        ds.distance(0, 1),
        ds.distance(1, 0),
        "driving distances must stay asymmetric"
    );
}

#[test]
fn dataset_version_is_the_generation_date() {
    assert_eq!(Dataset::bundled().version, "2026-05-03");
}

#[test]
fn bratislava_file_holds_the_17_districts() {
    let (nodes, version) = Dataset::bratislava_districts();
    assert_eq!(nodes.len(), 17);
    assert_eq!(version, "2026-10-08");
    assert!(nodes.iter().all(|n| n.kind == "district"));
    let idx: Vec<usize> = nodes.iter().map(|n| n.idx).collect();
    assert_eq!(idx, (1..=17).collect::<Vec<_>>());
}

#[test]
fn an_anchored_dataset_puts_the_anchor_at_index_0() {
    let (nodes, version) = Dataset::bratislava_districts();
    let anchor = Node {
        idx: 99,
        name: "Kancelária".into(),
        lat: 48.15,
        lon: 17.11,
        kind: "home".into(),
    };
    let n = nodes.len() + 1;
    let ds = Dataset::anchored(anchor, nodes, vec![vec![1.0; n]; n], version);
    assert_eq!(ds.len(), 18);
    assert_eq!(ds.nodes[0].name, "Kancelária");
    assert_eq!(ds.nodes[0].idx, 0, "the anchor is always index 0");
    assert_eq!(ds.nodes[17].idx, 17);
}
