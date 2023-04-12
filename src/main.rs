#[macro_use] extern crate slugify;
extern crate rocket;
use rocket::{launch, routes};
use rocket_dyn_templates::{ Template };
use slab_tree::tree::Tree;
use crate::views::establish_connection;
use pulldown_cmark::{Parser, Options, html};
use chrono::prelude::*;
use diesel::{prelude::*, sql_query};

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
    use self::models::Page;
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
        query.execute(connection).expect("Database error");
        let query = sql_query(
                r#"
                INSERT INTO search (id, title, markdown_content, sidebar_markdown_content)
                SELECT id, title, markdown_content, sidebar_markdown_content FROM pages
                "# // TODO load with redundancy - if ID matches, do not continue loading.
                // OR, use in-memory table, to avoid decoherence issues.
        );
        query.execute(connection).expect("Database error");
}
