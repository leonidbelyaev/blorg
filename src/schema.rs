// @generated automatically by Diesel CLI.

diesel::table! {
    admin (id) {
        id -> Nullable<Integer>,
        username -> Text,
        password_hash -> Text,
    }
}

diesel::table! {
    page (id) {
        id -> Nullable<Integer>,
        parent_id -> Nullable<Integer>,
        title -> Text,
        slug -> Text,
    }
}

diesel::table! {
    page_revision (id) {
        id -> Nullable<Integer>,
        page_id -> Nullable<Integer>,
        iso_time -> Text,
        unix_time -> Integer,
        html_content -> Text,
        markdown_content -> Text,
        sidebar_html_content -> Text,
        sidebar_markdown_content -> Text,
    }
}

diesel::joinable!(page_revision -> page (page_id));

diesel::allow_tables_to_appear_in_same_query!(
    admin,
    page,
    page_revision,
);
