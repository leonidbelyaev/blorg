use self::models::PageRevision;
use diesel::sql_types::{BigInt, Integer, Text};

use rocket::{http::CookieJar, response::Redirect};

extern crate diesel;
extern crate rocket;
use crate::{
    models::{self, AuthenticatedAdmin, SearchResult},
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

#[get("/search/pages?<query>")]
pub async fn search_pages(
    query: String,
    memory_connection: MemoryDatabase,
    _connection: PersistDatabase,
) -> Template {
    let results = SearchResult::run_search(&memory_connection, query.clone()).await;

    Template::render(
        "search_results",
        context! {search_results: results, search_term: query},
    )
}
