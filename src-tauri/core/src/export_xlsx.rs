//! xlsx export of the logbook: the same header, table and totals as the
//! printout on sheet 1, the route maps on sheet 2.
//!
//! A number cell holds the exact backend value; its number format shows the
//! decimals of the printout. The file has no formulas: every total is the
//! value the backend calculated (ADR-008).

use crate::export::{ExportData, ExportLabels, RouteMapPage};
use crate::export_table::{build_export_table, CellValue, RowKind};
use crate::models::VehicleType;
use base64::Engine;
use rust_xlsxwriter::{Color, Format, FormatBorder, Image, Workbook, Worksheet, XlsxError};

/// A4 in the Excel paper size list.
const PAPER_A4: u8 = 9;
/// The printout's `th` and `tr.month-end-synthetic` backgrounds.
const HEADER_FILL: u32 = 0xE8E8E8;
const SYNTHETIC_FILL: u32 = 0xE8EEF4;
/// Rows from one attachment heading to its attribution line: the image
/// (640 px high at the default 20 px row) fits between them.
const ROWS_PER_ATTACHMENT_IMAGE: u32 = 33;

/// The Excel number format that shows `decimals` decimals.
fn num_format(decimals: usize) -> &'static str {
    match decimals {
        0 => "0",
        1 => "0.0",
        _ => "0.00",
    }
}

/// Excel allows 31 characters and no `[]:*?/\` in a sheet name.
fn sheet_name(label: &str) -> String {
    label
        .chars()
        .map(|c| if "[]:*?/\\".contains(c) { '-' } else { c })
        .take(31)
        .collect()
}

fn err(e: XlsxError) -> String {
    format!("xlsx export failed: {e}")
}

/// Build the xlsx file for one vehicle and year.
pub fn generate_xlsx(data: &ExportData) -> Result<Vec<u8>, String> {
    let mut workbook = Workbook::new();

    let logbook = workbook.add_worksheet();
    logbook
        .set_name(sheet_name(&data.labels.sheet_logbook))
        .map_err(err)?;
    write_logbook(logbook, data).map_err(err)?;

    if !data.route_maps.is_empty() {
        let attachments = workbook.add_worksheet();
        attachments
            .set_name(sheet_name(&data.labels.sheet_attachments))
            .map_err(err)?;
        write_attachments(attachments, &data.route_maps, &data.labels)?;
    }

    workbook.save_to_buffer().map_err(err)
}

