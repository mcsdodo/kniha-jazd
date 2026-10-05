//! Read-only logbook queries for the MCP endpoint (task 89).
//!
//! `LogbookReader` is the only thing the `mcp` module holds. It keeps its
//! `Database` private and exposes three reads, so the MCP module has no path
//! to a write. The source guard in `mcp/tests.rs` checks this file too.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::commands_internal::get_vehicles_internal;
use crate::db::Database;
use crate::journeys::{group_journeys, overlaps, Journey, Leg};
use crate::models::{Trip, Vehicle};

pub const HOME_NOT_SET: &str =
    "Home place is not set. Mark a place as home on the Miesta page.";

/// `Invalid`: the caller can fix it (bad range, unknown vehicle, no home).
/// `Internal`: a database error.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadError {
    Invalid(String),
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct JourneyList {
    pub home_place: String,
    pub journeys: Vec<Journey>,
}

#[derive(Clone)]
pub struct LogbookReader {
    db: Arc<Database>,
}

fn internal(e: impl std::fmt::Display) -> ReadError {
    ReadError::Internal(e.to_string())
}

impl LogbookReader {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn vehicles(&self) -> Result<Vec<Vehicle>, ReadError> {
        get_vehicles_internal(&self.db).map_err(ReadError::Internal)
    }

    /// The vehicles a query covers: the one asked for, or ALL vehicles.
    /// `is_active` marks only the vehicle selected in the UI, so it is not a filter.
    fn vehicles_for(&self, vehicle_id: Option<&str>) -> Result<Vec<Vehicle>, ReadError> {
        match vehicle_id {
            Some(id) => self
                .db
                .get_vehicle(id)
                .map_err(internal)?
                .map(|v| vec![v])
                .ok_or_else(|| ReadError::Invalid("Vehicle not found".into())),
            None => self.vehicles(),
        }
    }

    fn check_range(from: NaiveDate, to: NaiveDate) -> Result<(), ReadError> {
        if from > to {
            return Err(ReadError::Invalid("date_from is after date_to".into()));
        }
        Ok(())
    }

    /// Trips of the range, all covered vehicles merged, oldest first.
    pub fn trips_in_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        vehicle_id: Option<&str>,
    ) -> Result<Vec<Trip>, ReadError> {
        Self::check_range(from, to)?;
        let mut trips = Vec::new();
        for vehicle in self.vehicles_for(vehicle_id)? {
            trips.extend(
                self.db
                    .get_trips_for_vehicle_in_range(&vehicle.id.to_string(), from, to)
                    .map_err(internal)?,
            );
        }
        trips.sort_by(|a, b| {
            a.start_datetime
                .cmp(&b.start_datetime)
                .then(a.odometer.total_cmp(&b.odometer))
        });
        Ok(trips)
    }

    /// Journeys that overlap `from..=to`. All legs of each vehicle are grouped
    /// first and filtered after, so a journey that starts before `from` is found.
    pub fn journeys(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        vehicle_id: Option<&str>,
    ) -> Result<JourneyList, ReadError> {
        Self::check_range(from, to)?;
        let home = self
            .db
            .get_home_place()
            .map_err(internal)?
            .ok_or_else(|| ReadError::Invalid(HOME_NOT_SET.into()))?;
        let home_id = Uuid::parse_str(&home.id).map_err(internal)?;

        let mut journeys = Vec::new();
        for vehicle in self.vehicles_for(vehicle_id)? {
            let trips = self
                .db
                .get_trips_for_vehicle(&vehicle.id.to_string())
                .map_err(internal)?;
            let trip_ids: Vec<String> = trips.iter().map(|t| t.id.to_string()).collect();
            let round_trip_ids: HashSet<Uuid> = self
                .db
                .get_route_maps_for_trips(&trip_ids)
                .map_err(internal)?
                .into_iter()
                .filter(|(_, map)| map.round_trip)
                .filter_map(|(id, _)| Uuid::parse_str(&id).ok())
                .collect();
            let legs: Vec<Leg> = trips.iter().map(Leg::from_trip).collect();
            journeys.extend(
                group_journeys(vehicle.id, &legs, home_id, &round_trip_ids)
                    .into_iter()
                    .filter(|j| overlaps(j, from, to)),
            );
        }
        journeys.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(JourneyList {
            home_place: home.name,
            journeys,
        })
    }
}

#[cfg(test)]
#[path = "journeys_cmd_tests.rs"]
mod tests;
