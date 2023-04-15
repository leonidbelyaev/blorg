use crate::schema::pages;
use crate::schema::admins;
use chrono::Utc;
use diesel::{prelude::*};
use pulldown_cmark::Options;
use serde::{Serialize, Deserialize};
use diesel::sql_types::{Nullable, Integer, Text, Bool, Binary};
use rocket::http::Status;
use crate::views::pages::PageInfo;
use rocket::outcome::IntoOutcome;
use rocket::request::{self, Request, FromRequest};
use slugify::slugify;

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

fn md2html(md: String, options: Options) -> String {
    let parser = Parser::new_ext(&md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
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

impl Page {
    pub fn new(parent_id: Option<i32>, page_info: PageInfo, parser_options: Options) -> Self {
        Page {
            id: None,
            parent_id: parent_id,
            title: page_info.title.clone(),
            slug: slugify!(&page_info.title.clone()),
            create_time: Utc::now().format("%Y-%m-%d").to_string(),
            update_time: Some(Utc::now().format("%Y-%m-%d").to_string()),
            html_content: md2html(page_info.markdown_content.clone(), parser_options),
            markdown_content: page_info.markdown_content.clone(),
            sidebar_html_content: md2html(page_info.sidebar_markdown_content.clone(), parser_options),
            sidebar_markdown_content: page_info.sidebar_markdown_content.clone()
        }
    }

    pub fn edit(page: Page, new_page_info: PageInfo, parser_options: Options) -> Self {
        Page {
            id: page.id,
            parent_id: page.parent_id,
            title: new_page_info.title.clone(),
            slug: slugify!(&new_page_info.title.clone()),
            create_time: page.create_time,
            update_time: Some(Utc::now().format("%Y-%m-%d").to_string()),
            html_content: md2html(new_page_info.markdown_content.clone(), parser_options),
            markdown_content: new_page_info.markdown_content.clone(),
            sidebar_html_content: md2html(new_page_info.sidebar_markdown_content.clone(), parser_options),
            sidebar_markdown_content: new_page_info.sidebar_markdown_content.clone()
        }
    }

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
