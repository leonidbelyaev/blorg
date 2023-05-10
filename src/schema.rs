// @generated automatically by Diesel CLI.

diesel::table! {
    admins (id) {
        id -> Nullable<Integer>,
        username -> Text,
        password_hash -> Text,
    }
}

diesel::table! {
    page_revision (id) {
        id -> Nullable<Integer>,
        page_id -> Nullable<Integer>,
        iso_time -> Text,
        unix_time -> BigInt,
        html_content -> Text,
        markdown_content -> Nullable<Text>,
        sidebar_html_content -> Text,
        sidebar_markdown_content -> Nullable<Text>,
    }
}

diesel::table! {
    pages (id) {
        id -> Nullable<Integer>,
        parent_id -> Nullable<Integer>,
        title -> Text,
        slug -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(admins, page_revision, pages,);
