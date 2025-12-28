//! PDF export functionality for Kniha jázd

use crate::models::{Settings, Trip, TripGridData, Vehicle};

/// Data needed to generate the PDF export
pub struct PdfExportData {
    pub vehicle: Vehicle,
    pub settings: Settings,
    pub grid_data: TripGridData,
    pub year: i32,
    pub totals: ExportTotals,
}

/// Calculated totals for the export footer
#[derive(Debug, Clone, PartialEq)]
pub struct ExportTotals {
    pub total_km: f64,
    pub total_fuel_liters: f64,
    pub total_fuel_cost: f64,
    pub total_other_costs: f64,
    pub avg_consumption: f64,
    pub deviation_percent: f64,
}

impl ExportTotals {
    /// Calculate totals from a list of trips
    ///
    /// # Arguments
    /// * `trips` - List of trips to summarize
    /// * `tp_consumption` - Vehicle's technical passport consumption rate (l/100km)
    ///
    /// # Returns
    /// ExportTotals with all calculated values
    pub fn calculate(trips: &[Trip], tp_consumption: f64) -> Self {
        let total_km: f64 = trips.iter().map(|t| t.distance_km).sum();
        let total_fuel_liters: f64 = trips.iter().filter_map(|t| t.fuel_liters).sum();
        let total_fuel_cost: f64 = trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
        let total_other_costs: f64 = trips.iter().filter_map(|t| t.other_costs_eur).sum();

        let avg_consumption = if total_km > 0.0 {
            (total_fuel_liters / total_km) * 100.0
        } else {
            0.0
        };

        let deviation_percent = if tp_consumption > 0.0 && total_fuel_liters > 0.0 {
            (avg_consumption / tp_consumption) * 100.0
        } else {
            100.0 // 100% = exactly at TP rate (no deviation)
        };

        Self {
            total_km,
            total_fuel_liters,
            total_fuel_cost,
            total_other_costs,
            avg_consumption,
            deviation_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    fn make_trip(km: f64, fuel: Option<f64>, fuel_cost: Option<f64>, other_cost: Option<f64>) -> Trip {
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: km,
            odometer: 10000.0,
            purpose: "test".to_string(),
            fuel_liters: fuel,
            fuel_cost_eur: fuel_cost,
            other_costs_eur: other_cost,
            other_costs_note: None,
            full_tank: true,
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_export_totals_basic() {
        let trips = vec![
            make_trip(100.0, Some(6.0), Some(10.0), None),
            make_trip(200.0, Some(12.0), Some(20.0), Some(5.0)),
        ];

        let totals = ExportTotals::calculate(&trips, 5.0);

        assert_eq!(totals.total_km, 300.0);
        assert_eq!(totals.total_fuel_liters, 18.0);
        assert_eq!(totals.total_fuel_cost, 30.0);
        assert_eq!(totals.total_other_costs, 5.0);
        // avg = 18/300*100 = 6.0 l/100km
        assert!((totals.avg_consumption - 6.0).abs() < 0.001);
        // deviation = 6.0/5.0*100 = 120%
        assert!((totals.deviation_percent - 120.0).abs() < 0.001);
    }

    #[test]
    fn test_export_totals_no_trips() {
        let trips: Vec<Trip> = vec![];
        let totals = ExportTotals::calculate(&trips, 5.0);

        assert_eq!(totals.total_km, 0.0);
        assert_eq!(totals.total_fuel_liters, 0.0);
        assert_eq!(totals.avg_consumption, 0.0);
        assert_eq!(totals.deviation_percent, 100.0); // No fuel = 100% (at TP)
    }

    #[test]
    fn test_export_totals_no_fuel() {
        let trips = vec![
            make_trip(100.0, None, None, None),
            make_trip(200.0, None, None, None),
        ];

        let totals = ExportTotals::calculate(&trips, 5.0);

        assert_eq!(totals.total_km, 300.0);
        assert_eq!(totals.total_fuel_liters, 0.0);
        assert_eq!(totals.avg_consumption, 0.0);
        assert_eq!(totals.deviation_percent, 100.0);
    }

    #[test]
    fn test_export_totals_zero_tp() {
        let trips = vec![make_trip(100.0, Some(6.0), Some(10.0), None)];

        // Edge case: tp_consumption = 0 should not panic
        let totals = ExportTotals::calculate(&trips, 0.0);

        assert_eq!(totals.total_km, 100.0);
        assert_eq!(totals.deviation_percent, 100.0); // Defaults to 100% when tp is 0
    }
}
