#[macro_use] extern crate slugify;
extern crate rocket;
use rocket::{launch, routes};
use rocket_dyn_templates::{ Template };

mod schema;
mod models;
mod views;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![views::get_page])
        .mount("/", routes![views::put_page_path])
        .mount("/", routes![views::files])
        .mount("/", routes![views::create_page])
        .mount("/api/", routes![views::put_page_id])
        .mount("/api/", routes![views::create_page_id])
        .attach(Template::fairing())
}
