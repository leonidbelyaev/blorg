use self::models::PageRevision;
use diesel::sql_types::{BigInt, Integer, Text};

use rocket::{http::CookieJar, response::Redirect};

extern crate diesel;
extern crate rocket;
use crate::{
    models::{self, AuthenticatedAdmin},
    schema, ManagedState, MemoryDatabase, PersistDatabase,
};

use diesel::{prelude::*, row::Row, sql_query, sql_types::Nullable};

use models::Page;
use pandoc::{PandocOption, PandocOutput};

use rocket::{
    form::Form,
    get, post,
    response::Debug,
    serde::{Deserialize, Serialize},
    uri, FromForm, State,
};
use rocket_dyn_templates::{context, Template};
use slab_tree::*;
use std::{collections::HashMap, path::PathBuf};

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(QueryableByName, Debug, Serialize)]
struct SearchResult {
    #[diesel(sql_type = Nullable<Integer>)]
    id: Option<i32>,
    #[diesel(sql_type = Text)]
    path: String,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    markdown_content: String,
    #[diesel(sql_type = Text)]
    sidebar_markdown_content: String,
}

#[get("/search/pages?<query>")]
pub async fn search_pages(
    query: String,
    memory_connection: MemoryDatabase,
    _connection: PersistDatabase,
) -> Template {
    let search_results = sql_query(
        r#"SELECT id, path, snippet(search, 2, '<span class="highlight">', '</span>', '...', 64) AS "title", snippet(search, 3, '<span class="highlight">', '</span>', '...', 64) AS "markdown_content", snippet(search, 4, '<span class="highlight">', '</span>', '...', 64) AS "sidebar_markdown_content" FROM search WHERE search MATCH '{title markdown_content sidebar_markdown_content}: ' || ? "#,
    );

    let qclone = query.clone();

    let binding = memory_connection
        .run(move |c| {
            search_results
                .bind::<Text, _>(qclone)
                .load::<SearchResult>(c)
                .expect("Database error")
        })
        .await;

    Template::render(
        "search_results",
        context! {search_results: binding, search_term: query},
    )
}