fn write_logbook(ws: &mut Worksheet, data: &ExportData) -> Result<(), XlsxError> {
    let l = &data.labels;
    let v = &data.vehicle;
    let bold = Format::new().set_bold();

    ws.write_string_with_format(0, 0, &l.page_title, &Format::new().set_bold().set_font_size(14))?;

    // Header block, in the printout order: label in A, value in B.
    let mut row: u32 = 2;
    let mut text_item = |ws: &mut Worksheet, row: &mut u32, label: &str, value: &str| {
        ws.write_string_with_format(*row, 0, label, &bold)?;
        ws.write_string(*row, 1, value)?;
        *row += 1;
        Ok::<(), XlsxError>(())
    };
    text_item(ws, &mut row, &l.header_company, &data.settings.company_name)?;
    text_item(ws, &mut row, &l.header_ico, &data.settings.company_ico)?;
    text_item(ws, &mut row, &l.header_vehicle, &v.name)?;
    text_item(ws, &mut row, &l.header_license_plate, &v.license_plate)?;

    // The printout prints the specs with `{}`, so General keeps their decimals.
    let mut spec = |ws: &mut Worksheet, row: &mut u32, label: &str, value: Option<f64>, unit: &str| {
        ws.write_string_with_format(*row, 0, label, &bold)?;
        ws.write_number_with_format(
            *row,
            1,
            value.unwrap_or(0.0),
            &Format::new().set_num_format(format!("General\" {unit}\"")),
        )?;
        *row += 1;
        Ok::<(), XlsxError>(())
    };
    if v.vehicle_type.uses_fuel() {
        spec(ws, &mut row, &l.header_tank_size, v.tank_size_liters, "L")?;
        spec(ws, &mut row, &l.header_tp_consumption, v.tp_consumption, "l/100km")?;
    }
    if v.vehicle_type.uses_electricity() {
        spec(ws, &mut row, &l.header_battery_capacity, v.battery_capacity_kwh, "kWh")?;
        spec(
            ws,
            &mut row,
            &l.header_baseline_consumption,
            v.baseline_consumption_kwh,
            "kWh/100km",
        )?;
    }
    ws.write_string_with_format(row, 0, &l.header_year, &bold)?;
    ws.write_number(row, 1, data.year)?;
    row += 1;
    text_item(ws, &mut row, &l.header_vin, v.vin.as_deref().unwrap_or(""))?;
    text_item(ws, &mut row, &l.header_driver, v.driver_name.as_deref().unwrap_or(""))?;

    // Trip table.
    let table = build_export_table(data);
    let header_row = row + 1;
    let header_format = Format::new()
        .set_bold()
        .set_text_wrap()
        .set_background_color(Color::RGB(HEADER_FILL))
        .set_border(FormatBorder::Thin);
    for (c, column) in table.columns.iter().enumerate() {
        let c = c as u16;
        ws.write_string_with_format(header_row, c, &column.label, &header_format)?;
        ws.set_column_width(c, if column.numeric { 10 } else { 22 })?;
    }

    for (r, table_row) in table.rows.iter().enumerate() {
        let excel_row = header_row + 1 + r as u32;
        let synthetic = table_row.kind != RowKind::Trip;
        let base = || {
            let format = Format::new().set_border(FormatBorder::Thin);
            if synthetic {
                format.set_background_color(Color::RGB(SYNTHETIC_FILL)).set_italic()
            } else {
                format
            }
        };
        for (c, cell) in table_row.cells.iter().enumerate() {
            let c = c as u16;
            match cell {
                CellValue::Empty => {
                    ws.write_blank(excel_row, c, &base())?;
                }
                CellValue::Text(text) => {
                    ws.write_string_with_format(excel_row, c, text, &base())?;
                }
                CellValue::Number { value, decimals } => {
                    ws.write_number_with_format(
                        excel_row,
                        c,
                        *value,
                        &base().set_num_format(num_format(*decimals)),
                    )?;
                }
                CellValue::DateTime(dt) => {
                    ws.set_column_width(c, 13)?;
                    ws.write_datetime_with_format(
                        excel_row,
                        c,
                        dt,
                        &base().set_num_format("dd.mm. hh:mm"),
                    )?;
                }
            }
        }
    }

    let last_row = header_row + table.rows.len() as u32;
    let last_col = table.columns.len().saturating_sub(1) as u16;
    ws.autofilter(header_row, 0, last_row, last_col)?;
    ws.set_freeze_panes(header_row + 1, 0)?;

    write_footer(ws, data, last_row + 2)?;

    ws.set_paper_size(PAPER_A4);
    ws.set_landscape();
    ws.set_print_fit_to_pages(1, 0);
    ws.set_repeat_rows(header_row, header_row)?;
    Ok(())
}

