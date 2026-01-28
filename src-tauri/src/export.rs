//! HTML export functionality for Kniha jázd

use crate::models::{Settings, Trip, TripGridData, Vehicle, VehicleType};
use serde::{Deserialize, Serialize};

/// Labels for HTML export (passed from frontend for i18n support)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLabels {
    // Language code for HTML lang attribute
    pub lang: String,
    // Page title
    pub page_title: String,
    // Header labels
    pub header_company: String,
    pub header_ico: String,
    pub header_vehicle: String,
    pub header_license_plate: String,
    pub header_tank_size: String,
    pub header_tp_consumption: String,
    pub header_year: String,
    // Header labels for BEV
    pub header_battery_capacity: String,
    pub header_baseline_consumption: String,
    // VIN and Driver
    pub header_vin: String,
    pub header_driver: String,
    // Column headers
    pub col_date: String,
    pub col_time: String,
    pub col_origin: String,
    pub col_destination: String,
    pub col_purpose: String,
    pub col_km: String,
    pub col_odo: String,
    pub col_fuel_liters: String,
    pub col_fuel_cost: String,
    pub col_fuel_consumed: String,
    pub col_other_costs: String,
    pub col_note: String,
    pub col_remaining: String,
    pub col_consumption: String,
    // Column headers for BEV
    pub col_energy_kwh: String,
    pub col_energy_cost: String,
    pub col_battery_remaining: String,
    pub col_energy_rate: String,
    // Footer labels
    pub footer_total_km: String,
    pub footer_total_fuel: String,
    pub footer_other_costs: String,
    pub footer_avg_consumption: String,
    pub footer_deviation: String,
    pub footer_tp_norm: String,
    // Footer labels for BEV
    pub footer_total_energy: String,
    pub footer_avg_energy_rate: String,
    pub footer_baseline_norm: String,
    // Print hint
    pub print_hint: String,
}

/// Data needed to generate the HTML export
pub struct ExportData {
    pub vehicle: Vehicle,
    pub settings: Settings,
    pub grid_data: TripGridData,
    pub year: i32,
    pub totals: ExportTotals,
    pub labels: ExportLabels,
    /// Columns hidden by user (e.g., ["time", "fuelConsumed", "fuelRemaining", "otherCosts", "otherCostsNote"])
    pub hidden_columns: Vec<String>,
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
    // Energy totals (for BEV/PHEV)
    pub total_energy_kwh: f64,
    pub total_energy_cost: f64,
    pub avg_energy_rate: f64,
    pub energy_deviation_percent: f64,
}

impl ExportTotals {
    /// Calculate totals from a list of trips
    ///
    /// # Arguments
    /// * `trips` - List of trips to summarize (excludes dummy rows with 0 km)
    /// * `tp_consumption` - Vehicle's technical passport consumption rate (l/100km)
    /// * `baseline_consumption_kwh` - Vehicle's baseline energy consumption (kWh/100km) for BEV
    ///
    /// # Returns
    /// ExportTotals with all calculated values
    pub fn calculate(trips: &[Trip], tp_consumption: f64, baseline_consumption_kwh: f64) -> Self {
        // Filter out dummy rows (trips with 0 km distance)
        let real_trips: Vec<_> = trips.iter().filter(|t| t.distance_km > 0.0).collect();

        let total_km: f64 = real_trips.iter().map(|t| t.distance_km).sum();
        let total_fuel_liters: f64 = real_trips.iter().filter_map(|t| t.fuel_liters).sum();
        let total_fuel_cost: f64 = real_trips.iter().filter_map(|t| t.fuel_cost_eur).sum();
        let total_other_costs: f64 = real_trips.iter().filter_map(|t| t.other_costs_eur).sum();
        let total_energy_kwh: f64 = real_trips.iter().filter_map(|t| t.energy_kwh).sum();
        let total_energy_cost: f64 = real_trips.iter().filter_map(|t| t.energy_cost_eur).sum();

        let avg_consumption = if total_km > 0.0 {
            (total_fuel_liters / total_km) * 100.0
        } else {
            0.0
        };

        let avg_energy_rate = if total_km > 0.0 {
            (total_energy_kwh / total_km) * 100.0
        } else {
            0.0
        };

        let deviation_percent = if tp_consumption > 0.0 && total_fuel_liters > 0.0 {
            (avg_consumption / tp_consumption) * 100.0
        } else {
            100.0 // 100% = exactly at TP rate (no deviation)
        };

        let energy_deviation_percent = if baseline_consumption_kwh > 0.0 && total_energy_kwh > 0.0 {
            (avg_energy_rate / baseline_consumption_kwh) * 100.0
        } else {
            100.0 // 100% = exactly at baseline rate (no deviation)
        };

        // Normalize near-zero values to avoid -0.00 display
        let normalize = |v: f64| if v.abs() < 0.001 { 0.0 } else { v };

        Self {
            total_km: normalize(total_km),
            total_fuel_liters: normalize(total_fuel_liters),
            total_fuel_cost: normalize(total_fuel_cost),
            total_other_costs: normalize(total_other_costs),
            avg_consumption: normalize(avg_consumption),
            deviation_percent,
            total_energy_kwh: normalize(total_energy_kwh),
            total_energy_cost: normalize(total_energy_cost),
            avg_energy_rate: normalize(avg_energy_rate),
            energy_deviation_percent,
        }
    }
}

