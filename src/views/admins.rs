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
use crate::{models, PersistDatabase};
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
pub async fn authenticate_form(connection: PersistDatabase) -> Template {
    use self::schema::admins::dsl::*;

    let count: i64 = connection.run( move |c| {
        admins.count().get_result(c).unwrap()
    }).await;

    Template::render("login_admin_form", context! {is_error: false, is_empty: count == 0})
}

#[post("/admins/authenticate", data="<admin_info>")]
pub async fn authenticate(admin_info: Form<AdminInfo>, jar: &CookieJar<'_>, connection: PersistDatabase) -> Either<Template, Redirect> {
     use self::schema::admins::dsl::*;

    let password_hashed = hash_password(&admin_info.password);

    let usrname = admin_info.username.clone();
    let pwhash = password_hashed.clone();

    let binding = connection.run(
        move |c|  {
            admins.filter(username.eq(usrname)).filter(password_hash.eq(pwhash)).load::<Admin>(c).expect("Database error")
        }).await;

    let maybe_user = binding.first();
    match maybe_user {
        Some(user) => {
            jar.add_private(Cookie::new("user_id", user.id.unwrap().to_string()));
        },
        None => {
            let count: i64 = connection.run(
                move |c| {
                    admins.count().get_result(c).unwrap()
                }
            ).await;
            // let count: i64 = admins.count().get_result(&mut connection).unwrap();

            if count == 0 {
                let new_user = Admin {
                    id: Some(1),
                    username: admin_info.username.clone(),
                    password_hash: password_hashed,
                };
                connection.run(
                    move |c| {
                        diesel::insert_into(admins).values(&new_user).execute(c)
                    }
                ).await;
                //diesel::insert_into(admins).values(&new_user).execute(&mut connection);
                jar.add_private(Cookie::new("user_id", Some(1).unwrap().to_string()));
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