/// One footer value per row: label in A, number in B, unit in the format.
/// The items per vehicle type are the printout's footer items; an item with
/// two values ("X L / Y €") becomes two rows with the same label.
fn write_footer(ws: &mut Worksheet, data: &ExportData, first_row: u32) -> Result<(), XlsxError> {
    let l = &data.labels;
    let t = &data.totals;
    let v = &data.vehicle;

    const KM: &str = "0\" km\"";
    const L: &str = "0.00\" L\"";
    const EUR: &str = "0.00\" €\"";
    const KWH: &str = "0.00\" kWh\"";
    const L100: &str = "0.00\" l/100km\"";
    const KWH100: &str = "0.00\" kWh/100km\"";
    const PERCENT: &str = "0.0\"%\"";
    const NORM_L100: &str = "General\" l/100km\"";
    const NORM_KWH100: &str = "General\" kWh/100km\"";

    let items: Vec<(&str, f64, &str)> = match v.vehicle_type {
        VehicleType::Ice => vec![
            (&l.footer_total_km, t.total_km, KM),
            (&l.footer_total_fuel, t.total_fuel_liters, L),
            (&l.footer_total_fuel, t.total_fuel_cost, EUR),
            (&l.footer_other_costs, t.total_other_costs, EUR),
            (&l.footer_avg_consumption, t.avg_consumption, L100),
            (&l.footer_deviation, t.deviation_percent, PERCENT),
            (&l.footer_tp_norm, v.tp_consumption.unwrap_or(0.0), NORM_L100),
        ],
        VehicleType::Bev => vec![
            (&l.footer_total_km, t.total_km, KM),
            (&l.footer_total_energy, t.total_energy_kwh, KWH),
            (&l.footer_total_energy, t.total_energy_cost, EUR),
            (&l.footer_other_costs, t.total_other_costs, EUR),
            (&l.footer_avg_energy_rate, t.avg_energy_rate, KWH100),
            (&l.footer_deviation, t.energy_deviation_percent, PERCENT),
            (
                &l.footer_baseline_norm,
                v.baseline_consumption_kwh.unwrap_or(0.0),
                NORM_KWH100,
            ),
        ],
        // The PHEV printout has no norm item.
        VehicleType::Phev => vec![
            (&l.footer_total_km, t.total_km, KM),
            (&l.footer_total_fuel, t.total_fuel_liters, L),
            (&l.footer_total_fuel, t.total_fuel_cost, EUR),
            (&l.footer_total_energy, t.total_energy_kwh, KWH),
            (&l.footer_total_energy, t.total_energy_cost, EUR),
            (&l.footer_other_costs, t.total_other_costs, EUR),
            (&l.footer_avg_consumption, t.avg_consumption, L100),
            (&l.footer_avg_energy_rate, t.avg_energy_rate, KWH100),
            (&l.footer_deviation, t.deviation_percent, PERCENT),
        ],
    };

    let bold = Format::new().set_bold();
    for (i, (label, value, format)) in items.into_iter().enumerate() {
        let row = first_row + i as u32;
        ws.write_string_with_format(row, 0, label, &bold)?;
        ws.write_number_with_format(row, 1, value, &Format::new().set_num_format(format))?;
    }
    Ok(())
}

/// One attachment per printed page: heading, map image, OSM attribution.
fn write_attachments(
    ws: &mut Worksheet,
    pages: &[RouteMapPage],
    l: &ExportLabels,
) -> Result<(), String> {
    let bold = Format::new().set_bold().set_font_size(12);
    let small = Format::new().set_font_size(8).set_font_color(Color::RGB(0x666666));
    let mut breaks = Vec::new();
    let mut row: u32 = 0;

    for page in pages {
        if row > 0 {
            breaks.push(row);
        }
        // The same heading text as the printout (export.rs build_map_pages).
        let heading = format!(
            "{} {} — {} {}",
            l.attachment_heading, page.attachment_no, l.record_reference, page.row_number
        );
        ws.write_string_with_format(row, 0, &heading, &bold).map_err(err)?;

        let png = base64::engine::general_purpose::STANDARD
            .decode(&page.png_base64)
            .map_err(|e| format!("route map {} is not valid base64: {e}", page.attachment_no))?;
        let image = Image::new_from_buffer(&png)
            .map_err(err)?
            .set_scale_to_size(1000, 640, true);
        ws.insert_image(row + 1, 0, &image).map_err(err)?;

        let attribution_row = row + 1 + ROWS_PER_ATTACHMENT_IMAGE;
        ws.write_string_with_format(attribution_row, 0, "© OpenStreetMap contributors", &small)
            .map_err(err)?;
        row = attribution_row + 2;
    }

    ws.set_page_breaks(&breaks).map_err(err)?;
    ws.set_paper_size(PAPER_A4);
    ws.set_landscape();
    ws.set_print_fit_to_pages(1, 0);
    Ok(())
}

#[cfg(test)]
#[path = "export_xlsx_tests.rs"]
mod tests;
