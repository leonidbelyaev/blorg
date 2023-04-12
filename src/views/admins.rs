use diesel::sql_types::{Text, Integer};
use rocket::fs::NamedFile;
use rocket::local::blocking::Client;
use rocket::response::Redirect;
use slugify::slugify;
use rocket::form::FromForm;
extern crate diesel;
extern crate rocket;
use diesel::sqlite::SqliteConnection;
use diesel::{prelude::*, sql_query};
use dotenvy::dotenv;
use pandoc::{PandocOutput, PandocOption};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{get, post, put, Response, Either};
use crate::models;
use crate::schema;
use rocket_dyn_templates::{context, Template};
use std::collections::HashMap;
use std::env;
use std::path::{PathBuf, Path};
use diesel::sql_types::{Nullable};
use diesel::{prelude::*};
use slab_tree::*;
use models::Page;
use crate::models::Admin;
use crate::views::establish_connection;
use crypto::sha3::Sha3;
use crypto::digest::Digest;
use rocket::http::{Cookie, CookieJar};
use rocket::form::Form;
use rocket::uri;

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(FromForm, Deserialize)]
pub struct AdminInfo {
    username: String,
    password: String
}

#[get("/admins/authenticate")]
pub fn authenticate_form() -> Template {
    use self::schema::admins::dsl::*;
    let connection = &mut establish_connection();

    let count: i64 = admins.count().get_result(connection).unwrap();



    Template::render("login_admin_form", context! {is_error: false, is_empty: count == 0})
}

#[post("/admins/authenticate", data="<admin_info>")]
pub fn authenticate(admin_info: Form<AdminInfo>, jar: &CookieJar<'_>) -> Either<Template, Redirect> {
    use self::schema::admins::dsl::*;

    let connection = &mut establish_connection();
    let password_hashed = hash_password(&admin_info.password);


    let binding = admins.filter(username.eq(&admin_info.username)).filter(password_hash.eq(password_hashed.clone())).load::<Admin>(connection).expect("Database error");
    let maybe_user = binding.first();
    match maybe_user {
        Some(user) => {
            jar.add_private(Cookie::new("user_id", user.id.unwrap().to_string()));
        },
        None => {
            let count: i64 = admins.count().get_result(connection).unwrap();

            if count == 0 {
                let new_user = Admin {
                    id: Some(1),
                    username: admin_info.username.clone(),
                    password_hash: password_hashed,
                };
                diesel::insert_into(admins).values(&new_user).execute(connection);
                jar.add_private(Cookie::new("user_id", new_user.id.unwrap().to_string()));
            } else {
                // nothing
                return Either::Left(Template::render("login_admin_form", context!{is_error: true, is_empty: false}));
            }
        }
    }
    Either::Right(Redirect::to(uri!(crate::views::pages::get_page(""))))
}

#[get("/admins/deauth")]
pub fn deauth(cookies: &CookieJar<'_>) -> () {
    cookies.remove_private(Cookie::named("user_id"));
}

fn hash_password(password: &String) -> String {
    let mut hasher = Sha3::sha3_256();
    hasher.input_str(password);
    hasher.result_str()
}
