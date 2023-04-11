use crate::schema::pages;
use crate::schema::admins;
use diesel::{prelude::*};
use serde::{Serialize, Deserialize};
use diesel::sql_types::{Nullable, Integer, Text, Bool, Binary};

use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{self, Request, FromRequest};

pub struct AuthenticatedAdmin {
    id: i32
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedAdmin {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request.cookies()
            .get_private("user_id")
            .and_then(|c| c.value().parse().ok())
            .map(|id| AuthenticatedAdmin{id})
            .or_forward(())
    }
}

#[derive(Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(id))]
#[diesel(table_name = pages)]
pub struct Page {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub parent_id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub slug: String,
    #[diesel(sql_type = Text)]
    pub create_time: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub update_time: Option<String>,
    #[diesel(sql_type = Text)]
    pub html_content: String,
    #[diesel(sql_type = Text)]
    pub markdown_content: String,
    #[diesel(sql_type = Text)]
    pub sidebar_html_content: String,
    #[diesel(sql_type = Text)]
    pub sidebar_markdown_content: String,
}

#[derive(Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(id))]
#[diesel(table_name = admins)]
pub struct Admin {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = Text)]
    pub password_hash: String,
}
