use crate::schema::pages;
use crate::schema::admins;
use crate::schema::config;
use diesel::{prelude::*};
use serde::{Serialize, Deserialize};
use diesel::sql_types::{Nullable, Integer, Text, Bool, Blob};

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
    pub html_content: String,
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

#[derive(Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(id))]
#[diesel(table_name = config)]
pub struct Config {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub root_url: String,
    #[diesel(sql_type = Blob)]
    pub serialized_page_tree: Vec<u8>,
}
