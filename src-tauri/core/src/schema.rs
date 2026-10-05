// @generated automatically by Diesel CLI.
// NOTE: Manually adjusted - use Double instead of Float for f64 compatibility

diesel::table! {
    routes (id) {
        id -> Nullable<Text>,
        vehicle_id -> Text,
        // Rebuilt by migration 2026-10-05-100000_places_as_entities (Task 88).
        origin_place_id -> Text,
        destination_place_id -> Text,
        distance_km -> Double,
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
        // Rebuilt by migration 2026-10-05-100000_places_as_entities (Task 88):
        // the two place ids keep the position the old strings had.
        origin_place_id -> Text,
        destination_place_id -> Text,
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
        // Added via migration 2026-02-11-100000_add_vehicle_ha_fillup_sensor
        ha_fillup_sensor -> Nullable<Text>,
        // Added via migration 2026-02-12-100000_add_vehicle_ha_fuel_level_sensor
        ha_fuel_level_sensor -> Nullable<Text>,
    }
}

// Rebuilt via migration 2026-07-15-100000_multi_invoice (Task 66)
diesel::table! {
    paperless_trip_links (paperless_document_id) {
        paperless_document_id -> BigInt,
        trip_id -> Text,
        assignment_type -> Text,
        amount_eur -> Nullable<Double>,
        title -> Nullable<Text>,
        applied_amount_cents -> Nullable<BigInt>,
        created_at -> Text,
        updated_at -> Text,
        // Added via migration 2026-09-11-120000_paperless_only_invoice_columns (Task 84)
        receipt_datetime -> Nullable<Text>,
        mismatch_override -> Bool,
    }
}

// Added via migration 2026-08-10-100000_add_trip_routes (Task 70)
diesel::table! {
    trip_routes (trip_id) {
        trip_id -> Text,
        waypoints -> Text,
        polyline -> Text,
        target_km -> Double,
        road_km -> Double,
        dataset_version -> Nullable<Text>,
        created_at -> Text,
        // Added via migration 2026-09-07-110000_add_trip_route_mode (Task 72).
        // Appended LAST, matching RouteMapRow field order: RouteMapRow is
        // Queryable and binds POSITIONALLY, and `mode`/`created_at` are both
        // Text -- a mismatched position here swaps the two silently.
        mode -> Text,
        // Added via migration 2026-09-07-120000_add_trip_route_round_trip
        // (Task 20). Appended LAST for the same reason as `mode` above: this
        // column must stay the last field in RouteMapRow. It is Bool, not
        // Text, so a swap with `mode` or `created_at` would at least fail to
        // compile -- but the ordering rule still applies, so a future TEXT
        // column added after this one does not inherit that accidental safety.
        round_trip -> Bool,
        // Added via migration 2026-09-09-100000_add_trip_route_turnaround_index
        // (Task 78). Appended LAST for the third time, for the same reason as
        // `mode` and `round_trip` above: RouteMapRow is Queryable and binds
        // POSITIONALLY. Any future column goes after this one, never between
        // existing ones.
        turnaround_index -> Nullable<Integer>,
        // Added via migration 2026-09-29-100000_add_trip_route_avoid (Task 85).
        // Appended LAST for the fourth time: RouteMapRow binds POSITIONALLY.
        // It is Text like `mode` and `created_at`, so a wrong position here
        // would compile and swap them silently.
        avoid -> Text,
        // Added via migration 2026-09-29-110000_add_trip_route_provider
        // (Task 86). Appended LAST for the fifth time: RouteMapRow binds
        // POSITIONALLY. Nullable Text, so a swap with `dataset_version` would
        // compile -- keep it last.
        provider -> Nullable<Text>,
    }
}

// Added via migration 2026-09-07-100000_add_places (Task 75), rebuilt by
// migration 2026-10-05-100000_places_as_entities (Task 88).
diesel::table! {
    places (id) {
        id -> Text,
        name -> Text,
        normalised_name -> Text,
        lat -> Nullable<Double>,
        lon -> Nullable<Double>,
        source -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::joinable!(routes -> vehicles (vehicle_id));
diesel::joinable!(trips -> vehicles (vehicle_id));
diesel::joinable!(paperless_trip_links -> trips (trip_id));
diesel::joinable!(trip_routes -> trips (trip_id));

diesel::allow_tables_to_appear_in_same_query!(paperless_trip_links, places, routes, settings, trip_routes, trips, vehicles,);
