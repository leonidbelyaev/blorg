#[macro_use]
extern crate slugify;
extern crate rocket;
use chrono::prelude::*;
use diesel::sql_types::{Integer, Text};
use diesel::{prelude::*, sql_query};
use pulldown_cmark::{html, Options, Parser};
use rocket::fairing::AdHoc;
use rocket::{launch, routes};
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::{database, diesel};
use slab_tree::tree::Tree;

mod models;
mod schema;
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
        .mount("/", routes![views::pages::upload_image])
        .mount("/", routes![views::pages::upload_image_form])
        .mount("/", routes![views::pages::search_pages])
        .mount("/", routes![views::admins::authenticate_form])
        .mount("/", routes![views::admins::authenticate])
        .mount("/", routes![views::admins::deauth])
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
                init_with_defaults(db, memdb).await;
            })
        }))
}

async fn init_with_defaults(connection: PersistDatabase, memory_connection: MemoryDatabase) {
    use self::models::Page;

    use self::schema::pages::dsl::*;

    let page_count: i64 = connection
        .run(move |c| pages.count().get_result(c).unwrap())
        .await;

    if page_count == 0 {
        let default_root = Page {
            id: None,
            parent_id: None,
            title: "".to_string(),
            slug: "".to_string(),
            create_time: Utc::now().format("%Y-%m-%d").to_string(),
            update_time: None,
            sidebar_html_content: "default root.".to_string(),
            sidebar_markdown_content: "default root.".to_string(),
            html_content: "default root.".to_string(),
            markdown_content: "default root.".to_string(),
        };
        connection
            .run(move |c| {
                diesel::insert_into(pages)
                    .values(&default_root)
                    .execute(c)
                    .unwrap();
            })
            .await;
    }

    memory_connection.run(move |c| {
         let query = sql_query(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS search USING FTS5(id, title, markdown_content, sidebar_markdown_content)
                "#
        ); // HACK we do this here because diesel does not support such sqlite virtual tables, which by definition have no explicit primary key.
        query.execute(c).expect("Database error");
        }
        ).await;

    let all_pages = connection
        .run(move |c| pages.load::<Page>(c).expect("Database error"))
        .await;

    for page in all_pages {
        memory_connection.run(
                        move |c| {
                                let query = sql_query(
                                        r#"
                                        INSERT INTO search (id, title, markdown_content, sidebar_markdown_content) VALUES (?, ?, ?, ?)
                                        "#
                                );
                                let binding = query.bind::<Integer, _>(page.id.unwrap())
                                        .bind::<Text, _>(page.title)
                                        .bind::<Text, _>(page.markdown_content)
                                        .bind::<Text, _>(page.sidebar_markdown_content);
                                binding.execute(c).expect("Database error");
                        }
                ).await;
    }
}
