use super::*;

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
