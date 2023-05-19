pub mod admins;
pub mod pages;
pub mod search;

use diesel::prelude::*;

use rocket::{fs::NamedFile, get, response::Redirect, uri};
use std::path::{Path, PathBuf};

#[get("/<file..>")] // HACK
pub async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}

#[get("/")]
pub async fn page_redirect() -> Redirect {
    Redirect::to(uri!(crate::views::pages::get_page(
        PathBuf::from(""),
        None::<usize>
    )))
}
