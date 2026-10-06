//! Fuelio route files -> [`Drive`]s.
//!
//! `route-<id>.data` is a zip with one CSV, no header, one GPS fix per row:
//! `epoch ms (UTC), lat, lon, metres from the previous fix, speed m/s,
//! altitude m, accuracy m`. The columns were checked on real files: the
//! metres column equals the haversine distance, and the speed column agrees
//! with distance / time in m/s.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};
use serde::Serialize;

use super::geo::haversine_m;

/// 100 km/h in m/s: the speed above which time counts as "fast".
const FAST_SPEED_MS: f64 = 100.0 / 3.6;
/// The longest time between two fixes that counts toward the fast minutes.
/// A longer gap is a hole in the recording, not driving.
pub const MAX_FIX_GAP_S: f64 = 60.0;
/// The minimum distance between two kept points of a [`Drive::track`].
const TRACK_STEP_M: f64 = 100.0;

/// One GPS fix of a Fuelio CSV.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub ts_ms: i64,
    pub lat: f64,
    pub lon: f64,
    /// Metres from the previous fix.
    pub seg_m: f64,
    pub speed_ms: f64,
}

/// One recorded Fuelio drive.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// The epoch-ms part of the file name, `route-<id>.data`.
    pub id: String,
    /// Local time (Europe/Bratislava), like `Trip::start_datetime`.
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub km: f64,
    /// Minutes above 100 km/h.
    pub fast_minutes: f64,
    pub max_kmh: f64,
    pub start_point: (f64, f64),
    pub end_point: (f64, f64),
    /// The fixes thinned to one point per [`TRACK_STEP_M`], ends kept.
    #[serde(skip)]
    pub track: Vec<(f64, f64)>,
}

/// Parse the CSV. Lines that do not have 5 numeric leading fields are skipped.
pub fn parse_csv(text: &str) -> Vec<Fix> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.split(',').map(str::trim);
            let ts_ms = f.next()?.parse().ok()?;
            let lat = f.next()?.parse().ok()?;
            let lon = f.next()?.parse().ok()?;
            let seg_m = f.next()?.parse().ok()?;
            let speed_ms = f.next()?.parse().ok()?;
            Some(Fix { ts_ms, lat, lon, seg_m, speed_ms })
        })
        .collect()
}

/// The last Sunday of `month` in `year`.
fn last_sunday(year: i32, month: u32) -> NaiveDate {
    let mut d = NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1);
    while d.weekday() != Weekday::Sun {
        d -= Duration::days(1);
    }
    d
}

/// UTC epoch ms -> local time in Europe/Bratislava, whole seconds.
///
/// The EU rule: summer time (UTC+2) from 01:00 UTC on the last Sunday of
/// March to 01:00 UTC on the last Sunday of October, otherwise UTC+1. Coded
/// here so the result does not depend on the container's TZ.
pub fn utc_ms_to_local(ms: i64) -> NaiveDateTime {
    let utc = DateTime::from_timestamp(ms.div_euclid(1000), 0)
        .unwrap_or_default()
        .naive_utc();
    let year = utc.year();
    let one_am = |d: NaiveDate| d.and_hms_opt(1, 0, 0).unwrap();
    let summer = utc >= one_am(last_sunday(year, 3)) && utc < one_am(last_sunday(year, 10));
    utc + Duration::hours(if summer { 2 } else { 1 })
}

/// Build a [`Drive`] from its fixes. `None` with fewer than two fixes.
pub fn drive_from_fixes(id: &str, fixes: &[Fix]) -> Option<Drive> {
    let (first, last) = (fixes.first()?, fixes.last()?);
    if fixes.len() < 2 {
        return None;
    }
    let km = fixes.iter().map(|f| f.seg_m).sum::<f64>() / 1000.0;
    let fast_s: f64 = fixes
        .windows(2)
        .filter(|w| w[1].speed_ms > FAST_SPEED_MS)
        .map(|w| ((w[1].ts_ms - w[0].ts_ms) as f64 / 1000.0).clamp(0.0, MAX_FIX_GAP_S))
        .sum();
    let max_kmh = fixes.iter().map(|f| f.speed_ms).fold(0.0, f64::max) * 3.6;

    let mut track = vec![(first.lat, first.lon)];
    for f in &fixes[1..fixes.len() - 1] {
        if haversine_m(*track.last().unwrap(), (f.lat, f.lon)) >= TRACK_STEP_M {
            track.push((f.lat, f.lon));
        }
    }
    track.push((last.lat, last.lon));

    Some(Drive {
        id: id.to_string(),
        start: utc_ms_to_local(first.ts_ms),
        end: utc_ms_to_local(last.ts_ms),
        km,
        fast_minutes: fast_s / 60.0,
        max_kmh,
        start_point: (first.lat, first.lon),
        end_point: (last.lat, last.lon),
        track,
    })
}

/// The drive ID of a file name: `route-1790562640107.data` and the copies
/// `route-1790562640107(1).data` / `route-1790562640107 (1).data` give
/// `1790562640107`.
pub(crate) fn drive_id(file_name: &str) -> Option<String> {
    let stem = file_name.strip_prefix("route-")?.strip_suffix(".data")?;
    let id: String = stem.chars().take_while(char::is_ascii_digit).collect();
    (!id.is_empty()).then_some(id)
}

/// The CSV text inside a `.data` zip (its first file).
pub fn read_data_file(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut entry = zip.by_index(0).map_err(|e| e.to_string())?;
    let mut text = String::new();
    entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
    Ok(text)
}

/// The path of one drive's `.data` file in `dir`.
pub fn data_file_path(dir: &Path, id: &str) -> std::path::PathBuf {
    dir.join(format!("route-{id}.data"))
}

/// Every drive in `dir`, one per ID, oldest first. A file that does not
/// parse is skipped with a log line; a missing folder gives no drives.
pub fn scan_dir(dir: &Path) -> Vec<Drive> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut by_id: HashMap<String, Drive> = HashMap::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = drive_id(&name) else { continue };
        if by_id.contains_key(&id) {
            continue;
        }
        match read_data_file(&entry.path()) {
            Ok(text) => {
                if let Some(d) = drive_from_fixes(&id, &parse_csv(&text)) {
                    by_id.insert(id, d);
                }
            }
            Err(e) => log::warn!("fuelio: skip {name}: {e}"),
        }
    }
    let mut drives: Vec<Drive> = by_id.into_values().collect();
    drives.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.id.cmp(&b.id)));
    drives
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
