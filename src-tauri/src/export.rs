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

use genpdf::fonts;
use genpdf::{elements, style, Document, Element, SimplePageDecorator, Size};

/// Generate PDF bytes for the logbook export
pub fn generate_pdf(data: PdfExportData) -> Result<Vec<u8>, String> {
    // Load fonts from embedded bytes
    let regular_bytes = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    let bold_bytes = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");

    let regular = fonts::FontData::new(regular_bytes.to_vec(), None)
        .map_err(|e| format!("Failed to load regular font: {}", e))?;
    let bold = fonts::FontData::new(bold_bytes.to_vec(), None)
        .map_err(|e| format!("Failed to load bold font: {}", e))?;

    let font_family = fonts::FontFamily {
        regular,
        bold,
        italic: fonts::FontData::new(regular_bytes.to_vec(), None)
            .map_err(|e| format!("Failed to load italic font: {}", e))?,
        bold_italic: fonts::FontData::new(bold_bytes.to_vec(), None)
            .map_err(|e| format!("Failed to load bold-italic font: {}", e))?,
    };

    // Create document with landscape A4 (297x210mm)
    let mut doc = Document::new(font_family);
    doc.set_paper_size(Size::new(297, 210)); // Landscape A4

    // Set up page margins using SimplePageDecorator
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);

    // Add title
    doc.push(
        elements::Paragraph::new("KNIHA JÁZD")
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(0.5));

    // Add company info
    let company_line = format!(
        "Firma: {} | IČO: {}",
        data.settings.company_name,
        data.settings.company_ico
    );
    doc.push(elements::Paragraph::new(company_line));

    // Add vehicle info
    let vehicle_line = format!(
        "Vozidlo: {} | ŠPZ: {} | Nádrž: {} L | TP spotreba: {} l/100km",
        data.vehicle.name,
        data.vehicle.license_plate,
        data.vehicle.tank_size_liters,
        data.vehicle.tp_consumption
    );
    doc.push(elements::Paragraph::new(vehicle_line));

    // Add year
    doc.push(elements::Paragraph::new(format!("Rok: {}", data.year)));
    doc.push(elements::Break::new(1.0));

    // Build trip table
    let table = build_trip_table(&data);
    doc.push(table);

    doc.push(elements::Break::new(1.0));

    // Add footer with totals
    let footer = build_footer(&data.totals);
    doc.push(footer);

    // Render to bytes
    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("Failed to render PDF: {}", e))?;

    Ok(buffer)
}

fn build_trip_table(data: &PdfExportData) -> elements::TableLayout {
    let mut table = elements::TableLayout::new(vec![
        1, // Dátum
        2, // Odkiaľ
        2, // Kam
        2, // Účel
        1, // Km
        1, // ODO
        1, // PHM (L)
        1, // € PHM
        1, // € Iné
        2, // Poznámka
        1, // Zostatok
        1, // Spotreba
    ]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

    // Header row
    let headers = vec![
        "Dátum", "Odkiaľ", "Kam", "Účel", "Km", "ODO",
        "PHM (L)", "€ PHM", "€ Iné", "Poznámka", "Zostatok", "Spotreba",
    ];

    let mut header_row = table.row();
    for h in headers {
        header_row.push_element(
            elements::Paragraph::new(h)
                .styled(style::Style::new().bold().with_font_size(8)),
        );
    }
    header_row.push().expect("Failed to push header row");

    // Data rows
    for trip in &data.grid_data.trips {
        let trip_id = trip.id.to_string();
        let rate = data.grid_data.rates.get(&trip_id).copied().unwrap_or(0.0);
        let zostatok = data.grid_data.fuel_remaining.get(&trip_id).copied().unwrap_or(0.0);

        let mut row = table.row();
        row.push_element(cell(&trip.date.format("%d.%m.%Y").to_string()));
        row.push_element(cell(&trip.origin));
        row.push_element(cell(&trip.destination));
        row.push_element(cell(&trip.purpose));
        row.push_element(cell(&format!("{:.0}", trip.distance_km)));
        row.push_element(cell(&format!("{:.0}", trip.odometer)));
        row.push_element(cell(&trip.fuel_liters.map(|f| format!("{:.2}", f)).unwrap_or_default()));
        row.push_element(cell(&trip.fuel_cost_eur.map(|f| format!("{:.2}", f)).unwrap_or_default()));
        row.push_element(cell(&trip.other_costs_eur.map(|f| format!("{:.2}", f)).unwrap_or_default()));
        row.push_element(cell(trip.other_costs_note.as_deref().unwrap_or("")));
        row.push_element(cell(&format!("{:.1}", zostatok)));
        row.push_element(cell(&format!("{:.2}", rate)));
        row.push().expect("Failed to push data row");
    }

    table
}

fn cell(text: &str) -> impl Element {
    elements::Paragraph::new(text).styled(style::Style::new().with_font_size(7))
}

fn build_footer(totals: &ExportTotals) -> impl Element {
    let footer_text = format!(
        "SPOLU: {:.0} km | PHM: {:.2} L / {:.2} € | Iné náklady: {:.2} € | \
         Priemerná spotreba: {:.2} l/100km | Odchýlka oproti TP: {:.1}%",
        totals.total_km,
        totals.total_fuel_liters,
        totals.total_fuel_cost,
        totals.total_other_costs,
        totals.avg_consumption,
        totals.deviation_percent
    );

    elements::Paragraph::new(footer_text).styled(style::Style::new().bold().with_font_size(9))
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
