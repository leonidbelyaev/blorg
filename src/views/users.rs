use diesel::sql_types::{Text, Integer};
use rocket::fs::NamedFile;
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

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(FromForm, Deserialize)]
struct UserInfo {
    username: String,
    password: String
}

#[post("/users/create", format="json", data="<user_info>")]
fn create(user_info: Json<UserInfo>)
  -> Result<Created<Json<User>>> {
    let connection = &mut establish_connection();
    let new_user = User
        {
            id: None,
            username: user_info.username.clone(),
            password_hash: hash_password(&user_info.password.clone()),
            is_admin: false,
        };
    diesel::insert_into(self::schema::users::dsl::users).values(&new_user).expect("Error saving new user");

    Ok(Created::new("/").body(Json(new_user))) // TODO this is insecure
}

fn hash_password(password: &String) -> String {
    let mut hasher = Sha3::sha3_256();
    hasher.input_str(password);
    hasher.result_str()
}
