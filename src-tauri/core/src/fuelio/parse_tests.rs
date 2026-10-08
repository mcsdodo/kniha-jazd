use super::*;
use chrono::NaiveDate;
use std::io::Write;

fn local(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(h, min, s)
        .unwrap()
}

// Three real rows of route-1790866465210.csv.
const CSV: &str = "\
1790866472453,48.9156482,20.5805701,0.0,0.13166487,495.6000061035156,10.004
1790866479518,48.9155498,20.5806715,13.228128,0.44082722,497.25965837620805,5.363
1790866497874,48.9161755,20.5818588,113.97531,15.93,487.7199573212328,3.79
";

#[test]
fn parse_csv_reads_time_position_segment_and_speed() {
    let fixes = parse_csv(CSV);
    assert_eq!(fixes.len(), 3);
    assert_eq!(fixes[1].ts_ms, 1_790_866_479_518);
    assert!((fixes[1].lat - 48.915_549_8).abs() < 1e-9);
    assert!((fixes[1].lon - 20.580_671_5).abs() < 1e-9);
    assert!((fixes[1].seg_m - 13.228_128).abs() < 1e-6);
    assert!((fixes[2].speed_ms - 15.93).abs() < 1e-9);
}

#[test]
fn parse_csv_skips_blank_and_broken_lines() {
    let text = "\nnot,a,row\n1790866472453,48.9,20.5,0,0,0,0\n1790866472453,48.9\n";
    assert_eq!(parse_csv(text).len(), 1);
}

#[test]
fn local_time_uses_summer_time_in_september() {
    // 2026-09-28 02:30:40 UTC = 04:30:40 CEST.
    assert_eq!(utc_ms_to_local(1_790_562_640_107), local(2026, 9, 28, 4, 30, 40));
}

#[test]
fn local_time_uses_winter_time_in_january() {
    // 2026-01-15 12:00:00 UTC = 13:00 CET.
    let ms = NaiveDate::from_ymd_opt(2026, 1, 15)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    assert_eq!(utc_ms_to_local(ms), local(2026, 1, 15, 13, 0, 0));
}

#[test]
fn local_time_switches_at_one_utc_on_the_last_sunday() {
    // 2026: summer time starts 29 March 01:00 UTC, ends 25 October 01:00 UTC.
    let at = |m, d, h, min| {
        NaiveDate::from_ymd_opt(2026, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis()
    };
    assert_eq!(utc_ms_to_local(at(3, 29, 0, 59)), local(2026, 3, 29, 1, 59, 0));
    assert_eq!(utc_ms_to_local(at(3, 29, 1, 0)), local(2026, 3, 29, 3, 0, 0));
    assert_eq!(utc_ms_to_local(at(10, 25, 0, 59)), local(2026, 10, 25, 2, 59, 0));
    assert_eq!(utc_ms_to_local(at(10, 25, 1, 0)), local(2026, 10, 25, 2, 0, 0));
}

fn fix(ts_ms: i64, lat: f64, lon: f64, seg_m: f64, speed_ms: f64) -> Fix {
    Fix { ts_ms, lat, lon, seg_m, speed_ms }
}

#[test]
fn drive_sums_km_and_minutes_above_100_kmh() {
    let t0 = 1_790_562_640_107;
    let fixes = vec![
        fix(t0, 48.0, 20.0, 0.0, 0.0),
        // 60 s at 30 m/s (108 km/h) -> 1 fast minute.
        fix(t0 + 60_000, 48.0, 20.02, 1800.0, 30.0),
        // 60 s at 20 m/s (72 km/h) -> not fast.
        fix(t0 + 120_000, 48.0, 20.04, 1200.0, 20.0),
    ];
    let d = drive_from_fixes("1790562640107", &fixes).unwrap();
    assert_eq!(d.id, "1790562640107");
    assert!((d.km - 3.0).abs() < 1e-9);
    assert!((d.fast_minutes - 1.0).abs() < 1e-9);
    assert!((d.max_kmh - 108.0).abs() < 1e-9);
    assert_eq!(d.start, local(2026, 9, 28, 4, 30, 40));
    assert_eq!(d.end, local(2026, 9, 28, 4, 32, 40));
    assert_eq!(d.start_point, (48.0, 20.0));
    assert_eq!(d.end_point, (48.0, 20.04));
}

#[test]
fn a_gap_in_the_recording_does_not_count_as_fast_time() {
    let t0 = 1_790_562_640_107;
    let fixes = vec![
        fix(t0, 48.0, 20.0, 0.0, 30.0),
        // 10 minutes without a fix: only MAX_FIX_GAP_S of it counts.
        fix(t0 + 600_000, 48.0, 20.1, 9000.0, 30.0),
    ];
    let d = drive_from_fixes("x", &fixes).unwrap();
    assert!((d.fast_minutes - MAX_FIX_GAP_S / 60.0).abs() < 1e-9);
}

#[test]
fn drive_needs_two_fixes() {
    assert!(drive_from_fixes("x", &[]).is_none());
    assert!(drive_from_fixes("x", &[fix(1, 48.0, 20.0, 0.0, 0.0)]).is_none());
}

#[test]
fn track_keeps_the_ends_and_drops_points_closer_than_the_step() {
    let t0 = 1_790_562_640_107;
    // 0.0001 deg lat is about 11 m: the middle points are dropped.
    let fixes: Vec<Fix> = (0..20)
        .map(|i| fix(t0 + i * 1000, 48.0 + f64::from(i as i32) * 0.0001, 20.0, 11.0, 10.0))
        .collect();
    let d = drive_from_fixes("x", &fixes).unwrap();
    assert_eq!(d.track.first(), Some(&(48.0, 20.0)));
    assert_eq!(d.track.last(), Some(&d.end_point));
    assert!(d.track.len() < 6, "track has {} points", d.track.len());
}

fn write_data_file(dir: &std::path::Path, id: &str, csv: &str) {
    let file = std::fs::File::create(dir.join(format!("route-{id}.data"))).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(
        format!("route-{id}.csv"),
        zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated),
    )
    .unwrap();
    zip.write_all(csv.as_bytes()).unwrap();
    zip.finish().unwrap();
}

