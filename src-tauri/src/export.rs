//! HTML export functionality for Kniha jázd

use crate::models::{Settings, Trip, TripGridData, Vehicle};

/// Data needed to generate the HTML export
pub struct ExportData {
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

/// Generate HTML string for the logbook export
pub fn generate_html(data: ExportData) -> Result<String, String> {
    let mut rows = String::new();

    for trip in &data.grid_data.trips {
        let trip_id = trip.id.to_string();
        let rate = data.grid_data.rates.get(&trip_id).copied().unwrap_or(0.0);
        let zostatok = data
            .grid_data
            .fuel_remaining
            .get(&trip_id)
            .copied()
            .unwrap_or(0.0);

        let fuel_liters = trip
            .fuel_liters
            .map(|f| format!("{:.1}", f))
            .unwrap_or_default();
        let fuel_cost = trip
            .fuel_cost_eur
            .map(|f| format!("{:.2}", f))
            .unwrap_or_default();
        let other_costs = trip
            .other_costs_eur
            .map(|f| format!("{:.2}", f))
            .unwrap_or_default();
        let other_note = trip.other_costs_note.as_deref().unwrap_or("");

        rows.push_str(&format!(
            r#"        <tr>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td class="num">{:.0}</td>
          <td class="num">{:.0}</td>
          <td class="num">{}</td>
          <td class="num">{}</td>
          <td class="num">{}</td>
          <td>{}</td>
          <td class="num">{:.1}</td>
          <td class="num">{:.2}</td>
        </tr>
"#,
            trip.date.format("%d.%m.%Y"),
            html_escape(&trip.origin),
            html_escape(&trip.destination),
            html_escape(&trip.purpose),
            trip.distance_km,
            trip.odometer,
            fuel_liters,
            fuel_cost,
            other_costs,
            html_escape(other_note),
            zostatok,
            rate
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="sk">
<head>
  <meta charset="UTF-8">
  <title>Kniha jázd - {} - {}</title>
  <style>
    @media print {{
      @page {{
        size: A4 landscape;
        margin: 10mm;
      }}
      body {{
        -webkit-print-color-adjust: exact;
        print-color-adjust: exact;
      }}
    }}

    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }}

    body {{
      font-family: 'Segoe UI', Arial, sans-serif;
      font-size: 11px;
      line-height: 1.3;
      padding: 15px;
      max-width: 297mm;
    }}

    h1 {{
      font-size: 18px;
      margin-bottom: 10px;
      text-align: center;
    }}

    .header {{
      display: flex;
      justify-content: space-between;
      margin-bottom: 15px;
      padding: 10px;
      background: #f5f5f5;
      border-radius: 4px;
    }}

    .header-section {{
      flex: 1;
    }}

    .header-section p {{
      margin: 2px 0;
    }}

    .label {{
      font-weight: bold;
      color: #555;
    }}

    table {{
      width: 100%;
      border-collapse: collapse;
      font-size: 10px;
    }}

    th, td {{
      border: 1px solid #ccc;
      padding: 4px 6px;
      text-align: left;
    }}

    th {{
      background: #e8e8e8;
      font-weight: bold;
      text-align: center;
    }}

    td.num {{
      text-align: right;
      font-variant-numeric: tabular-nums;
    }}

    tr:nth-child(even) {{
      background: #fafafa;
    }}

    .footer {{
      margin-top: 15px;
      padding: 10px;
      background: #f0f0f0;
      border-radius: 4px;
      font-size: 11px;
    }}

    .footer-grid {{
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 10px;
    }}

    .footer-item {{
      text-align: center;
    }}

    .footer-value {{
      font-size: 14px;
      font-weight: bold;
      color: #333;
    }}

    .footer-label {{
      font-size: 9px;
      color: #666;
    }}

    .print-hint {{
      text-align: center;
      margin-top: 20px;
      color: #999;
      font-size: 10px;
    }}

    @media print {{
      .print-hint {{
        display: none;
      }}
    }}
  </style>
</head>
<body>
  <h1>KNIHA JÁZD</h1>

  <div class="header">
    <div class="header-section">
      <p><span class="label">Firma:</span> {}</p>
      <p><span class="label">IČO:</span> {}</p>
    </div>
    <div class="header-section">
      <p><span class="label">Vozidlo:</span> {}</p>
      <p><span class="label">ŠPZ:</span> {}</p>
    </div>
    <div class="header-section">
      <p><span class="label">Nádrž:</span> {} L</p>
      <p><span class="label">TP spotreba:</span> {} l/100km</p>
    </div>
    <div class="header-section">
      <p><span class="label">Rok:</span> {}</p>
    </div>
  </div>

  <table>
    <thead>
      <tr>
        <th>Dátum</th>
        <th>Odkiaľ</th>
        <th>Kam</th>
        <th>Účel</th>
        <th>Km</th>
        <th>ODO</th>
        <th>PHM L</th>
        <th>€ PHM</th>
        <th>€ Iné</th>
        <th>Poznámka</th>
        <th>Zost.</th>
        <th>Spotr.</th>
      </tr>
    </thead>
    <tbody>
{}    </tbody>
  </table>

  <div class="footer">
    <div class="footer-grid">
      <div class="footer-item">
        <div class="footer-value">{:.0} km</div>
        <div class="footer-label">Celkom km</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} L / {:.2} €</div>
        <div class="footer-label">Celkom PHM</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} €</div>
        <div class="footer-label">Iné náklady</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} l/100km</div>
        <div class="footer-label">Priemerná spotreba</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.1}%</div>
        <div class="footer-label">Odchýlka od TP</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{} l/100km</div>
        <div class="footer-label">TP norma</div>
      </div>
    </div>
  </div>

  <p class="print-hint">Pre export do PDF použite Ctrl+P → Uložiť ako PDF</p>
</body>
</html>
"#,
        data.vehicle.license_plate,
        data.year,
        html_escape(&data.settings.company_name),
        html_escape(&data.settings.company_ico),
        html_escape(&data.vehicle.name),
        html_escape(&data.vehicle.license_plate),
        data.vehicle.tank_size_liters,
        data.vehicle.tp_consumption,
        data.year,
        rows,
        data.totals.total_km,
        data.totals.total_fuel_liters,
        data.totals.total_fuel_cost,
        data.totals.total_other_costs,
        data.totals.avg_consumption,
        data.totals.deviation_percent,
        data.vehicle.tp_consumption
    );

    Ok(html)
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    fn make_trip(
        km: f64,
        fuel: Option<f64>,
        fuel_cost: Option<f64>,
        other_cost: Option<f64>,
    ) -> Trip {
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

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("\"test\""), "&quot;test&quot;");
    }
}
