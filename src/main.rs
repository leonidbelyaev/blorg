#[macro_use] extern crate slugify;
extern crate rocket;
use rocket::{launch, routes};
use rocket_dyn_templates::{ Template };
use slab_tree::tree::Tree;
use crate::views::{establish_connection, establish_memory_connection};
use pulldown_cmark::{Parser, Options, html};
use chrono::prelude::*;
use diesel::{prelude::*, sql_query};
use diesel::sql_types::{Text, Integer};

mod schema;
mod models;
mod views;

pub struct ManagedState {
        parser_options: Options
}

#[launch]
fn rocket() -> _ {

        init_with_defaults();

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
        .manage(ManagedState{parser_options: options})
        .attach(Template::fairing())
}

fn init_with_defaults() {
        let connection = &mut establish_connection();
        let memory_connection = &mut establish_memory_connection();
        use self::models::Page;

        use self::schema::pages::dsl::*;
        use self::schema::pages;

        let page_count = pages::table.count().get_result::<i64>(connection).unwrap();

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
                        markdown_content: "default root.".to_string()
                };
                diesel::insert_into(pages::table).values(&default_root).execute(connection).unwrap();
        }

        let query = sql_query(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS search USING FTS5(id, title, markdown_content, sidebar_markdown_content)
                "#
        ); // HACK we do this here because diesel does not support such sqlite virtual tables, which by definition have no explicit primary key.
        query.execute(memory_connection).expect("Database error");

        let all_pages = pages.load::<Page>(connection).expect("Database error");
        for page in all_pages {
                let query = sql_query(
                        r#"
                        INSERT INTO search (id, title, markdown_content, sidebar_markdown_content) VALUES (?, ?, ?, ?)
                        "#
                );
                let binding = query.bind::<Integer, _>(page.id.unwrap())
                        .bind::<Text, _>(page.title)
                        .bind::<Text, _>(page.markdown_content)
                        .bind::<Text, _>(page.sidebar_markdown_content);
                binding.execute(memory_connection).expect("Database error");
        }
}
