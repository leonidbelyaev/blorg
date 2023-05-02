use crate::schema::admins;
use crate::schema::pages;
use crate::views::pages::PageInfo;
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{Binary, Bool, Integer, Nullable, Text};
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest, Request};
use serde::{Deserialize, Serialize};
use slugify::slugify;

use pulldown_cmark::{html, Options, Parser};

pub struct AuthenticatedAdmin {
    id: i32,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedAdmin {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request
            .cookies()
            .get_private("user_id")
            .and_then(|c| c.value().parse().ok())
            .map(|id| AuthenticatedAdmin { id })
            .or_forward(())
    }
}

fn md2html(md: String, options: Options) -> String {
    let parser = Parser::new_ext(&md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[derive(QueryableByName, Debug, Serialize)]
struct IntegerContainer {
    #[diesel(sql_type = Nullable<Integer>)]
    int: Option<i32>,
}

#[derive(
    Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone,
)]
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
    pub fn create_and_insert(parent_id: Option<i32>, page_info: PageInfo, parser_options:Options) -> () {
	let page = Page {
	    id: None,
	    parent_id: parent_id,
	    title: page_info.title.clone(),
	    slug: page_info.slug.clone(),
	}

	connection
	    .run(move |c| {
		diesel::insert_into(self::schema::pages::dsl::pages)
		    .values(page)
		    .execute(c)
		    .expect("Error saving new page");
	    }).await;

	// HACK: We do this because diesel does not support RETURNING for Sqlite Backend
    let page_id: Option<i32> = connection
        .run(move |c| {
            let query = sql_query("SELECT last_insert_rowid() AS int");
            let binding = query.load::<IntegerContainer>(c).expect("Database error");
            binding.first().expect("Database error").int
        })
        .await;

	let page_revision = PageRevision {
	    id: None,
	    page_id: page_id,
	    iso_8601_time: Utc::now().format("%Y-%m-%d").to_string(),
	    unix_time: Utc::now().timestamp(),
	    html_content: md2html(page_info.markdown_content.clone(), parser_options),
	    markdown_content: page_info.markdown_content.clone(),
	    sidebar_html_content: md2html(page_info.sidebar_markdown_content.clone(), parser_options),
	    sidebar_markdown_content: page_info.sidebar_markdown_content.clone(),
	}

	connection
	    .run(move |c| {
		diesel::insert_into(self::schema::pages::dsl::page_revision)
		    .values(page_revision)
		    .execute(c)
		    .expect("Error saving page revision");
	    }).await;

    }

    pub fn new(parent_id: Option<i32>, page_info: PageInfo, parser_options: Options) -> Self {
        Page {
            id: None,
            parent_id: parent_id,
            title: page_info.title.clone(),
            slug: page_info.slug.clone(),
            create_time: Utc::now().format("%Y-%m-%d").to_string(),
            update_time: Some(Utc::now().format("%Y-%m-%d").to_string()),
            html_content: md2html(page_info.markdown_content.clone(), parser_options),
            markdown_content: page_info.markdown_content.clone(),
            sidebar_html_content: md2html(
                page_info.sidebar_markdown_content.clone(),
                parser_options,
            ),
            sidebar_markdown_content: page_info.sidebar_markdown_content.clone(),
        }
    }

    pub fn edit(page: Page, new_page_info: PageInfo, parser_options: Options) -> Self {
        Page {
            id: page.id,
            parent_id: page.parent_id,
            title: new_page_info.title.clone(),
            slug: new_page_info.slug.clone(),
            create_time: page.create_time,
            update_time: Some(Utc::now().format("%Y-%m-%d").to_string()),
            html_content: md2html(new_page_info.markdown_content.clone(), parser_options),
            markdown_content: new_page_info.markdown_content.clone(),
            sidebar_html_content: md2html(
                new_page_info.sidebar_markdown_content.clone(),
                parser_options,
            ),
            sidebar_markdown_content: new_page_info.sidebar_markdown_content.clone(),
        }
    }
}

#[derive(
    Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone,
)]
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
