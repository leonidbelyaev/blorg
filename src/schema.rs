// @generated automatically by Diesel CLI.

diesel::table! {
    pages (id) {
        id -> Integer,
        parent_id -> Nullable<Integer>,
        title -> Text,
        slug -> Text,
        html_content -> Text,
    }
}
