// @generated automatically by Diesel CLI.

diesel::table! {
    admins (id) {
        id -> Nullable<Integer>,
        username -> Text,
        password_hash -> Text,
    }
}

diesel::table! {
    pages (id) {
        id -> Nullable<Integer>,
        parent_id -> Nullable<Integer>,
        title -> Text,
        slug -> Text,
        html_content -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    admins,
    pages,
);
