pub mod admins;
pub mod pages;

use diesel::{prelude::*, sqlite::SqliteConnection};
use dotenvy::dotenv;
use rocket::{fs::NamedFile, get, http::CookieJar, post, put};
use std::{
    env,
    path::{Path, PathBuf},
};

// pub fn establish_connection() -> SqliteConnection {
//     dotenv().ok();
//     let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
//     SqliteConnection::establish(&database_url)
//         .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
// }

// pub fn establish_memory_connection() -> SqliteConnection {
//     SqliteConnection::establish(":memory:")
//         .unwrap_or_else(|_| panic!("Error connecting to in-memory DB."))
// }

#[get("/<file..>")] // HACK
pub async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}

// pub fn is_logged_in(jar: &CookieJar<'_>) -> bool {
//     match jar.get_private("user_id") {
//         Some(_) => true,
//         None => false
//     }
// }
