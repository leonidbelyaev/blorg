pub mod pages;
pub mod users;
use std::env;
use diesel::{prelude::*};
use diesel::sqlite::SqliteConnection;
use dotenvy::dotenv;
use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};
use rocket::{get, post, put};

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[get("/<file..>")] // HACK
pub async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}
