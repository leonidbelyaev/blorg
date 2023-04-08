#[macro_use] extern crate slugify;
extern crate rocket;
use rocket::{launch, routes};
use rocket_dyn_templates::{ Template };
use slab_tree::tree::Tree;
use crate::views::establish_connection;
use diesel::{prelude::*};

mod schema;
mod models;
mod views;

#[launch]
fn rocket() -> _ {

        init_with_defaults();

    rocket::build()
        .mount("/", routes![views::pages::get_page])
        .mount("/", routes![views::pages::create_child_page])
        .mount("/", routes![views::pages::create_child_page_form])
        .mount("/", routes![views::pages::edit_page_form])
        .mount("/", routes![views::pages::edit_page])
        .mount("/", routes![views::pages::delete_page])

        .mount("/", routes![views::admins::authenticate_form])
        .mount("/", routes![views::admins::authenticate])

        .mount("/api/", routes![views::pages::put_page_id])
        .mount("/api/", routes![views::pages::create_page_id])
        .mount("/", routes![views::files])
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
                        html_content: "default root.".to_string()
                };
                diesel::insert_into(pages::table).values(&default_root).execute(connection).unwrap();
        }
}
