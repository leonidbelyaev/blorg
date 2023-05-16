#[macro_use]
extern crate slugify;
extern crate rocket;

use diesel::{
    prelude::*,
    sql_query,
    sql_types::{Integer, Nullable, Text},
};
use pulldown_cmark::Options;
use rocket::{fairing::AdHoc, launch, routes, State};
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::{database, diesel};
use serde::{Deserialize, Serialize};

mod models;
mod schema;
mod util;
mod views;

pub struct ManagedState {
    parser_options: Options,
}

#[database("persist_database")]
pub struct PersistDatabase(diesel::SqliteConnection);

#[database("memory_database")]
pub struct MemoryDatabase(diesel::SqliteConnection);

#[launch]
async fn rocket() -> _ {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    rocket::build()
        .mount("/", routes![views::pages::get_page])
        .mount("/", routes![views::pages::create_child_page])
        .mount("/", routes![views::pages::create_child_page_form])
        .mount("/", routes![views::pages::edit_page_form])
        .mount("/", routes![views::pages::edit_page])
        .mount("/", routes![views::pages::delete_page])
        .mount("/", routes![views::pages::download_page_markdown])
        .mount("/", routes![views::admins::upload_image])
        .mount("/", routes![views::admins::upload_image_form])
        .mount("/", routes![views::admins::admin_panel])
        .mount("/", routes![views::admins::authenticate_form])
        .mount("/", routes![views::admins::authenticate])
        .mount("/", routes![views::admins::deauth])
        .mount("/", routes![views::search::search_pages])
        .mount("/", routes![views::files])
        .manage(ManagedState {
            parser_options: options,
        })
        .attach(Template::fairing())
        .attach(PersistDatabase::fairing())
        .attach(MemoryDatabase::fairing())
        .attach(AdHoc::on_liftoff("Init Databases", |rocket| {
            Box::pin(async move {
                let db = PersistDatabase::get_one(rocket).await.unwrap();
                let memdb = MemoryDatabase::get_one(rocket).await.unwrap();
                let state = rocket.state::<ManagedState>().unwrap();
                init_with_defaults(&db, &memdb, state.into()).await;
            })
        }))
}

async fn init_with_defaults(
    connection: &PersistDatabase,
    memory_connection: &MemoryDatabase,
    state: &State<ManagedState>,
) {
    use self::models::{Page, SearchResult};

    use self::schema::page::dsl::*;

    SearchResult::init_memory_table(memory_connection).await;

    let page_count: i64 = connection
        .run(move |c| page.count().get_result(c).unwrap())
        .await;

    if page_count == 0 {
        Page::populate_default_root(connection, memory_connection, state).await;
    }

    SearchResult::populate_with_revisions(connection, memory_connection).await;
}
