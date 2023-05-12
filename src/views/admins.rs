use diesel::sql_types::{Integer, Text};
use rocket::form::FromForm;
use rocket::fs::NamedFile;
use rocket::local::blocking::Client;
use rocket::response::Redirect;
use slugify::slugify;
extern crate diesel;
extern crate rocket;
use crate::models::Admin;
use crate::schema;
use crate::{PersistDatabase};
use crypto::digest::Digest;
use crypto::sha3::Sha3;
use diesel::prelude::*;
use diesel::sql_types::Nullable;
use diesel::sqlite::SqliteConnection;
use diesel::{prelude::*, sql_query};
use dotenvy::dotenv;
use models::Page;
use pandoc::{PandocOption, PandocOutput};
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::{status::Created, Debug};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::uri;
use rocket::{get, post, put, Either, Response};
use rocket_dyn_templates::{context, Template};
use slab_tree::*;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use crate::models::{self, AuthenticatedAdmin};
use image::ImageFormat;
use image::RgbImage;
use image::{open, DynamicImage};
use image::imageops::colorops::dither;
use image::imageops::{BiLevel, ColorMap};
use image::io::Reader;
use tempdir::TempDir;
use rocket::fs::TempFile;


type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(FromForm, Deserialize)]
pub struct AdminInfo {
    username: String,
    password: String,
}

#[derive(FromForm)]
pub struct Upload<'f> {
    filename: String,
    image: TempFile<'f>,
}

#[get("/admins/authenticate")]
pub async fn authenticate_form(connection: PersistDatabase) -> Template {
    use self::schema::admin::dsl::*;

    let count: i64 = connection
        .run(move |c| admin.count().get_result(c).unwrap())
        .await;

    Template::render(
        "login_admin_form",
        context! {is_error: false, is_empty: count == 0},
    )
}

#[post("/admins/authenticate", data = "<admin_info>")]
pub async fn authenticate(
    admin_info: Form<AdminInfo>,
    jar: &CookieJar<'_>,
    connection: PersistDatabase,
) -> Either<Template, Redirect> {
    use self::schema::admin::dsl::*;

    let password_hashed = hash_password(&admin_info.password);

    let usrname = admin_info.username.clone();
    let pwhash = password_hashed.clone();

    let binding = connection
        .run(move |c| {
            admin
                .filter(username.eq(usrname))
                .filter(password_hash.eq(pwhash))
                .load::<Admin>(c)
                .expect("Database error")
        })
        .await;

    let maybe_user = binding.first();
    match maybe_user {
        Some(user) => {
            jar.add_private(Cookie::new("user_id", user.id.unwrap().to_string()));
        }
        None => {
            let count: i64 = connection
                .run(move |c| admin.count().get_result(c).unwrap())
                .await;
            // let count: i64 = admins.count().get_result(&mut connection).unwrap();

            if count == 0 {
                let new_user = Admin {
                    id: Some(1),
                    username: admin_info.username.clone(),
                    password_hash: password_hashed,
                };
                connection
                    .run(move |c| diesel::insert_into(admin).values(&new_user).execute(c))
                    .await;
                //diesel::insert_into(admins).values(&new_user).execute(&mut connection);
                jar.add_private(Cookie::new("user_id", Some(1).unwrap().to_string()));
            } else {
                // nothing
                return Either::Left(Template::render(
                    "login_admin_form",
                    context! {is_error: true, is_empty: false},
                ));
            }
        }
    }
    Either::Right(Redirect::to(uri!(crate::views::pages::get_page(
        "",
        None::<usize>
    ))))
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


#[post("/upload/image", data = "<form>")]
pub async fn upload_image(mut form: Form<Upload<'_>>) -> std::io::Result<()> {
    // hand is forced by https://github.com/SergioBenitez/Rocket/issues/2296 for now

    let tmp_dir = TempDir::new("blorg")?;
    let tmppath = tmp_dir.path().join(form.filename.clone());
    form.image.move_copy_to(&tmppath).await;

    let mut image = Reader::open(&tmppath)?.decode().unwrap().into_luma8();
    dither(&mut image, &BiLevel);

    let persist_path = format!("./static/img/runtime/{}", form.filename.clone());
    image.save(persist_path);

    Ok(())
}

#[get("/upload/image")]
pub fn upload_image_form(admin: AuthenticatedAdmin) -> Template {
    Template::render("upload_image_form", context! {})
}

/// Admin panel, exposing misc. admin-only functionality.
#[get("/admins/panel")]
pub fn admin_panel(admin: AuthenticatedAdmin) -> Template {
    let admin_url_spec = vec![("/upload/image", "Upload Image")];

    Template::render("url_list", context! {url_spec: admin_url_spec})
}
