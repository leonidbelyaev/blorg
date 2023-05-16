use self::models::PageRevision;
use crate::util::page2raw;
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

#[derive(Serialize, Deserialize, FromForm, Clone)]
pub struct PageInfo {
    pub title: String,
    pub slug: String,
    pub markdown_content: String,
    pub sidebar_markdown_content: String,
}

#[post("/pages/<path..>", data = "<child_page>")]
pub async fn create_child_page(
    state: &State<ManagedState>,
    child_page: Form<PageInfo>,
    path: PathBuf,
    _admin: AuthenticatedAdmin,
    connection: PersistDatabase,
    memory_connection: MemoryDatabase,
) -> Redirect {
    use models::Page;

    let parent = Page::from_path(&path, &connection).await;

    let child_page = child_page.into_inner();

    let mut child_path = path.clone();
    child_path.push(&child_page.slug);

    Page::create_child_and_insert(
        parent.id,
        path.clone(),
        child_page,
        state,
        &connection,
        &memory_connection,
    )
    .await;

    Redirect::to(uri!(get_page(child_path, None::<usize>)))
}

#[get("/create/pages/<path..>")]
pub fn create_child_page_form(path: PathBuf, _admin: AuthenticatedAdmin) -> Template {
    Template::render("create_child_page_form", context! {path: path})
}

#[get("/download/pages/<path..>?<revision>")]
pub async fn download_page_markdown(
    path: PathBuf,
    revision: Option<usize>,
    connection: PersistDatabase,
) -> String {
    let page = Page::from_path(&path, &connection).await;
    let nth_rev = PageRevision::get_nth_revision(&connection, page.id.unwrap(), revision).await;

    page2raw(
        &page.title.clone(),
        &nth_rev.markdown_content.clone(),
        &nth_rev.sidebar_markdown_content.clone(),
    )
}

#[get("/pages/<path..>?<revision>")]
pub async fn get_page(
    path: PathBuf,
    revision: Option<usize>,
    jar: &CookieJar<'_>,
    connection: PersistDatabase,
) -> Template {
    use self::models::PageRevision;

    let page = Page::from_path(&path, &connection).await;

    let nav_element = Page::build_nav_element(&connection, &path).await;

    let is_user = match jar.get_private("user_id") {
        Some(_other_id) => true,
        None => false,
    };

    let all_revisions = connection
        .run(move |c| {
            use crate::schema::page_revision::dsl::*;
            page_revision
                .filter(crate::schema::page_revision::dsl::page_id.eq(page.id))
                .order(unix_time)
                .load::<PageRevision>(c)
                .expect("Database error finding page revision")
        })
        .await;

    let is_latest = PageRevision::is_latest(&connection, revision, page.id.unwrap()).await;

    let nth_rev = PageRevision::get_nth_revision(&connection, page.id.unwrap(), revision).await;

    Template::render(
        "page",
        context! {page: &page, page_revision: nth_rev, all_revisions: all_revisions, nav: &nav_element, is_user: is_user, path: path, is_latest: is_latest, revision_number: revision},
    )
}

#[post("/edit/pages/<path..>", data = "<new_page>")]
pub async fn edit_page(
    state: &State<ManagedState>,
    new_page: Form<PageInfo>,
    path: PathBuf,
    _admin: AuthenticatedAdmin,
    connection: PersistDatabase,
    memory_connection: MemoryDatabase,
) -> Redirect {
    Page::edit_and_update(
        path.clone(),
        new_page.into_inner(),
        &connection,
        &memory_connection,
        state,
    )
    .await;

    Redirect::to(uri!(get_page(path, None::<usize>)))
}

#[get("/edit/pages/<path..>")]
pub async fn edit_page_form(
    _admin: AuthenticatedAdmin,
    path: PathBuf,
    connection: PersistDatabase,
) -> Template {
    let page = Page::from_path(&path, &connection).await;
    let latest_revision = PageRevision::get_nth_revision(&connection, page.id.unwrap(), None).await;

    Template::render(
        "edit_page_form",
        context! {page: page, latest_revision: latest_revision, path: path},
    )
}

#[get("/delete/pages/<path..>")]
pub async fn delete_page(
    path: PathBuf,
    _admin: AuthenticatedAdmin,
    connection: PersistDatabase,
    memory_connection: MemoryDatabase,
) -> Redirect {
    let spath = format!("/{}", path.to_str().unwrap().to_string());
    if spath == "/" {
        panic!()
    }
    let to_delete = Page::from_path(&path, &connection)
        .await
        .delete(&connection, &memory_connection);

    let mut path = path.clone();
    path.pop();
    Redirect::to(uri!(get_page(path, None::<usize>)))
}