/// Generate HTML string for the logbook export
pub fn generate_html(data: ExportData) -> Result<String, String> {
    let vehicle_type = data.vehicle.vehicle_type;
    let show_fuel = vehicle_type.uses_fuel();
    let show_energy = vehicle_type.uses_electricity();

    // Helper to check if a column is visible (not in hidden_columns)
    let is_visible = |col: &str| !data.hidden_columns.contains(&col.to_string());

    let mut rows = String::new();

    for trip in &data.grid_data.trips {
        let trip_id = trip.id.to_string();

        // Common fields
        let other_costs = trip
            .other_costs_eur
            .map(|f| format!("{:.2}", f))
            .unwrap_or_default();
        let other_note = trip.other_costs_note.as_deref().unwrap_or("");

        // Build row - start with date (always shown)
        let mut row = format!(
            r#"        <tr>
          <td>{}</td>"#,
            trip.date.format("%d.%m.%Y"),
        );

        // Time column (hideable)
        if is_visible("time") {
            row.push_str(&format!(
                r#"
          <td>{}</td>"#,
                trip.datetime.format("%H:%M"),
            ));
        }

        // Origin, destination, purpose, distance, odometer (always shown)
        row.push_str(&format!(
            r#"
          <td>{}</td>
          <td>{}</td>
          <td>{}</td>
          <td class="num">{:.0}</td>
          <td class="num">{:.0}</td>"#,
            html_escape(&trip.origin),
            html_escape(&trip.destination),
            html_escape(&trip.purpose),
            trip.distance_km,
            trip.odometer,
        ));

        // Fuel columns (ICE + PHEV)
        if show_fuel {
            let rate = data.grid_data.rates.get(&trip_id).copied().unwrap_or(0.0);
            let fuel_consumed = data
                .grid_data
                .fuel_consumed
                .get(&trip_id)
                .copied()
                .unwrap_or(0.0);
            let fuel_remaining = data
                .grid_data
                .fuel_remaining
                .get(&trip_id)
                .copied()
                .unwrap_or(0.0);
            let fuel_liters = trip
                .fuel_liters
                .map(|f| format!("{:.2}", f))
                .unwrap_or_default();
            let fuel_cost = trip
                .fuel_cost_eur
                .map(|f| format!("{:.2}", f))
                .unwrap_or_default();

            // Fuel liters and cost (always shown when fuel vehicle)
            row.push_str(&format!(
                r#"
          <td class="num">{}</td>
          <td class="num">{}</td>"#,
                fuel_liters, fuel_cost,
            ));

            // Fuel consumed (hideable)
            if is_visible("fuelConsumed") {
                row.push_str(&format!(
                    r#"
          <td class="num">{:.2}</td>"#,
                    fuel_consumed,
                ));
            }

            // Fuel remaining (hideable)
            if is_visible("fuelRemaining") {
                row.push_str(&format!(
                    r#"
          <td class="num">{:.1}</td>"#,
                    fuel_remaining,
                ));
            }

            // Consumption rate (always shown when fuel vehicle)
            row.push_str(&format!(
                r#"
          <td class="num">{:.2}</td>"#,
                rate,
            ));
        }

        // Energy columns (BEV + PHEV)
        if show_energy {
            let energy_rate = data
                .grid_data
                .energy_rates
                .get(&trip_id)
                .copied()
                .unwrap_or(0.0);
            let battery_remaining = data
                .grid_data
                .battery_remaining_kwh
                .get(&trip_id)
                .copied()
                .unwrap_or(0.0);
            let energy_kwh = trip
                .energy_kwh
                .map(|e| format!("{:.1}", e))
                .unwrap_or_default();
            let energy_cost = trip
                .energy_cost_eur
                .map(|e| format!("{:.2}", e))
                .unwrap_or_default();

            // Energy kWh and cost (always shown when electric vehicle)
            row.push_str(&format!(
                r#"
          <td class="num">{}</td>
          <td class="num">{}</td>"#,
                energy_kwh, energy_cost,
            ));

            // Battery remaining (hideable - same setting as fuelRemaining)
            if is_visible("fuelRemaining") {
                row.push_str(&format!(
                    r#"
          <td class="num">{:.1}</td>"#,
                    battery_remaining,
                ));
            }

            // Energy rate (always shown when electric vehicle)
            row.push_str(&format!(
                r#"
          <td class="num">{:.2}</td>"#,
                energy_rate,
            ));
        }

        // Other costs (hideable)
        if is_visible("otherCosts") {
            row.push_str(&format!(
                r#"
          <td class="num">{}</td>"#,
                other_costs,
            ));
        }

        // Other costs note (hideable)
        if is_visible("otherCostsNote") {
            row.push_str(&format!(
                r#"
          <td>{}</td>"#,
                html_escape(other_note),
            ));
        }

        row.push_str(
            r#"
        </tr>
"#,
        );

        rows.push_str(&row);
    }

    let l = &data.labels;

    // Build vehicle specs section based on vehicle type
    let specs_section = match vehicle_type {
        VehicleType::Ice => format!(
            r#"    <div class="header-section">
      <p><span class="label">{}</span> {} L</p>
      <p><span class="label">{}</span> {} l/100km</p>
    </div>"#,
            html_escape(&l.header_tank_size),
            data.vehicle.tank_size_liters.unwrap_or(0.0),
            html_escape(&l.header_tp_consumption),
            data.vehicle.tp_consumption.unwrap_or(0.0),
        ),
        VehicleType::Bev => format!(
            r#"    <div class="header-section">
      <p><span class="label">{}</span> {} kWh</p>
      <p><span class="label">{}</span> {} kWh/100km</p>
    </div>"#,
            html_escape(&l.header_battery_capacity),
            data.vehicle.battery_capacity_kwh.unwrap_or(0.0),
            html_escape(&l.header_baseline_consumption),
            data.vehicle.baseline_consumption_kwh.unwrap_or(0.0),
        ),
        VehicleType::Phev => format!(
            r#"    <div class="header-section">
      <p><span class="label">{}</span> {} L</p>
      <p><span class="label">{}</span> {} l/100km</p>
      <p><span class="label">{}</span> {} kWh</p>
      <p><span class="label">{}</span> {} kWh/100km</p>
    </div>"#,
            html_escape(&l.header_tank_size),
            data.vehicle.tank_size_liters.unwrap_or(0.0),
            html_escape(&l.header_tp_consumption),
            data.vehicle.tp_consumption.unwrap_or(0.0),
            html_escape(&l.header_battery_capacity),
            data.vehicle.battery_capacity_kwh.unwrap_or(0.0),
            html_escape(&l.header_baseline_consumption),
            data.vehicle.baseline_consumption_kwh.unwrap_or(0.0),
        ),
    };

    // Build column headers based on vehicle type and visibility settings
    // Date (always shown)
    let mut col_headers = format!(
        r#"        <th>{}</th>"#,
        html_escape(&l.col_date),
    );

    // Time (hideable)
    if is_visible("time") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_time),
        ));
    }

    // Origin, destination, purpose, km, odometer (always shown)
    col_headers.push_str(&format!(
        r#"
        <th>{}</th>
        <th>{}</th>
        <th>{}</th>
        <th>{}</th>
        <th>{}</th>"#,
        html_escape(&l.col_origin),
        html_escape(&l.col_destination),
        html_escape(&l.col_purpose),
        html_escape(&l.col_km),
        html_escape(&l.col_odo),
    ));

    // Fuel columns (ICE + PHEV)
    if show_fuel {
        // Fuel liters and cost (always shown when fuel vehicle)
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>
        <th>{}</th>"#,
            html_escape(&l.col_fuel_liters),
            html_escape(&l.col_fuel_cost),
        ));

        // Fuel consumed (hideable)
        if is_visible("fuelConsumed") {
            col_headers.push_str(&format!(
                r#"
        <th>{}</th>"#,
                html_escape(&l.col_fuel_consumed),
            ));
        }

        // Fuel remaining (hideable)
        if is_visible("fuelRemaining") {
            col_headers.push_str(&format!(
                r#"
        <th>{}</th>"#,
                html_escape(&l.col_remaining),
            ));
        }

        // Consumption rate (always shown when fuel vehicle)
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_consumption),
        ));
    }

    // Energy columns (BEV + PHEV)
    if show_energy {
        // Energy kWh and cost (always shown when electric vehicle)
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>
        <th>{}</th>"#,
            html_escape(&l.col_energy_kwh),
            html_escape(&l.col_energy_cost),
        ));

        // Battery remaining (hideable - same setting as fuelRemaining)
        if is_visible("fuelRemaining") {
            col_headers.push_str(&format!(
                r#"
        <th>{}</th>"#,
                html_escape(&l.col_battery_remaining),
            ));
        }

        // Energy rate (always shown when electric vehicle)
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_energy_rate),
        ));
    }

    // Other costs (hideable)
    if is_visible("otherCosts") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_other_costs),
        ));
    }

    // Other costs note (hideable)
    if is_visible("otherCostsNote") {
        col_headers.push_str(&format!(
            r#"
        <th>{}</th>"#,
            html_escape(&l.col_note),
        ));
    }

    // Build footer based on vehicle type
    let footer_items = match vehicle_type {
        VehicleType::Ice => format!(
            r#"      <div class="footer-item">
        <div class="footer-value">{:.0} km</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} L / {:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} l/100km</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.1}%</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{} l/100km</div>
        <div class="footer-label">{}</div>
      </div>"#,
            data.totals.total_km,
            html_escape(&l.footer_total_km),
            data.totals.total_fuel_liters,
            data.totals.total_fuel_cost,
            html_escape(&l.footer_total_fuel),
            data.totals.total_other_costs,
            html_escape(&l.footer_other_costs),
            data.totals.avg_consumption,
            html_escape(&l.footer_avg_consumption),
            data.totals.deviation_percent,
            html_escape(&l.footer_deviation),
            data.vehicle.tp_consumption.unwrap_or(0.0),
            html_escape(&l.footer_tp_norm),
        ),
        VehicleType::Bev => format!(
            r#"      <div class="footer-item">
        <div class="footer-value">{:.0} km</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} kWh / {:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} kWh/100km</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.1}%</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{} kWh/100km</div>
        <div class="footer-label">{}</div>
      </div>"#,
            data.totals.total_km,
            html_escape(&l.footer_total_km),
            data.totals.total_energy_kwh,
            data.totals.total_energy_cost,
            html_escape(&l.footer_total_energy),
            data.totals.total_other_costs,
            html_escape(&l.footer_other_costs),
            data.totals.avg_energy_rate,
            html_escape(&l.footer_avg_energy_rate),
            data.totals.energy_deviation_percent,
            html_escape(&l.footer_deviation),
            data.vehicle.baseline_consumption_kwh.unwrap_or(0.0),
            html_escape(&l.footer_baseline_norm),
        ),
        VehicleType::Phev => format!(
            r#"      <div class="footer-item">
        <div class="footer-value">{:.0} km</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} L / {:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} kWh / {:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} €</div>
        <div class="footer-label">{}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.2} l/100km / {:.2} kWh/100km</div>
        <div class="footer-label">{} / {}</div>
      </div>
      <div class="footer-item">
        <div class="footer-value">{:.1}%</div>
        <div class="footer-label">{}</div>
      </div>"#,
            data.totals.total_km,
            html_escape(&l.footer_total_km),
            data.totals.total_fuel_liters,
            data.totals.total_fuel_cost,
            html_escape(&l.footer_total_fuel),
            data.totals.total_energy_kwh,
            data.totals.total_energy_cost,
            html_escape(&l.footer_total_energy),
            data.totals.total_other_costs,
            html_escape(&l.footer_other_costs),
            data.totals.avg_consumption,
            data.totals.avg_energy_rate,
            html_escape(&l.footer_avg_consumption),
            html_escape(&l.footer_avg_energy_rate),
            data.totals.deviation_percent,
            html_escape(&l.footer_deviation),
        ),
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
  <meta charset="UTF-8">
  <title>{page_title} - {license_plate} - {year}</title>
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
  <h1>{page_title}</h1>

  <div class="header">
    <div class="header-section">
      <p><span class="label">{header_company}</span> {company_name}</p>
      <p><span class="label">{header_ico}</span> {company_ico}</p>
    </div>
    <div class="header-section">
      <p><span class="label">{header_vehicle}</span> {vehicle_name}</p>
      <p><span class="label">{header_license_plate}</span> {license_plate}</p>
    </div>
{specs_section}
    <div class="header-section">
      <p><span class="label">{header_year}</span> {year}</p>
      <p><span class="label">{header_vin}</span> {vin}</p>
      <p><span class="label">{header_driver}</span> {driver_name}</p>
    </div>
  </div>

  <table>
    <thead>
      <tr>
{col_headers}
      </tr>
    </thead>
    <tbody>
{rows}    </tbody>
  </table>

  <div class="footer">
    <div class="footer-grid">
{footer_items}
    </div>
  </div>

  <p class="print-hint">{print_hint}</p>
