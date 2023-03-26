use diesel::sql_types::{Text, Integer};
use rocket::fs::NamedFile;
use rocket::local::blocking::Client;
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
use rocket::{get, post, put};
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
use crate::models::User;
use crate::views::establish_connection;
use crypto::sha3::Sha3;
use crypto::digest::Digest;
use rocket::http::{Cookie, CookieJar};
use rocket::form::Form;

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(FromForm, Deserialize)]
pub struct UserInfo {
    username: String,
    password: String
}

#[get("/users/create")]
pub fn create_form() -> Template {
    Template::render("create_user_form", context! {})
}

#[post("/users/create", data="<user_info>")] // TODO need to be a logged-in admin to add new users
pub fn create(user_info: Form<UserInfo>)
  -> Result<Created<Json<User>>> {
    use self::schema::users;
    let connection = &mut establish_connection();
    let count: i64 = *users::table.count().get_results(connection).expect("Database error").first().unwrap();
    let new_user = User
        {
            id: None,
            username: user_info.username.clone(),
            password_hash: hash_password(&user_info.password.clone()),
            is_admin: if count > 0 { true } else { false } // first user is the admin.
        };
    diesel::insert_into(self::schema::users::dsl::users).values(&new_user).execute(connection)
                                                        .expect("Error saving new user");

    Ok(Created::new("/").body(Json(new_user))) // TODO this is insecure
}

#[get("/users/login")]
pub fn login_form() -> Template {
    Template::render("login_user_form", context! {})
}

#[post("/users/login", data="<login_info>")]
pub fn login(login_info: Form<UserInfo>, jar: &CookieJar<'_>) -> Json<Option<i32>> {
    use self::schema::users::dsl::*;
    let connection = &mut establish_connection();
    let password_hashed = hash_password(&login_info.password);
    let binding = users.filter(username.eq(&login_info.username)).filter(password_hash.eq(password_hashed)).load::<User>(connection).expect("Database error");
    let maybe_user = binding.first();
    match maybe_user {
        Some(user) => {
            jar.add_private(Cookie::new("user_id", user.id.unwrap().to_string()));
            Json(user.id)
        },
        None => {
            Json(None)
        }
    }
}

// #[post("/users/logout", format="json")]
// pub fn logout(mut cookies: Cookies) -> () {
//     cookies.remove_private(Cookie::named("user_id"));
// }

fn hash_password(password: &String) -> String {
    let mut hasher = Sha3::sha3_256();
    hasher.input_str(password);
    hasher.result_str()
}
