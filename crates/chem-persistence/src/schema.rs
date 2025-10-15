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
diesel::table! {
    molecules (inchikey) {
        inchikey -> Text,
        smiles -> Text,
        inchi -> Text,
        metadata -> Text,
        structure -> Nullable<Text>,
        id -> Nullable<Text>,
        created_at_ts -> Nullable<BigInt>,
        updated_at_ts -> Nullable<BigInt>,
        version -> Nullable<Integer>,
        molecular_formula -> Nullable<Text>,
    }
}
diesel::table! {
    families (id) {
        id -> Text,
        name -> Nullable<Text>,
        description -> Nullable<Text>,
        family_hash -> Text,
        provenance -> Text,
        frozen -> Bool,
    }
}
diesel::table! {
    family_properties (id) {
        id -> Text,
        family_id -> Text,
        property_type -> Text,
        value -> Text,
        quality -> Nullable<Text>,
        preferred -> Bool,
        value_hash -> Text,
        metadata -> Text,
    }
}
diesel::table! {
    molecular_properties (id) {
        id -> Text,
        molecule_inchikey -> Text,
        property_type -> Text,
        value -> Text,
        quality -> Nullable<Text>,
        preferred -> Bool,
        value_hash -> Text,
        metadata -> Text,
    }
}
diesel::table! {
    family_members (id) {
        id -> Text,
        family_id -> Text,
        molecule_inchikey -> Text,
    }
}
// allow_tables_to_appear_in_same_query!(molecules, families, family_properties,
// molecular_properties, family_members);

diesel::table! {
    users (id) {
        id -> Text,
        name -> Text,
        email -> Text,
        university -> Nullable<Text>,
        password_hash -> Text,
        created_at -> BigInt,
        updated_at -> BigInt,
    }
}

diesel::table! {
    teams (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        created_at -> BigInt,
        updated_at -> BigInt,
    }
}

diesel::table! {
    team_members (team_id, user_id) {
        team_id -> Text,
        user_id -> Text,
    }
}

diesel::table! {
    molecule_family_access (family_id, accessor_id, accessor_type) {
        family_id -> Text,
        accessor_id -> Text,
        accessor_type -> Text,
    }
}

diesel::table! {
    molecule_access (molecule_id, accessor_id, accessor_type) {
        molecule_id -> Text,
        accessor_id -> Text,
        accessor_type -> Text,
    }
}

diesel::table! {
    flow_access (flow_id, accessor_id, accessor_type) {
        flow_id -> Text,
        accessor_id -> Text,
        accessor_type -> Text,
    }
}

allow_tables_to_appear_in_same_query!(users,
                                      teams,
                                      team_members,
                                      molecule_family_access,
                                      molecule_access,
                                      flow_access,
                                      flows,
                                      molecules,
                                      families);
