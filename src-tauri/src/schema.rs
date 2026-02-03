// @generated automatically by Diesel CLI.
// NOTE: Manually adjusted - use Double instead of Float for f64 compatibility

diesel::table! {
    receipts (id) {
        id -> Nullable<Text>,
        vehicle_id -> Nullable<Text>,
        trip_id -> Nullable<Text>,
        file_path -> Text,
        file_name -> Text,
        scanned_at -> Text,
        liters -> Nullable<Double>,
        total_price_eur -> Nullable<Double>,
        receipt_datetime -> Nullable<Text>,
        station_name -> Nullable<Text>,
        station_address -> Nullable<Text>,
        source_year -> Nullable<Integer>,
        status -> Text,
        confidence -> Text,
        raw_ocr_text -> Nullable<Text>,
        error_message -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        vendor_name -> Nullable<Text>,
        cost_description -> Nullable<Text>,
        original_amount -> Nullable<Double>,
        original_currency -> Nullable<Text>,
        // Added via migration 2026-02-03-100000_receipt_assignment_type
        assignment_type -> Nullable<Text>,
        mismatch_override -> Integer,
    }
}

diesel::table! {
    routes (id) {
        id -> Nullable<Text>,
        vehicle_id -> Text,
        origin -> Text,
        destination -> Text,
        distance_km -> Double,
        usage_count -> Integer,
        last_used -> Text,
    }
}

diesel::table! {
    settings (id) {
        id -> Nullable<Text>,
        company_name -> Text,
        company_ico -> Text,
        buffer_trip_purpose -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    trips (id) {
        id -> Nullable<Text>,
        vehicle_id -> Text,
        origin -> Text,
        destination -> Text,
        distance_km -> Double,
        odometer -> Double,
        purpose -> Text,
        fuel_liters -> Nullable<Double>,
        fuel_cost_eur -> Nullable<Double>,
        other_costs_eur -> Nullable<Double>,
        other_costs_note -> Nullable<Text>,
        // Note: column order matches actual database (migrations added columns at end)
        created_at -> Text,
        updated_at -> Text,
        sort_order -> Integer,
        full_tank -> Integer,
        energy_kwh -> Nullable<Double>,
        energy_cost_eur -> Nullable<Double>,
        full_charge -> Nullable<Integer>,
        soc_override_percent -> Nullable<Double>,
        start_datetime -> Text,
        end_datetime -> Nullable<Text>,
    }
}

diesel::table! {
    vehicles (id) {
        id -> Nullable<Text>,
        name -> Text,
        license_plate -> Text,
        vehicle_type -> Text,
        tank_size_liters -> Nullable<Double>,
        tp_consumption -> Nullable<Double>,
        battery_capacity_kwh -> Nullable<Double>,
        baseline_consumption_kwh -> Nullable<Double>,
        initial_battery_percent -> Nullable<Double>,
        initial_odometer -> Double,
        is_active -> Integer,
        created_at -> Text,
        updated_at -> Text,
        vin -> Nullable<Text>,
        driver_name -> Nullable<Text>,
        ha_odo_sensor -> Nullable<Text>,
    }
}

diesel::joinable!(receipts -> trips (trip_id));
diesel::joinable!(receipts -> vehicles (vehicle_id));
diesel::joinable!(routes -> vehicles (vehicle_id));
diesel::joinable!(trips -> vehicles (vehicle_id));

diesel::allow_tables_to_appear_in_same_query!(receipts, routes, settings, trips, vehicles,);
