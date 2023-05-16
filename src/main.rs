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
mod views;

pub struct ManagedState {
    parser_options: Options,
}

#[derive(Queryable, QueryableByName, Serialize, Deserialize, Debug, Clone)]
pub struct SearchablePage {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub path: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub markdown_content: String,
    #[diesel(sql_type = Text)]
    pub sidebar_markdown_content: String,
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
    use self::models::Page;

    use self::schema::page::dsl::*;

    memory_connection.run(move |c| {
         let query = sql_query(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS search USING FTS5(id, path, title, markdown_content, sidebar_markdown_content)
                "#
        ); // HACK we do this here because diesel does not support such sqlite virtual tables, which by definition have no explicit primary key.
        query.execute(c).expect("Database error");
        }
        ).await;

    let page_count: i64 = connection
        .run(move |c| page.count().get_result(c).unwrap())
        .await;

    if page_count == 0 {
        Page::populate_default_root(connection, memory_connection, state).await;
    }

    // TODO this query must be changed for the page revision model
    // Modify to use multiple select
    let query = sql_query(
        r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path, title--, markdown_content, sidebar_markdown_content
             FROM page
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || slug, p.title--, p.markdown_content, p.sidebar_markdown_content
             FROM page p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT * FROM CTE
           LEFT JOIN page_revision
           ON CTE.id = page_revision.page_id;
"#,
    );

    let binding = connection
        .run(move |c| query.load::<SearchablePage>(c).expect("Database error"))
        .await;

    for searchable_page in binding {
        memory_connection.run(
                        move |c| {
                                let query = sql_query(
                                        r#"
                                        INSERT INTO search (id, path, title, markdown_content, sidebar_markdown_content) VALUES (?, ?, ?, ?, ?)
                                        "#
                                );
                                let binding = query.bind::<Integer, _>(searchable_page.id.unwrap())
                                    .bind::<Text, _>(searchable_page.path)
                                        .bind::<Text, _>(searchable_page.title)
                                        .bind::<Text, _>(searchable_page.markdown_content)
                                        .bind::<Text, _>(searchable_page.sidebar_markdown_content);
                                binding.execute(c).expect("Database error");
                        }
                ).await;
    }
}
