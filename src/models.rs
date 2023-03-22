use crate::schema::pages;
use diesel::{prelude::*};
use serde::{Serialize, Deserialize};

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(primary_key(id))]
#[diesel(table_name = pages)]
pub struct Page {
    pub id: Option<i32>,
    pub parent_id: Option<i32>,
    pub title: String,
    pub slug: String,
    pub html_content: String,
}
