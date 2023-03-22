// @generated automatically by Diesel CLI.

diesel::table! {
    pages (id) {
        id -> Nullable<Integer>,
        parent_id -> Nullable<Integer>,
        title -> Text,
        slug -> Text,
        html_content -> Text,
    }
}