</body>
</html>
"#,
        lang = html_escape(&l.lang),
        page_title = html_escape(&l.page_title),
        license_plate = html_escape(&data.vehicle.license_plate),
        year = data.year,
        header_company = html_escape(&l.header_company),
        company_name = html_escape(&data.settings.company_name),
        header_ico = html_escape(&l.header_ico),
        company_ico = html_escape(&data.settings.company_ico),
        header_vehicle = html_escape(&l.header_vehicle),
        vehicle_name = html_escape(&data.vehicle.name),
        header_license_plate = html_escape(&l.header_license_plate),
        specs_section = specs_section,
        header_year = html_escape(&l.header_year),
        header_vin = html_escape(&l.header_vin),
        vin = html_escape(data.vehicle.vin.as_deref().unwrap_or("")),
        header_driver = html_escape(&l.header_driver),
        driver_name = html_escape(data.vehicle.driver_name.as_deref().unwrap_or("")),
        col_headers = col_headers,
        rows = rows,
        footer_items = footer_items,
        print_hint = html_escape(&l.print_hint),
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
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        Trip {
            id: Uuid::new_v4(),
            vehicle_id: Uuid::new_v4(),
            date,
            datetime: date.and_hms_opt(0, 0, 0).unwrap(),
            origin: "A".to_string(),
            destination: "B".to_string(),
            distance_km: km,
            odometer: 10000.0,
            purpose: "test".to_string(),
            fuel_liters: fuel,
            fuel_cost_eur: fuel_cost,
            full_tank: true,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: other_cost,
            other_costs_note: None,
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

        let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

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
        let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

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

        let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

        assert_eq!(totals.total_km, 300.0);
        assert_eq!(totals.total_fuel_liters, 0.0);
        assert_eq!(totals.avg_consumption, 0.0);
        assert_eq!(totals.deviation_percent, 100.0);
    }

    #[test]
    fn test_export_totals_zero_tp() {
        let trips = vec![make_trip(100.0, Some(6.0), Some(10.0), None)];

        // Edge case: tp_consumption = 0 should not panic
        let totals = ExportTotals::calculate(&trips, 0.0, 0.0);

        assert_eq!(totals.total_km, 100.0);
        assert_eq!(totals.deviation_percent, 100.0); // Defaults to 100% when tp is 0
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("\"test\""), "&quot;test&quot;");
    }

    #[test]
    fn test_export_totals_excludes_dummy_rows() {
        // Dummy row (0 km) should be excluded from totals
        let trips = vec![
            make_trip(0.0, None, None, Some(999.0)),  // Dummy row - should be excluded
            make_trip(100.0, Some(6.0), Some(10.0), Some(5.0)),
            make_trip(200.0, Some(12.0), Some(20.0), None),
        ];

        let totals = ExportTotals::calculate(&trips, 5.0, 0.0);

        // Should only count trips with km > 0
        assert_eq!(totals.total_km, 300.0);      // 100 + 200, not 0 + 100 + 200
        assert_eq!(totals.total_fuel_liters, 18.0);
        assert_eq!(totals.total_fuel_cost, 30.0);
        assert_eq!(totals.total_other_costs, 5.0); // Only from second trip, dummy's 999 excluded
    }

}
