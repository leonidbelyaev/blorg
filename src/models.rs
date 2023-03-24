use crate::schema::pages;
use diesel::{prelude::*};
use serde::{Serialize, Deserialize};
use diesel::sql_types::{Nullable, Integer, Text};

#[derive(Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug)]
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
