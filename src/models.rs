use crate::{
    schema::{admin, page, page_revision},
    util::md2html,
    views::pages::PageInfo,
    ManagedState, MemoryDatabase, PersistDatabase,
};
use chrono::Utc;
use diesel::{
    prelude::*,
    sql_query,
    sql_types::{Integer, Nullable, Text},
};
use rocket::{
    outcome::IntoOutcome,
    request::{self, FromRequest, Request},
    State,
};
use serde::{Deserialize, Serialize};
use slab_tree::*;
use std::collections::HashMap;

use std::path::PathBuf;

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

impl PageRevision {
    pub async fn get_nth_revision(
        connection: &PersistDatabase,
        target_page_id: i32,
        revision: Option<usize>,
    ) -> Self {
        let all_revisions = connection
            .run(move |c| {
                use crate::schema::page_revision::dsl::*;
                page_revision
                    .filter(crate::schema::page_revision::dsl::page_id.eq(target_page_id))
                    .order(unix_time)
                    .load::<PageRevision>(c)
                    .expect("Database error finding page revision")
            })
            .await;
        let to_view = match revision {
            Some(rev) => all_revisions
                .iter()
                .nth(rev)
                .expect("No such page revision found."),
            None => all_revisions.last().expect("No such page revision found."),
        };
        to_view.clone()
    }

    pub async fn delete_nth_revision(connection: &PersistDatabase, target_page_id: i32, revision: Option<usize>) {
	let nth = Self::get_nth_revision(connection, target_page_id, revision).await;
	nth.delete(connection).await;
    }

    pub async fn is_latest(
        connection: &PersistDatabase,
        revision: Option<usize>,
        target_page_id: i32,
    ) -> bool {
        let target_revision_count: i64 = connection
            .run(move |c| {
                use crate::schema::page_revision::dsl::*;
                page_revision
                    .filter(crate::schema::page_revision::dsl::page_id.eq(target_page_id))
                    .order(unix_time)
                    .count()
                    .get_result(c)
                    .expect("Error finding count of revisions")
            })
            .await;
        (revision.is_some() && revision.unwrap() == (target_revision_count - 1) as usize)
            || revision.is_none()
    }

    pub async fn delete(self, connection: &PersistDatabase) {
	connection
	    .run(move |c| {
                use crate::schema::page_revision::dsl::*;
                diesel::delete(page_revision)
                    .filter(crate::schema::page_revision::id.eq(self.id))
                    .execute(c)
                    .expect("Failed to delete page_revision.")
	    })
	    .await;
	// TODO update the search table
    }
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
            iso_time: Utc::now().format("%Y-%m-%d").to_string(),
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

        let _cloned_path = edit_path.clone();

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

    pub async fn delete(self, connection: &PersistDatabase, memory_connection: &MemoryDatabase) {
        connection
            .run(move |c| {
                use crate::schema::page::dsl::*;
                diesel::delete(page)
                    .filter(crate::schema::page::id.eq(self.id))
                    .execute(c)
                    .expect("Failed to delete page.")
            })
            .await;

        // Cascade delete should take care of children TODO and page revisions

        memory_connection
            .run(move |c| {
                let query = sql_query("DELETE FROM search WHERE id = ?");
                query
                    .bind::<Nullable<Integer>, _>(self.id)
                    .execute(c)
                    .expect("Database error");
            })
            .await;
    }

    pub async fn build_nav_element(connection: &PersistDatabase, path: &PathBuf) -> String {
        enum Padding {
            Blank,
            Bar,
        }
        let tree_source = connection
            .run(move |c| {
                use crate::schema::page::dsl::*;
                page.select((crate::schema::page::dsl::id, parent_id, slug))
                    .load::<(Option<i32>, Option<i32>, String)>(c)
                    .expect("Database error")
            })
            .await;

        // build a tree structure

        let mut tree = TreeBuilder::new().with_root("".to_owned()).build();
        let root_id = tree.root_id().expect("root doesn't exist");

        let mut tree_map = HashMap::new();

        for (ret_id, ret_parent_id, ret_slug) in tree_source {
            if ret_slug == "" {
                tree_map.insert(ret_id, root_id);
            } else if ret_parent_id == None {
                let mut root = tree.get_mut(root_id).unwrap();
                let slug_node = root.append(ret_slug);

                let slug_node_id = slug_node.node_id();
                tree_map.insert(ret_id, slug_node_id);
            } else {
                let tree_parent_id = tree_map.get(&ret_parent_id).unwrap();
                let mut tree_parent = tree.get_mut(*tree_parent_id).unwrap();

                let slug_node = tree_parent.append(ret_slug);

                let slug_node_id = slug_node.node_id();
                tree_map.insert(ret_id, slug_node_id);
            }
        }

        // traverse tree to emit nav element

        let mut nav_element = String::from("");
        let acc_path = String::from("");
        let binding = path.to_str().unwrap().to_string();
        let mut segments: Vec<&str> = binding.split('/').collect();
        segments.insert(0, ""); // root is on every path

        nav_element.push_str("<ul>");
        process_node(
            &tree,
            root_id,
            root_id,
            &mut nav_element,
            acc_path,
            &mut segments,
            false,
            &mut Vec::new(),
        );
        nav_element.push_str("</ul>");

        fn process_node(
            tree: &Tree<String>,
            current_node_id: NodeId,
            root_id: NodeId,
            nav_element: &mut String,
            acc_path: String,
            segments: &mut Vec<&str>,
            is_last_child: bool,
            prev: &mut Vec<Padding>,
        ) {
            let current_node = tree.get(current_node_id).unwrap();
            nav_element.push_str("<li>");
            if !prev.is_empty() {
                for i in 0..(prev.len() - 1) {
                    match prev[i] {
                        Padding::Blank => nav_element.push_str("&nbsp;&nbsp;&nbsp;"),
                        Padding::Bar => nav_element.push_str("|&nbsp;&nbsp;"),
                    }
                }
                if is_last_child {
                    nav_element.push_str("└──");
                } else {
                    nav_element.push_str("├──");
                }
            }

            let children: Vec<NodeRef<String>> = current_node.children().collect();
            let new_seg = if children.len() != 0 {
                format!("{}/", current_node.data())
            } else {
                format!("{}", current_node.data())
            };
            let new_path = format!("{}{}", acc_path, new_seg);
            nav_element
                .push_str(format!("<a href=\"/pages{}\">{}</a>", new_path, new_seg).as_str());

            if children.len() != 0 && segments.len() != 0 && current_node.data() == segments[0] {
                segments.remove(0);
                nav_element.push_str("<ul>");

                let mut children = current_node.children().peekable();

                while let Some(child) = children.next() {
                    if children.peek().is_some() {
                        prev.push(Padding::Bar);
                        process_node(
                            tree,
                            child.node_id(),
                            root_id,
                            nav_element,
                            new_path.clone(),
                            segments,
                            false,
                            prev,
                        );
                        prev.pop();
                    } else {
                        prev.push(Padding::Blank);
                        process_node(
                            tree,
                            child.node_id(),
                            root_id,
                            nav_element,
                            new_path.clone(),
                            segments,
                            true,
                            prev,
                        );
                        prev.pop();
                    }
                }
                nav_element.push_str("</ul>");
            }
            nav_element.push_str("</li>");
        }

        nav_element
    }

