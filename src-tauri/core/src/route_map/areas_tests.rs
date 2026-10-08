use super::areas::*;
use crate::route_map::Dataset;

fn home() -> (f64, f64) {
    let n = &Dataset::bundled().nodes[0];
    (n.lat, n.lon)
}

/// A point `km` north of `p` (1 degree of latitude is 111.195 km).
fn north(p: (f64, f64), km: f64) -> (f64, f64) {
    (p.0 + km / 111.195, p.1)
}

#[test]
fn selects_home_at_home() {
    let h = home();
    assert_eq!(loop_area(h.0, h.1), Some(LoopArea::Home));
}

#[test]
fn selects_home_just_inside_the_radius() {
    let p = north(home(), HOME_RADIUS_KM - 0.1);
    assert_eq!(loop_area(p.0, p.1), Some(LoopArea::Home));
}

#[test]
fn selects_nothing_just_outside_the_home_radius() {
    let p = north(home(), HOME_RADIUS_KM + 0.1);
    assert_eq!(loop_area(p.0, p.1), None);
}

#[test]
fn selects_bratislava_in_the_centre() {
    assert_eq!(loop_area(48.1486, 17.1077), Some(LoopArea::Bratislava));
}

#[test]
fn selects_bratislava_just_inside_the_radius() {
    let p = north(BRATISLAVA_CENTRE, BRATISLAVA_RADIUS_KM - 0.1);
    assert_eq!(loop_area(p.0, p.1), Some(LoopArea::Bratislava));
}

#[test]
fn selects_nothing_just_outside_the_bratislava_radius() {
    let p = north(BRATISLAVA_CENTRE, BRATISLAVA_RADIUS_KM + 0.1);
    assert_eq!(loop_area(p.0, p.1), None);
}

#[test]
fn selects_no_area_far_away() {
    // Žilina
    assert_eq!(loop_area(49.2231, 18.7394), None);
}

#[test]
fn every_district_is_inside_the_bratislava_area() {
    for n in Dataset::bratislava_districts().0 {
        assert_eq!(loop_area(n.lat, n.lon), Some(LoopArea::Bratislava), "{}", n.name);
    }
}

#[test]
fn haversine_is_zero_on_one_point_and_about_44_km_ba_to_trnava() {
    assert_eq!(haversine_km((48.0, 17.0), (48.0, 17.0)), 0.0);
    let d = haversine_km((48.1486, 17.1077), (48.3774, 17.5872));
    assert!((d - 43.7).abs() < 0.5, "got {d}");
}
