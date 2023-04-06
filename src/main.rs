#[macro_use] extern crate slugify;
extern crate rocket;
use rocket::{launch, routes};
use rocket_dyn_templates::{ Template };
use slab_tree::tree::Tree;
use crate::views::establish_connection;

mod schema;
mod models;
mod views;

#[launch]
fn rocket() -> _ {
    let connection = &mut establish_connection();

    rocket::build()
        .mount("/", routes![views::pages::get_page])
        .mount("/", routes![views::pages::put_page_path])
        .mount("/", routes![views::pages::create_page])

        .mount("/", routes![views::admins::authenticate_form])
        .mount("/", routes![views::admins::authenticate])

        .mount("/api/", routes![views::pages::put_page_id])
        .mount("/api/", routes![views::pages::create_page_id])
        .mount("/", routes![views::files])
        .attach(Template::fairing())
}