    pub async fn from_path(path: &PathBuf, connection: &PersistDatabase) -> Self {
        let query = sql_query(
            r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path
             FROM page
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || slug
             FROM page p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT * FROM page WHERE id = (
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
        let binding = connection
            .run(move |c| {
                query
                    .bind::<Text, _>(path_spec)
                    .load::<Page>(c)
                    .expect("Database error finding page")
            })
            .await;
        let child = binding.first().expect("No such page found");
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

#[derive(Queryable, QueryableByName, Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    #[diesel(sql_type = Nullable<Integer>)]
    pub id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub path: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub markdown_content: String,
    #[diesel(sql_type = Text)]
    pub sidebar_markdown_content: String,
}

impl SearchResult {
    pub async fn init_memory_table(memory_connection: &MemoryDatabase) {
        memory_connection.run(move |c| {
	    let query = sql_query(
		    r#"
		    CREATE VIRTUAL TABLE IF NOT EXISTS search USING FTS5(id, path, title, markdown_content, sidebar_markdown_content)
		    "#
	    ); // HACK we do this here because diesel does not support such sqlite virtual tables, which by definition have no explicit primary key.
	    query.execute(c).expect("Database error");
	    }
	    ).await;
    }

    pub async fn populate_with_revisions(
        connection: &PersistDatabase,
        memory_connection: &MemoryDatabase,
    ) {
        let query = sql_query(
            r#"
             WITH RECURSIVE CTE AS (
             SELECT id, slug AS path, title--, markdown_content, sidebar_markdown_content
             FROM page
             WHERE parent_id IS NULL
             UNION ALL
             SELECT p.id, path || '/' || p.slug, p.title--, p.markdown_content, p.sidebar_markdown_content
             FROM page p
             JOIN CTE ON p.parent_id = CTE.id
           )
           SELECT * FROM CTE
           LEFT JOIN page_revision
-- https://stackoverflow.com/questions/725153/most-recent-record-in-a-left-join
           ON CTE.id = page_revision.page_id
           AND page_revision.unix_time = (SELECT MAX(unix_time) FROM page_revision z WHERE z.page_id = page_revision.page_id);
"#,
        );

        let binding = connection
            .run(move |c| query.load::<SearchResult>(c).expect("Database error"))
            .await;

        for searchable_page in binding {
            memory_connection.run(
                        move |c| {
                                let query = sql_query(
                                        r#"
                                        INSERT INTO search (id, path, title, markdown_content, sidebar_markdown_content) VALUES (?, ?, ?, ?, ?)
                                        "#
                                );
                                let binding = query.bind::<Integer, _>(searchable_page.id.unwrap())
                                    .bind::<Text, _>(searchable_page.path)
                                        .bind::<Text, _>(searchable_page.title)
                                        .bind::<Text, _>(searchable_page.markdown_content)
                                        .bind::<Text, _>(searchable_page.sidebar_markdown_content);
                                binding.execute(c).expect("Database error");
                        }
                ).await;
        }
    }

    pub async fn run_search(memory_connection: &MemoryDatabase, query: String) -> Vec<Self> {
        let search_results = sql_query(
            r#"SELECT id, path, snippet(search, 2, '<span class="highlight">', '</span>', '...', 64) AS "title", snippet(search, 3, '<span class="highlight">', '</span>', '...', 64) AS "markdown_content", snippet(search, 4, '<span class="highlight">', '</span>', '...', 64) AS "sidebar_markdown_content" FROM search WHERE search MATCH '{title markdown_content sidebar_markdown_content}: ' || ? "#,
        );

        memory_connection
            .run(move |c| {
                search_results
                    .bind::<Text, _>(query)
                    .load::<SearchResult>(c)
                    .expect("Database error")
            })
            .await
    }
}
