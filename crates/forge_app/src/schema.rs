// @generated automatically by Diesel CLI.

diesel::table! {
    configuration_table (id) {
        id -> Text,
        data -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    conversations (id) {
        id -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        content -> Text,
        archived -> Bool,
        title -> Nullable<Text>,
    }
}

diesel::table! {
    snapshots (id) {
        id -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        file_path -> Text,
        archived -> Bool,
        snapshot_path -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    configuration_table,
    conversations,
    snapshots,
);
