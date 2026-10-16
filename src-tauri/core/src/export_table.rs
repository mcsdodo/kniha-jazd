//! The printed trip table as data: one column list and typed cells.
//!
//! The HTML printout and the xlsx file both render this model, so the two
//! exports cannot disagree on which columns exist, in which order the rows
//! come, or which value a cell holds.

use crate::constants::date_formats;
use crate::export::ExportData;
use crate::models::{MonthEndRow, Trip};
use chrono::NaiveDateTime;
use uuid::Uuid;

/// One cell. A `Number` keeps the exact backend value; `decimals` is how many
/// decimals the printout shows.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Empty,
    Text(String),
    Number { value: f64, decimals: usize },
    DateTime(NaiveDateTime),
}

impl CellValue {
    /// The text the printout shows for this cell.
    pub fn display(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::Text(text) => text.clone(),
            CellValue::Number { value, decimals } => format!("{:.*}", decimals, value),
            CellValue::DateTime(dt) => dt.format(date_formats::DISPLAY_DATE_TIME).to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Trip,
    /// The synthetic year-opening row (`Uuid::nil()`).
    FirstRecord,
    MonthEnd,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportColumn {
    pub label: String,
    /// Right-aligned number column in the printout.
    pub numeric: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportTableRow {
    pub kind: RowKind,
    pub cells: Vec<CellValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportTable {
    pub columns: Vec<ExportColumn>,
    pub rows: Vec<ExportTableRow>,
}

/// Every column the table can have, in print order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Col {
    TripNumber,
    StartDatetime,
    EndDatetime,
    Driver,
    Origin,
    Destination,
    Purpose,
    Km,
    OdoStart,
    Odo,
    FuelLiters,
    FuelCost,
    FuelConsumed,
    FuelRemaining,
    Consumption,
    EnergyKwh,
    EnergyCost,
    BatteryRemaining,
    EnergyRate,
    OtherCosts,
    Note,
}

impl Col {
    fn numeric(self) -> bool {
        !matches!(
            self,
            Col::StartDatetime
                | Col::EndDatetime
                | Col::Driver
                | Col::Origin
                | Col::Destination
                | Col::Purpose
                | Col::Note
        )
    }
}

/// The visible columns for this export: vehicle type gates first, then the
/// hidden-column keys ("time", "fuelConsumed", "fuelRemaining",
/// "otherCosts", "otherCostsNote").
fn visible_columns(data: &ExportData) -> Vec<Col> {
    let vehicle_type = data.vehicle.vehicle_type;
    let visible = |key: &str| !data.hidden_columns.iter().any(|h| h == key);

    let mut cols = vec![Col::TripNumber, Col::StartDatetime];
    if visible("time") {
        cols.push(Col::EndDatetime);
    }
    cols.extend([
        Col::Driver,
        Col::Origin,
        Col::Destination,
        Col::Purpose,
        Col::Km,
        Col::OdoStart,
        Col::Odo,
    ]);
    if vehicle_type.uses_fuel() {
        cols.extend([Col::FuelLiters, Col::FuelCost]);
        if visible("fuelConsumed") {
            cols.push(Col::FuelConsumed);
        }
        if visible("fuelRemaining") {
            cols.push(Col::FuelRemaining);
        }
        cols.push(Col::Consumption);
    }
    if vehicle_type.uses_electricity() {
        cols.extend([Col::EnergyKwh, Col::EnergyCost]);
        // Battery remaining shares the fuelRemaining visibility setting.
        if visible("fuelRemaining") {
            cols.push(Col::BatteryRemaining);
        }
        cols.push(Col::EnergyRate);
    }
    if visible("otherCosts") {
        cols.push(Col::OtherCosts);
    }
    if visible("otherCostsNote") {
        cols.push(Col::Note);
    }
    cols
}

fn label(data: &ExportData, col: Col) -> String {
    let l = &data.labels;
    match col {
        Col::TripNumber => &l.col_trip_number,
        Col::StartDatetime => &l.col_start_datetime,
        Col::EndDatetime => &l.col_end_datetime,
        Col::Driver => &l.col_driver,
        Col::Origin => &l.col_origin,
        Col::Destination => &l.col_destination,
        Col::Purpose => &l.col_purpose,
        Col::Km => &l.col_km,
        Col::OdoStart => &l.col_odo_start,
        Col::Odo => &l.col_odo,
        Col::FuelLiters => &l.col_fuel_liters,
        Col::FuelCost => &l.col_fuel_cost,
        Col::FuelConsumed => &l.col_fuel_consumed,
        Col::FuelRemaining => &l.col_remaining,
        Col::Consumption => &l.col_consumption,
        Col::EnergyKwh => &l.col_energy_kwh,
        Col::EnergyCost => &l.col_energy_cost,
        Col::BatteryRemaining => &l.col_battery_remaining,
        Col::EnergyRate => &l.col_energy_rate,
        Col::OtherCosts => &l.col_other_costs,
        Col::Note => &l.col_note,
    }
    .clone()
}

fn num(value: f64, decimals: usize) -> CellValue {
    CellValue::Number { value, decimals }
}

fn opt_num(value: Option<f64>, decimals: usize) -> CellValue {
    value.map_or(CellValue::Empty, |v| num(v, decimals))
}

/// An empty text is an empty cell, so the xlsx file has no "" strings.
fn text(s: &str) -> CellValue {
    if s.is_empty() {
        CellValue::Empty
    } else {
        CellValue::Text(s.to_string())
    }
}

fn trip_cell(data: &ExportData, trip: &Trip, col: Col) -> CellValue {
    let grid = &data.grid_data;
    let id = trip.id.to_string();
    let get = |map: &std::collections::HashMap<String, f64>| map.get(&id).copied().unwrap_or(0.0);
    match col {
        Col::TripNumber => {
            if trip.id == Uuid::nil() {
                CellValue::Empty
            } else {
                num(grid.trip_numbers.get(&id).copied().unwrap_or(0) as f64, 0)
            }
        }
        Col::StartDatetime => CellValue::DateTime(trip.start_datetime),
        Col::EndDatetime => match trip.end_datetime {
            Some(end) => CellValue::DateTime(end),
            None => CellValue::Text(format!("{} -", trip.start_datetime.format("%d.%m."))),
        },
        Col::Driver => text(data.vehicle.driver_name.as_deref().unwrap_or("")),
        Col::Origin => text(&trip.origin),
        Col::Destination => text(&trip.destination),
        Col::Purpose => text(&trip.purpose),
        Col::Km => num(trip.distance_km, 0),
        Col::OdoStart => num(
            grid.odometer_start
                .get(&id)
                .copied()
                .unwrap_or(trip.odometer - trip.distance_km),
            0,
        ),
        Col::Odo => num(trip.odometer, 0),
        Col::FuelLiters => opt_num(trip.fuel_liters, 2),
        Col::FuelCost => opt_num(trip.fuel_cost_eur, 2),
        Col::FuelConsumed => num(get(&grid.fuel_consumed), 2),
        Col::FuelRemaining => num(get(&grid.fuel_remaining), 1),
        Col::Consumption => num(get(&grid.rates), 2),
        Col::EnergyKwh => opt_num(trip.energy_kwh, 1),
        Col::EnergyCost => opt_num(trip.energy_cost_eur, 2),
        Col::BatteryRemaining => num(get(&grid.battery_remaining_kwh), 1),
        Col::EnergyRate => num(get(&grid.energy_rates), 2),
        Col::OtherCosts => opt_num(trip.other_costs_eur, 2),
        Col::Note => text(trip.other_costs_note.as_deref().unwrap_or("")),
    }
}

/// A month-end row fills only what it can answer for: the date, the
/// carried-forward odometer and the fuel state. Battery state is not tracked
/// in month-end rows yet.
fn month_end_cell(month_end: &MonthEndRow, col: Col) -> CellValue {
    match col {
        Col::StartDatetime => CellValue::Text(format!("{} -", month_end.date.format("%d.%m."))),
        Col::OdoStart | Col::Odo => num(month_end.odometer, 0),
        Col::FuelRemaining => num(month_end.fuel_remaining, 1),
        _ => CellValue::Empty,
    }
}

/// Build the printed table: trips and month-end rows interleaved by sort key
/// (the same order as the grid), in the export's sort direction.
pub fn build_export_table(data: &ExportData) -> ExportTable {
    let cols = visible_columns(data);

    enum Source<'a> {
        Trip(&'a Trip),
        MonthEnd(&'a MonthEndRow),
    }
    let grid = &data.grid_data;
    let mut sources: Vec<(f64, Source)> = grid
        .trips
        .iter()
        .map(|t| {
            let key = grid.trip_numbers.get(&t.id.to_string()).copied().unwrap_or(0) as f64;
            (key, Source::Trip(t))
        })
        .chain(grid.month_end_rows.iter().map(|m| (m.sort_key, Source::MonthEnd(m))))
        .collect();

    // "desc" = newest first (highest trip # on top), anything else = asc.
    let descending = data.sort_direction.eq_ignore_ascii_case("desc");
    sources.sort_by(|a, b| {
        let ordering = a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal);
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });

    let rows = sources
        .iter()
        .map(|(_, source)| match source {
            Source::Trip(trip) => ExportTableRow {
                kind: if trip.id == Uuid::nil() {
                    RowKind::FirstRecord
                } else {
                    RowKind::Trip
                },
                cells: cols.iter().map(|&c| trip_cell(data, trip, c)).collect(),
            },
            Source::MonthEnd(m) => ExportTableRow {
                kind: RowKind::MonthEnd,
                cells: cols.iter().map(|&c| month_end_cell(m, c)).collect(),
            },
        })
        .collect();

    ExportTable {
        columns: cols
            .iter()
            .map(|&c| ExportColumn {
                label: label(data, c),
                numeric: c.numeric(),
            })
            .collect(),
        rows,
    }
}

#[cfg(test)]
#[path = "export_table_tests.rs"]
mod tests;
