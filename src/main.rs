#[macro_use] extern crate slugify;
extern crate rocket;
use rocket::{launch, routes};
use rocket_dyn_templates::{ Template };
use slab_tree::tree::Tree;
use crate::views::establish_connection;
use diesel::{prelude::*};
use pulldown_cmark::{Parser, Options, html};

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
                        title: "Root".to_string(),
                        slug: "".to_string(),
                        html_content: "default root.".to_string(),
                        markdown_content: "default root.".to_string()
                };
                diesel::insert_into(pages::table).values(&default_root).execute(connection).unwrap();
        }
}
