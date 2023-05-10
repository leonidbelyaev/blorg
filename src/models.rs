use crate::schema::admin;
use crate::schema::page;
use crate::schema::page_revision;
use crate::views::pages::PageInfo;
use crate::{ManagedState, MemoryDatabase, PersistDatabase};
use chrono::Utc;
use diesel::sql_types::{BigInt, Binary, Bool, Integer, Nullable, Text};
use diesel::{prelude::*, sql_query};
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest, Request};
use rocket::State;
use serde::{Deserialize, Serialize};
use slugify::slugify;
use std::path::PathBuf;

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

#[derive(Queryable, QueryableByName, Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(id))]
#[diesel(table_name = page_revision)]
pub struct PageRevision {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub page_id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub iso_time: String,
    #[diesel(sql_type = Integer)]
    pub unix_time: i32, // HACK Diesel uses i32 for the default sqlite integer, so this fails in 2038
    #[diesel(sql_type = Text)]
    pub html_content: String,
    #[diesel(sql_type = Text)]
    pub markdown_content: String,
    #[diesel(sql_type = Text)]
    pub sidebar_html_content: String,
    #[diesel(sql_type = Text)]
    pub sidebar_markdown_content: String,
}

#[derive(
    Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = page)]
pub struct Page {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub parent_id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub slug: String,
}

impl Page {
    pub async fn populate_default_root(
        connection: &PersistDatabase,
        memory_connection: &MemoryDatabase,
        state: &State<ManagedState>,
    ) -> () {
        let default_root_info = PageInfo {
            title: "Root".to_string(),
            slug: "".to_string(),
            markdown_content: "Default root".to_string(),
            sidebar_markdown_content: "".to_string(),
        };

        let empty = PathBuf::new();

        Self::create_child_and_insert(
            None,
            empty,
            default_root_info,
            state,
            connection,
            memory_connection,
        )
        .await;
        // TODO figure out if passing a blank path to create_and_insert will work okay
    }

    pub async fn create_child_and_insert(
        parent_id: Option<i32>,
        parent_path: PathBuf,
        page_info: PageInfo,
        state: &State<ManagedState>,
        connection: &PersistDatabase,
        memory_connection: &MemoryDatabase,
    ) -> () {
        let page = Page {
            id: None,
            parent_id: parent_id,
            title: page_info.title.clone(),
            slug: page_info.slug.clone(),
        };

        let mut page_path = parent_path.clone();
        page_path.push(page_info.slug.clone());

        connection
            .run(move |c| {
                diesel::insert_into(crate::schema::page::dsl::page)
                    .values(page)
                    .execute(c)
                    .expect("Error saving new page");
            })
            .await;

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
            iso_time: Utc::now().format("%Y-%m-%d").to_string(),
            unix_time: Utc::now().timestamp() as i32,
            html_content: md2html(page_info.markdown_content.clone(), state.parser_options),
            markdown_content: page_info.markdown_content.clone(),
            sidebar_html_content: md2html(
                page_info.sidebar_markdown_content.clone(),
                state.parser_options,
            ),
            sidebar_markdown_content: page_info.sidebar_markdown_content.clone(),
        };

        connection
            .run(move |c| {
                diesel::insert_into(crate::schema::page_revision::dsl::page_revision)
                    .values(page_revision)
                    .execute(c)
                    .expect("Error saving page revision");
            })
            .await;

        memory_connection
	    .run(move |c| {
            let query = sql_query(
                    r#"
                    INSERT INTO search (id, path, title, markdown_content, sidebar_markdown_content) VALUES (?, ?, ?, ?, ?)
                    "#
            );
            let binding = query.bind::<Nullable<Integer>, _>(page_id)
                    .bind::<Text, _>(page_path.display().to_string())
                    .bind::<Text, _>(page_info.title.clone())
                    .bind::<Text, _>(page_info.markdown_content.clone())
                    .bind::<Text, _>(page_info.sidebar_markdown_content.clone());
            binding.execute(c).expect("Database error");
	    }).await;
    }

    pub async fn edit_and_update(
        edit_path: PathBuf,
        edit_page_info: PageInfo,
        connection: &PersistDatabase,
        memory_connection: &MemoryDatabase,
        state: &State<ManagedState>,
    ) {
        let to_edit = Self::from_path(&edit_path, connection).await;

        let edited = Page {
            id: to_edit.id,
            parent_id: to_edit.parent_id,
            title: edit_page_info.title.clone(),
            slug: edit_page_info.slug.clone(),
        };

        let new_revision = PageRevision {
            id: None,
            page_id: to_edit.id,
            iso_time: Utc::now().to_string(),
            unix_time: Utc::now().timestamp() as i32,
            html_content: md2html(
                edit_page_info.markdown_content.clone(),
                state.parser_options,
            ),
            markdown_content: edit_page_info.markdown_content.clone(),
            sidebar_html_content: md2html(
                edit_page_info.sidebar_markdown_content.clone(),
                state.parser_options,
            ),
            sidebar_markdown_content: edit_page_info.sidebar_markdown_content.clone(),
        };

        connection
            .run(move |c| {
                use crate::schema::page::dsl::*;
                diesel::update(page)
                    .filter(id.eq(to_edit.id))
                    .set(&edited)
                    .execute(c)
                    .expect("Failed to update page from path")
            })
            .await;

        connection
            .run(move |c| {
                diesel::insert_into(crate::schema::page_revision::dsl::page_revision)
                    .values(new_revision)
                    .execute(c)
                    .expect("Error saving page revision");
            })
            .await;

        let cloned_path = edit_path.clone();

        memory_connection
        .run(move |c| {
	    let query = sql_query("UPDATE search SET path=?, title=?, markdown_content=?, sidebar_markdown_content=? WHERE id = ?");
	    query
        .bind::<Text, _>(edit_path.display().to_string())
        .bind::<Text, _>(edit_page_info.title.clone())
        .bind::<Text, _>(edit_page_info.markdown_content.clone())
        .bind::<Text, _>(edit_page_info.sidebar_markdown_content.clone())
        .bind::<Nullable<Integer>, _>(to_edit.id)
        .execute(c).expect("Database error");
	}).await;
    }

    async fn from_path(path: &PathBuf, connection: &PersistDatabase) -> Self {
        use crate::schema::page::dsl::*;

        let query = sql_query(
            r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path
             FROM pages
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || slug
             FROM pages p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT * FROM pages WHERE id = (
           SELECT id FROM CTE WHERE path = ?
           );
"#,
        );
        let path = path.to_str().unwrap().to_string();
        let path_spec = if path != "" {
            format!("/{}", path)
        } else {
            path
        };
        println!("path spec is {:?}", path_spec);
        let binding = connection
            .run(move |c| {
                query
                    .bind::<Text, _>(path_spec)
                    .load::<Page>(c)
                    .expect("Database error finding page")
            })
            .await;
        let child = binding.first().expect("No such page found");
        println!("Child is: {:?}", child);
        child.clone()
    }
}

#[derive(
    Queryable, QueryableByName, Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone,
)]
#[diesel(primary_key(id))]
#[diesel(table_name = admin)]
pub struct Admin {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = Text)]
    pub password_hash: String,
}
