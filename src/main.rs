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
        .mount("/", routes![views::pages::get_page])
        .mount("/", routes![views::pages::put_page_path])
        .mount("/", routes![views::pages::create_page])

        .mount("/", routes![views::users::login_form])
        .mount("/", routes![views::users::login])

        .mount("/api/", routes![views::pages::put_page_id])
        .mount("/api/", routes![views::pages::create_page_id])
        .mount("/", routes![views::files])
        .attach(Template::fairing())
}
