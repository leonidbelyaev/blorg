extern crate diesel;
extern crate rocket;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{get, post };
use crate::models;
use crate::schema;
use rocket_dyn_templates::{context, Template};
use std::env;

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[derive(Serialize, Deserialize)]
pub struct NewPage {
    parent_id: Option<i32>,
    title: String,
    org_content: String,
}

#[post("/pages", format = "json", data = "<page>")]
pub fn create_page(page: Json<NewPage>) -> Result<Created<Json<NewPage>>> {
    use self::schema::pages::dsl::*;
    use models::Page;
    let connection = &mut establish_connection();

    let new_page = Page {
        id: 1,
        parent_id: page.parent_id,
        title: page.title.to_string(),
        slug: page.title.to_string(), // TODO
        html_content: page.org_content.to_string(), // TODO
    };

    diesel::insert_into(self::schema::pages::dsl::pages)
        .values(&new_page)
        .execute(connection)
        .expect("Error saving new page");

    Ok(Created::new("/").body(page))
}

#[get("/pages")]
pub fn list() -> Template {
    use self::models::Page;
    let connection = &mut establish_connection();
    let results = self::schema::pages::dsl::pages
        .load::<Page>(connection)
        .expect("Error loading pages");
    Template::render("pages", context! {pages: &results, count: results.len()})
}