#[test]
fn scan_dir_reads_data_files_sorted_by_start() {
    let dir = tempfile::tempdir().unwrap();
    write_data_file(
        dir.path(),
        "1790866497874",
        "1790866497874,48.9,20.5,0,10,0,0\n1790866507874,48.91,20.5,1100,10,0,0\n",
    );
    write_data_file(dir.path(), "1790866465210", CSV);
    std::fs::write(dir.path().join("route-1790866465210.route"), "aaa").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "x").unwrap();

    let drives = scan_year(dir.path(), 2026);
    let ids: Vec<&str> = drives.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, vec!["1790866465210", "1790866497874"]);
}

#[test]
fn scan_dir_keeps_one_drive_per_id() {
    // rclone and Drive keep copies such as "route-1(1).data".
    let dir = tempfile::tempdir().unwrap();
    write_data_file(dir.path(), "1790866465210", CSV);
    let copy = dir.path().join("route-1790866465210(1).data");
    std::fs::copy(dir.path().join("route-1790866465210.data"), copy).unwrap();
    assert_eq!(scan_year(dir.path(), 2026).len(), 1);
}

#[test]
fn scan_dir_skips_a_broken_file_and_a_missing_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("route-1790866465210.data"), "not a zip").unwrap();
    assert!(scan_year(dir.path(), 2026).is_empty());
    assert!(scan_year(&dir.path().join("nope"), 2026).is_empty());
}

// The year comes from the file name, before the file is read: a page load
// does not unzip the drives of the other years. The sync uses the same rule.
#[test]
fn scan_year_takes_the_year_from_the_file_name() {
    let dir = tempfile::tempdir().unwrap();
    write_data_file(dir.path(), "1790866465210", CSV); // 2026-10-01
    // Named 2025-12-31 23:59:58 local, first fix 2026-01-01 00:00:05 local
    write_data_file(
        dir.path(),
        "1767221998000",
        "1767222005000,48.9,20.5,0,10,0,0\n1767222015000,48.91,20.5,1100,10,0,0\n",
    );

    let ids = |year| -> Vec<String> {
        scan_year(dir.path(), year).into_iter().map(|d| d.id).collect()
    };
    assert_eq!(ids(2026), vec!["1790866465210"]);
    assert_eq!(ids(2025), vec!["1767221998000"]);
}

#[test]
fn file_year_is_the_local_year_of_the_name() {
    assert_eq!(file_year("route-1767221998000.data"), Some(2025));
    assert_eq!(file_year("route-1767222005000.data"), Some(2026));
    assert_eq!(file_year("route-1767222005000(1).data"), Some(2026));
    assert_eq!(file_year("notes.txt"), None);
}
