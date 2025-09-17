// Simplified Diesel schema for SQLite used in tests.
// Tablas: flows, flow_data, snapshots

use diesel::allow_tables_to_appear_in_same_query;

diesel::table! {
    flows (id) {
        id -> Text,
        name -> Nullable<Text>,
        status -> Nullable<Text>,
        created_by -> Nullable<Text>,
        created_at_ts -> BigInt,
        current_cursor -> BigInt,
        current_version -> BigInt,
        parent_flow_id -> Nullable<Text>,
        parent_cursor -> Nullable<BigInt>,
        metadata -> Text,
    }
}

diesel::table! {
    flow_data (id) {
        id -> Text,
        flow_id -> Text,
        cursor -> BigInt,
        key -> Text,
        payload -> Text,
        metadata -> Text,
        command_id -> Nullable<Text>,
        created_at_ts -> BigInt,
    }
}

diesel::table! {
    snapshots (id) {
        id -> Text,
        flow_id -> Text,
        cursor -> BigInt,
        state_ptr -> Text,
        metadata -> Text,
        created_at_ts -> BigInt,
    }
}

allow_tables_to_appear_in_same_query!(flows, flow_data, snapshots);
