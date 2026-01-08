// @generated automatically by Diesel CLI.

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
        receipt_date -> Nullable<Text>,
        station_name -> Nullable<Text>,
        station_address -> Nullable<Text>,
        source_year -> Nullable<Integer>,
        status -> Text,
        confidence -> Text,
        raw_ocr_text -> Nullable<Text>,
        error_message -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
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
        date -> Text,
        origin -> Text,
        destination -> Text,
        distance_km -> Double,
        odometer -> Double,
        purpose -> Text,
        fuel_liters -> Nullable<Double>,
        fuel_cost_eur -> Nullable<Double>,
        other_costs_eur -> Nullable<Double>,
        other_costs_note -> Nullable<Text>,
        full_tank -> Integer,
        sort_order -> Integer,
        energy_kwh -> Nullable<Double>,
        energy_cost_eur -> Nullable<Double>,
        full_charge -> Nullable<Integer>,
        soc_override_percent -> Nullable<Double>,
        created_at -> Text,
        updated_at -> Text,
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
    }
}

diesel::joinable!(receipts -> trips (trip_id));
diesel::joinable!(receipts -> vehicles (vehicle_id));
diesel::joinable!(routes -> vehicles (vehicle_id));
diesel::joinable!(trips -> vehicles (vehicle_id));

diesel::allow_tables_to_appear_in_same_query!(receipts, routes, settings, trips, vehicles,);
